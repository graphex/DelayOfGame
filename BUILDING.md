# Building and Flashing Guide

Complete guide for building and deploying Delay of Game to your Daisy Pod.

## Initial Setup

### 1. Install Rust

```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Follow the prompts and restart your terminal
source $HOME/.cargo/env
```

### 2. Add ARM Target

The STM32H750 uses Cortex-M7 with hardware floating point, so we need the `thumbv7em-none-eabihf` target:

```bash
rustup target add thumbv7em-none-eabihf
```

### 3. Install Flash Tools

You have several options for flashing the Daisy Pod:

#### Option A: probe-rs (Recommended)

```bash
# Install probe-rs with CLI tools
cargo install probe-rs-tools --locked

# Verify installation
probe-rs --version
```

#### Option B: OpenOCD + GDB (Alternative)

```bash
# macOS
brew install openocd
brew install arm-none-eabi-gcc

# Linux (Debian/Ubuntu)
sudo apt-get install openocd gcc-arm-none-eabi

# Verify
openocd --version
```

### 4. Install cargo-embed (Optional but Recommended)

```bash
cargo install cargo-embed
```

### 5. Hardware Connection

1. Connect your Daisy Pod to your computer via USB
2. The Daisy Pod should appear as a USB device
3. For ST-Link debugging: Connect your ST-Link V2 to the SWD pins

**Daisy Pod SWD Pins:**
- SWDIO
- SWCLK
- GND
- 3.3V (optional, can power from USB)

## Building the Project

### Debug Build

```bash
cargo build
```

The binary will be at `target/thumbv7em-none-eabihf/debug/delay-of-game`

### Release Build (Optimized)

```bash
cargo build --release
```

The binary will be at `target/thumbv7em-none-eabihf/release/delay-of-game`

**Note:** Release builds are significantly smaller and faster. Always use release builds for testing real audio performance.

### Build Examples

```bash
# Build the blink example
cargo build --example blink --release
```

## Flashing to Daisy Pod

### Method 1: Using cargo run (Easiest)

The `.cargo/config.toml` is configured to use probe-rs as the runner:

```bash
# Flash and run the main program
cargo run --release

# Flash and run the blink example
cargo run --example blink --release
```

This will:
1. Build the project
2. Flash it to the device
3. Reset the device
4. Attach RTT for logging (if enabled)

### Method 2: Using cargo-embed

```bash
# Uses Embed.toml configuration
cargo embed --release

# For examples
cargo embed --example blink --release
```

### Method 3: Using probe-rs directly

```bash
# Build first
cargo build --release

# Flash with probe-rs
probe-rs run --chip STM32H750IBKx target/thumbv7em-none-eabihf/release/delay-of-game
```

### Method 4: Using DFU (Device Firmware Update)

If your Daisy is in DFU mode:

```bash
# Convert to binary first
cargo objcopy --release -- -O binary target/delay-of-game.bin

# Flash using dfu-util
dfu-util -a 0 -s 0x08000000 -D target/delay-of-game.bin
```

To enter DFU mode on Daisy Pod:
1. Hold the BOOT button
2. Press and release RESET
3. Release BOOT

## Debugging

### RTT (Real-Time Transfer) Logging

probe-rs supports RTT for logging without affecting real-time performance:

```bash
# Attach to running device with RTT
probe-rs attach --chip STM32H750IBKx
```

### GDB Debugging

```bash
# Terminal 1: Start OpenOCD
openocd -f interface/stlink.cfg -f target/stm32h7x.cfg

# Terminal 2: Start GDB
arm-none-eabi-gdb target/thumbv7em-none-eabihf/debug/delay-of-game
(gdb) target remote :3333
(gdb) monitor reset halt
(gdb) load
(gdb) continue
```

## Troubleshooting

### "No probe found" Error

```bash
# List available probes
probe-rs list

# If nothing shows:
# - Check USB connection
# - Check ST-Link connection to SWD pins
# - Try different USB port/cable
# - Check udev rules (Linux) or driver (Windows)
```

### Linux udev Rules

If probe-rs can't find your probe on Linux:

```bash
# Create udev rules file
sudo nano /etc/udev/rules.d/99-probe-rs.rules

# Add these lines:
# ST-Link V2
ATTRS{idVendor}=="0483", ATTRS{idProduct}=="3748", MODE="666"
# ST-Link V2-1
ATTRS{idVendor}=="0483", ATTRS{idProduct}=="374b", MODE="666"

# Reload rules
sudo udevadm control --reload-rules
sudo udevadm trigger

# Unplug and replug your probe
```

### Build Errors

```bash
# Clean build and retry
cargo clean
cargo build --release

# Update dependencies
cargo update
```

### "Linking failed" Errors

Make sure you have:
1. `memory.x` in the project root
2. `build.rs` that copies it to the build output
3. Correct target: `thumbv7em-none-eabihf`

### Flash Verification Failed

The STM32H750 has limited internal flash (128KB). The Daisy typically boots from QSPI flash. You may need to:

1. Use the Daisy bootloader
2. Flash to QSPI instead of internal flash
3. Keep your binary under 128KB

## Testing the Blink Example

After flashing the blink example:

1. The LED (on PC7) should blink at 1Hz (500ms on, 500ms off)
2. If the LED blinks, your toolchain is working!
3. If not:
   - Check the LED pin (PC7 for Daisy Seed/Pod)
   - Verify the code flashed successfully
   - Try a full power cycle

## Next Steps

Once you have the blink example working:

1. ✓ Toolchain is confirmed working
2. → Implement audio passthrough
3. → Add SDRAM initialization
4. → Build the delay buffer
5. → Add knob control

## Useful Commands

```bash
# Check what target your build is using
rustc --version --verbose

# See binary size
cargo size --release

# Examine binary
cargo objdump --release -- -d

# Check for issues
cargo clippy

# Format code
cargo fmt

# Run tests (on host, not embedded)
cargo test --lib
```

## References

- [probe-rs Documentation](https://probe.rs/)
- [Embedded Rust Book](https://rust-embedded.github.io/book/)
- [STM32H7xx HAL](https://github.com/stm32-rs/stm32h7xx-hal)
- [Daisy BSP](https://github.com/antoinevg/daisy_bsp)
- [Electrosmith Daisy Wiki](https://github.com/electro-smith/DaisyWiki/wiki)
