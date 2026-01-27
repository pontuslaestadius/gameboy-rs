mod immediate;
mod instruction_set;
mod operand;
mod register;
mod snapshot;
mod step_flow_controller_enum;

use crate::mmu::Memory;
use crate::*;
use log::{debug, info};
pub use snapshot::CpuSnapshot;
use std::fmt;
use step_flow_controller_enum::StepFlowController;

#[derive(Debug)]
pub struct Cpu {
    // 8-bit Registers
    pub a: u8,
    pub f: u8, // Flags Register
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,

    // 16-bit Special Registers
    pub pc: u16, // Program Counter
    pub sp: u16, // Stack Pointer

    // Internal state
    pub halted: bool,

    pub ime: bool, // Interrupt Master Enable (The "Master Switch")

    // Use an enum or a counter for the EI delay.
    // Since it's exactly one instruction, a simple 0, 1, 2 counter works well.
    pub ime_scheduled: u8,

    halt_bug_triggered: bool,
}

struct AluResult {
    value: u8,
    z: bool,
    n: bool,
    h: bool,
    c: bool,
}

impl Default for Cpu {
    fn default() -> Self {
        Self::new()
    }
}

impl Cpu {
    pub fn new() -> Self {
        debug!("Creating CPU");
        Self {
            // These values are standard for the GB after the boot ROM runs
            a: 0x01,
            f: 0xB0, // Flags
            b: 0x00,
            c: 0x13,
            d: 0x00,
            e: 0xD8,
            h: 0x01,
            l: 0x4D,
            pc: 0x0100, // Entry point for cartridges
            sp: 0xFFFE,
            halted: false,
            ime: false,
            ime_scheduled: 0,
            halt_bug_triggered: false,
        }
    }
    pub fn reset_post_boot(&mut self) {
        self.a = 0x01;
        // F = 0xB0 -> Z:1, N:0, H:1, C:1 (Upper nibble is 1011)
        self.f = 0xB0;
        self.b = 0x00;
        self.c = 0x13;
        self.d = 0x00;
        self.e = 0xD8;
        self.h = 0x01;
        self.l = 0x4D;
        self.sp = 0xFFFE;
        self.pc = 0x0100; // The standard entry point for all cartridges
    }
    fn service_interrupt(&mut self, bit: u8, bus: &mut impl Memory) {
        assert!(bit < 5, "Service interrupt only supports bit < 5.");
        assert!(self.ime, "Clearing interrupt bit while IME is disabled!");
        // 1. Disable interrupts to prevent recursive nesting
        self.ime = false;

        // 2. Acknowledge the interrupt by clearing the specific bit in IF
        // Note: Use bitmask clearing, not XOR, to be safe.
        let mut if_reg = bus.read_if();
        if_reg &= !(1 << bit);
        // info!("service_interrupt: set if to {if_reg}");
        bus.write_if(if_reg);

        // 3. Push current PC onto the stack
        // The stack grows downwards, so we decrement SP before each write.
        let pc_high = (self.pc >> 8) as u8;
        let pc_low = (self.pc & 0xFF) as u8;

        self.sp = self.sp.wrapping_sub(1);
        bus.write_byte(self.sp, pc_high);
        // info!("service_interrupt: addr: {} = {}", self.sp, pc_high);

        self.sp = self.sp.wrapping_sub(1);
        bus.write_byte(self.sp, pc_low);
        // info!("service_interrupt: addr: {} = {}", self.sp, pc_low);

        // 4. Jump to the vector address
        // Priority: V-Blank (0x40), LCD (0x48), Timer (0x50), Serial (0x58), Joypad (0x60)

        self.pc = match bit {
            0 => ADDR_VEC_VBLANK,
            1 => ADDR_VEC_LCD_STAT,
            2 => ADDR_VEC_TIMER,
            3 => ADDR_VEC_SERIAL,
            4 => ADDR_VEC_JOYPAD,
            _ => panic!("Should not be possible."), // Should never happen
        };
    }

    fn apply_flags(&mut self, spec: &FlagSpec, res: InstructionResult) {
        self.update_single_flag(FLAG_Z, spec.z, res.z);
        self.update_single_flag(FLAG_N, spec.n, res.n); // N is almost always hardcoded in Spec
        self.update_single_flag(FLAG_H, spec.h, res.h);
        self.update_single_flag(FLAG_C, spec.c, res.c);
    }

    fn update_single_flag(&mut self, bit: u8, action: FlagAction, proposed: bool) {
        match action {
            FlagAction::Calculate => self.set_flag(bit, proposed),
            FlagAction::Set => self.set_flag(bit, true),
            FlagAction::Reset => self.set_flag(bit, false),
            FlagAction::Invert => {
                let current = self.get_flag(bit);
                self.set_flag(bit, !current);
            }
            FlagAction::None => {} // Instruction doesn't touch this flag. Keep old value.
        }
    }
    fn alu_8bit_add(&self, a: u8, b: u8, use_carry: bool) -> AluResult {
        let c_in = if use_carry && self.get_flag(FLAG_C) {
            1
        } else {
            0
        };

        let res = (a as u16) + (b as u16) + (c_in as u16);
        let res_u8 = res as u8;

        // Half-Carry: Carry out of bit 3 into bit 4
        // We check if the sum of the lower nibbles exceeds 0xF
        let h_bit = (a & 0x0F) + (b & 0x0F) + c_in > 0x0F;

        AluResult {
            value: res_u8,
            z: res_u8 == 0,
            n: false,
            h: h_bit,
            c: res > 0xFF,
        }
    }

    fn alu_8bit_sub(&self, a: u8, b: u8, use_carry: bool) -> AluResult {
        let c_in = if use_carry && self.get_flag(FLAG_C) {
            1
        } else {
            0
        };

        // Standard subtraction result
        let res = (a as i16) - (b as i16) - (c_in as i16);
        let res_u8 = res as u8;

        // Half-Carry (Half-Borrow): Set if there is no borrow from bit 4.
        // In GB terms: bit 3 of 'a' was less than (bit 3 of 'b' + c_in)
        let h_bit = (a & 0x0F) < (b & 0x0F) + c_in;

        // Carry (Borrow): Set if the result is negative (a borrow from bit 8)
        let c_bit = (a as u16) < (b as u16) + (c_in as u16);

        AluResult {
            value: res_u8,
            z: res_u8 == 0,
            n: true,
            h: h_bit,
            c: c_bit,
        }
    }

    pub fn set_c(&mut self, value: bool) {
        self.set_flag(FLAG_C, value);
    }

