#!/bin/sh
# Attach to the running Daisy over the ST-Link (no reflash, no reset) and
# stream its defmt logs. Ctrl-C detaches; the Daisy keeps running.
#
# Usage: scripts/logs.sh [elf]   (defaults to the release build)
cd "$(dirname "$0")/.." || exit 1
exec probe-rs attach --chip STM32H750IBKx "${1:-target/thumbv7em-none-eabihf/release/delay-of-game}"
