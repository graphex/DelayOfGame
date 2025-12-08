#![no_std]
#![no_main]

mod buffer;
mod encoder;

use buffer::DelayBuffer;
use core::sync::atomic::{AtomicU32, Ordering};
use daisy_embassy::new_daisy_board;
use daisy_embassy::sdram::SDRAM_SIZE;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::{Input, Pull};
use embassy_time::{Delay, Timer};
use encoder::RotaryEncoder;
use panic_probe as _;

// Shared delay value (in samples) between encoder task and audio callback
// Using atomic for lock-free access from audio interrupt
// 48kHz sample rate: 48000 samples = 1 second
static DELAY_SAMPLES: AtomicU32 = AtomicU32::new(48000); // Start at 1 second

// Task to read encoder and adjust delay
#[embassy_executor::task]
async fn encoder_reader(mut encoder: RotaryEncoder, button: ExtiInput<'static>) {
    const SAMPLE_RATE: u32 = 48000; // 48kHz
    const MAX_DELAY_SAMPLES: u32 = SAMPLE_RATE * 10; // 10 seconds max

    let mut delay_samples = 48000_u32; // Start at 1 second
    let mut step_samples = 4800_u32; // 0.1s increments (48000 * 0.1)

    defmt::info!(
        "Encoder task started - delay: {}s, step: {}s",
        delay_samples / SAMPLE_RATE,
        step_samples / SAMPLE_RATE
    );

    loop {
        // Poll encoder at 1kHz (1ms intervals)
        // This is fast enough to catch all rotation events
        Timer::after_micros(1000).await;

        let increment = encoder.update();

        if increment != 0 {
            // Adjust delay based on encoder rotation
            if increment > 0 {
                delay_samples = delay_samples.saturating_add(step_samples).min(MAX_DELAY_SAMPLES);
            } else {
                delay_samples = delay_samples.saturating_sub(step_samples);
            }

            // Update atomic (visible to audio callback)
            DELAY_SAMPLES.store(delay_samples, Ordering::Relaxed);

            // Log in seconds with 2 decimal places
            let delay_sec = delay_samples as f32 / SAMPLE_RATE as f32;
            let whole = delay_sec as u32;
            let frac = ((delay_sec - whole as f32) * 100.0) as u32;
            defmt::info!("Delay: {}.{:02}s", whole, frac);
        }

        // Check for button press (non-blocking)
        if button.is_low() {
            // Toggle step size between 0.1s (4800) and 0.01s (480)
            step_samples = if step_samples == 4800 { 480 } else { 4800 };

            let step_sec = step_samples as f32 / SAMPLE_RATE as f32;
            defmt::info!("Step size: {}s", step_sec);

            // Wait for button release to avoid repeated toggles
            while button.is_low() {
                Timer::after_millis(10).await;
            }
        }
    }
}