    pub fn set_h(&mut self, value: bool) {
        self.set_flag(FLAG_H, value);
    }

    pub fn set_n(&mut self, value: bool) {
        self.set_flag(FLAG_N, value);
    }
    pub fn set_z(&mut self, value: bool) {
        self.set_flag(FLAG_Z, value);
    }

    pub fn get_flag(&self, flag: u8) -> bool {
        (self.f & flag) != 0
    }

    pub fn set_flag(&mut self, flag: u8, value: bool) {
        if value {
            self.f |= flag;
        } else {
            self.f &= !flag;
        }
    }

    pub fn request_ime(&mut self) {
        self.ime_scheduled = 1;
    }

    /// Read the current opcode without mutating the current state.
    /// Returns the OpcodeInfo, and the number of bytes to move the pc forward)
    pub fn get_current_opcode(&self, bus: &impl Memory) -> (OpcodeInfo, u8) {
        let opcode = bus.read_byte(self.pc);

        if opcode == CB_PREFIX_OPCODE_BYTE {
            // If the halt bug is active, the PC didn't move,
            // but for logging, we still need to know the CB instruction.
            let cb_opcode = bus.read_byte(self.pc.wrapping_add(1));
            let info = CB_OPCODES[cb_opcode as usize].expect("Invalid CB opcode");
            (info, 2) // It's a 2-byte instruction
        } else {
            let info = OPCODES[opcode as usize].expect("Invalid opcode");
            (info, 1) // It's a 1-byte instruction
        }
    }

    /// Return number of cycles.
    pub fn step(&mut self, bus: &mut impl Memory) -> u8 {
        let mut total_cycles: u8 = 0;

        // 1. Handle Halt Logic
        if let StepFlowController::EarlyReturn(n) = self.handle_halt_logic(bus) {
            // info!("step: halt early exit");
            // bus.tick_components(n as u8);
            return n;
        }

        // 2. Handle Interrupt Hijack
        if bus.pending_interrupt() && self.ime
            && let StepFlowController::EarlyReturn(n) = self.handle_interrupts(bus) {
                // Add the 20 hijack cycles to our total for this step
                total_cycles += n;
                // DO NOT return here.
                // PC is now at the vector (e.g., 0x0050).
                // We want to fall through and execute the instruction at 0x0050 now.
            }

        self.update_ime_delay();

        // 3. Log State (Optional: Place your logger here)
        // At this point, if an interrupt fired, PC is 0x0050.
        // If no interrupt fired, PC is the original address.

        // 4. Fetch and Execute (Either the original instruction OR the ISR instruction)
        // We add these cycles to our hijack cycles
        total_cycles += self.fetch_and_execute(bus);

        total_cycles
    }

    fn handle_interrupts(&mut self, bus: &mut impl Memory) -> StepFlowController {
        if !self.ime {
            return StepFlowController::Continue;
        }

        let pending = bus.read_ie() & bus.read_if();
        if pending == 0 {
            return StepFlowController::Continue;
        }

        let bit = pending.trailing_zeros() as u8;
        // info!("handle_interrupts: bit {bit}");
        if bit < 5 {
            self.service_interrupt(bit, bus); // Pushes PC, jumps to vector, clears IF bit
            // Hijack successful
            return StepFlowController::EarlyReturn(20);
        }
        StepFlowController::Continue
    }
    fn update_ime_delay(&mut self) {
        if self.ime_scheduled > 0 {
            self.ime_scheduled -= 1;
            if self.ime_scheduled == 0 {
                // info!("self.ime = true");
                self.ime = true;
            }
        }
    }
    fn fetch_and_execute(&mut self, bus: &mut impl Memory) -> u8 {
        let opcode = self.fetch_byte(bus);

        let op = if opcode == CB_PREFIX_OPCODE_BYTE {
            let cb = self.fetch_byte(bus);
            CB_OPCODES[cb as usize]
        } else {
            OPCODES[opcode as usize]
        };

        if let Some(code) = op {
            // info!("if: {}, ie: {}", bus.read_if(), bus.read_ie());
            // info!("{}", self);
            // info!("Dispatching: {}, bytes: {}", code, code.bytes);
            let result = self.dispatch(code, bus);
            self.apply_flags(&code.flags, result);
            result.cycles
        } else {
            panic!("Shouldn't happen.");
        }
    }

    fn handle_halt_logic(&mut self, bus: &mut impl Memory) -> StepFlowController {
        if self.halted {
            if bus.pending_interrupt() {
                self.halted = false;
            } else {
                return StepFlowController::EarlyReturn(4);
            }
        }

        StepFlowController::Continue
    }
    fn fetch_byte(&mut self, bus: &mut impl Memory) -> u8 {
        let byte = bus.read_byte(self.pc);
        if self.halt_bug_triggered {
            info!("Halt bug triggered");
            self.halt_bug_triggered = false;
            // Do NOT increment PC. Next fetch will get the same byte.
        } else {
            self.pc = self.pc.wrapping_add(1);
        }
        byte
    }

    fn calculate_dec_8bit(&self, value: u8) -> (u8, bool, bool, bool) {
        let res = value.wrapping_sub(1);

        // Flags:
        let z = res == 0;
        let n = true; // Always true for DEC
        // Half-Carry: Set if there was a borrow from bit 4
        // (i.e., the lower nibble was 0x0 before the decrement)
        let h = (value & 0x0F) == 0;

        (res, z, n, h)
    }
    fn check_condition(&self, target: Target) -> bool {
        match target {
            Target::Condition(cond) => match cond {
                Condition::NotZero => (self.f & FLAG_Z) == 0,
                Condition::Zero => (self.f & FLAG_Z) != 0,
                Condition::NotCarry => (self.f & FLAG_C) == 0,
                Condition::Carry => (self.f & FLAG_C) != 0,
                // _ => true, // Always true for unconditional
            },
            _ => true, // Not a conditional target
        }
    }

