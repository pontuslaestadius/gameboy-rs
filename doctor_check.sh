#!/bin/bash

# Configuration
ROM_PATH="${HOME}/roms/blargg/cpu_instrs/individual/10-bit ops.gb"
LOG_FILE="./log.txt"
DURATION=5

echo "--- Starting Emulator for ${DURATION} seconds ---"

rm "${LOG_FILE}"

# 1. Run the emulator in the background and redirect stdout to LOG_FILE
# We use 'timeout' to automatically kill it after X seconds
cargo build --quiet > /dev/null 2>&1
timeout --foreground $DURATION ./target/debug/gameboy_rs --load-rom "${ROM_PATH}" --log-path "${LOG_FILE}"

# 2. Check if the log file was actually created
if [ ! -s "$LOG_FILE" ]; then
    echo "Error: Log file is empty or emulator failed to run."
    exit 1
fi

echo "--- Emulator Finished. Running Gameboy Doctor ---"

# 3. Run the doctor tool
python3 "${HOME}/gameboy-doctor/gameboy-doctor" "${LOG_FILE}" cpu_instrs 10
