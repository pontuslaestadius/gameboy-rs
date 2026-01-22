use crate::cpu::Cpu;
use crate::mmu::Memory;

use super::*;

// THE TRICK: Include the generated code right here.
// The generated code will "see" Target, Reg8, etc. because they are in scope.
include!(concat!(env!("OUT_DIR"), "/opcodes_generated.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cb_opcode_decoding() {
        // We will test a few key instructions to ensure the logic is sound
        let cases = vec![
            // (Opcode, Mnemonic)
            (0x4E, Mnemonic::BIT),
            (0xDE, Mnemonic::SET),
        ];

        for (opcode, expected_name) in cases {
            let info = CB_OPCODES[opcode].unwrap();

            assert_eq!(
                info.mnemonic, expected_name,
                "Mnemonic mismatch for opcode {:02X}",
                opcode
            );
        }
    }

    #[test]
    fn verify_jr_gets_carry_instead_of_c_registry() {
        let info = OPCODES[0x38].unwrap();
        let carry_target = info.operands[0].0;
        assert_eq!(carry_target, Target::Condition(Condition::Carry));
    }
}
