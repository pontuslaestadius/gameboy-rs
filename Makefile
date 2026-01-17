
.PHONY: build run clean release


build:
	cargo build

release:
	cargo build --release
	cargo  install cargo-strip
	cargo strip
	strip target/release/gameboy_rs

