//! Circular buffer for audio delay
//!
//! Stores interleaved stereo samples (L, R, L, R, ...) in external SDRAM.
//! "Samples" here means individual u32 words, so one second of audio is
//! SAMPLE_RATE * CHANNELS samples.

use daisy_embassy::audio::HALF_DMA_BUFFER_LENGTH;
use daisy_embassy::sdram::SDRAM_SIZE;

/// Maximum delay time in seconds - the single source of truth for max delay.
/// 64MB SDRAM holds ~174 seconds of 32-bit stereo at 48kHz.
pub const MAX_DELAY_SECONDS: usize = 170;

pub const SAMPLE_RATE: usize = 48_000;

pub const CHANNELS: usize = 2;

/// Total buffer length in samples: 48kHz × 170s × 2 channels = 16,320,000 (65.3 MB of the 67.1 MB SDRAM)
pub const BUFFER_LENGTH: usize = SAMPLE_RATE * MAX_DELAY_SECONDS * CHANNELS;

/// Samples per audio callback (32 frames × 2 channels, interleaved)
pub const BLOCK_LENGTH: usize = HALF_DMA_BUFFER_LENGTH;

const _: () = assert!(
    BUFFER_LENGTH * core::mem::size_of::<u32>() <= SDRAM_SIZE,
    "MAX_DELAY_SECONDS does not fit in SDRAM"
);

pub struct DelayBuffer<'a> {
    buffer: &'a mut [u32],
    write_pos: usize,
    read_pos: usize,
    delay_samples: usize,
}

impl<'a> DelayBuffer<'a> {
    /// Create a new delay buffer from an SDRAM slice, filled with silence
    ///
    /// # Panics
    /// If the slice is shorter than BUFFER_LENGTH
    pub fn new(buffer: &'a mut [u32]) -> Self {
        assert!(buffer.len() >= BUFFER_LENGTH, "Buffer too small for delay");

        // Samples are signed (two's complement), so silence is zero
        buffer[..BUFFER_LENGTH].fill(0);

        Self {
            buffer,
            write_pos: 0,
            read_pos: 0,
            delay_samples: 0,
        }
    }

    /// Set delay in samples (interleaved, so 2 per frame)
    ///
    /// Clamped to leave room for one block. The read pointer jumps immediately,
    /// which can produce an audible click when the delay changes.
    pub fn set_delay_samples(&mut self, samples: usize) {
        let max_delay = BUFFER_LENGTH.saturating_sub(BLOCK_LENGTH);
        self.delay_samples = samples.min(max_delay);

        self.read_pos = if self.write_pos >= self.delay_samples {
            self.write_pos - self.delay_samples
        } else {
            BUFFER_LENGTH - (self.delay_samples - self.write_pos)
        };
    }

    /// Process one block of audio (called from audio callback):
    /// write input to the buffer and read delayed output
    pub fn process(&mut self, input: &[u32], output: &mut [u32]) {
        assert_eq!(input.len(), BLOCK_LENGTH, "Input block wrong size");
        assert_eq!(output.len(), BLOCK_LENGTH, "Output block wrong size");

        self.write_block(input);
        self.read_block(output);

        self.write_pos = (self.write_pos + BLOCK_LENGTH) % BUFFER_LENGTH;
        self.read_pos = (self.read_pos + BLOCK_LENGTH) % BUFFER_LENGTH;
    }

    /// Write a block to the buffer, handling wrap-around
    fn write_block(&mut self, input: &[u32]) {
        let remain = BLOCK_LENGTH.min(BUFFER_LENGTH - self.write_pos);
        let frac = BLOCK_LENGTH - remain;

        self.buffer[self.write_pos..(self.write_pos + remain)].copy_from_slice(&input[..remain]);
        if frac > 0 {
            self.buffer[0..frac].copy_from_slice(&input[remain..]);
        }
    }

    /// Read a block from the buffer, handling wrap-around
    fn read_block(&self, output: &mut [u32]) {
        let remain = BLOCK_LENGTH.min(BUFFER_LENGTH - self.read_pos);
        let frac = BLOCK_LENGTH - remain;

        output[..remain].copy_from_slice(&self.buffer[self.read_pos..(self.read_pos + remain)]);
        if frac > 0 {
            output[remain..].copy_from_slice(&self.buffer[0..frac]);
        }
    }
}
