#![no_std]
#![no_main]

mod buffer;
mod encoder;
mod volume;

use buffer::{DelayBuffer, CHANNELS, MAX_DELAY_SECONDS, SAMPLE_RATE};
use core::sync::atomic::{AtomicU32, Ordering};
use daisy_embassy::new_daisy_board;
use daisy_embassy::sdram::SDRAM_SIZE;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::adc::{Adc, SampleTime};
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::{Input, Pull};
use embassy_stm32::peripherals::{ADC1, PC4};
use embassy_time::{Delay, Timer};
use encoder::RotaryEncoder;
use panic_probe as _;
use volume::GainRamp;

/// Frames (one sample per channel) per second
const FRAMES_PER_SECOND: u32 = SAMPLE_RATE as u32;
const INITIAL_DELAY_FRAMES: u32 = FRAMES_PER_SECOND; // 1 second
const MAX_DELAY_FRAMES: u32 = FRAMES_PER_SECOND * MAX_DELAY_SECONDS as u32;
const COARSE_STEP_FRAMES: u32 = FRAMES_PER_SECOND / 10; // 0.1s
const FINE_STEP_FRAMES: u32 = FRAMES_PER_SECOND / 100; // 0.01s

/// Full-scale ADC reading at the default 16-bit resolution
const KNOB_FULL_SCALE: u32 = u16::MAX as u32;

/// Delay in frames, shared between the encoder task and the audio callback.
/// The delay buffer stores interleaved stereo, so the callback converts
/// frames to buffer samples by multiplying by CHANNELS.
static DELAY_FRAMES: AtomicU32 = AtomicU32::new(INITIAL_DELAY_FRAMES);

/// Output gain in Q16 (volume::UNITY = 1.0), set by Knob 1. Starts at 0 so
/// audio fades in to the knob's level at power-up instead of popping.
static VOLUME_GAIN: AtomicU32 = AtomicU32::new(0);

/// Log a frame count as seconds with two decimal places
fn log_seconds(label: &str, frames: u32) {
    let hundredths = frames * 100 / FRAMES_PER_SECOND;
    defmt::info!("{}: {}.{:02}s", label, hundredths / 100, hundredths % 100);
}

#[embassy_executor::task]
async fn encoder_reader(mut encoder: RotaryEncoder, button: ExtiInput<'static>) {
    let mut delay_frames = INITIAL_DELAY_FRAMES;
    let mut step_frames = COARSE_STEP_FRAMES;

    log_seconds("Delay", delay_frames);
    log_seconds("Step size", step_frames);

    loop {
        // 1kHz polling is fast enough to catch every encoder transition
        Timer::after_millis(1).await;

        let increment = encoder.update();
        if increment != 0 {
            delay_frames = if increment > 0 {
                delay_frames
                    .saturating_add(step_frames)
                    .min(MAX_DELAY_FRAMES)
            } else {
                delay_frames.saturating_sub(step_frames)
            };
            DELAY_FRAMES.store(delay_frames, Ordering::Relaxed);
            log_seconds("Delay", delay_frames);
        }

        if button.is_low() {
            step_frames = if step_frames == COARSE_STEP_FRAMES {
                FINE_STEP_FRAMES
            } else {
                COARSE_STEP_FRAMES
            };
            log_seconds("Step size", step_frames);

            // Wait for release, then let contacts settle so bounce isn't a second press
            while button.is_low() {
                Timer::after_millis(10).await;
            }
            Timer::after_millis(20).await;
        }
    }
}

/// Reads Knob 1 and publishes the output volume
#[embassy_executor::task]
async fn volume_reader(mut adc: Adc<'static, ADC1>, mut knob: PC4) {
    // Exponential moving average, stored scaled by 8 to keep precision
    let mut smoothed_x8 = adc.blocking_read(&mut knob) as u32 * 8;
    let mut last_percent = u32::MAX;

    loop {
        let raw = adc.blocking_read(&mut knob) as u32;
        smoothed_x8 = smoothed_x8 - smoothed_x8 / 8 + raw;

        let gain = volume::knob_to_gain(smoothed_x8 / 8, KNOB_FULL_SCALE);
        VOLUME_GAIN.store(gain, Ordering::Relaxed);

        // Log only meaningful changes so ADC noise doesn't flood the console
        let percent = gain * 100 / volume::UNITY;
        if percent.abs_diff(last_percent) >= 2
            || (percent != last_percent && (percent == 0 || percent == 100))
        {
            defmt::info!("Volume: {}%", percent);
            last_percent = percent;
        }

        Timer::after_millis(10).await;
    }
}

