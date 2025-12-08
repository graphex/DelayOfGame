# Delay of Game

Audio delay system for syncing Dave Logan's Broncos radio commentary with TV broadcast.

## Quick Start

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add ARM Cortex-M7 target
rustup target add thumbv7em-none-eabihf

# Install probe-rs for flashing and debugging
cargo install probe-rs-tools --locked
```

### Build and Flash

```bash
# Build the project
cargo build --release

# Flash to Daisy Pod
cargo run --release

# Or run the blink example
cargo run --example blink --release
```

## Project Status

- [x] Phase 1: Toolchain and basic blink
- [ ] Phase 2: Audio passthrough
- [ ] Phase 3: Basic delay buffer
- [ ] Phase 4: Variable delay control
- [ ] Phase 5: Polish and display

## Hardware

**Electrosmith Daisy Pod**
- STM32H750 @ 400MHz
- 64MB SDRAM for audio buffer
- AK4556 audio codec (24-bit, 48kHz)
- Stereo line in/out via 1/8" jacks
- Control via knobs and buttons

## Documentation

See [Claude.md](Claude.md) for detailed project documentation, architecture, and development notes.

## Go Broncos! 🏈