    fn get_src_val(&mut self, instruction: &OpcodeInfo, bus: &mut impl Memory) -> u8 {
        if instruction.operands.len() == 1 {
            // Single operand instructions (like INC C or POP BC)
            let (target, _) = instruction.operands[0];
            self.read_target(target, bus).as_u8()
        } else {
            // Two operand instructions (like CP A, n8 or OR A, C)
            // Usually, the second one (index 1) is the "Source"
            let (target, _) = instruction.operands[1];
            self.read_target(target, bus).as_u8()
        }
    }
    /// Reads the actual value for a given operand target.
    /// This may increment PC if it reads immediate values from memory.
    fn read_target(&mut self, target: Target, bus: &mut impl Memory) -> OperandValue {
        match target {
            Target::Register8(reg) => OperandValue::U8(self.get_reg8(reg)),

            Target::Register16(reg) => OperandValue::U16(self.get_reg16(reg)),

            Target::Immediate8 => {
                let val = bus.read_byte(self.pc);
                self.pc = self.pc.wrapping_add(1);
                OperandValue::U8(val)
            }

            Target::Immediate16 => {
                let val = bus.read_u16(self.pc);
                self.pc = self.pc.wrapping_add(2);
                OperandValue::U16(val)
            }

            // Memory access: (HL), (BC), (DE)
            Target::AddrRegister16(reg) => {
                let addr = self.get_reg16(reg);
                OperandValue::U8(bus.read_byte(addr))
            }
            Target::AddrRegister8(_) => todo!(),

            // LDH (a8) - High RAM access (0xFF00 + immediate byte)
            Target::AddrImmediate8 => {
                let offset = bus.read_byte(self.pc) as u16;
                self.pc = self.pc.wrapping_add(1);
                OperandValue::U8(bus.read_byte(0xFF00 | offset))
            }

            // (nn) - 16-bit address read
            Target::AddrImmediate16 => {
                let addr = bus.read_u16(self.pc);
                self.pc = self.pc.wrapping_add(2);
                OperandValue::U8(bus.read_byte(addr))
            }
            // 1. Indirect Read with Side Effects (e.g., LD A, (HL+))
            Target::AddrRegister16Increment(reg) => {
                let addr = self.get_reg16(reg);
                let val = bus.read_byte(addr);
                self.set_reg16(reg, addr.wrapping_add(1)); // Increment side effect
                OperandValue::U8(val)
            }
            Target::AddrRegister16Decrement(reg) => {
                let addr = self.get_reg16(reg);
                let val = bus.read_byte(addr);
                self.set_reg16(reg, addr.wrapping_sub(1)); // Decrement side effect
                OperandValue::U8(val)
            }

            // 2. Relative Offset (JR instructions)
            Target::Relative8 => {
                let val = bus.read_byte(self.pc) as i8; // Cast to signed immediately
                self.pc = self.pc.wrapping_add(1);
                OperandValue::I8(val) // You need an I8 variant in OperandValue
            }

            // 3. Conditions (Check flags)
            Target::Condition(cond) => {
                let met = match cond {
                    Condition::NotZero => !self.get_flag(FLAG_Z),
                    Condition::Zero => self.get_flag(FLAG_Z),
                    Condition::NotCarry => !self.get_flag(FLAG_C),
                    Condition::Carry => self.get_flag(FLAG_C),
                };
                OperandValue::Bool(met) // You need a Bool variant in OperandValue
            }

            // 4. RST Vectors
            Target::Vector(v) => OperandValue::U16(v as u16),

            Target::StackPointer => OperandValue::U16(self.sp),

            Target::Bit(b) => OperandValue::U8(b),
        }
    }

    pub fn write_target(&mut self, target: Target, value: OperandValue, mmu: &mut impl Memory) {
        match (target, value) {
            (Target::Register8(reg), OperandValue::U8(v)) => self.set_reg8(reg, v),
            (Target::Register16(reg), OperandValue::U16(v)) => self.set_reg16(reg, v),
            (Target::StackPointer, OperandValue::U16(v)) => self.sp = v,
            // Matches (HL), (BC), or (DE)
            // a16 is a common write target (e.g., LD (a16), SP)
            (Target::AddrRegister16(reg), OperandValue::U8(v)) => {
                let addr = self.get_reg16(reg);
                mmu.write_byte(addr, v);
            }

            (Target::AddrRegister16Decrement(reg), OperandValue::U8(v)) => {
                let addr = self.get_reg16(reg);
                mmu.write_byte(addr, v);

                // The side effect: decrement the pointer
                let new_val = addr.wrapping_sub(1);
                self.set_reg16(reg, new_val);
            }
            (Target::AddrRegister16Increment(reg), OperandValue::U8(v)) => {
                let addr = self.get_reg16(reg);
                mmu.write_byte(addr, v);

                // The side effect: increment the pointer
                let new_val = addr.wrapping_add(1);
                self.set_reg16(reg, new_val);
            }

            (Target::AddrImmediate16, value) => {
                // Read the 16-bit address (LSB first)
                let low = mmu.read_byte(self.pc) as u16;
                let high = mmu.read_byte(self.pc.wrapping_add(1)) as u16;
                let addr = (high << 8) | low;
                self.pc = self.pc.wrapping_add(2);

                match value {
                    OperandValue::U8(v) => mmu.write_byte(addr, v),
                    OperandValue::U16(v) => {
                        // e.g., LD (a16), SP writes 16 bits
                        mmu.write_byte(addr, (v & 0xFF) as u8);
                        mmu.write_byte(addr.wrapping_add(1), (v >> 8) as u8);
                    }
                    _ => todo!(),
                }
            }

            (Target::AddrImmediate8, v) => {
                // 1. Read the 8-bit offset following the opcode
                let offset = mmu.read_byte(self.pc);
                self.pc = self.pc.wrapping_add(1);

                // 2. Construct the High RAM address
                let addr = 0xFF00 | (offset as u16);

                // 3. Write the 8-bit value to that address
                mmu.write_byte(addr, v.as_u8());
            }
            _ => panic!(
                "write_target: Invalid write target or value mismatch, {:?}, {:?}",
                target, value
            ),
        }
    }

    pub fn get_reg16(&self, reg: Reg16) -> u16 {
        match reg {
            Reg16::BC => u16::from_be_bytes([self.b, self.c]),
            Reg16::DE => u16::from_be_bytes([self.d, self.e]),
            Reg16::HL => u16::from_be_bytes([self.h, self.l]),
            Reg16::SP => self.sp,
            Reg16::AF => u16::from_be_bytes([self.a, self.f]),
            _ => panic!("Cannot get PC reg."),
        }
    }

    pub fn set_reg16(&mut self, reg: Reg16, val: u16) {
        let bytes = val.to_be_bytes();
        match reg {
            Reg16::BC => {
                self.b = bytes[0];
                self.c = bytes[1];
            }
            Reg16::DE => {
                self.d = bytes[0];
                self.e = bytes[1];
            }
            Reg16::HL => {
                self.h = bytes[0];
                self.l = bytes[1];
            }
            Reg16::SP => self.sp = val,
            Reg16::AF => {
                self.a = bytes[0];
                // Note: The lower 4 bits of the F register are always 0 on Game Boy
                self.f = bytes[1] & 0xF0;
            }
            _ => panic!("Cannot get PC reg."),
        }
    }

