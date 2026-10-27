//! Saves the delay to the Seed's QSPI flash so it survives power cycles
//!
//! Records are appended to one of two 4 KB sectors at the top of the 8 MB
//! IS25LP064. When one fills, the other is erased and takes over, so the
//! last saved value survives a power loss at any point. The chip is driven
//! in plain single-line SPI mode, and erase/program completion is polled with
//! timers so the audio callback keeps running: daisy-embassy's driver
//! busy-waits, and a sector erase can take up to 300 ms.

use daisy_embassy::flash::FlashBuilder;
use embassy_stm32::mode::Blocking;
use embassy_stm32::peripherals::QUADSPI;
use embassy_stm32::qspi::enums::{
    AddressSize, ChipSelectHighTime, DummyCycles, FIFOThresholdLevel, MemorySize, QspiWidth,
};
use embassy_stm32::qspi::{Config, Qspi, TransferConfig};
use embassy_time::Timer;

// IS25LP064 commands (SPI mode)
const CMD_READ: u8 = 0x03;
const CMD_PAGE_PROGRAM: u8 = 0x02;
const CMD_SECTOR_ERASE: u8 = 0x20;
const CMD_READ_STATUS: u8 = 0x05;
const CMD_WRITE_ENABLE: u8 = 0x06;
const CMD_EXIT_QPI: u8 = 0xF5;
const STATUS_BUSY: u8 = 0x01;

const SECTOR_SIZE: u32 = 4096;
const SECTORS: [u32; 2] = [0x7F_E000, 0x7F_F000];
/// Divides the 256-byte page, so a record never straddles two pages
const RECORD_SIZE: usize = 16;
const RECORDS_PER_SECTOR: u32 = SECTOR_SIZE / RECORD_SIZE as u32;
const MAGIC: u32 = 0xDE1A_70F6;

#[derive(Clone, Copy)]
struct Record {
    sequence: u32,
    delay_frames: u32,
}

impl Record {
    fn check(&self) -> u32 {
        self.sequence ^ self.delay_frames ^ MAGIC
    }

    fn to_bytes(self) -> [u8; RECORD_SIZE] {
        let mut bytes = [0; RECORD_SIZE];
        for (i, word) in [self.sequence, self.delay_frames, self.check(), MAGIC]
            .into_iter()
            .enumerate()
        {
            bytes[4 * i..4 * i + 4].copy_from_slice(&word.to_le_bytes());
        }
        bytes
    }

    /// None for an erased slot or a write cut short by power loss
    fn from_bytes(bytes: &[u8; RECORD_SIZE]) -> Option<Self> {
        let word = |i: usize| u32::from_le_bytes(bytes[4 * i..4 * i + 4].try_into().unwrap());
        let record = Self {
            sequence: word(0),
            delay_frames: word(1),
        };
        (word(2) == record.check() && word(3) == MAGIC).then_some(record)
    }
}

pub struct Settings {
    qspi: Qspi<'static, QUADSPI, Blocking>,
    /// Where the next record goes
    sector: usize,
    slot: u32,
    sequence: u32,
}

impl Settings {
    /// Set up the flash and find the most recently saved delay. Reads both
    /// sectors (a few ms of blocking I/O), so call it before audio starts.
    pub fn load(flash: FlashBuilder) -> (Self, Option<u32>) {
        let config = Config {
            memory_size: MemorySize::_8MiB,
            address_size: AddressSize::_24bit,
            prescaler: 7, // 240 MHz / 8 = 30 MHz, within the 50 MHz limit for CMD_READ
            cs_high_time: ChipSelectHighTime::_2Cycle,
            fifo_threshold: FIFOThresholdLevel::_1Bytes,
        };
        let pins = flash.pins;
        let qspi = Qspi::new_blocking_bank1(
            flash.qspi, pins.IO0, pins.IO1, pins.IO2, pins.IO3, pins.SCK, pins.CS, config,
        );
        let mut settings = Self {
            qspi,
            sector: 0,
            slot: 0,
            sequence: 0,
        };

        // Other Daisy firmware (and daisy-embassy's driver) switch the chip to
        // QPI mode, which survives an MCU reset. In SPI mode this is ignored.
        settings.command(QspiWidth::QUAD, CMD_EXIT_QPI, None);

        let mut latest: Option<(usize, u32, Record)> = None;
        for sector in 0..SECTORS.len() {
            for slot in 0..RECORDS_PER_SECTOR {
                let mut bytes = [0; RECORD_SIZE];
                settings.read(Self::address(sector, slot), &mut bytes);
                if let Some(record) = Record::from_bytes(&bytes) {
                    if latest.is_none_or(|(_, _, l)| record.sequence > l.sequence) {
                        latest = Some((sector, slot, record));
                    }
                }
            }
        }

        let saved = latest.map(|(sector, slot, record)| {
            settings.sector = sector;
            settings.slot = slot + 1;
            settings.sequence = record.sequence;
            record.delay_frames
        });
        (settings, saved)
    }

