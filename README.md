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
# Build the project (uses seed_1_1/WM8731 by default)
cargo build --release

# Flash to Daisy Pod
cargo run --release

# For seed_1_2/PCM3060 codec:
cargo run --release --no-default-features --features seed_1_2

# Or run the blink example
cargo run --example blink --release
```

## Project Status

- [x] Phase 1: Toolchain and basic blink
- [x] Phase 2: Audio passthrough (codec-agnostic implementation)
- [x] Phase 3: Basic delay buffer (fixed 1-second delay)
- [ ] Phase 4: Variable delay control
- [ ] Phase 5: Polish and display

## Hardware

**Electrosmith Daisy Pod**
- STM32H750 @ 400MHz
- 64MB SDRAM for audio buffer
- WM8731 codec (Seed 1.1) or PCM3060 codec (Seed 1.2)
- 24-bit stereo, 48kHz sample rate
- Stereo line in/out via 1/8" jacks
- Control via knobs and buttons

## Documentation

See [Claude.md](Claude.md) for detailed project documentation, architecture, and development notes.

## Go Broncos! 🏈