    pub fn get_reg8(&self, reg: Reg8) -> u8 {
        match reg {
            Reg8::A => self.a,
            Reg8::B => self.b,
            Reg8::C => self.c,
            Reg8::D => self.d,
            Reg8::E => self.e,
            Reg8::H => self.h,
            Reg8::L => self.l,
        }
    }

    pub fn set_reg8(&mut self, reg: Reg8, val: u8) {
        match reg {
            Reg8::A => self.a = val,
            Reg8::B => self.b = val,
            Reg8::C => self.c = val,
            Reg8::D => self.d = val,
            Reg8::E => self.e = val,
            Reg8::H => self.h = val,
            Reg8::L => self.l = val,
        }
    }

    /// Reads a 16-bit value from the current Stack Pointer and increments SP by 2.
    /// Little-Endian: The byte at SP is the low byte, SP+1 is the high byte.
    pub fn pop_u16(&mut self, bus: &impl Memory) -> u16 {
        let low = bus.read_byte(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);

        let high = bus.read_byte(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);

        (high << 8) | low
    }

    /// Decrements SP by 2 and writes a 16-bit value to the stack.
    /// Little-Endian: The high byte goes to SP-1, the low byte goes to SP-2.
    pub fn push_u16(&mut self, bus: &mut impl Memory, val: u16) {
        let high = (val >> 8) as u8;
        let low = (val & 0xFF) as u8;

        self.sp = self.sp.wrapping_sub(1);
        bus.write_byte(self.sp, high);

        self.sp = self.sp.wrapping_sub(1);
        bus.write_byte(self.sp, low);
    }

    fn get_reg16_from_target(&self, target: Target) -> u16 {
        match target {
            Target::Register16(reg) => self.get_reg16(reg),
            Target::StackPointer => self.sp,
            // Add any other 16-bit targets your build.rs might generate
            _ => panic!("Target {:?} is not a 16-bit register", target),
        }
    }
    fn set_reg16_from_target(&mut self, target: Target, value: u16) {
        match target {
            Target::Register16(reg) => self.set_reg16(reg, value),
            Target::StackPointer => self.sp = value,
            _ => panic!("Target {:?} is not a 16-bit register", target),
        }
    }

    pub fn take_snapshot(&self, bus: &impl Memory) -> CpuSnapshot {
        CpuSnapshot {
            a: self.a,
            f: self.f,
            b: self.b,
            c: self.c,
            d: self.d,
            e: self.e,
            h: self.h,
            l: self.l,
            sp: self.sp,
            pc: self.pc,

            // n a very accurate emulator, reading 4 bytes at $PC$ every single step might
            // technically trigger "bus reads" that shouldn't happen (if you have
            // side-effect-heavy hardware mapped to memory). For debugging purposes, this
            // is usually fine, but ensure your bus.read() for the snapshot doesn't
            // accidentally "consume" or "trigger" hardware events (clearing a serial flag).
            pcmem: [
                bus.read_byte(self.pc),
                bus.read_byte(self.pc.wrapping_add(1)),
                bus.read_byte(self.pc.wrapping_add(2)),
                bus.read_byte(self.pc.wrapping_add(3)),
            ],
        }
    }
}

impl fmt::Display for Cpu {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Format Flags: [ZNHC] (uppercase if set, lowercase/dash if clear)
        let z = if self.get_flag(FLAG_Z) { 'Z' } else { '-' };
        let n = if self.get_flag(FLAG_N) { 'N' } else { '-' };
        let h = if self.get_flag(FLAG_H) { 'H' } else { '-' };
        let c = if self.get_flag(FLAG_C) { 'C' } else { '-' };

        write!(
            f,
            "A:{:02X} B:{:02X} C:{:02X} D:{:02X} E:{:02X} H:{:02X} L:{:02X} PC:{:04X} SP:{:04X} Flags:[{}{}{}{}] IME:{} HALT:{} BUG:{}",
            self.get_reg8(Reg8::A),
            self.get_reg8(Reg8::B),
            self.get_reg8(Reg8::C),
            self.get_reg8(Reg8::D),
            self.get_reg8(Reg8::E),
            self.get_reg8(Reg8::H),
            self.get_reg8(Reg8::L),
            self.pc,
            self.sp,
            z,
            n,
            h,
            c,
            if self.ime { "ON" } else { "OFF" },
            if self.halted { "YES" } else { "NO " },
            if self.halt_bug_triggered { "!" } else { "." }
        )
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::input::DummyInput;

    fn bootstrap() -> (Cpu, Bus<DummyInput>) {
        // RUST_LOG=trace cargo test cpu::test::test_ei_delay_timing -- --nocapture
        // let _ = env_logger::builder().is_test(true).try_init();
        let bus: Bus<DummyInput> = Bus::new(Vec::new()); // Your memory/system component
        let cpu = Cpu::new();
        (cpu, bus)
    }

    #[test]
    fn test_ei_delay_timing() {
        let (mut cpu, mut bus) = bootstrap();

        // 1. Execute EI (Opcode 0xFB)
        cpu.pc = 0x100;
        bus.force_write_byte(0x100, 0xFB); // EI
        cpu.step(&mut bus);

        // After EI, IME should still be false, but scheduled
        assert!(!cpu.ime, "IME should not be enabled immediately after EI");
        assert_eq!(
            cpu.ime_scheduled, 1,
            "IME should be scheduled for next step"
        );

        // 2. Execute a NOP (Opcode 0x00)
        bus.write_byte(0x101, 0x00);
        cpu.step(&mut bus);

        // After the instruction FOLLOWING EI, IME becomes true
        assert!(
            cpu.ime,
            "IME should be enabled after the instruction following EI"
        );
        assert_eq!(cpu.ime_scheduled, 0);
    }

    #[test]
    fn test_ei_timing_strict() {
        let (mut cpu, mut bus) = bootstrap();

        // 1. Execute EI
        cpu.pc = 0x100;
        bus.force_write_byte(0x100, 0xFB); // EI
        cpu.step(&mut bus);

        // After EI finishes, the 'delay' should be primed
        // If your step decrements BEFORE fetch, this should be 1.
        assert_eq!(
            cpu.ime_scheduled, 1,
            "IME should be 1 step away from enabling"
        );
        assert!(!cpu.ime, "IME should still be false");

        // 2. Execute any other instruction (e.g., NOP)
        bus.write_byte(0x101, 0x00);
        cpu.step(&mut bus);

        // Now IME must be true
        assert!(
            cpu.ime,
            "IME should have enabled after this instruction finished"
        );
    }

