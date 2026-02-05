mod alu;
mod immediate;
mod instruction_set;
mod operand;
mod register;
mod snapshot;
mod step_flow_controller_enum;

use crate::*;
use crate::{input::DummyInput, mmu::Memory};
pub use alu::{Alu, AluOutput};
use log::{debug, trace};
pub use snapshot::CpuSnapshot;
use std::fmt;
pub use step_flow_controller_enum::StepFlowController;

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

    pub halt_bug_triggered: bool,
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
    fn alu_8bit_add(&self, a: u8, b: u8, use_carry: bool) -> AluOutput {
        AluOutput::alu_8bit_add(a, b, use_carry && self.get_flag(FLAG_C))
    }

    fn alu_8bit_sub(&self, a: u8, b: u8, use_carry: bool) -> AluOutput {
        AluOutput::alu_8bit_sub(a, b, use_carry && self.get_flag(FLAG_C))
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

    pub fn get_z(&mut self) -> bool {
        self.get_flag(FLAG_Z)
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
    pub fn get_current_opcode(&self, bus: &Bus<DummyInput>) -> (OpcodeInfo, u8) {
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
        // 1. Handle Halt Logic
        if let StepFlowController::EarlyReturn(n) = self.handle_halt_logic(bus) {
            trace!("step: halt early exit");
            // bus.tick_components(n as u8);
            return n;
        }

        let mut total_cycles: u8 = 0;

        // 2. Handle Interrupt Hijack
        if
        // bus.pending_interrupt()
        //     && self.ime
        let StepFlowController::EarlyReturn(n) = self.handle_interrupts(bus) {
            trace!("step: interrupt hijack early exit");
            // Add the 20 hijack cycles to our total for this step
            // total_cycles += n;
            // DO NOT return here.
            // PC is now at the vector (e.g., 0x0050).
            // We want to fall through and execute the instruction at 0x0050 now.
            return n;
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

    pub fn handle_interrupts(&mut self, bus: &mut impl Memory) -> StepFlowController {
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
    pub fn fetch_and_execute(&mut self, bus: &mut impl Memory) -> u8 {
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
            trace!("{}", code);
            let result = self.dispatch(code, bus);
            self.apply_flags(&code.flags, result);
            result.cycles
        } else {
            panic!("Shouldn't happen.");
        }
    }

    pub fn handle_halt_logic(&mut self, bus: &mut impl Memory) -> StepFlowController {
        if self.halted {
            if bus.pending_interrupt() {
                self.halted = false;
            } else {
                return StepFlowController::EarlyReturn(4);
            }
        }

        StepFlowController::Continue
    }
    pub fn fetch_byte(&mut self, bus: &mut impl Memory) -> u8 {
        let byte = bus.read_byte(self.pc);
        if self.halt_bug_triggered {
            trace!("Halt bug triggered");
            self.halt_bug_triggered = false;
            // Do NOT increment PC. Next fetch will get the same byte.
        } else {
            self.pc = self.pc.wrapping_add(1);
        }
        byte
    }

    // fn calculate_dec_8bit(&self, value: u8) -> (u8, bool, bool, bool) {
    //     let alu = AluOutput::alu_8bit_dec(value);
    //     (alu.value, alu.z, alu.n, alu.h)
    // }
    pub fn check_condition(&self, target: Target) -> bool {
        match target {
            Target::Condition(cond) => match cond {
                Condition::NotZero => (self.f & FLAG_Z) == 0,
                Condition::Zero => (self.f & FLAG_Z) != 0,
                Condition::NotCarry => (self.f & FLAG_C) == 0,
                Condition::Carry => (self.f & FLAG_C) != 0,
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
        trace!("{} <- {:X}", reg, val);
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

    pub fn take_snapshot(&self, bus: &Bus<DummyInput>) -> CpuSnapshot {
        CpuSnapshot::from_cpu(self, bus)
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
