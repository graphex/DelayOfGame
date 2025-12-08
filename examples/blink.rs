#![no_std]
#![no_main]

use panic_halt as _;

use cortex_m_rt::entry;
use stm32h7xx_hal::{pac, prelude::*};

#[entry]
fn main() -> ! {
    // Get access to device peripherals
    let dp = pac::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();

    // Constrain and freeze power configuration
    let pwr = dp.PWR.constrain();
    let pwrcfg = pwr.freeze();

    // Constrain and freeze clock configuration
    // Configure for 400 MHz (max speed for STM32H750)
    let rcc = dp.RCC.constrain();
    let ccdr = rcc
        .sys_ck(400.MHz())
        .freeze(pwrcfg, &dp.SYSCFG);

    // Get delay provider
    let mut delay = cp.SYST.delay(ccdr.clocks);

    // Configure the user LED pin (PC7 on Daisy Seed/Pod)
    let gpioc = dp.GPIOC.split(ccdr.peripheral.GPIOC);
    let mut user_led = gpioc.pc7.into_push_pull_output();

    // Blink the LED forever
    loop {
        user_led.set_high();
        delay.delay_ms(500_u32);

        user_led.set_low();
        delay.delay_ms(500_u32);
    }
}