    #[test]
    fn test_interrupt_trigger_timing_sequence() {
        let (mut cpu, mut bus) = bootstrap();

        // 1. Initialize to a known clean state
        cpu.pc = 0x100;
        cpu.sp = 0xDFFD;
        cpu.ime = false;
        cpu.ime_scheduled = 0;
        cpu.a = 0x01; // Value to be written to IF

        // Setup: Enable V-Blank in IE (0xFFFF)
        bus.write_byte(0xFFFF, 0x01);

        // --- STEP 1: EI (FB) ---
        bus.write_byte(0x100, 0xFB);
        cpu.step(&mut bus);

        assert_eq!(cpu.pc, 0x101, "PC should move to next instr");
        assert!(!cpu.ime, "IME should not be active yet");
        assert_eq!(cpu.ime_scheduled, 1, "IME should be scheduled");

        // --- STEP 2: LDH (0xFF0F), A (E0 0F) ---
        // This instruction enables the interrupt flag.
        bus.write_byte(0x101, 0xE0);
        bus.write_byte(0x102, 0x0F);
        cpu.step(&mut bus);

        assert_eq!(cpu.pc, 0x103, "PC should move past LDH");
        assert!(
            cpu.ime,
            "IME should enable AFTER the instruction following EI"
        );
        assert_eq!(bus.read_byte(0xFF0F), 0x01, "IF should now be set");

        // --- STEP 3: THE INTERRUPT HIJACK ---
        // The CPU is at 0x103. IME is true. IF is 0x01.
        // In a real Game Boy, the interrupt is serviced BEFORE 0x103 executes.
        bus.write_byte(0x103, 0x00); // NOP (should be 'skipped' or 'delayed')
        cpu.step(&mut bus);

        // Assertions for a successful Hijack
        assert_eq!(cpu.pc, 0x0041, "PC should be at the V-Blank vector");
        assert_eq!(cpu.sp, 0xDFFB, "SP should have decreased by 2");

        // Verify what was pushed to the stack
        let low = bus.read_byte(cpu.sp);
        let high = bus.read_byte(cpu.sp + 1);
        let return_addr = ((high as u16) << 8) | (low as u16);
        assert_eq!(
            return_addr, 0x103,
            "Stack must save the address of the instruction we jumped over"
        );
    }

