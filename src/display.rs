//! Adafruit quad 14-segment alphanumeric backpack (HT16K33) over async I2C
//!
//! Writes go out by DMA and await completion, so the audio callback keeps
//! running while bytes are on the wire. A blocking driver here starved the
//! executor and overran the SAI.

use embassy_stm32::i2c::{Error, I2c};
use embassy_stm32::mode::Async;

/// Backpack addresses: 0x70 plus the A0 (+1), A1 (+2) and A2 (+4) jumpers
const ADDRESSES: core::ops::RangeInclusive<u8> = 0x70..=0x77;

pub const DIGITS: usize = 4;

/// Brightness levels: 0 (dimmest, but still lit) to 15 (brightest)
pub const MAX_BRIGHTNESS: u8 = 15;

const CMD_OSCILLATOR_ON: u8 = 0x21;
const CMD_DISPLAY_ON: u8 = 0x81; // blink off
const CMD_BRIGHTNESS: u8 = 0xE0;

/// Decimal point segment, OR'd onto a character
const DP: u16 = 1 << 14;

/// Segment bits: A=0 B=1 C=2 D=3 E=4 F=5 G1=6 G2=7 H=8 J=9 K=10 L=11 M=12 N=13
const DIGIT_SEGMENTS: [u16; 10] = [
    0x003F, 0x0006, 0x00DB, 0x008F, 0x00E6, 0x00ED, 0x00FD, 0x0007, 0x00FF, 0x00EF,
];

/// Glyph for a character (case-insensitive); unsupported characters are blank
fn glyph(c: u8) -> u16 {
    match c.to_ascii_uppercase() {
        b'0'..=b'9' => DIGIT_SEGMENTS[(c - b'0') as usize],
        b'A' => 0x00F7,
        b'C' => 0x0039,
        b'D' => 0x120F,
        b'E' => 0x00F9,
        b'F' => 0x0071,
        b'G' => 0x00BD,
        b'I' => 0x1209,
        b'L' => 0x0038,
        b'M' => 0x0536,
        b'N' => 0x2136,
        b'O' => 0x003F,
        b'R' => 0x20F3,
        b'S' => 0x00ED,
        b'V' => 0x0C30,
        b'Y' => 0x1500,
        _ => 0,
    }
}

/// Segments for a word of up to four characters
pub fn text(word: &[u8; DIGITS]) -> [u16; DIGITS] {
    word.map(glyph)
}

/// Longest message `scroll_frame` handles, counted in displayed characters
const MAX_MESSAGE: usize = 32;

/// Glyphs for a message, with each '.' folded into the decimal point of the
/// character before it
fn message_glyphs(message: &[u8]) -> heapless::Vec<u16, MAX_MESSAGE> {
    let mut glyphs = heapless::Vec::new();
    for &c in message {
        match glyphs.last_mut() {
            Some(last) if c == b'.' => *last |= DP,
            _ => {
                let _ = glyphs.push(if c == b'.' { DP } else { glyph(c) });
            }
        }
    }
    glyphs
}

/// Number of frames for a message to scroll in from the right and fully off
/// the left, ending blank
pub fn scroll_steps(message: &[u8]) -> usize {
    message_glyphs(message).len() + DIGITS
}

/// Frame `step` (0..scroll_steps) of a message scrolling right to left
pub fn scroll_frame(message: &[u8], step: usize) -> [u16; DIGITS] {
    let glyphs = message_glyphs(message);
    core::array::from_fn(|i| {
        (step + i)
            .checked_sub(DIGITS - 1)
            .and_then(|index| glyphs.get(index).copied())
            .unwrap_or(0)
    })
}

/// Segments for a delay given in hundredths of a second: "12.34" below
/// 100 s, "123.4" from 100 s up. A leading zero is blanked (" 1.00").
pub fn seconds(hundredths: u32) -> [u16; DIGITS] {
    let (value, dp_digit) = if hundredths < 10_000 {
        (hundredths, 1)
    } else {
        (hundredths / 10, 2)
    };
    let mut segments = [0; DIGITS];
    let mut rest = value;
    for i in (0..DIGITS).rev() {
        segments[i] = DIGIT_SEGMENTS[(rest % 10) as usize];
        rest /= 10;
    }
    if value < 1000 {
        segments[0] = 0;
    }
    segments[dp_digit] |= DP;
    segments
}

/// Map a smoothed knob reading (0..=full_scale) to a brightness level. A
/// reading must cross a level boundary by a quarter of a level before the
/// level changes, so a knob resting on a boundary doesn't flicker.
pub fn knob_to_brightness(reading: u32, full_scale: u32, current: u8) -> u8 {
    let levels = MAX_BRIGHTNESS as u32 + 1;
    let band = (full_scale + 1) / levels;
    let margin = band / 4;
    let target = (reading / band).min(MAX_BRIGHTNESS as u32) as u8;
    let current_u32 = current as u32;
    if (target > current && reading >= (current_u32 + 1) * band + margin)
        || (target < current && reading + margin < current_u32 * band)
    {
        target
    } else {
        current
    }
}

pub struct Display {
    i2c: I2c<'static, Async>,
    address: u8,
    /// Must be DMA-reachable: DMA1 can't read DTCM, where the stack lives
    tx: &'static mut [u8; 1 + 2 * DIGITS],
}

impl Display {
    pub fn new(i2c: I2c<'static, Async>, tx: &'static mut [u8; 1 + 2 * DIGITS]) -> Self {
        Self {
            i2c,
            address: *ADDRESSES.start(),
            tx,
        }
    }

    /// Find the backpack at whichever address its jumpers select, then start
    /// the oscillator, turn the display on and set brightness. Safe to
    /// repeat, e.g. after the backpack was unplugged. Returns the address.
    pub async fn init(&mut self, brightness: u8) -> Result<u8, Error> {
        let mut result = Err(Error::Nack);
        for address in ADDRESSES {
            self.tx[0] = CMD_OSCILLATOR_ON;
            result = self.i2c.write(address, &self.tx[..1]).await;
            if result.is_ok() {
                self.address = address;
                break;
            }
        }
        result?;

        self.tx[0] = CMD_DISPLAY_ON;
        self.i2c.write(self.address, &self.tx[..1]).await?;
        self.set_brightness(brightness).await?;
        Ok(self.address)
    }

    pub async fn set_brightness(&mut self, level: u8) -> Result<(), Error> {
        self.tx[0] = CMD_BRIGHTNESS | level.min(MAX_BRIGHTNESS);
        self.i2c.write(self.address, &self.tx[..1]).await
    }

    pub async fn show(&mut self, segments: &[u16; DIGITS]) -> Result<(), Error> {
        self.tx[0] = 0x00; // display RAM start; each digit is two bytes, low first
        for (i, s) in segments.iter().enumerate() {
            self.tx[1 + 2 * i..3 + 2 * i].copy_from_slice(&s.to_le_bytes());
        }
        self.i2c.write(self.address, &self.tx[..]).await
    }
}
