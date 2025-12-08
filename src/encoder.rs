//! Rotary encoder with button for Daisy Pod
//!
//! Implements quadrature decoding for the Daisy Pod's rotary encoder.
//! Hardware connections:
//! - Encoder A: PD11 (Seed pin D26)
//! - Encoder B: PA0 (Seed pin D25)
//! - Button: PB6 (Seed pin D13)

use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::Input;

/// Rotary encoder state machine for quadrature decoding
pub struct RotaryEncoder {
    pin_a: Input<'static>,
    pin_b: Input<'static>,
    last_a: bool,
    last_b: bool,
    position: i32,
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
            position: 0,
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
                self.position += 1;
            }
            // If B is high, counter-clockwise rotation
            else {
                increment = -1;
                self.position -= 1;
            }
        }
        // Detect falling edge on B
        else if self.last_b && !b {
            // If A is low, counter-clockwise rotation
            if !a {
                increment = -1;
                self.position -= 1;
            }
            // If A is high, clockwise rotation
            else {
                increment = 1;
                self.position += 1;
            }
        }

        self.last_a = a;
        self.last_b = b;

        increment
    }

    /// Get current encoder position (total rotation count)
    pub fn position(&self) -> i32 {
        self.position
    }

    /// Reset position to zero
    pub fn reset(&mut self) {
        self.position = 0;
    }
}

/// Complete encoder with button
pub struct Encoder {
    pub encoder: RotaryEncoder,
    pub button: ExtiInput<'static>,
}

impl Encoder {
    /// Create new encoder with button
    pub fn new(
        pin_a: Input<'static>,
        pin_b: Input<'static>,
        button: ExtiInput<'static>,
    ) -> Self {
        Self {
            encoder: RotaryEncoder::new(pin_a, pin_b),
            button,
        }
    }
}
