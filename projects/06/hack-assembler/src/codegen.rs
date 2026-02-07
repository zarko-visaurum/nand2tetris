use crate::parser::ResolvedInstruction;

/// Zero-cost extension point for different output formats
/// Now uses a buffer-based approach for zero allocations
pub trait Backend {
    fn encode_a(&self, value: u16, buf: &mut String);
    fn encode_c(&self, dest: u8, comp: u8, jump: u8, buf: &mut String);
}

/// Hack binary format (15-bit addresses, 16-bit instructions)
pub struct HackBinary;

impl Backend for HackBinary {
    fn encode_a(&self, value: u16, buf: &mut String) {
        let value = value & 0x7FFF; // 15-bit address
        // Manual bit manipulation - cannot fail, zero allocations, no unwrap
        for i in (0..16).rev() {
            buf.push(if value & (1 << i) != 0 { '1' } else { '0' });
        }
    }

    fn encode_c(&self, dest: u8, comp: u8, jump: u8, buf: &mut String) {
        let word =
            0b1110_0000_0000_0000 | ((comp as u16) << 6) | ((dest as u16) << 3) | (jump as u16);
        // Manual bit manipulation - cannot fail, zero allocations, no unwrap
        for i in (0..16).rev() {
            buf.push(if word & (1 << i) != 0 { '1' } else { '0' });
        }
    }
}

/// Code generator (generic over backend for zero-cost extension)
pub struct CodeGen<B: Backend> {
    backend: B,
}

impl<B: Backend> CodeGen<B> {
    pub fn new(backend: B) -> Self {
        Self { backend }
    }

    /// Encode instruction to buffer (zero-allocation design)
    pub fn encode(&self, inst: &ResolvedInstruction, buf: &mut String) {
        match inst {
            ResolvedInstruction::AValue(value) => self.backend.encode_a(*value, buf),
            ResolvedInstruction::CInstruction { dest, comp, jump } => {
                self.backend.encode_c(*dest, *comp, *jump, buf)
            }
        }
    }
}

// Type alias for current implementation
pub type HackCodeGen = CodeGen<HackBinary>;

impl HackCodeGen {
    pub fn hack() -> Self {
        Self::new(HackBinary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_a_value() {
        let codegen = HackCodeGen::hack();
        let mut buf = String::new();

        codegen.encode(&ResolvedInstruction::AValue(0), &mut buf);
        assert_eq!(buf, "0000000000000000");

        buf.clear();
        codegen.encode(&ResolvedInstruction::AValue(17), &mut buf);
        assert_eq!(buf, "0000000000010001");

        buf.clear();
        codegen.encode(&ResolvedInstruction::AValue(32767), &mut buf);
        assert_eq!(buf, "0111111111111111");
    }

    #[test]
    fn test_encode_c_instruction() {
        let codegen = HackCodeGen::hack();
        let mut buf = String::new();

        // D=M
        let inst = ResolvedInstruction::CInstruction {
            dest: 0b010,     // D
            comp: 0b1110000, // M
            jump: 0b000,     // no jump
        };
        codegen.encode(&inst, &mut buf);
        assert_eq!(buf, "1111110000010000");

        buf.clear();
        // D;JGT
        let inst = ResolvedInstruction::CInstruction {
            dest: 0b000,
            comp: 0b0001100, // D
            jump: 0b001,     // JGT
        };
        codegen.encode(&inst, &mut buf);
        assert_eq!(buf, "1110001100000001");

        buf.clear();
        // MD=D+1;JMP
        let inst = ResolvedInstruction::CInstruction {
            dest: 0b011,     // MD
            comp: 0b0011111, // D+1
            jump: 0b111,     // JMP
        };
        codegen.encode(&inst, &mut buf);
        assert_eq!(buf, "1110011111011111");
    }
}
