# gameboy-rs

An **educational Game Boy (DMG-01) emulator** written in **Rust**. The goal of this project is to deeply understand classic hardware by re‑implementing it faithfully, while keeping the codebase clean, readable, and well‑documented.

This emulator is not focused on performance or commercial use — it is primarily a learning project exploring CPU emulation, memory mapping, graphics pipelines, and timing‑accurate systems programming.

---

## ✨ Features (Current / Planned)

* ✅ LR35902 (Game Boy CPU) instruction decoding & execution
* ✅ CPU registers, flags, and basic timing model
* ⏳ Memory map (ROM, RAM, VRAM, HRAM)
* ⏳ Cartridge loading (MBC0 initially)
* ⏳ PPU (graphics) emulation
* ⏳ LCD modes & scanline timing
* ⏳ Input (joypad)
* ⏳ Timers & interrupts
* ⏳ Audio Processing Unit (APU)
* ⏳ Save files & battery‑backed RAM

Legend:

* ✅ Implemented
* ⏳ In progress / planned

---

## 🎯 Project Goals

* **Accuracy over speed** — emulate hardware behavior as closely as practical
* **Clarity over cleverness** — readable Rust code with explicit intent
* **Strong documentation** — comments explaining *why*, not just *what*
* **Testability** — CPU and subsystem tests using known ROMs and test suites
* **Educational value** — suitable for others learning emulation or Rust

---

## 🧠 Architecture Overview

The emulator is structured into independent subsystems that mirror the original hardware:

* **CPU** – Instruction decoding, execution, registers, flags, and cycles
* **MMU** – Central memory bus handling reads/writes and address mapping
* **PPU** – Pixel processing, scanlines, and LCD state
* **APU** – Audio channels and mixing
* **Timer** – DIV/TIMA registers and clock behavior
* **Interrupts** – IF/IE handling and dispatch
* **Cartridge** – ROM parsing and memory bank controllers (MBCs)

Each component advances according to CPU cycles to maintain correct timing.

---

## 🕹️ Running the Emulator

### Prerequisites

* Rust (stable)
* Cargo

### Build

```bash
cargo build --release
```

### Run

```bash
cargo run --release -- path/to/rom.gb
```

> ⚠️ At early stages, many commercial ROMs may not boot correctly.

---

## 🧪 Testing

Planned testing strategy includes:

* Blargg CPU instruction test ROMs
* Timing test ROMs
* Manual disassembly comparisons
* Unit tests for individual instructions

```bash
cargo test
```

---

## 📚 Learning Resources & References

This project relies heavily on public documentation and community research:

* Z80 / LR35902 instruction reference
  [http://www.z80.info](http://www.z80.info)

* RGBDS Game Boy documentation
  [https://rgbds.gbdev.io/](https://rgbds.gbdev.io/)

* Opcode decoding and disassembly
  [http://searchdatacenter.techtarget.com/tip/Basic-disassembly-Decoding-the-opcode](http://searchdatacenter.techtarget.com/tip/Basic-disassembly-Decoding-the-opcode)

* Pan Docs (Game Boy technical reference)

* [Game Boy Doctor](https://github.com/robert/gameboy-doctor)

* Blargg test ROM documentation

* gbdev.io community resources

---

## 🚧 Project Status

This emulator is **under active development** and should be considered incomplete.
Expect breaking changes, missing features, and inaccurate behavior as development progresses.

---

## ⚠️ Legal Disclaimer

This project does **not** include:

* Game Boy BIOS
* Commercial ROMs

You must provide your own legally obtained ROMs.
Game Boy is a trademark of Nintendo.

---

## 🤝 Contributing

Contributions are welcome!

* Bug reports
* Documentation improvements
* Accuracy fixes
* Test ROM integration

Please open an issue or submit a pull request.

---

## 🧩 Why Rust?

Rust offers:

* Memory safety without a garbage collector
* Excellent enums and pattern matching for opcode decoding
* Strong type system for modeling hardware state
* Great tooling and test support

---

## 📌 License

MIT License for included Rust code. The opcode.json is derived from https://github.com/gbdev/rgbds which is also licensed under MIT.

---

Happy hacking 👾
