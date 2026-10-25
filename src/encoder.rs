//! Quadrature decoder for the Daisy Pod's rotary encoder
//!
//! Hardware connections:
//! - Encoder A: PD11 (Seed pin D26)
//! - Encoder B: PA0 (Seed pin D25)
//! - Push button: PB6 (Seed pin D13), read directly in main.rs

use embassy_stm32::gpio::Input;

/// Rotary encoder state machine for quadrature decoding
pub struct RotaryEncoder {
    pin_a: Input<'static>,
    pin_b: Input<'static>,
    last_a: bool,
    last_b: bool,
}

impl RotaryEncoder {
    /// Create new encoder from GPIO pins
    /// Pins should be configured with pull-ups enabled
    pub fn new(pin_a: Input<'static>, pin_b: Input<'static>) -> Self {
        let last_a = pin_a.is_high();
        let last_b = pin_b.is_high();

        Self {
            pin_a,
            pin_b,
            last_a,
            last_b,
        }
    }

    /// Update encoder state and return increment (+1, -1, or 0)
    /// Call this frequently (e.g., in a fast polling loop or from interrupts)
    ///
    /// Uses standard quadrature decoding:
    /// - CW rotation: A leads B (A falls while B is low)
    /// - CCW rotation: B leads A (A falls while B is high)
    pub fn update(&mut self) -> i32 {
        let a = self.pin_a.is_high();
        let b = self.pin_b.is_high();

        let mut increment = 0;

        // Detect falling edge on A
        if self.last_a && !a {
            // If B is low, clockwise rotation
            if !b {
                increment = 1;
            }
            // If B is high, counter-clockwise rotation
            else {
                increment = -1;
            }
        }
        // Detect falling edge on B
        else if self.last_b && !b {
            // If A is low, counter-clockwise rotation
            if !a {
                increment = -1;
            }
            // If A is high, clockwise rotation
            else {
                increment = 1;
            }
        }

        self.last_a = a;
        self.last_b = b;

        increment
    }
}
