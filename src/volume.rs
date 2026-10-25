//! Output volume: knob-to-gain mapping and click-free gain application
//!
//! Gain is unsigned Q16 fixed point: UNITY (65536) = 1.0. The knob only
//! attenuates; it never boosts, so it can't clip a full-scale input.

pub const UNITY: u32 = 1 << 16;

/// Largest gain change per block (~43 ms for a full sweep at 0.67 ms/block),
/// so fast knob turns ramp instead of producing zipper noise
const MAX_STEP_PER_BLOCK: u32 = UNITY / 64;

/// Map a smoothed knob reading (0..=full_scale) to gain using a squared
/// (audio-taper) curve, with a dead zone at each end so the knob reaches
/// true mute and true unity
pub fn knob_to_gain(reading: u32, full_scale: u32) -> u32 {
    let dead_zone = full_scale / 50; // 2%
    if reading <= dead_zone {
        return 0;
    }
    if reading >= full_scale - dead_zone {
        return UNITY;
    }
    let x =
        ((reading - dead_zone) as u64 * UNITY as u64 / (full_scale - 2 * dead_zone) as u64) as u32;
    ((x as u64 * x as u64) >> 16) as u32
}

/// Applies a target gain to audio blocks, ramping toward it across blocks
pub struct GainRamp {
    current: u32,
}

impl GainRamp {
    pub fn new(initial: u32) -> Self {
        Self { current: initial }
    }

    /// Scale an interleaved block of 24-bit samples in place, interpolating
    /// from the current gain to the next step toward `target`
    pub fn process(&mut self, block: &mut [u32], target: u32) {
        let start = self.current;
        let end = if target > start {
            start + (target - start).min(MAX_STEP_PER_BLOCK)
        } else {
            start - (start - target).min(MAX_STEP_PER_BLOCK)
        };
        self.current = end;

        if start == UNITY && end == UNITY {
            return;
        }

        let len = block.len() as i64;
        let delta = end as i64 - start as i64;
        for (i, sample) in block.iter_mut().enumerate() {
            let gain = start as i64 + delta * i as i64 / len;
            // SAI delivers 24-bit samples in the low bits; sign-extend before scaling
            let s = ((*sample << 8) as i32 >> 8) as i64;
            *sample = (((s * gain) >> 16) as u32) & 0x00FF_FFFF;
        }
    }
}
