/// Circular buffer for audio delay
///
/// This module implements a circular buffer for delaying audio samples.
/// The buffer is stored in external SDRAM (64MB) allowing for long delays.

use daisy_embassy::audio::HALF_DMA_BUFFER_LENGTH;

/// Maximum delay time in seconds
pub const MAX_DELAY_SECONDS: usize = 10;

/// Sample rate (48kHz)
pub const SAMPLE_RATE: usize = 48_000;

/// Number of channels (stereo)
pub const CHANNELS: usize = 2;

/// Total buffer length in samples (u32 words)
/// 48kHz × 10 seconds × 2 channels = 960,000 samples = 3.84 MB
pub const BUFFER_LENGTH: usize = SAMPLE_RATE * MAX_DELAY_SECONDS * CHANNELS;

/// Size of each audio processing block (from Embassy)
pub const BLOCK_LENGTH: usize = HALF_DMA_BUFFER_LENGTH;

/// Circular delay buffer
pub struct DelayBuffer<'a> {
    /// Reference to SDRAM buffer
    buffer: &'a mut [u32],

    /// Write pointer (where new audio is written)
    write_pos: usize,

    /// Read pointer (where delayed audio is read from)
    read_pos: usize,

    /// Current delay in samples
    delay_samples: usize,
}

impl<'a> DelayBuffer<'a> {
    /// Create a new delay buffer from an SDRAM slice
    ///
    /// # Safety
    /// The buffer slice must be valid for the lifetime 'a and at least BUFFER_LENGTH in size
    pub fn new(buffer: &'a mut [u32]) -> Self {
        assert!(buffer.len() >= BUFFER_LENGTH, "Buffer too small for delay");

        // Clear buffer to silence
        const SILENCE: u32 = u32::MAX / 2; // Mid-point for signed audio
        for sample in buffer[..BUFFER_LENGTH].iter_mut() {
            *sample = SILENCE;
        }

        Self {
            buffer,
            write_pos: 0,
            read_pos: 0,
            delay_samples: 0,
        }
    }

    /// Set delay time in samples
    ///
    /// This is called from outside the audio callback to adjust delay.
    /// The delay will be clamped to valid range [0, BUFFER_LENGTH-BLOCK_LENGTH]
    pub fn set_delay_samples(&mut self, samples: usize) {
        // Clamp to valid range (leave space for at least one block)
        let max_delay = BUFFER_LENGTH.saturating_sub(BLOCK_LENGTH);
        self.delay_samples = samples.min(max_delay);

        // Update read pointer to maintain delay offset from write pointer
        self.read_pos = if self.write_pos >= self.delay_samples {
            self.write_pos - self.delay_samples
        } else {
            BUFFER_LENGTH - (self.delay_samples - self.write_pos)
        };
    }

    /// Set delay time in seconds (convenience method)
    pub fn set_delay_seconds(&mut self, seconds: f32) {
        let samples = (seconds * SAMPLE_RATE as f32 * CHANNELS as f32) as usize;
        self.set_delay_samples(samples);
    }

    /// Get current delay in samples
    pub fn delay_samples(&self) -> usize {
        self.delay_samples
    }

    /// Get current delay in seconds
    pub fn delay_seconds(&self) -> f32 {
        self.delay_samples as f32 / (SAMPLE_RATE * CHANNELS) as f32
    }

    /// Process one block of audio (called from audio callback)
    ///
    /// Writes input to buffer and reads delayed output.
    /// This is the core delay processing function.
    pub fn process(&mut self, input: &[u32], output: &mut [u32]) {
        assert_eq!(input.len(), BLOCK_LENGTH, "Input block wrong size");
        assert_eq!(output.len(), BLOCK_LENGTH, "Output block wrong size");

        // Write input samples to buffer at write_pos
        self.write_block(input);

        // Read delayed samples from buffer at read_pos
        self.read_block(output);

        // Advance pointers
        self.write_pos = (self.write_pos + BLOCK_LENGTH) % BUFFER_LENGTH;
        self.read_pos = (self.read_pos + BLOCK_LENGTH) % BUFFER_LENGTH;
    }

    /// Write a block to the buffer, handling wrap-around
    fn write_block(&mut self, input: &[u32]) {
        let remain = BLOCK_LENGTH.min(BUFFER_LENGTH - self.write_pos);
        let frac = BLOCK_LENGTH - remain;

        // Copy first part (up to end of buffer)
        self.buffer[self.write_pos..(self.write_pos + remain)]
            .copy_from_slice(&input[..remain]);

        // Copy wrap-around part (if needed)
        if frac > 0 {
            self.buffer[0..frac].copy_from_slice(&input[remain..]);
        }
    }

    /// Read a block from the buffer, handling wrap-around
    fn read_block(&self, output: &mut [u32]) {
        let remain = BLOCK_LENGTH.min(BUFFER_LENGTH - self.read_pos);
        let frac = BLOCK_LENGTH - remain;

        // Copy first part (up to end of buffer)
        output[..remain].copy_from_slice(
            &self.buffer[self.read_pos..(self.read_pos + remain)]
        );

        // Copy wrap-around part (if needed)
        if frac > 0 {
            output[remain..].copy_from_slice(&self.buffer[0..frac]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_size() {
        // Verify buffer size calculation
        assert_eq!(BUFFER_LENGTH, 960_000);

        // Verify it fits in SDRAM (64MB = 16M u32 words)
        const SDRAM_WORDS: usize = 64 * 1024 * 1024 / 4;
        assert!(BUFFER_LENGTH < SDRAM_WORDS);
    }

    #[test]
    fn test_delay_calculation() {
        // 1 second delay = 48k samples/sec × 2 channels = 96k samples
        let one_second_samples = SAMPLE_RATE * CHANNELS;
        assert_eq!(one_second_samples, 96_000);
    }
}