    /// Append a record, switching to (and erasing) the other sector when the
    /// current one is full. Slots that aren't blank, e.g. from an interrupted
    /// write, are skipped.
    pub async fn save(&mut self, delay_frames: u32) {
        loop {
            if self.slot >= RECORDS_PER_SECTOR {
                self.sector = 1 - self.sector;
                self.slot = 0;
                self.erase(SECTORS[self.sector]).await;
            }
            let address = Self::address(self.sector, self.slot);
            self.slot += 1;

            let mut existing = [0; RECORD_SIZE];
            self.read(address, &mut existing);
            if existing.iter().any(|&b| b != 0xFF) {
                continue;
            }

            let record = Record {
                sequence: self.sequence.wrapping_add(1),
                delay_frames,
            };
            let bytes = record.to_bytes();
            self.program(address, &bytes).await;

            let mut written = [0; RECORD_SIZE];
            self.read(address, &mut written);
            if written == bytes {
                self.sequence = record.sequence;
                return;
            }
            defmt::warn!("Flash verify failed at {:#x}; trying next slot", address);
        }
    }

    fn address(sector: usize, slot: u32) -> u32 {
        SECTORS[sector] + slot * RECORD_SIZE as u32
    }

    fn command(&mut self, iwidth: QspiWidth, instruction: u8, address: Option<u32>) {
        self.qspi.blocking_command(TransferConfig {
            iwidth,
            awidth: if address.is_some() {
                QspiWidth::SING
            } else {
                QspiWidth::NONE
            },
            dwidth: QspiWidth::NONE,
            instruction,
            address,
            dummy: DummyCycles::_0,
        });
    }

    fn read(&mut self, address: u32, buffer: &mut [u8]) {
        self.qspi.blocking_read(
            buffer,
            TransferConfig {
                iwidth: QspiWidth::SING,
                awidth: QspiWidth::SING,
                dwidth: QspiWidth::SING,
                instruction: CMD_READ,
                address: Some(address),
                dummy: DummyCycles::_0,
            },
        );
    }

    async fn program(&mut self, address: u32, data: &[u8]) {
        self.command(QspiWidth::SING, CMD_WRITE_ENABLE, None);
        self.qspi.blocking_write(
            data,
            TransferConfig {
                iwidth: QspiWidth::SING,
                awidth: QspiWidth::SING,
                dwidth: QspiWidth::SING,
                instruction: CMD_PAGE_PROGRAM,
                address: Some(address),
                dummy: DummyCycles::_0,
            },
        );
        self.wait_ready(1).await; // typically 0.2 ms, at most 0.8 ms
    }

    async fn erase(&mut self, address: u32) {
        self.command(QspiWidth::SING, CMD_WRITE_ENABLE, None);
        self.command(QspiWidth::SING, CMD_SECTOR_ERASE, Some(address));
        self.wait_ready(10).await; // typically 45 ms, at most 300 ms
    }

    /// Poll the busy bit, sleeping between polls so other tasks keep running
    async fn wait_ready(&mut self, poll_ms: u64) {
        loop {
            let mut status = [0xFF];
            self.qspi.blocking_read(
                &mut status,
                TransferConfig {
                    iwidth: QspiWidth::SING,
                    awidth: QspiWidth::NONE,
                    dwidth: QspiWidth::SING,
                    instruction: CMD_READ_STATUS,
                    address: None,
                    dummy: DummyCycles::_0,
                },
            );
            if status[0] & STATUS_BUSY == 0 {
                return;
            }
            Timer::after_millis(poll_ms).await;
        }
    }
}
