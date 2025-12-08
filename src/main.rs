#![no_std]
#![no_main]

use daisy_embassy::new_daisy_board;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_time::Timer;
use panic_probe as _;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // Initialize with default configuration for Daisy board
    let config = daisy_embassy::default_rcc();
    let p = embassy_stm32::init(config);

    // Create Daisy board instance (handles all peripheral initialization)
    let board = new_daisy_board!(p);

    defmt::info!("=================================");
    defmt::info!("  Delay of Game - Audio Delay   ");
    defmt::info!("  Broncos Radio Sync System     ");
    defmt::info!("=================================");

    // Startup LED sequence - quick blinks to show we're alive
    let mut led = board.user_led;
    defmt::info!("Starting up...");
    for _ in 0..6 {
        led.set_high();
        Timer::after_millis(100).await;
        led.set_low();
        Timer::after_millis(100).await;
    }

    // TODO Phase 2: Initialize audio codec
    // TODO Phase 2: Configure SAI for audio I/O
    // TODO Phase 3: Initialize SDRAM for delay buffer
    // TODO Phase 4: Initialize ADC for Knob 1 (delay control)
    // TODO Phase 4: Set up delay adjustment logic

    defmt::info!("Initialization complete");

    // Keep LED on to indicate "ready" state
    led.set_high();

    // Main loop - placeholder until audio processing is implemented
    loop {
        Timer::after_millis(1000).await;
        defmt::debug!("Heartbeat - waiting for audio implementation");
    }
}
