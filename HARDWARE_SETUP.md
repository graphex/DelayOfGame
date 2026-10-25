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

**Update the ST-Link firmware first.** On firmware V3J7, probe-rs failed with USB errors ("pipe stalled", "interfaces are claimed"), OpenOCD couldn't read memory, and the probe kept locking up until it was replugged. V3J18 works. Update with ST's free STSW-LINK007 tool (a Java app: `java -jar STLinkUpgrade.jar`, then **Open in update mode** → **Upgrade**). A flaky USB cable to the ST-Link causes similar symptoms, so try another cable too.

**Hardware Required:**
- ST-Link V3 Mini programmer
- 14-pin ribbon cable (usually included with ST-Link V3 Mini)
- Daisy Pod with debug header populated
- **A second USB cable from the Daisy to the computer**

**Connection Steps:**

1. **Connect the Ribbon Cable:**
   - The Daisy Seed's debug header has **10 pins**; the ST-Link's ribbon plug has **14**. Center the plug so one unused position hangs off each end. The outer positions of the 14-pin plug aren't connected.
   - The ST-Link end is shrouded and only fits one way.
   - At the Daisy end, the plug fits either way round and only one works. Hold the Seed so its USB port is on your **left**. The ribbon should then leave the plug pointing **up**, away from you:

```
                  to ST-Link V3 Mini
                             ▲
                       │ │ │ │ │ │ │
 ┌─────────────────────┼─┼─┼─┼─┼─┼─┼──────────────────────┐
 │                     │ │ │ │ │ │ │                      │
 │                  ┌──┴─┴─┴─┴─┴─┴─┴──┐                   │
 │                  │  ○ ● ● ● ● ● ○  │                   │
 │                  │  ○ ● ● ● ● ● ○  │                   │
┌┴─────┐            └─────────────────┘                   │
│ USB  │                                                  │
└┬─────┘                                                  │
 │                                Daisy Seed (top view)   │
 └────────────────────────────────────────────────────────┘

 ● = position on a header pin    ○ = empty position hanging off the end
```

   - **Check it:** `openocd -f interface/stlink.cfg -f target/stm32h7x.cfg -c "init; shutdown"` should report `Target voltage: 3.3` and `Cortex-M7 ... processor detected`. A reading around 4.7 V or 1.7 V means the plug is reversed or off-center. Unplug both USB cables before re-seating it.

2. **Plug both USB cables into the computer (IMPORTANT!):**
   - ST-Link V3 Mini → computer (programming, RTT logs)
   - Daisy Seed's USB port → **the same computer**, not a separate charger or power brick
   - A wall supply alone for the Daisy isn't enough. It needs the computer's USB connection too.

3. **Verify Connection:**
   ```bash
   probe-rs list
   # Should show: ST-Link V3 Mini and STM32H750IBKx
   ```

4. **Flash Your Program:**
   ```bash
   cargo run --example blink --release
   ```

**Why two cables?** The ST-Link V3 Mini doesn't power the target, so the Daisy's own USB supplies it. Plugging that cable into the computer also gives both boards a common ground, and you can switch to DFU without moving cables.

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
1. **Check ribbon cable orientation** against the diagram in Option 2A (Seed USB on the left, ribbon pointing up, plug centered)
2. **Check both USB cables** - the ST-Link AND the Daisy must be plugged into the computer
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
- Remember: ribbon orientation matters (see the diagram in Option 2A), and both the ST-Link and the Daisy must be plugged into the computer
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
- [ ] Ribbon plug centered on the Daisy debug header, ribbon pointing up with the Seed's USB on the left (see diagram)
- [ ] **Daisy USB cable plugged into the same computer** (not a charger)
- [ ] Run `probe-rs list` to verify detection
- [ ] Run `cargo run --example blink --release` to test

## Connection Summary (Verified Setup)

```
Computer USB Port 1 → ST-Link V3 Mini
                           |
                    14-pin ribbon cable (centered, pointing up with Seed USB on the left)
                           ↓
                    Daisy Debug Header

Computer USB Port 2 → Daisy Seed USB (power + common ground)
```

Both cables must go to the computer!
