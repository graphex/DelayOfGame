#![no_std]
#![no_main]

mod buffer;
mod display;
mod encoder;
mod settings;
mod volume;

use buffer::{DelayBuffer, CHANNELS, MAX_DELAY_SECONDS, SAMPLE_RATE};
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};
use daisy_embassy::new_daisy_board;
use daisy_embassy::sdram::SDRAM_SIZE;
use defmt_rtt as _;
use display::Display;
use embassy_executor::Spawner;
use embassy_stm32::adc::{Adc, SampleTime};
use embassy_stm32::bind_interrupts;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::{Input, Pull};
use embassy_stm32::i2c::{self, I2c};
use embassy_stm32::peripherals::{ADC1, I2C1, PC0, PC4};
use embassy_stm32::time::Hertz;
use embassy_time::{Delay, Duration, Instant, Timer};
use encoder::{DebouncedButton, RotaryEncoder};
use panic_probe as _;
use settings::Settings;
use volume::GainRamp;

/// Frames (one sample per channel) per second
const FRAMES_PER_SECOND: u32 = SAMPLE_RATE as u32;
const DEFAULT_DELAY_FRAMES: u32 = FRAMES_PER_SECOND; // 1 second, until one is saved
/// How long the delay must stay unchanged before it's saved to flash
const SAVE_AFTER: Duration = Duration::from_secs(3);
const MAX_DELAY_FRAMES: u32 = FRAMES_PER_SECOND * MAX_DELAY_SECONDS as u32;
const COARSE_STEP_FRAMES: u32 = FRAMES_PER_SECOND / 10; // 0.1s
const FINE_STEP_FRAMES: u32 = FRAMES_PER_SECOND / 100; // 0.01s
const JUMP_FRAMES: u32 = FRAMES_PER_SECOND * 10; // Pod buttons: -/+ 10s

/// Full-scale ADC reading at the default 16-bit resolution
const KNOB_FULL_SCALE: u32 = u16::MAX as u32;

/// Delay in frames, shared between the encoder task and the audio callback.
/// The delay buffer stores interleaved stereo, so the callback converts
/// frames to buffer samples by multiplying by CHANNELS.
static DELAY_FRAMES: AtomicU32 = AtomicU32::new(DEFAULT_DELAY_FRAMES);

/// Output gain in Q16 (volume::UNITY = 1.0), set by Knob 1. Starts at 0 so
/// audio fades in to the knob's level at power-up instead of popping.
static VOLUME_GAIN: AtomicU32 = AtomicU32::new(0);

/// True while the encoder steps by 0.01s instead of 0.1s
static STEP_FINE: AtomicBool = AtomicBool::new(false);

/// Scrolled across the display once at startup
const INTRO_MESSAGE: &[u8] = b"Delay of game v2.1";
const INTRO_STEP_MS: u64 = 200;

/// Display brightness level (0-15), set by Knob 2
static BRIGHTNESS: AtomicU8 = AtomicU8::new(display::MAX_BRIGHTNESS);

/// I2C transmit buffer for the display. DMA1 can't reach DTCM, so it lives
/// in D2 SRAM like daisy-embassy's audio buffers.
#[link_section = ".sram1_bss"]
static mut DISPLAY_TX: [u8; 1 + 2 * display::DIGITS] = [0; 1 + 2 * display::DIGITS];

bind_interrupts!(struct Irqs {
    I2C1_EV => i2c::EventInterruptHandler<I2C1>;
    I2C1_ER => i2c::ErrorInterruptHandler<I2C1>;
});

fn frames_to_hundredths(frames: u32) -> u32 {
    frames * 100 / FRAMES_PER_SECOND
}

/// Log a frame count as seconds with two decimal places
fn log_seconds(label: &str, frames: u32) {
    let hundredths = frames_to_hundredths(frames);
    defmt::info!("{}: {}.{:02}s", label, hundredths / 100, hundredths % 100);
}

