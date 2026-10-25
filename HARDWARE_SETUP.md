# Daisy Pod Hardware Connection Guide

## Connection Methods

You have **two main options** for connecting your Daisy Pod to your computer for programming:

### Option 1: USB Connection (Built-in Bootloader) - EASIEST

The Daisy Seed has a built-in USB bootloader for programming.

**Steps:**
1. **Enter DFU Mode** (Device Firmware Update):
   - Hold down the **BOOT button** on the Daisy Seed
   - While holding BOOT, press and release the **RESET button**
   - Release the BOOT button
   - The Daisy is now in DFU mode

2. **Verify DFU Connection:**
   ```bash
   # Check if device is detected
   dfu-util -l

   # Or use system_profiler (macOS)
   system_profiler SPUSBDataType | grep -A 10 "STM"
   ```

3. **Flash with dfu-util** (DFU writes to internal flash; probe-rs does not speak DFU):
   ```bash
   cargo objcopy --release -- -O binary target/delay-of-game.bin
   dfu-util -a 0 -s 0x08000000:leave -D target/delay-of-game.bin
   ```
   `cargo objcopy` comes from `cargo install cargo-binutils`.

**Note:** DFU gives you no RTT logs, so you won't see the delay value. Use an ST-Link for that.

### Option 2: ST-Link Programmer (External Hardware) - MORE RELIABLE ⭐ RECOMMENDED

For more reliable development and debugging, use an ST-Link V2/V3 programmer.

**What you need:**
- ST-Link V2 programmer (~$10-20 online)
- **ST-Link V3 Mini** (~$10-15) - Compact, works great with Daisy
- Or ST-Link V2 clone
- Or J-Link programmer

#### Option 2A: ST-Link V3 Mini with Daisy Pod (VERIFIED WORKING ✓)

**Hardware Required:**
- ST-Link V3 Mini programmer
- 14-pin ribbon cable (usually included with ST-Link V3 Mini)
- Daisy Pod with debug header populated
- **Separate USB cable for Daisy power**

**Connection Steps:**

1. **Connect the Ribbon Cable:**
   - Locate the 14-pin debug header on your Daisy Pod/Seed
   - Plug the 14-pin ribbon cable into the ST-Link V3 Mini
   - **CRITICAL: Plug the ribbon cable into the Daisy with the cable going UPWARDS**
   - The ribbon cable has a notch/key - make sure it's oriented correctly
   - Cable orientation matters! If it doesn't work, try flipping it 180°

   ```
   [ST-Link V3 Mini]
         |
         | 14-pin ribbon cable
         ↓
   [Daisy Debug Header] ← Cable goes UP from here
   ```

2. **Power Requirements (IMPORTANT!):**
   - **You MUST provide separate power to the Daisy Pod**
   - Connect a USB cable to the Daisy Pod's USB port (for power)
   - Connect the ST-Link V3 Mini to your computer via USB (for programming)
   - **Both USB connections are required:**
     - Daisy USB = Power supply
     - ST-Link USB = Programming/debugging interface

3. **Verify Connection:**
   ```bash
   probe-rs list
   # Should show: ST-Link V3 Mini and STM32H750IBKx
   ```

4. **Flash Your Program:**
   ```bash
   cargo run --example blink --release
   ```

**Why Separate Power?**
- The ST-Link V3 Mini typically doesn't provide enough current for the Daisy's full operation
- The Daisy Pod has audio codec, SDRAM, and other peripherals that need stable power
- Using separate USB power ensures reliable operation and programming

#### Option 2B: ST-Link V2 with Individual Wires

**Wiring (ST-Link to Daisy Seed):**

The Daisy Seed exposes SWD (Serial Wire Debug) pins:

| ST-Link Pin | Daisy Seed Pin | Signal |
|-------------|----------------|--------|
| SWDIO       | SWDIO          | Data   |
| SWCLK       | SWCLK          | Clock  |
| GND         | GND            | Ground |
| 3.3V        | 3.3V (optional)| Power  |

**Daisy Seed SWD Location:**
- The Daisy Seed has 6 small pads near the USB connector
- Usually labeled: `3V3`, `GND`, `SWDIO`, `SWCLK`, `NRST`, `BOOT0`
- You may need to solder header pins or use pogo pins

**Connection Steps:**
1. Power OFF both devices
2. Connect the 4 wires (SWDIO, SWCLK, GND, and optionally 3.3V)
3. **Recommended:** Don't provide 3.3V from ST-Link, power Daisy via separate USB instead
4. Connect ST-Link to your computer

**Verify Connection:**
```bash
probe-rs list
# Should show: STM32H750IBKx
```

