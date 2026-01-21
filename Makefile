# Paths
TRUTH_DIR = testing_tools/gameboy-doctor/truth
ZIPPED_DIR = $(TRUTH_DIR)/zipped/cpu_instrs
UNZIPPED_DIR = $(TRUTH_DIR)/unzipped/cpu_instrs

# List of expected output files (the unzipped versions)
# This converts '1.zip' to 'testing_tools/.../unzipped/cpu_instrs/1.log'
# Note: Adjust the extension (.txt, .log) to match what's inside the zips!
NUMBERS = 1 2 3 4 5 6 7 8 9 10 11
TRUTH_FILES = $(patsubst %, $(UNZIPPED_DIR)/%.log, $(NUMBERS))

ROM_1  = 01-special.gb
ROM_2  = 02-interrupts.gb
ROM_3  = 03-op sp,hl.gb
ROM_4  = 04-op r,imm.gb
ROM_5  = 05-op rp.gb
ROM_6  = 06-ld r,r.gb
ROM_7  = 07-jr,jp,call,ret,rst.gb
ROM_8  = 08-misc instrs.gb
ROM_9  = 09-op r,r.gb
ROM_10 = 10-bit ops.gb
ROM_11 = 11-op a,(hl).gb

ROM_DIR = ./testing_tools/gb-test-roms/cpu_instrs/individual/


.PHONY: all clean unzip_truth build run clean release test-all

all: unzip_truth

# The main task: ensure all truth files exist
unzip_truth: $(UNZIPPED_DIR) $(TRUTH_FILES)

# Create the output directory if it doesn't exist
$(UNZIPPED_DIR):
	mkdir -p $(UNZIPPED_DIR)

# Pattern Rule: How to create a .txt file from a .zip file
# % acts as a wildcard
$(UNZIPPED_DIR)/%.log: $(ZIPPED_DIR)/%.zip
	@echo "Unzipping $<..."
	unzip -p $< > $@

clean:
	rm -rf $(UNZIPPED_DIR)
	cargo clean

build:
	cargo build

# Pattern rule for running the doctor
# Example: 'make doctor-2'
doctor-%: $(UNZIPPED_DIR)/%.log
	@echo "Testing ROM: $(ROM_$*)"
	@RUST_BACKTRACE=1 cargo run --quiet --release --features doctor -- \
		--golden-log $< \
		--log-path /tmp/doctor-output.log \
		--load-rom "$(ROM_DIR)/$(ROM_$*)"


# Convert the numbers into target names: doctor-1, doctor-2, etc.
ALL_TARGETS = $(patsubst %, doctor-%, $(NUMBERS))

# The main command to run everything
test-all: $(ALL_TARGETS)
	@echo "--------------------------------------"
	@echo "🎉 ALL DOCTOR TESTS PASSED SUCCESSFULLY!"
	@echo "--------------------------------------"

release:
	cargo build --release
	cargo  install cargo-strip
	cargo strip
	strip target/release/gameboy_rs