    #[test]
    fn test_halt_bug_trigger() {
        let (mut cpu, mut bus) = bootstrap();

        cpu.ime = false;
        bus.write_byte(0xFFFF, 0x01); // IE: Enable V-Blank
        bus.write_byte(0xFF0F, 0x01); // IF: Request V-Blank (Already pending!)

        // Execute HALT (Opcode 0x76)
        bus.write_byte(0x100, 0x76);
        cpu.step(&mut bus);

        assert!(
            cpu.halt_bug_triggered,
            "Halt bug should trigger when IME is off and interrupt is pending"
        );
        assert!(
            !cpu.halted,
            "CPU should NOT enter halt state when halt bug triggers"
        );
    }
    #[test]
    fn test_halt_bug_execution_cycle() {
        let (mut cpu, mut bus) = bootstrap();

        // 1. Setup Halt Bug conditions: IME off, but Interrupt Pending
        cpu.pc = 0x4000;
        cpu.ime = false;
        cpu.halt_bug_triggered = true; // Simulating the trigger from a previous HALT

        // 2. Place an 'INC A' (0x3C) at 0x4000
        // And place a 'DEC A' (0x3D) at 0x4001
        bus.write_byte(0x4000, 0x3C);
        bus.write_byte(0x4001, 0x3D);

        cpu.a = 5;

        // 3. First Step: Should execute INC A but PC stays at 0x4000
        cpu.step(&mut bus);
        assert_eq!(cpu.a, 6, "Instruction INC A should have executed");
        assert_eq!(
            cpu.pc, 0x4000,
            "PC should NOT have moved forward (Halt Bug)"
        );
        assert!(
            !cpu.halt_bug_triggered,
            "Halt bug flag should clear after one use"
        );

        // 4. Second Step: Should execute INC A AGAIN because PC is still 0x4000
        cpu.step(&mut bus);
        assert_eq!(
            cpu.a, 7,
            "Instruction INC A should have executed a second time"
        );
        assert_eq!(cpu.pc, 0x4001, "PC should move forward normally now");
    }
    #[test]
    fn test_ei_invincibility_window() {
        let (mut cpu, mut bus) = bootstrap();

        // 1. Setup: Interrupt is already pending, but IME is off
        cpu.pc = 0x100;
        cpu.ime = false;
        bus.force_write_byte(0xFFFF, 0x01); // IE: V-Blank enabled
        bus.force_write_byte(0xFF0F, 0x01); // IF: V-Blank pending

        // 2. Execute EI
        bus.force_write_byte(0x100, 0xFB); // EI
        cpu.step(&mut bus);

        // PC should be 0x101. Interrupt should NOT have fired yet.
        assert_eq!(
            cpu.pc, 0x101,
            "Interrupt should not hijack the EI instruction itself"
        );
        assert_eq!(cpu.ime_scheduled, 1);

        // 3. Execute NOP at 0x101
        bus.write_byte(0x101, 0x00);
        cpu.step(&mut bus);

        // PC should be 0x102. IME is now true.
        // The interrupt still shouldn't have fired because the "Instruction after EI"
        // is protected.
        assert_eq!(
            cpu.pc, 0x102,
            "Interrupt should not hijack the instruction immediately following EI"
        );
        assert!(cpu.ime);

        // 4. Next Step: NOW the jump happens
        cpu.step(&mut bus);
        assert_eq!(cpu.pc, 0x0041, "Interrupt should finally fire here");
    }
    #[test]
    fn test_interrupt_masking() {
        let (mut cpu, mut bus) = bootstrap();

        cpu.ime = true;
        bus.force_write_byte(0xFFFF, 0x01); // IE: Only V-Blank (bit 0)
        bus.force_write_byte(0xFF0F, 0x02); // IF: LCD Stat (bit 1) requested

        // Step the CPU
        cpu.pc = 0x200;
        bus.force_write_byte(0x200, 0x00); // NOP
        cpu.step(&mut bus);

        assert_eq!(
            cpu.pc, 0x201,
            "Should NOT jump because LCD Stat is not enabled in IE"
        );
    }
    #[test]
    fn test_halt_bug_multi_byte_shift() {
        let (mut cpu, mut bus) = bootstrap();

        cpu.pc = 0x4000;
        cpu.ime = false;
        cpu.halt_bug_triggered = true;

        // 0x3E is 'LD A, n8'.
        // It normally reads 0x3E, then reads the next byte as data.
        bus.force_write_byte(0x4000, 0x3E);
        bus.force_write_byte(0x4001, 0xFF); // This was supposed to be the data

        cpu.a = 0;

        // EXECUTION:
        // 1. Fetch 0x3E. PC does NOT increment (stays at 0x4000).
        // 2. LD A, n8 needs a byte. It reads bus.read(PC).
        // 3. Since PC is 0x4000, it reads 0x3E AGAIN.
        cpu.step(&mut bus);

        assert_eq!(
            cpu.a, 0x3E,
            "A should contain the OPCODE, not the DATA, because of the PC shift"
        );
        assert_eq!(
            cpu.pc, 0x4001,
            "PC should end up at 0x4001 (only incremented once for the operand)"
        );
    }
    #[test]
    fn test_timer_interrupt_trigger() {
        let (mut cpu, mut bus) = bootstrap();

        // 1. Enable Timer at fastest speed (4MHz / 16)
        // TAC: Bit 2 (Enable) = 1, Bits 0-1 (Speed 01) = 1 -> 0b101 (0x05)
        bus.force_write_byte(0xFF07, 0x05);
        bus.force_write_byte(0xFF05, 0xFE); // Set TIMA near overflow
        bus.force_write_byte(0xFF06, 0xAA); // Set TMA reload value

        // 2. Step the CPU (or just the timer) for enough cycles to overflow
        // Fastest speed is 16 cycles. If your step() increments cycles:
        for _ in 0..10 {
            cpu.step(&mut bus);
        }

        // 3. Verify
        let if_reg = bus.read_byte(0xFF0F);
        assert!(
            if_reg & 0x04 != 0,
            "Timer interrupt bit (2) should be set in IF"
        );
        assert_eq!(
            bus.read_byte(0xFF05),
            0xAA,
            "TIMA should have reloaded from TMA"
        );
    }
    #[test]
    fn test_log_alignment_interrupt_hijack() {
        let (mut cpu, mut bus) = bootstrap();

        // 1. Initial State from Log 151345
        cpu.pc = 0xC2BE;
        cpu.sp = 0xDFFD;
        cpu.a = 0x04;
        cpu.b = 0x01;
        cpu.ime = true;

        // Enable Timer Interrupt in IE
        bus.force_write_byte(0xFFFF, 0x04);

        // 2. Setup Memory
        // C2BE: LDH (0xFF0F), A  -> This triggers the interrupt
        bus.force_write_byte(0xC2BE, 0xE0);
        bus.force_write_byte(0xC2BF, 0x0F);

        // C2C0: DEC B -> This should be "skipped" (pushed to stack)
        bus.force_write_byte(0xC2C0, 0x05);

        // 0050: INC A -> First instruction of ISR
        bus.force_write_byte(0x0050, 0x3C);

        // --- STEP 1: Execute LDH ---
        cpu.step(&mut bus);
        // After this, PC should be C2C0, and IF bit 2 should be set.
        assert_eq!(cpu.pc, 0xC2C0);
        assert_eq!(
            bus.read_byte(0xFF0F) & 0x04,
            0x04,
            "Timer interrupt should be pending"
        );

        // --- STEP 2: The Hijack Step ---
        // This is where your 'Was' differed from 'Expected'.
        // The Doctor expects that the NEXT step shows the result of the first ISR instruction.
        cpu.step(&mut bus);

        // Assertions based on "Expected" log 151347
        assert_eq!(
            cpu.pc, 0x0051,
            "PC should be at 0x051 (Vector 0x050 + INC A executed)"
        );
        assert_eq!(cpu.a, 0x05, "A should be 0x05 (INC A executed)");
        assert_eq!(
            cpu.sp, 0xDFFB,
            "SP should be DFFB (PC C2C0 pushed to stack)"
        );

        let stack_low = bus.read_byte(0xDFFB);
        let stack_high = bus.read_byte(0xDFFC);
        assert_eq!(stack_low, 0xC0);
        assert_eq!(stack_high, 0xC2);
    }
    #[test]
    fn test_handle_interrupts_return_state() {
        let (mut cpu, mut bus) = bootstrap();

        // Setup state before interrupt
        cpu.pc = 0xC2C0;
        cpu.ime = true;
        bus.force_write_byte(0xFFFF, 0x04); // IE: Timer
        bus.force_write_byte(0xFF0F, 0x04); // IF: Timer

        // Call the function
        let result = cpu.handle_interrupts(&mut bus);

        // 1. Check Flow Control
        match result {
            StepFlowController::EarlyReturn(cycles) => assert_eq!(cycles, 20),
            _ => panic!("Expected EarlyReturn(20)"),
        }

        // 2. Check side effects
        assert_eq!(cpu.pc, 0x0050, "PC should be at Timer Vector");
        assert_eq!(cpu.ime, false, "IME should be disabled after service");
        assert_eq!(bus.read_byte(0xFF0F) & 0x04, 0, "IF bit should be cleared");
    }
    #[test]
    fn test_halt_bug_pc_behavior() {
        let (mut cpu, mut bus) = bootstrap();

        cpu.pc = 0xC000;
        cpu.ime = false; // IME must be OFF for the bug

        // 1. Setup HALT followed by a NOP
        bus.force_write_byte(0xC000, 0x76); // HALT
        bus.force_write_byte(0xC001, 0x3C); // INC A (The instruction that will be affected)

        // 2. Make an interrupt pending
        bus.force_write_byte(0xFFFF, 0x01); // IE: V-Blank
        bus.force_write_byte(0xFF0F, 0x01); // IF: V-Blank

        // 3. Step once (Executes HALT)
        cpu.step(&mut bus);

        // In the HALT BUG, the CPU doesn't stop,
        // and it fails to increment PC for the NEXT instruction.
        assert!(!cpu.halted, "CPU should not be halted due to HALT bug");
        assert_eq!(cpu.pc, 0xC001, "PC should point to INC A");

        // 4. Step again (Executes INC A)
        cpu.step(&mut bus);

        // THE BUG: The PC should still be 0xC001 because the increment was skipped!
        assert_eq!(
            cpu.pc, 0xC001,
            "HALT Bug failed: PC should not have advanced after INC A"
        );
        assert_eq!(cpu.a, 1, "INC A should have executed once");

        // 5. Step again (Executes INC A a second time)
        cpu.step(&mut bus);
        assert_eq!(cpu.pc, 0xC002, "PC should finally advance now");
        assert_eq!(cpu.a, 2, "INC A should have executed twice total");
    }
    #[test]
    fn test_halt_bug_lifecycle() {
        let (mut cpu, mut bus) = bootstrap();

        cpu.pc = 0xC000;
        cpu.ime = false;

        bus.force_write_byte(0xC000, 0x76); // HALT
        bus.force_write_byte(0xC001, 0x3C); // INC A

        // Trigger condition for HALT Bug: IME=0 and (IE & IF) != 0
        bus.force_write_byte(0xFFFF, 0x01);
        bus.force_write_byte(0xFF0F, 0x01);

        // Step 1: Execute HALT
        cpu.step(&mut bus);

        // After HALT executes, the bug flag should be true, but we shouldn't be "halted"
        assert!(
            cpu.halt_bug_triggered,
            "Flag should be set after 0x76 execution"
        );
        assert!(!cpu.halted, "Should not be in halted state");

        // Step 2: Execute INC A (The first time)
        cpu.step(&mut bus);

        // The flag MUST be false now. If it's still true, the next step will double-execute.
        assert!(
            !cpu.halt_bug_triggered,
            "Flag should have been cleared by fetch_byte"
        );
        assert_eq!(cpu.a, 1, "INC A should have executed once");
        assert_eq!(cpu.pc, 0xC001, "PC should still be 0xC001 due to the bug");
    }

