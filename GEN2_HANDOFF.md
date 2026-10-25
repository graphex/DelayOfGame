# Delay of Game: Gen 1 → Gen 2 Handoff

This document summarizes the first-generation build for planning the second generation. It's written to be dropped into a Claude Project as reference material. Gen 1 source: https://github.com/graphex/DelayOfGame

## The problem

A Broncos game on TV or streaming runs anywhere from a few seconds to over a minute behind live. The radio call (Dave Logan on KOA 850 AM / 94.1 FM / iHeartRadio) is close to live. The goal is to delay the radio audio until it matches the picture, adjustable in real time, from the couch.

## What Gen 1 is

- **Hardware:** Electrosmith Daisy Pod with a Daisy Seed 1.1. STM32H750 Cortex-M7 at 480 MHz, 64 MB SDRAM, WM8731 codec, 48 kHz / 24-bit stereo line in and out.
- **Software:** Rust, `no_std`, Embassy async (`daisy-embassy` 0.2.2), about 350 lines.
- **Features:**
  - Stereo delay from 0 to 60 s
  - Encoder adjusts the delay in 0.1 s steps, or 0.01 s steps after a button press
  - LED heartbeat
  - The delay value can only be read over defmt/RTT logs, which needs an ST-Link and a laptop
- **Core algorithm:** a circular buffer in SDRAM. Write each 32-frame block at `write_pos` and read from `write_pos − delay`. It costs almost no CPU.

## What worked

- Embassy plus `daisy-embassy` got from blink to working delay in about a day. The SDRAM, codec and SAI setup is handled by the board crate.
- A circular buffer with one atomic shared between the control task and the audio callback. Simple and glitch-free in steady state.
- Coarse and fine encoder steps (0.1 s / 0.01 s) suit how syncing actually goes: get close fast, then nudge.
- An ST-Link V3 Mini on the 14-pin header, with separate USB power to the Pod, was reliable for flashing and RTT logs.

## What didn't work, and why

1. **I2C displays caused audio dropouts.** Both an SSD1306 OLED and an HT16K33 LED backpack made the audio buffer overrun.
   - The audio callback has a 0.67 ms deadline per 32-frame block.
   - The I2C drivers blocked for milliseconds (roughly 20–50 ms for full OLED refreshes).
   - Every task, audio included, ran on one cooperative executor, so a blocked display task starved the audio.
   - This is a software-architecture limit, not a CPU-speed limit. Possible fixes on the same chip: run audio on a high-priority `InterruptExecutor`, use DMA/async I2C or SPI, or update the display in small chunks between blocks.
2. **No way to see the delay without a laptop.** This is the main usability gap and the main reason for Gen 2.
3. **Delay was off by 2× until 2026-10-24.** The control code counted frames and the buffer counted interleaved samples, so the real delay was half of what the logs showed and the real maximum was about 30 s. It's fixed in code but not yet re-checked on hardware. Gen 2 should use one unit (frames) everywhere and test it with a known signal.

## Known Gen 1 limitations (carry the fixes into Gen 2)

- **Clicks when the delay changes.** The read pointer jumps. Gen 2 should crossfade (about 5–20 ms) or briefly mute.
- **The delay isn't saved.** It resets to 1 s on every power-up. Gen 2 should save it to flash, ideally with a few presets for different streaming services and channels, which have different lags.
- **60 s maximum** is a constant (`MAX_DELAY_SECONDS`). The Daisy's 64 MB would allow about 174 s with 32-bit samples or about 349 s with 16-bit samples.
- **Encoder pulses aren't accelerated,** so long changes take a lot of turning at 0.1 s per step.
- **Controls only on the device.** No remote or app control.

## Is new hardware actually needed?

Worth deciding deliberately. The Daisy can likely hold 3–6 minutes of delay and drive a display if the firmware is restructured (see "What didn't work" above). New hardware makes sense if Gen 2 also wants several of these:

- A large, readable display: a TFT, or a screen that's readable from the couch
- Network audio input (play the iHeartRadio stream directly instead of an analog radio), Wi-Fi, a phone or web UI, or remote control
- Automatic sync (lining up radio and TV audio by cross-correlation), which needs a second audio input from the TV and more DSP headroom
- An enclosure, a power supply and a non-developer UX

## Gen 2 requirements draft

| Area | Requirement |
|------|-------------|
| Delay range | 0 to at least 120 s (confirm the worst-case streaming lag) at 0.01 s resolution |
| Audio | Stereo line in/out, 48 kHz, 16-bit or better, under 5 ms added latency, no dropouts |
| Delay changes | Click-free (crossfade) |
| Display | Shows the current delay and step size, readable from about 3 m, updates without disturbing audio |
| Controls | Encoder with acceleration, coarse/fine toggle, preset buttons |
| Persistence | Remembers the last delay and at least 4 presets across power cycles |
| Robustness | Audio runs at a higher priority than UI or network. A UI fault can't stop the audio. |
| Verification | Test mode that confirms the actual delay equals the displayed delay |

## Open questions for Gen 2

- What's the longest lag needed? YouTube TV, Hulu Live, cable and OTA all differ.
- Is the audio source an analog radio, or should the device stream KOA itself? Streams have their own variable delay.
- Stay with embedded Rust, or move to a Linux single-board computer, where buffering and displays are trivial but real-time audio and boot time are harder?
- Budget, enclosure and power constraints?
- Is automatic sync in scope, or only manual?

## Reusable from Gen 1

- `src/buffer.rs`: the circular delay buffer (move it to frame units)
- `src/encoder.rs`: the polled quadrature decoder
- The coarse/fine step UX
- `HARDWARE_SETUP.md`: ST-Link V3 Mini setup, if Gen 2 stays on STM32
