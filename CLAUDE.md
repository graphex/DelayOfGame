# Delay of Game (Gen 1): Development Notes

Variable stereo audio delay (0–170 s) on an Electrosmith Daisy Pod. It syncs KOA radio commentary (Dave Logan, Broncos) with a delayed TV/streaming picture. Written in Rust with Embassy and `daisy-embassy`.

**Status:** Gen 1 works and is getting incremental upgrades. It now has a 170 s delay, a volume knob, a 4-character display with a brightness knob, and ±10 s buttons. A second-generation design on different hardware is being planned separately. See `GEN2_HANDOFF.md`.

Secondary goals: learn embedded Rust, microcontrollers, audio DSP and product development.

## Hardware (as built)

- **Board:** Daisy Pod carrying a Daisy Seed 1.1 (Rev5)
- **MCU:** STM32H750IBK6, Cortex-M7 at 480 MHz (`daisy_embassy::default_rcc()`)
- **Memory:** 128 KB internal flash (the firmware runs from here, about 45 KB), 512 KB AXI SRAM, 128 KB DTCM (stack/`.bss`), 64 MB external SDRAM (delay buffer), 8 MB QSPI flash (the top two 4 KB sectors hold the saved delay)
- **Codec:** WM8731, 48 kHz, 24-bit stereo, line in/out on 1/8" jacks. The `seed_1_2` feature selects the PCM3060 for a Seed 1.2.
- **Controls used:** the Pod encoder (A = D26/PD11, B = D25/PA0, push button = D13/PB6), buttons SW1 = D27/PG9 (−10 s) and SW2 = D28/PA2 (+10 s), Knob 1 (D21/PC4, ADC1) for output volume and Knob 2 (D15/PC0, ADC1) for display brightness. The RGB LEDs and gate inputs are free.
- **Display:** Adafruit quad 14-segment alphanumeric backpack (HT16K33) on I2C1: SCL = D11/PB8, SDA = D12/PB9, 400 kHz, async with DMA1_CH3/CH4. The firmware scans addresses 0x70–0x77, so the address jumpers don't matter (the current one is at 0x73). Power: + to 3V3 Analog (pin 21), Vi2c to 3V3 Digital (pin 38). VIN (pin 39) sagged too far under load.
- **Debug:** ST-Link V3 Mini (firmware V3J18 or newer) on the debug header. The Daisy's own USB must also go to the computer (power + common ground). See the orientation diagram in `HARDWARE_SETUP.md`. The firmware enables debug-in-sleep in `DBGMCU`, so the probe can attach while the executor sleeps.

## Code layout

```
src/main.rs      Board init, SDRAM, encoder task, LED task, audio callback
src/buffer.rs    DelayBuffer: circular buffer over SDRAM; MAX_DELAY_SECONDS lives here
src/display.rs   HT16K33 driver over async I2C, 14-segment font, delay formatting, brightness curve
src/settings.rs  Saves/restores the delay in QSPI flash with non-blocking erase/program
src/encoder.rs   RotaryEncoder (polled quadrature decoder) and DebouncedButton
src/volume.rs    Knob-to-gain curve and click-free GainRamp
examples/blink.rs  LED smoke test
```

The linker script comes from `daisy-embassy` (it defines `.sram1_bss` for the DMA buffers and `.sdram_bss`). The project doesn't need its own `memory.x`.

## How it works

- `start_callback` runs on the **thread-mode executor** in `main`, once per 32-frame block (about 0.67 ms at 48 kHz). The block is `HALF_DMA_BUFFER_LENGTH` = 64 interleaved `u32` samples (L, R, L, R…).
- `DelayBuffer` writes each input block at `write_pos` and reads the output from `read_pos = write_pos − delay`. Both pointers wrap around a buffer of `48 000 × 170 × 2` samples (65 MB, nearly all of the SDRAM).
- The encoder task polls every 1 ms. It keeps the delay in **frames** and publishes it through `DELAY_FRAMES: AtomicU32`. The callback multiplies by `CHANNELS` to get buffer samples.
- The encoder button switches the step between 0.1 s and 0.01 s (`STEP_FINE`). SW1/SW2 jump the delay by 10 s; they're polled in the same task through `DebouncedButton`.
- The display task polls the shared atomics every 20 ms and writes to the backpack only when something changed. It shows `FINE`/`CRSE` for a second after a step toggle, and it re-scans for the backpack every second if it's missing or a write fails.
- The knob task reads Knob 1 every 10 ms, smooths it, maps it through a squared curve (center = 25% gain) and publishes Q16 gain in `VOLUME_GAIN`. The callback applies it after the delay with `GainRamp`, which limits each block's change and interpolates within the block, so there's no zipper noise. Gain starts at 0, so audio fades in at power-up. It also reads Knob 2 and publishes a brightness level 0–15 in `BRIGHTNESS`, with hysteresis so a knob sitting on a level boundary doesn't flicker.
- The ADC is clocked from PLL3_R (`config.rcc.mux.adcsel`). `default_rcc()` leaves the ADC clock unset.
- `delay_saver` writes the delay to QSPI flash once it's been unchanged for 3 s. `Settings` appends 16-byte records (sequence, delay, check, magic) to one of two sectors at 0x7FE000/0x7FF000. When one fills, it erases the other and switches, so a power loss never loses both copies. At boot, `Settings::load` scans both sectors and the highest valid sequence wins.
- Delay, step size, volume and brightness are logged over defmt/RTT.