## Checking Your Current Setup

Let's see what's detected:

```bash
# List all connected probes
probe-rs list

# More verbose info
probe-rs info
```

**Expected output with ST-Link V2:**
```
The following debug probes were found:
[0]: STLink V2 (VID: 0483, PID: 3748, Serial: ..., STLink)
```

**Expected output with ST-Link V3 Mini:**
```
The following debug probes were found:
[0]: STLink V3 (VID: 0483, PID: 374e, Serial: ..., STLink)
```

**DFU mode** doesn't show up in `probe-rs list`. Check it with `dfu-util -l` instead.

## Method Comparison

| Feature | USB/DFU | ST-Link V2/V3 |
|---------|---------|---------------|
| Cost | Free (built-in) | $10-20 |
| Speed | Slower | Faster |
| Debugging | No RTT logs | ✓ RTT logs work |
| Reliability | Sometimes flaky | Very reliable |
| Setup | Press buttons | Ribbon cable + dual USB |
| Power | USB only | **Dual USB required** |

## Troubleshooting

### "No probe found" Error

**If using USB/DFU:**
1. Make sure you entered DFU mode (BOOT + RESET sequence)
2. Check USB cable is data-capable (not just power)
3. Try a different USB port
4. On macOS: Check System Report → USB to see if STM32 device appears

**If using ST-Link V3 Mini:**
1. **Check ribbon cable orientation** - Cable should go UPWARDS from Daisy
2. **Verify dual USB power** - BOTH Daisy USB and ST-Link USB must be connected
3. Try reseating the 14-pin ribbon cable (both ends)
4. Verify ST-Link V3 appears in USB devices
5. Try different USB ports
6. Check if ST-Link LED is solid green (connected) or blinking (error)

**If using ST-Link V2 with wires:**
1. Check all 4 wires are connected firmly
2. Verify ST-Link appears in USB devices
3. Try different USB port
4. Check if ST-Link LED is blinking/solid
5. Install ST-Link drivers (Windows) or check udev rules (Linux)

### macOS: Check USB Devices
```bash
system_profiler SPUSBDataType
# Look for "STM32" or "STLink"
```

### Linux: udev Rules for ST-Link
```bash
# Create udev rules
sudo nano /etc/udev/rules.d/99-stlink.rules

# Add these lines:
# ST-Link V2
ATTRS{idVendor}=="0483", ATTRS{idProduct}=="3748", MODE="666"
# ST-Link V2-1
ATTRS{idVendor}=="0483", ATTRS{idProduct}=="374b", MODE="666"
# STM32 DFU
ATTRS{idVendor}=="0483", ATTRS{idProduct}=="df11", MODE="666"

# Reload rules
sudo udevadm control --reload-rules
sudo udevadm trigger
```

## Recommended Approach

**For getting started quickly:**
1. Try USB/DFU mode first (no extra hardware needed)
2. For delay readout via RTT logs, use an ST-Link (V3 Mini verified)

**For serious development:**
- **ST-Link V3 Mini is highly recommended** (compact, works great with Daisy)
- Use 14-pin ribbon cable to debug header (if populated on your Daisy)
- Remember: Cable orientation matters (upwards from Daisy) + dual USB power required
- This enables RTT logging via `defmt` which is incredibly useful for debugging

## Alternative: Web Programmer

Electrosmith also provides a web-based programmer:
- https://electro-smith.github.io/Programmer/
- Requires Chrome/Edge browser
- Uses WebUSB API
- Good for quick testing

## Next Steps

Once connected, run:
```bash
# Test probe detection
probe-rs list

# Flash the blink example
cargo run --example blink --release

# You should see defmt logs if using ST-Link:
# INFO  Daisy Pod Blink Example
# INFO  LED will blink at 300ms intervals
# TRACE LED ON
# TRACE LED OFF
```

## Quick Setup Checklist (ST-Link V3 Mini + Daisy Pod)

- [ ] ST-Link V3 Mini connected to computer via USB
- [ ] 14-pin ribbon cable connected to ST-Link V3 Mini
- [ ] Ribbon cable connected to Daisy debug header (cable going UPWARDS)
- [ ] **Separate USB cable connected to Daisy Pod for power**
- [ ] Run `probe-rs list` to verify detection
- [ ] Run `cargo run --example blink --release` to test

## Connection Summary (Verified Setup)

```
Computer USB Port 1 → ST-Link V3 Mini
                           |
                    14-pin ribbon cable (upward orientation)
                           ↓
                    Daisy Debug Header

Computer USB Port 2 → Daisy USB (for power)
```

Both USB connections are required for reliable operation!
