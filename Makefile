DATA_DIR := data
OPCODES := $(DATA_DIR)/opcodes.json

.PHONY: build run clean

$(OPCODES):
	mkdir -p $(DATA_DIR)
	curl -L https://gbdev.io/gb-opcodes/Opcodes.json -o $@

build: $(OPCODES)
	cargo build

run: $(OPCODES)
	cargo run

clean:
	cargo clean
	rm -rf $(DATA_DIR)
