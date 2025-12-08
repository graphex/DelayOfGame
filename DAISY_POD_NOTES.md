# Daisy Pod with daisy-embassy

## Important: Daisy Pod Compatibility

The `daisy-embassy` crate officially supports:
- Daisy Seed 1.1 (Rev5) - WM8731 codec
- Daisy Seed 1.2 (Rev7) - PCM3060 codec
- Daisy Seed (Rev4) - AK4556 codec
- Daisy Patch SM - PCM3060 codec

**The Daisy Pod is NOT explicitly listed**, but it's essentially a **Daisy Seed carrier board** with:
- Same STM32H750 MCU
- Same SDRAM (64MB)
- Same QSPI Flash
- Audio codec (depends on which Seed is installed)
- **Additional**: Knobs, buttons, encoder, gate inputs on the Pod PCB

## Determining Your Codec

The Daisy Pod's audio codec depends on which Daisy Seed version is installed:

1. **Check your Daisy Seed revision**:
   - Look at the silkscreen on the Seed board itself
   - Or check your purchase date from Electrosmith

2. **Select the appropriate feature flag**:
   ```bash
   # For Rev5 with WM8731 codec:
   cargo run --release --features seed_1_1

   # For Rev7 with PCM3060 codec:
   cargo run --release --features seed_1_2
   ```

## If You're Unsure

Try this approach:
1. Start with the default build (no features)
2. If that doesn't work, try `--features seed_1_1`
3. If still no audio, try `--features seed_1_2`

The blink example should work regardless of codec selection since it only uses GPIO.

## Pod-Specific Features

Once we get audio working, we'll need to configure:
- **Knob 1** (Pot 1): Delay amount control - likely on ADC pin
- **Knob 2** (Pot 2): Could use for delay increment mode or mix
- **Encoder**: Could use for fine delay adjustment
- **Buttons**: Mode selection, bypass, etc.
- **Gate Inputs**: Could trigger delay presets

These controls are on the Pod carrier board and should be accessible via GPIO/ADC regardless of the Seed version.

## Next Steps

1. **First**: Get the blink example running to verify toolchain
2. **Then**: Determine which codec feature to use
3. **Finally**: Test audio passthrough before adding delay logic

## References

- [Daisy Pod Hardware](https://github.com/electro-smith/Hardware/tree/master/pod)
- [Daisy Seed Revisions](https://github.com/electro-smith/DaisyWiki/wiki/1b.-Daisy-Seed-Overview)
- [daisy-embassy](https://github.com/daisy-embassy/daisy-embassy)
