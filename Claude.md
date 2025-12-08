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

### Embassy Async Architecture
**Why Async/Await?**
- Write clear, sequential code without callback hell
- Efficient: only one stack needed for multiple tasks
- Hardware events (audio interrupts, button presses) naturally map to async

**Task Structure:**
```rust
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // Initialize hardware
    let board = new_daisy_board!(p);

    // Spawn concurrent tasks
    spawner.spawn(led_blinker(board.user_led)).ok();
    spawner.spawn(knob_reader(board.adc)).ok();

    // Main audio processing in callback (not async)
    audio_interface.start(|input, output| {
        // Process audio synchronously in real-time
        process_audio(input, output, delay_buffer);
    }).await;
}
```

**Key Pattern:**
- UI tasks (LED, knob reading) run async
- Audio callback runs synchronously (hard real-time requirement)
- Communication via lock-free channels or atomics

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

### Phase 1: Toolchain and Blink ✓ (Complete)
- [x] Set up Rust embedded toolchain
- [x] Configure STM32H750 target with Embassy framework
- [x] Implement LED blink example with async/await
- [x] Configure probe-rs and defmt logging
- [x] Ready to verify flash and debug workflow

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

### Embassy Framework (Async Runtime)
- `daisy-embassy` (v0.2.2) - Daisy board support with Embassy
- `embassy-executor` - Async task executor with Cortex-M support
- `embassy-stm32` - STM32 HAL with async drivers
- `embassy-time` - Async timers and delays
- `embassy-sync` - Async synchronization primitives
- `embassy-futures` - Additional async utilities

### Core Embedded
- `cortex-m` (v0.7.7) - Cortex-M runtime with inline assembly
- `cortex-m-rt` (v0.7.0) - Runtime support and startup
- `static_cell` - Static allocation for Embassy tasks
- `critical-section` - Critical section implementation

### Debug and Logging
- `defmt` (v0.3.8) - Efficient logging for embedded (like printf)
- `defmt-rtt` (v0.4.1) - RTT transport for defmt logs
- `panic-probe` (v0.3.2) - Panic handler with defmt support

### Audio/DSP
- `heapless` (v0.8) - Static data structures (no heap allocations)
- `micromath` (v2.0.0) - Fast math for embedded
- Audio codec drivers included via daisy-embassy

### Why Embassy?
**Async/Await**: Write sequential code that's actually efficient
**Modern**: Active development, better ergonomics than traditional RTIC
**Examples**: daisy-embassy has audio looper example with SDRAM buffering
**Integration**: Seamless integration with Daisy hardware abstraction

## Build and Flash

### Prerequisites
```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add ARM Cortex-M target (automatic via rust-toolchain.toml)
rustup target add thumbv7em-none-eabihf

# Install probe-rs for flashing and debugging
cargo install probe-rs-tools --locked
```

### Build
```bash
# Debug build
cargo build

# Release build (optimized for size)
cargo build --release
```

### Flash and Run
```bash
# Flash and run with probe-rs (configured in .cargo/config.toml)
cargo run --release

# Run the blink example
cargo run --example blink --release

# Flash with cargo-embed (uses Embed.toml configuration)
cargo embed --release
```

### Debug with RTT Logging
defmt logging over RTT (Real-Time Transfer) is built-in:
```bash
# Logs appear automatically when you run:
cargo run --release

# Set log level via environment variable
DEFMT_LOG=debug cargo run --release
# Levels: trace, debug, info, warn, error
```

## Resources

### Daisy Pod Documentation
- [Electrosmith Daisy Wiki](https://github.com/electro-smith/DaisyWiki/wiki)
- [Daisy Pod Schematic](https://github.com/electro-smith/Hardware/tree/master/pod)

### Rust Embedded Resources
- [daisy-embassy (GitHub)](https://github.com/daisy-embassy/daisy-embassy) - Primary framework
- [daisy-embassy examples](https://github.com/daisy-embassy/daisy-embassy/tree/master/examples) - Including looper.rs with SDRAM buffering
- [Embassy Book](https://embassy.dev/book/) - Async embedded Rust guide
- [Embassy API Docs](https://docs.embassy.dev/) - API documentation
- [Embedded Rust Book](https://rust-embedded.github.io/book/) - General embedded Rust
- [defmt Book](https://defmt.ferrous-systems.com/) - Efficient logging

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
