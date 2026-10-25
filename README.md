# Delay of Game

A stereo audio delay that lines up Dave Logan's Broncos radio call (KOA 850 AM / 94.1 FM / iHeartRadio) with the TV picture. It runs on an Electrosmith Daisy Pod and is written in Rust with Embassy.

This is the **first generation** of the project. It works and is still getting upgrades. [GEN2_HANDOFF.md](GEN2_HANDOFF.md) has the lessons learned and requirements for the next hardware generation.

## Using it

1. Plug the radio source into the Pod's **input** jack and the speakers or receiver into the **output** jack.
2. Power the Pod over USB. The LED blinks quickly during startup, then pulses once per second.
3. **Turn the encoder** to change the delay. It starts at 1.00 s and goes from 0 to 170 s.
4. **Press the encoder** to switch between 0.1 s (coarse) and 0.01 s (fine) steps.
5. **Knob 1** sets the output volume, from mute to full. Leave it fully clockwise if the output feeds a receiver that has its own volume control.

There's no display. To read the current delay, connect an ST-Link and run `scripts/logs.sh`. It attaches without reflashing; expect a brief audio blip when it connects.

```
INFO Delay: 12.40s
INFO Step size: 0.01s
INFO Volume: 64%
```

The delay goes back to 1 s every time the Pod powers up.

## Building and flashing

```bash
# One-time setup
rustup target add thumbv7em-none-eabihf     # also handled by rust-toolchain.toml
cargo install probe-rs-tools --locked

# Build, flash and stream logs over an ST-Link
cargo run --release

# LED-only smoke test
cargo run --example blink --release
```

The default build targets Daisy Seed 1.1 (WM8731 codec). For a Seed 1.2 (PCM3060):

```bash
cargo run --release --no-default-features --features seed_1_2
```

See [HARDWARE_SETUP.md](HARDWARE_SETUP.md) for probe wiring (a verified ST-Link V3 Mini setup) and flashing over USB DFU without a probe.

## Hardware

- Electrosmith Daisy Pod with a Daisy Seed 1.1: STM32H750 at 480 MHz, 64 MB SDRAM, WM8731 codec
- 48 kHz, 24-bit stereo, line in and line out on 1/8" jacks
- Pod rotary encoder with push button (pins D26, D25, D13); Knob 1 (D21) for volume

## Go Broncos! 🏈