## Gotchas

- **Frames vs samples.** The buffer counts interleaved samples (2 per frame). Mixing the two units up once made the real delay half of what the logs showed, fixed on 2026-10-24.
- **Anything that blocks the executor kills the audio.** Every task shares the executor with the audio callback, so a task that blocks for more than about 0.67 ms causes an SAI overrun. Blocking I2C display drivers (SSD1306 OLED, HT16K33 LED backpack) failed for exactly this reason. The display now uses embassy's async I2C with DMA, which yields while bytes are on the wire, and it writes only on changes. Keep new peripherals async. If that isn't enough, move the audio to a higher-priority `InterruptExecutor` (the `executor-interrupt` feature is already enabled).
- **DMA1 can't reach DTCM.** The stack and `.bss` live in DTCM, so DMA buffers must be statics with `#[link_section = ".sram1_bss"]` (D2 SRAM), like `DISPLAY_TX`. The D-cache is off, so no cache maintenance is needed.
- **The embassy I2C driver busy-waits on a stuck bus.** `i2c_config.timeout` is 2 ms, so a wedged bus costs at most one audio blip per attempt instead of a 1 s stall.
- **probe-rs leaves the defmt channel in blocking mode.** It switches RTT to `BlockIfFull` on attach and doesn't switch it back when it exits, so the firmware froze once the log buffer filled. `defmt-rtt`'s `disable-blocking-mode` feature makes the firmware ignore that and drop log lines instead.
- **Attaching a debugger glitches the audio.** probe-rs briefly halts the core on attach, which overruns the SAI. The callback loop logs a warning and restarts instead of panicking, so you hear a blip, not permanent silence.
- **ST-Link firmware must be current.** On firmware V3J7 (2020), probe-rs failed with USB "pipe stalled" / "interfaces are claimed" errors and OpenOCD couldn't read memory. V3J18 works. Update with ST's STSW-LINK007.
- **daisy-embassy's `Flash` driver busy-waits.** Its `write` erases a whole 4 KB sector first (up to 300 ms), which would overrun the SAI. `settings.rs` drives the QSPI chip itself in single-line SPI mode and polls the busy bit with `Timer`. Internal flash is a single 128 KB sector that holds the firmware, so it can't store settings.
- **Changing the delay causes a click.** `set_delay_samples` jumps the read pointer with no crossfade.
- **Samples are 24-bit in the low bits of a `u32`.** Sign-extend (`(s << 8) as i32 >> 8`) before doing math on them, and mask to `0x00FF_FFFF` afterward. Silence is 0.
- **No host tests.** The crate is `no_std` for `thumbv7em-none-eabihf`, so `cargo test` doesn't work. `cargo clippy --release --bins --examples` does.

## Build and run

```bash
cargo build --release
cargo run --release                         # flash with probe-rs and stream defmt logs
scripts/logs.sh                             # attach to the running Daisy and stream logs (no reflash)
cargo run --example blink --release
cargo run --release --no-default-features --features seed_1_2   # PCM3060 boards
```

`DEFMT_LOG` defaults to `trace` (`.cargo/config.toml`). Override it with `DEFMT_LOG=info cargo run --release`.

## Key dependencies

`daisy-embassy` 0.2.2, `embassy-stm32` 0.2, `embassy-executor` 0.7, `embassy-time` 0.4, `defmt` 0.3. Exact versions are pinned in `Cargo.lock`.

Project by graphex. Go Broncos! 🏈