    #[test]
    fn test_halt_no_bug_if_ime_on() {
        let (mut cpu, mut bus) = bootstrap();
        cpu.pc = 0xC000;
        cpu.ime = true; // IME is ON

        bus.force_write_byte(0xC000, 0x76); // HALT
        bus.force_write_byte(0xC001, 0x3C); // INC A
        bus.force_write_byte(0xFFFF, 0x01); // IE
        bus.force_write_byte(0xFF0F, 0x01); // IF (Interrupt is pending!)

        cpu.step(&mut bus);

        // Because IME is ON, it should NOT trigger the halt bug.
        // It should service the interrupt instead (PC jumps to vector).
        assert!(!cpu.halt_bug_triggered);
        assert_ne!(cpu.pc, 0xC001, "Should have jumped to interrupt vector");
    }
    #[test]
    fn test_halt_wakeup_and_stay_awake() {
        let (mut cpu, mut bus) = bootstrap();

        cpu.pc = 0xC000;
        cpu.ime = false;

        bus.force_write_byte(0xC000, 0x76); // HALT
        bus.force_write_byte(0xC001, 0x00); // NOP
        bus.force_write_byte(0xC002, 0x00); // NOP

        // 1. Execute HALT
        cpu.step(&mut bus);
        assert!(cpu.halted, "Should be halted now");

        // 2. Trigger interrupt to wake it up
        bus.force_write_byte(0xFFFF, 0x01);
        bus.force_write_byte(0xFF0F, 0x01);

        // 3. This step should wake up and execute the NOP at C001
        cpu.step(&mut bus);
        assert!(!cpu.halted, "Should have woken up");
        assert_eq!(cpu.pc, 0xC002, "Should have moved past the first NOP");

        // 4. This step should execute the NOP at C002
        // If your bug exists, this will return "early exit" instead!
        cpu.step(&mut bus);
        assert_eq!(cpu.pc, 0xC003, "Should have moved past the second NOP");
    }
    #[test]
    fn test_halt_prohibit_immediate_rehalt() {
        let (mut cpu, mut bus) = bootstrap();

        cpu.pc = 0xC000;
        bus.force_write_byte(0xC000, 0x76); // HALT
        bus.force_write_byte(0xC001, 0x00); // NOP

        // 1. Execute HALT
        cpu.step(&mut bus);
        assert!(cpu.halted);
        // PC should have incremented to C001 after fetching the 0x76
        assert_eq!(cpu.pc, 0xC001);

        // 2. Wake up
        bus.write_byte(0xFFFF, 0x01);
        bus.write_byte(0xFF0F, 0x01);

        cpu.step(&mut bus); // Should execute NOP
        assert!(!cpu.halted, "CPU should be awake");
        assert_eq!(cpu.pc, 0xC002, "PC should have moved to C002");

        // 3. Next step should NOT be a halt exit
        let result = cpu.handle_halt_logic(&mut bus);
        assert!(matches!(result, StepFlowController::Continue));
    }
    #[test]
    fn test_halt_bug_step_isolation() {
        let (mut cpu, mut bus) = bootstrap();
        cpu.pc = 0xC000;
        cpu.ime = false;

        bus.force_write_byte(0xC000, 0x76); // HALT
        bus.force_write_byte(0xC001, 0x3C); // INC A
        bus.force_write_byte(0xFFFF, 0x01); // IE
        bus.force_write_byte(0xFF0F, 0x01); // IF

        // Execute exactly ONE step. This should ONLY execute HALT.
        cpu.step(&mut bus);

        assert_eq!(
            cpu.a, 0,
            "A should STILL be 0. If it is 1, HALT is executing the next op immediately."
        );
        assert_eq!(
            cpu.pc, 0xC001,
            "PC should have moved to the next byte (INC A)"
        );
        assert!(
            cpu.halt_bug_triggered,
            "Bug should be armed for the NEXT step"
        );
    }
    #[test]
    fn test_halt_bug_step_by_step() {
        let (mut cpu, mut bus) = bootstrap();

        cpu.pc = 0xC000;
        cpu.ime = false;
        bus.force_write_byte(0xC000, 0x76); // HALT
        bus.force_write_byte(0xC001, 0x3C); // INC A
        bus.force_write_byte(0xFFFF, 0x01); // IE
        bus.force_write_byte(0xFF0F, 0x01); // IF

        // --- MANUALLY SIMULATE STEP 1 (HALT) ---

        // 1. Fetch the opcode
        let opcode = cpu.fetch_byte(&mut bus);
        assert_eq!(opcode, 0x76, "Should fetch HALT");
        assert_eq!(
            cpu.pc, 0xC001,
            "PC should increment to C001 after fetching 0x76"
        );

        // 2. Dispatch/Execute
        // We assume your dispatch calls your 'halt' function internally
        cpu.fetch_and_execute(&mut bus);
        // Wait! If you call fetch_and_execute here, it will fetch the NEXT byte.
        // Let's call the logic directly if possible, or just look at the state:

        assert!(cpu.halt_bug_triggered, "Flag must be true now");
        assert_eq!(cpu.a, 0, "A should not have changed yet");
        assert_eq!(cpu.pc, 0xC001, "PC should still be at C001");

        // --- MANUALLY SIMULATE STEP 2 (The Buggy Fetch) ---

        // 1. First fetch of INC A
        let opcode2 = cpu.fetch_byte(&mut bus);
        assert_eq!(opcode2, 0x3C, "Should fetch INC A");

        // THE CRITICAL CHECK:
        assert_eq!(cpu.pc, 0xC001, "HALT BUG: PC should NOT have incremented!");
        assert!(
            !cpu.halt_bug_triggered,
            "Flag should have been cleared by fetch_byte"
        );

        // 2. Execute the INC A
        // (Manual dispatch for INC A logic)
        cpu.a += 1;

        // --- MANUALLY SIMULATE STEP 3 (The Second Fetch) ---

        // 1. Second fetch of INC A (because PC is still C001)
        let opcode3 = cpu.fetch_byte(&mut bus);
        assert_eq!(opcode3, 0x3C, "Should fetch INC A again");
        assert_eq!(cpu.pc, 0xC002, "Now PC should finally increment to C002");

        cpu.a += 1;

        assert_eq!(cpu.a, 2, "A should be 2 after the double execution");
    }
    #[test]
    fn test_halt_pc_movement_only() {
        let (mut cpu, mut bus) = bootstrap();
        cpu.pc = 0xC36F;
        bus.force_write_byte(0xC36F, 0x76); // HALT
        bus.force_write_byte(0xC370, 0x00); // NOP

        // We use the same conditions as your log (IME=0, IF=0)
        cpu.ime = false;
        bus.force_write_byte(0xFF0F, 0x00);

        cpu.step(&mut bus);

        // After HALT, PC should be exactly one byte forward.
        assert_eq!(
            cpu.pc, 0xC370,
            "PC should move from C36F to C370 after HALT fetch"
        );
        assert!(cpu.halted, "CPU should be halted");
    }
    #[test]
    fn test_halt_bug_pc_locking() {
        let (mut cpu, mut bus) = bootstrap();
        cpu.pc = 0xC000;
        cpu.ime = false;

        bus.force_write_byte(0xC000, 0x76); // HALT
        bus.force_write_byte(0xFFFF, 0x01); // IE
        bus.force_write_byte(0xFF0F, 0x01); // IF (Bug triggered!)

        // 1. Fetch the 0x76
        let op = cpu.fetch_byte(&mut bus);
        assert_eq!(cpu.pc, 0xC001, "PC must move to C001 after fetching HALT");

        // 2. Execute HALT
        let info = OPCODES[0x76].unwrap();
        assert_eq!(info.mnemonic, Mnemonic::HALT);
        cpu.halt(info, &mut bus);
        assert!(cpu.halt_bug_triggered);

        // 3. The NEXT fetch (the bugged one)
        let next_op = cpu.fetch_byte(&mut bus);
        assert_eq!(cpu.pc, 0xC001, "BUG: PC should NOT move during this fetch!");

        // 4. The THIRD fetch (the recovery)
        let final_op = cpu.fetch_byte(&mut bus);
        assert_eq!(cpu.pc, 0xC002, "PC should finally move to C002 now");
    }
    #[test]
    fn test_manual_bug_execution() {
        let (mut cpu, mut bus) = bootstrap();
        cpu.pc = 0xC000;
        cpu.ime = false;

        bus.force_write_byte(0xC000, 0x76); // HALT
        bus.force_write_byte(0xC001, 0x3C); // INC A
        bus.force_write_byte(0xFFFF, 0x01); // IE
        bus.force_write_byte(0xFF0F, 0x01); // IF

        // 1. Manually run the first instruction (HALT)
        cpu.fetch_and_execute(&mut bus);
        assert!(cpu.halt_bug_triggered);
        assert_eq!(cpu.pc, 0xC001);

        // 2. Manually run the second instruction (The first INC A)
        cpu.fetch_and_execute(&mut bus);
        assert_eq!(cpu.a, 1, "A should be 1 after one fetch_and_execute");
        assert_eq!(cpu.pc, 0xC001, "PC should STILL be C001");

        // 3. Manually run the third instruction (The second INC A)
        cpu.fetch_and_execute(&mut bus);
        assert_eq!(cpu.a, 2, "A should be 2 after second fetch_and_execute");
        assert_eq!(cpu.pc, 0xC002, "PC should finally be C002");
    }
    #[test]
    fn test_fetch_byte_bug_isolation() {
        let (mut cpu, mut bus) = bootstrap();
        cpu.pc = 0xC000;

        // Arm the bug manually
        cpu.halt_bug_triggered = true;
        bus.force_write_byte(0xC000, 0x3C); // INC A

        // First fetch: should NOT increment PC
        let op1 = cpu.fetch_byte(&mut bus);
        assert_eq!(op1, 0x3C);
        assert_eq!(cpu.pc, 0xC000, "PC should not have moved!");
        assert!(!cpu.halt_bug_triggered, "Flag should be reset");

        // Second fetch: should increment PC
        let op2 = cpu.fetch_byte(&mut bus);
        assert_eq!(op2, 0x3C);
        assert_eq!(cpu.pc, 0xC001, "PC should move now");
    }
    #[test]
    fn test_pc_and_flag_alignment() {
        let (mut cpu, mut bus) = bootstrap();
        cpu.pc = 0xC000;
        cpu.ime = false;

        bus.force_write_byte(0xC000, 0x76); // HALT
        bus.force_write_byte(0xC001, 0x3C); // INC A
        bus.force_write_byte(0xFFFF, 0x01); // IE
        bus.force_write_byte(0xFF0F, 0x01); // IF

        // Step 1: Execute HALT
        cpu.fetch_and_execute(&mut bus);
        // PC should be C001, Bug should be true
        assert_eq!(cpu.pc, 0xC001);
        assert!(cpu.halt_bug_triggered);

        // Step 2: Execute INC A
        cpu.fetch_and_execute(&mut bus);
        // PC should be C001 (because fetch_byte skipped increment)
        // BUT! Did your dispatch/length-adder move it to C002?

        println!("PC after first INC A: {:04X}", cpu.pc);
        println!("Bug Flag after first INC A: {}", cpu.halt_bug_triggered);
    }
}
