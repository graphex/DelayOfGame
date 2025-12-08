# Delay of Game - Audio Delay for Dave Logan Sync

## Project Overview

An embedded Rust project for the Electrosmith Daisy Pod (STM32H750) that provides variable audio delay to synchronize Dave Logan's Broncos radio commentary with TV broadcast video.

### Purpose
Sync KOA radio (850 AM / 94.1 FM / iHeartRadio) audio with home theater video stream by introducing controllable delay up to 10 seconds.

### Secondary Goals
- Learn Rust programming in embedded context
- Gain microcontroller programming experience
- Understand audio DSP and buffering
- Develop product development skills

## Hardware Specifications

### Daisy Pod
- **MCU**: STM32H750IBK6 (Cortex-M7, 400MHz)
- **RAM**: 512KB SRAM + 64MB SDRAM (external)
- **Flash**: 128KB internal + 64MB QSPI Flash
- **Audio Codec**: AK4556 (24-bit stereo)
- **Sample Rate**: 48kHz typical
- **Input**: 1/8" stereo phono jack
- **Output**: 1/8" stereo phono jack
- **Controls**: 2 knobs, 1 encoder, 2 buttons, 4 gate inputs
- **Display**: None (future enhancement)

### Audio Specifications
- **Input**: Line-level stereo analog
- **Output**: Line-level stereo analog
- **Max Delay**: ~10 seconds
- **Delay Adjustment**: 0.1s or 0.01s increments via Knob 1
- **Latency**: Minimize processing latency

## Technical Architecture

### Memory Layout
- **Delay Buffer**: Use 64MB SDRAM for audio samples
  - At 48kHz stereo (2 channels × 16-bit): ~192KB/second
  - 10 seconds = ~1.92MB (plenty of headroom in 64MB)
- **Code**: QSPI Flash (XIP - Execute In Place)
- **Stack/Heap**: Internal SRAM (512KB)

### Buffer Strategy
Circular buffer implementation:
- Write pointer: current audio input position
- Read pointer: delayed output position (controlled by knob)
- Delay = (write_ptr - read_ptr) % buffer_size

### Processing Flow
```
Audio In (AK4556)
  → ADC (48kHz stereo)
  → Circular Buffer (SDRAM)
  → DAC (48kHz stereo)
  → Audio Out (AK4556)

Knob 1 → ADC → Delay Amount → Read Pointer Offset
```

## Project Structure

```
DelayOfGame/
├── Claude.md                 # This file
├── Cargo.toml               # Rust project manifest
├── .cargo/
│   └── config.toml          # Build and runner configuration
├── memory.x                 # STM32H750 linker script
├── src/
│   ├── main.rs              # Entry point and main loop
│   ├── audio.rs             # Audio processing and codec interface
│   ├── buffer.rs            # Circular delay buffer implementation
│   ├── controls.rs          # Knob/button interface
│   └── display.rs           # (Future) Display driver
├── examples/
│   └── blink.rs             # LED blink test program
└── README.md                # User-facing documentation
```

## Development Phases

### Phase 1: Toolchain and Blink ✓ (Current)
- [x] Set up Rust embedded toolchain
- [x] Configure STM32H750 target
- [x] Implement LED blink
- [x] Verify flash and debug workflow

### Phase 2: Audio Passthrough
- [ ] Initialize AK4556 codec
- [ ] Configure SAI (Serial Audio Interface)
- [ ] Implement audio callback/interrupt
- [ ] Verify clean audio passthrough (zero delay)

### Phase 3: Basic Delay
- [ ] Initialize SDRAM controller
- [ ] Implement circular buffer
- [ ] Add fixed delay (e.g., 1 second test)
- [ ] Verify audio quality

### Phase 4: Variable Delay Control
- [ ] Read ADC from Knob 1
- [ ] Map knob position to delay time
- [ ] Implement smooth delay adjustment
- [ ] Add delay increment modes (0.1s vs 0.01s)