/// Encoder turns step the delay; Pod buttons SW1/SW2 jump it by -/+10s
#[embassy_executor::task]
async fn delay_controls(
    mut encoder: RotaryEncoder,
    button: ExtiInput<'static>,
    mut minus: DebouncedButton,
    mut plus: DebouncedButton,
) {
    let mut delay_frames = DELAY_FRAMES.load(Ordering::Relaxed);
    let mut step_frames = COARSE_STEP_FRAMES;

    log_seconds("Delay", delay_frames);
    log_seconds("Step size", step_frames);

    loop {
        // 1kHz polling is fast enough to catch every encoder transition
        Timer::after_millis(1).await;

        // Poll every input each tick so the button debouncers keep counting
        let turn = encoder.update();
        let minus_pressed = minus.just_pressed();
        let plus_pressed = plus.just_pressed();
        let change = match turn {
            1 => Some((true, step_frames)),
            -1 => Some((false, step_frames)),
            _ if minus_pressed => Some((false, JUMP_FRAMES)),
            _ if plus_pressed => Some((true, JUMP_FRAMES)),
            _ => None,
        };
        if let Some((up, frames)) = change {
            delay_frames = if up {
                delay_frames.saturating_add(frames).min(MAX_DELAY_FRAMES)
            } else {
                delay_frames.saturating_sub(frames)
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
            STEP_FINE.store(step_frames == FINE_STEP_FRAMES, Ordering::Relaxed);
            log_seconds("Step size", step_frames);

            // Wait for release, then let contacts settle so bounce isn't a second press
            while button.is_low() {
                Timer::after_millis(10).await;
            }
            Timer::after_millis(20).await;
        }
    }
}

/// Reads Knob 1 (output volume) and Knob 2 (display brightness)
#[embassy_executor::task]
async fn knob_reader(mut adc: Adc<'static, ADC1>, mut knob1: PC4, mut knob2: PC0) {
    // Exponential moving averages, stored scaled by 8 to keep precision
    let mut volume_x8 = adc.blocking_read(&mut knob1) as u32 * 8;
    let mut brightness_x8 = adc.blocking_read(&mut knob2) as u32 * 8;
    let mut brightness = display::knob_to_brightness(brightness_x8 / 8, KNOB_FULL_SCALE, 0);
    BRIGHTNESS.store(brightness, Ordering::Relaxed);
    let mut last_percent = u32::MAX;

    loop {
        let raw = adc.blocking_read(&mut knob1) as u32;
        volume_x8 = volume_x8 - volume_x8 / 8 + raw;
        let raw = adc.blocking_read(&mut knob2) as u32;
        brightness_x8 = brightness_x8 - brightness_x8 / 8 + raw;

        let level = display::knob_to_brightness(brightness_x8 / 8, KNOB_FULL_SCALE, brightness);
        if level != brightness {
            brightness = level;
            BRIGHTNESS.store(brightness, Ordering::Relaxed);
            defmt::info!("Brightness: {}", brightness);
        }

        let gain = volume::knob_to_gain(volume_x8 / 8, KNOB_FULL_SCALE);
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

/// Shows the delay on the HT16K33 backpack, redrawing only when it changes.
/// Pressing the encoder shows FINE or CRSE for a second. Brightness follows
/// Knob 2. The first time the backpack is found, INTRO_MESSAGE scrolls past;
/// changing the delay cuts it short.
#[embassy_executor::task]
async fn display_writer(mut display: Display) {
    let mut connected = false;
    let mut warned = false;
    let mut shown = None;
    let mut intro_played = false;
    let mut brightness = 0;
    let mut last_delay = DELAY_FRAMES.load(Ordering::Relaxed);
    let mut fine = STEP_FINE.load(Ordering::Relaxed);
    let mut banner_until = Instant::MIN;

    loop {
        if !connected {
            brightness = BRIGHTNESS.load(Ordering::Relaxed);
            match display.init(brightness).await {
                Ok(address) => defmt::info!("Display connected at {:#x}", address),
                Err(_) => {
                    if !warned {
                        defmt::warn!("Display not found at 0x70-0x77; retrying");
                        warned = true;
                    }
                    Timer::after_secs(1).await;
                    continue;
                }
            }
            connected = true;
            warned = false;
            shown = None;

            if !intro_played {
                intro_played = true;
                let start_delay = DELAY_FRAMES.load(Ordering::Relaxed);
                for step in 0..display::scroll_steps(INTRO_MESSAGE) {
                    if DELAY_FRAMES.load(Ordering::Relaxed) != start_delay {
                        break;
                    }
                    let frame = display::scroll_frame(INTRO_MESSAGE, step);
                    if display.show(&frame).await.is_err() {
                        connected = false;
                        break;
                    }
                    Timer::after_millis(INTRO_STEP_MS).await;
                }
                if !connected {
                    continue;
                }
            }
        }

        let level = BRIGHTNESS.load(Ordering::Relaxed);
        if level != brightness {
            if display.set_brightness(level).await.is_err() {
                defmt::warn!("Display write failed");
                connected = false;
                continue;
            }
            brightness = level;
        }

        let delay = DELAY_FRAMES.load(Ordering::Relaxed);
        let now_fine = STEP_FINE.load(Ordering::Relaxed);
        if now_fine != fine {
            fine = now_fine;
            banner_until = Instant::now() + Duration::from_secs(1);
        }
        if delay != last_delay {
            // Turning the encoder cuts the step banner short
            last_delay = delay;
            banner_until = Instant::MIN;
        }

        let segments = if Instant::now() < banner_until {
            display::text(if fine { b"FINE" } else { b"CRSE" })
        } else {
            display::seconds(frames_to_hundredths(delay))
        };
        if shown != Some(segments) {
            if display.show(&segments).await.is_err() {
                defmt::warn!("Display write failed");
                connected = false;
                continue;
            }
            shown = Some(segments);
        }

        Timer::after_millis(20).await;
    }
}

/// Saves the delay to flash once it has stopped changing, so it's restored
/// at the next power-up
#[embassy_executor::task]
async fn delay_saver(mut settings: Settings, mut saved: u32) {
    let mut last_seen = saved;
    let mut changed_at = Instant::now();
    loop {
        Timer::after_millis(250).await;
        let delay = DELAY_FRAMES.load(Ordering::Relaxed);
        if delay != last_seen {
            last_seen = delay;
            changed_at = Instant::now();
        } else if delay != saved && changed_at.elapsed() >= SAVE_AFTER {
            settings.save(delay).await;
            saved = delay;
            log_seconds("Saved delay", delay);
        }
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

    let (settings, saved_delay) = Settings::load(board.flash);
    let initial_delay = match saved_delay {
        Some(frames) if frames <= MAX_DELAY_FRAMES => {
            log_seconds("Restored delay", frames);
            frames
        }
        _ => DEFAULT_DELAY_FRAMES,
    };
    DELAY_FRAMES.store(initial_delay, Ordering::Relaxed);

    let mut delay_buffer = DelayBuffer::new(ram_slice);
    delay_buffer.set_delay_samples(initial_delay as usize * CHANNELS);

    // Pod encoder: A = D26 (PD11), B = D25 (PA0), push button = D13 (PB6)
    let enc_a = Input::new(board.pins.d26, Pull::Up);
    let enc_b = Input::new(board.pins.d25, Pull::Up);
    let enc_button = ExtiInput::new(board.pins.d13, p.EXTI6, Pull::Up);
    let encoder = RotaryEncoder::new(enc_a, enc_b);

    // Pod buttons: SW1 = D27 (PG9) for -10s, SW2 = D28 (PA2) for +10s
    let minus = DebouncedButton::new(Input::new(board.pins.d27, Pull::Up));
    let plus = DebouncedButton::new(Input::new(board.pins.d28, Pull::Up));

    // Pod Knob 1 = D21 (PC4) sets output volume, Knob 2 = D15 (PC0) brightness
    let mut adc = Adc::new(p.ADC1);
    adc.set_sample_time(SampleTime::CYCLES64_5);

    // HT16K33 backpack: SCL = D11 (PB8), SDA = D12 (PB9). The internal
    // pull-ups keep the bus idle-high when no backpack is plugged in.
    let mut i2c_config = i2c::Config::default();
    i2c_config.scl_pullup = true;
    i2c_config.sda_pullup = true;
    // A stuck bus busy-waits inside the driver, so give up quickly
    i2c_config.timeout = Duration::from_millis(2);
    let i2c = I2c::new(
        p.I2C1,
        board.pins.d11,
        board.pins.d12,
        Irqs,
        p.DMA1_CH3,
        p.DMA1_CH4,
        Hertz::khz(400),
        i2c_config,
    );
    // SAFETY: the only reference to DISPLAY_TX, handed to the display task
    let display_tx = unsafe { &mut *core::ptr::addr_of_mut!(DISPLAY_TX) };
    let display = Display::new(i2c, display_tx);

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

    spawner
        .spawn(delay_controls(encoder, enc_button, minus, plus))
        .ok();
    spawner
        .spawn(knob_reader(adc, board.pins.d21, board.pins.d15))
        .ok();
    spawner.spawn(display_writer(display)).ok();
    spawner.spawn(delay_saver(settings, initial_delay)).ok();
    spawner.spawn(led_blinker(led)).ok();

    defmt::info!(
        "Max delay {}s. Encoder: turn to adjust, press for 0.1s/0.01s steps. SW1/SW2: -/+10s. Knob 1: volume. Knob 2: brightness",
        MAX_DELAY_SECONDS
    );

    // Runs once per 32-frame block (~0.67ms at 48kHz). Everything else on this
    // executor must yield quickly or the SAI will overrun.
    let mut last_delay = initial_delay;
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