/// Heartbeat LED shows the firmware is running
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
    defmt::info!("Delay of Game - Broncos radio sync");

    let mut config = daisy_embassy::default_rcc();
    // Clock the ADC from PLL3_R (~24.6MHz), which default_rcc already enables
    config.rcc.mux.adcsel = embassy_stm32::pac::rcc::vals::Adcsel::PLL3_R;
    let p = embassy_stm32::init(config);

    // Keep the debug port alive while the executor sleeps between audio blocks,
    // so an ST-Link can attach (flash, RTT logs) without connect-under-reset
    embassy_stm32::pac::DBGMCU.cr().modify(|w| {
        w.set_dbgsleep_d1(true);
        w.set_dbgstop_d1(true);
        w.set_dbgstby_d1(true);
        w.set_d1dbgcken(true);
        w.set_d3dbgcken(true);
    });

    // Cortex-M peripherals are needed to configure the MPU/cache for SDRAM
    let mut c = cortex_m::Peripherals::take().unwrap();

    let board = new_daisy_board!(p);

    // Quick startup blinks
    let mut led = board.user_led;
    for _ in 0..6 {
        led.on();
        Timer::after_millis(100).await;
        led.off();
        Timer::after_millis(100).await;
    }

    let mut sdram = board.sdram.build(&mut c.MPU, &mut c.SCB);
    let ram_slice = unsafe {
        let ram_ptr: *mut u32 = sdram.init(&mut Delay) as *mut _;
        core::slice::from_raw_parts_mut(ram_ptr, SDRAM_SIZE / core::mem::size_of::<u32>())
    };
    defmt::info!("SDRAM initialized: {} words", ram_slice.len());

    let mut delay_buffer = DelayBuffer::new(ram_slice);
    delay_buffer.set_delay_samples(INITIAL_DELAY_FRAMES as usize * CHANNELS);

    // Pod encoder: A = D26 (PD11), B = D25 (PA0), push button = D13 (PB6)
    let enc_a = Input::new(board.pins.d26, Pull::Up);
    let enc_b = Input::new(board.pins.d25, Pull::Up);
    let enc_button = ExtiInput::new(board.pins.d13, p.EXTI6, Pull::Up);
    let encoder = RotaryEncoder::new(enc_a, enc_b);

    // Pod Knob 1 = D21 (PC4) sets output volume
    let mut adc = Adc::new(p.ADC1);
    adc.set_sample_time(SampleTime::CYCLES64_5);

    // Codec (WM8731 or PCM3060) is selected at compile time by feature flag
    let interface = board
        .audio_peripherals
        .prepare_interface(Default::default())
        .await;
    let mut interface = match interface.start_interface().await {
        Ok(i) => i,
        Err(e) => {
            defmt::error!("Failed to start audio interface: {:?}", e);
            panic!("Audio initialization failed");
        }
    };
    defmt::info!("Audio interface started");

    spawner.spawn(encoder_reader(encoder, enc_button)).ok();
    spawner.spawn(volume_reader(adc, board.pins.d21)).ok();
    spawner.spawn(led_blinker(led)).ok();

    defmt::info!(
        "Max delay {}s. Encoder: turn to adjust, press to toggle 0.1s/0.01s steps. Knob 1: volume",
        MAX_DELAY_SECONDS
    );

    // Runs once per 32-frame block (~0.67ms at 48kHz). Everything else on this
    // executor must yield quickly or the SAI will overrun.
    let mut last_delay = INITIAL_DELAY_FRAMES;
    let mut gain = GainRamp::new(0);
    let mut callback = move |input: &[u32], output: &mut [u32]| {
        let current_delay = DELAY_FRAMES.load(Ordering::Relaxed);
        if current_delay != last_delay {
            delay_buffer.set_delay_samples(current_delay as usize * CHANNELS);
            last_delay = current_delay;
        }
        delay_buffer.process(input, output);
        gain.process(output, VOLUME_GAIN.load(Ordering::Relaxed));
    };

    // start_callback only returns on error (its Ok type is Infallible). An
    // overrun (e.g. the core was halted by a debugger) resets the DMA ring
    // buffers, so resume instead of going silent until a power cycle.
    loop {
        let Err(e) = interface.start_callback(&mut callback).await;
        defmt::warn!("Audio callback error: {:?} - restarting", e);
    }
}
