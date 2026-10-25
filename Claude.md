# Delay of Game (Gen 1): Development Notes

Variable stereo audio delay (0–170 s) on an Electrosmith Daisy Pod. It syncs KOA radio commentary (Dave Logan, Broncos) with a delayed TV/streaming picture. Written in Rust with Embassy and `daisy-embassy`.

**Status:** Gen 1 works and is getting incremental upgrades: longer delay, volume knob, and a display next. A second-generation design on different hardware is being planned separately. See `GEN2_HANDOFF.md`.

Secondary goals: learn embedded Rust, microcontrollers, audio DSP and product development.

## Hardware (as built)

- **Board:** Daisy Pod carrying a Daisy Seed 1.1 (Rev5)
- **MCU:** STM32H750IBK6, Cortex-M7 at 480 MHz (`daisy_embassy::default_rcc()`)
- **Memory:** 128 KB internal flash (the firmware runs from here, about 41 KB), 512 KB AXI SRAM, 128 KB DTCM (stack/`.bss`), 64 MB external SDRAM (delay buffer), 8 MB QSPI (unused)
- **Codec:** WM8731, 48 kHz, 24-bit stereo, line in/out on 1/8" jacks. The `seed_1_2` feature selects the PCM3060 for a Seed 1.2.
- **Controls used:** the Pod encoder (A = D26/PD11, B = D25/PA0, push button = D13/PB6) and Knob 1 (D21/PC4, ADC1) for output volume. Knob 2 (D15/PC0), the buttons and the gate inputs are free.
- **Debug:** ST-Link V3 Mini on the 14-pin header, with separate USB power to the Pod. See `HARDWARE_SETUP.md`.

## Code layout

```
src/main.rs      Board init, SDRAM, encoder task, LED task, audio callback
src/buffer.rs    DelayBuffer: circular buffer over SDRAM; MAX_DELAY_SECONDS lives here
src/encoder.rs   RotaryEncoder: polled quadrature decoder
src/volume.rs    Knob-to-gain curve and click-free GainRamp
examples/blink.rs  LED smoke test
```

The linker script comes from `daisy-embassy` (it defines `.sram1_bss` for the DMA buffers and `.sdram_bss`). The project doesn't need its own `memory.x`.

## How it works

- `start_callback` runs on the **thread-mode executor** in `main`, once per 32-frame block (about 0.67 ms at 48 kHz). The block is `HALF_DMA_BUFFER_LENGTH` = 64 interleaved `u32` samples (L, R, L, R…).
- `DelayBuffer` writes each input block at `write_pos` and reads the output from `read_pos = write_pos − delay`. Both pointers wrap around a buffer of `48 000 × 170 × 2` samples (65 MB, nearly all of the SDRAM).
- The encoder task polls every 1 ms. It keeps the delay in **frames** and publishes it through `DELAY_FRAMES: AtomicU32`. The callback multiplies by `CHANNELS` to get buffer samples.
- The button switches the step between 0.1 s and 0.01 s.
- The volume task reads Knob 1 every 10 ms, smooths it, maps it through a squared curve (center = 25% gain) and publishes Q16 gain in `VOLUME_GAIN`. The callback applies it after the delay with `GainRamp`, which limits each block's change and interpolates within the block, so there's no zipper noise. Gain starts at 0, so audio fades in at power-up.
- The ADC is clocked from PLL3_R (`config.rcc.mux.adcsel`). `default_rcc()` leaves the ADC clock unset.
- Delay, step size and volume are logged over defmt/RTT.

## Gotchas

- **Frames vs samples.** The buffer counts interleaved samples (2 per frame). Mixing the two units up once made the real delay half of what the logs showed, fixed on 2026-10-24.
- **Anything that blocks the executor kills the audio.** Every task shares the executor with the audio callback, so a task that blocks for more than about 0.67 ms causes an SAI overrun. Blocking I2C display drivers (SSD1306 OLED, HT16K33 LED backpack) failed for exactly this reason. Moving the audio to an `InterruptExecutor` at higher priority (the `executor-interrupt` feature is already enabled), or using async DMA drivers, would probably fix this, but neither has been tried.
- **Changing the delay causes a click.** `set_delay_samples` jumps the read pointer with no crossfade.
- **Samples are 24-bit in the low bits of a `u32`.** Sign-extend (`(s << 8) as i32 >> 8`) before doing math on them, and mask to `0x00FF_FFFF` afterward. Silence is 0.
- **No host tests.** The crate is `no_std` for `thumbv7em-none-eabihf`, so `cargo test` doesn't work. `cargo clippy --release --bins --examples` does.

## Build and run

```bash
cargo build --release
cargo run --release                         # flash with probe-rs and stream defmt logs
cargo run --example blink --release
cargo run --release --no-default-features --features seed_1_2   # PCM3060 boards
```

`DEFMT_LOG` defaults to `trace` (`.cargo/config.toml`). Override it with `DEFMT_LOG=info cargo run --release`.

## Key dependencies

`daisy-embassy` 0.2.2, `embassy-stm32` 0.2, `embassy-executor` 0.7, `embassy-time` 0.4, `defmt` 0.3. Exact versions are pinned in `Cargo.lock`.

Project by graphex. Go Broncos! 🏈