### Phase 5: Polish and Features
- [ ] Add visual feedback (LED patterns)
- [ ] Store/recall delay preset
- [ ] Consider adding display module
- [ ] Optimize for minimal latency

## Rust Dependencies

### Core
- `cortex-m` - Cortex-M runtime
- `cortex-m-rt` - Runtime support and startup
- `panic-halt` - Panic handler for embedded

### STM32H7 HAL
- `stm32h7xx-hal` - Hardware abstraction layer
- `daisy` or `daisy_bsp` - Daisy Board Support Package (if available)

### Audio/DSP
- `heapless` - Static data structures
- `micromath` - Fast math for embedded
- Consider `dsp` or custom DSP routines

## Build and Flash

### Prerequisites
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add ARM Cortex-M target
rustup target add thumbv7em-none-eabihf

# Install cargo-binutils for objcopy
cargo install cargo-binutils
rustup component add llvm-tools-preview

# Install probe-rs for flashing
cargo install probe-rs --features cli
```

### Build
```bash
cargo build --release
```

### Flash
```bash
# Using probe-rs
probe-rs run --chip STM32H750IBKx --release

# Or using cargo-embed
cargo embed --release
```

### Debug
```bash
# RTT logging
probe-rs attach --chip STM32H750IBKx
```

## Resources

### Daisy Pod Documentation
- [Electrosmith Daisy Wiki](https://github.com/electro-smith/DaisyWiki/wiki)
- [Daisy Pod Schematic](https://github.com/electro-smith/Hardware/tree/master/pod)

### Rust Embedded Resources
- [Daisy BSP (Rust)](https://github.com/antoinevg/daisy_bsp)
- [STM32H7xx HAL](https://github.com/stm32-rs/stm32h7xx-hal)
- [Embedded Rust Book](https://rust-embedded.github.io/book/)
- [Cortex-M Quickstart](https://github.com/rust-embedded/cortex-m-quickstart)

### Audio DSP
- Circular buffer algorithms
- Sample rate conversion (if needed)
- Anti-aliasing and filtering

## Design Decisions

### Why SDRAM for Buffer?
- 64MB provides ample space for 10+ seconds of audio
- Fast enough for real-time audio (48kHz is ~180KB/s throughput)
- Frees internal SRAM for program execution

### Why Circular Buffer?
- Constant-time operations
- No memory allocation needed
- Natural fit for continuous audio streams
- Simple delay calculation

### Delay Granularity
- 0.1s increments: Coarse adjustment (100ms steps)
- 0.01s increments: Fine tuning (10ms steps)
- Could add button to toggle between modes

## Future Enhancements

### Display Options
- **OLED Display** (I2C/SPI): Show exact delay time
- **7-Segment LED**: Numeric delay display
- **RGB LED**: Color-coded delay range

### Additional Features
- Store up to 4 delay presets (for different streaming services)
- Auto-sync mode (detect audio patterns)
- Stereo width control
- Simple EQ for radio input
- Bypass footswitch

### Advanced DSP
- Pitch correction (if sample rates drift)
- Noise gate for radio static
- Compressor for consistent levels

## Known Considerations

### Latency
- Codec latency: ~1-2ms
- Processing latency: Minimize via efficient circular buffer
- Total system latency should be <5ms plus user-selected delay

### Audio Quality
- 16-bit or 24-bit path (AK4556 supports 24-bit)
- 48kHz sample rate standard
- Monitor for buffer under/overruns

### Power
- USB-powered via Daisy Pod
- Consider power consumption if battery-powering in future

## Development Environment

### Recommended Tools
- **IDE**: JetBrains RustRover
- **CLI**: Claude Code for code generation and assistance
- **Debugger**: probe-rs with RTT or GDB
- **Flashing**: probe-rs or dfu-util

### Version Control
```bash
git init
git add .
git commit -m "Initial Delay of Game project structure"
```

## Contact and Credits

Project by: graphex
Purpose: Enhance Broncos game viewing experience with Dave Logan commentary
License: (TBD - MIT/Apache 2.0 typical for Rust projects)

---

**Go Broncos! 🏈**
