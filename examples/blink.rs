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

    defmt::info!("Daisy Pod Blink Example");
    defmt::info!("LED will blink at 300ms intervals");

    // Blink the LED forever
    let mut led = board.user_led;
    loop {
        led.set_high();
        defmt::trace!("LED ON");
        Timer::after_millis(300).await;

        led.set_low();
        defmt::trace!("LED OFF");
        Timer::after_millis(300).await;
    }
}