// Task to blink LED while audio is running (shows sign of life)
#[embassy_executor::task]
async fn led_blinker(mut led: daisy_embassy::led::UserLed<'static>) {
    loop {
        led.on();
        Timer::after_millis(500).await;
        led.off();
        Timer::after_millis(500).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    defmt::info!("=================================");
    defmt::info!("  Delay of Game - Audio Delay   ");
    defmt::info!("  Broncos Radio Sync System     ");
    defmt::info!("=================================");

    // Initialize with default configuration for Daisy board
    let config = daisy_embassy::default_rcc();
    let p = embassy_stm32::init(config);

    // Get Cortex-M peripherals for MPU/SCB (needed for SDRAM)
    let mut c = cortex_m::Peripherals::take().unwrap();

    // Create Daisy board instance (handles all peripheral initialization)
    let board = new_daisy_board!(p);

    defmt::info!("Hardware initialized");

    // Startup LED sequence - quick blinks to show we're alive
    let mut led = board.user_led;
    defmt::info!("Starting up...");
    for _ in 0..6 {
        led.on();
        Timer::after_millis(100).await;
        led.off();
        Timer::after_millis(100).await;
    }

    defmt::info!("Initializing SDRAM...");

    // Phase 3: Initialize SDRAM for delay buffer
    let mut sdram = board.sdram.build(&mut c.MPU, &mut c.SCB);
    let mut delay_peripheral = Delay;

    // Initialize SDRAM and get pointer to memory
    let ram_slice = unsafe {
        let ram_ptr: *mut u32 = sdram.init(&mut delay_peripheral) as *mut _;
        core::slice::from_raw_parts_mut(ram_ptr, SDRAM_SIZE / core::mem::size_of::<u32>())
    };

    defmt::info!("SDRAM initialized: {} words available", ram_slice.len());

    // Create delay buffer (wraps SDRAM slice)
    let mut delay_buffer = DelayBuffer::new(ram_slice);

    // Phase 4: Set initial 1-second delay (will be controlled by encoder)
    delay_buffer.set_delay_samples(48000 * 2); // 1 second in stereo samples
    defmt::info!("Delay buffer ready: {} seconds ({} samples)",
        delay_buffer.delay_seconds(),
        delay_buffer.delay_samples()
    );

    // Phase 4: Initialize encoder pins
    // Encoder A: PD11 (d26), Encoder B: PA0 (d25), Button: PB6 (d13)
    defmt::info!("Initializing encoder...");

    let enc_a = Input::new(board.pins.d26, Pull::Up);
    let enc_b = Input::new(board.pins.d25, Pull::Up);
    let enc_button = ExtiInput::new(board.pins.d13, p.EXTI6, Pull::Up);

    let encoder = RotaryEncoder::new(enc_a, enc_b);

    defmt::info!("Encoder initialized on pins D26(PD11), D25(PA0), D13(PB6)");

    defmt::info!("Initializing audio interface...");

    // Phase 2: Initialize audio codec
    // This works with both WM8731 (seed_1_1) and PCM3060 (seed_1_2)
    // The codec selection happens at compile-time via feature flags
    let interface = board
        .audio_peripherals
        .prepare_interface(Default::default())
        .await;

    defmt::info!("Starting audio interface...");
    let mut interface = match interface.start_interface().await {
        Ok(i) => {
            defmt::info!("Audio interface started successfully");
            i
        }
        Err(e) => {
            defmt::error!("Failed to start audio interface: {:?}", e);
            panic!("Audio initialization failed");
        }
    };

    // Phase 4: Spawn encoder reader task (runs concurrently with audio)
    spawner.spawn(encoder_reader(encoder, enc_button)).ok();

    // Spawn LED blinker task (runs concurrently with audio)
    spawner.spawn(led_blinker(led)).ok();

    defmt::info!("Starting audio delay callback...");
    defmt::info!("Initial delay: {} seconds", delay_buffer.delay_seconds());
    defmt::info!("Turn encoder to adjust delay (0.1s steps)");
    defmt::info!("Press encoder button to toggle between 0.1s and 0.01s steps");

    // Phase 4: Audio delay processing with variable delay
    // This callback runs in real-time at 48kHz (32 samples per block = ~0.67ms)
    // The delay is controlled by the encoder task via DELAY_SAMPLES atomic
    let mut last_delay = DELAY_SAMPLES.load(Ordering::Relaxed);

    match interface
        .start_callback(move |input, output| {
            // Check if delay changed (non-blocking atomic read)
            let current_delay = DELAY_SAMPLES.load(Ordering::Relaxed);
            if current_delay != last_delay {
                delay_buffer.set_delay_samples(current_delay as usize);
                last_delay = current_delay;
            }

            // Process audio through delay buffer
            delay_buffer.process(input, output);
        })
        .await
    {
        Ok(_) => {
            // This should never return (callback loops forever)
            defmt::error!("Audio callback exited unexpectedly");
        }
        Err(e) => {
            defmt::error!("Audio callback error: {:?}", e);
        }
    }

    // If we get here, something went wrong
    panic!("Audio processing stopped");
}
