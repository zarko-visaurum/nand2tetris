use crate::memory::{SegmentAccess, segment_access};
use crate::parser::{ArithmeticOp, Segment, VMCommand};

/// Zero-cost extension point for different assembly backends
pub trait Backend {
    fn translate_command(&mut self, cmd: &VMCommand, buf: &mut String);
}

/// Hack assembly code generator
pub struct HackAssembly {
    label_counter: usize,
    static_filename: String,
}

/// Zero-allocation helper: Write decimal number to buffer
/// Manual digit writing eliminates format! allocations (hack-assembler v1.2 pattern)
fn write_decimal(buf: &mut String, mut value: u16) {
    if value == 0 {
        buf.push('0');
        return;
    }
    let mut digits = [0u8; 5]; // Max 65535 = 5 digits
    let mut i = 0;
    while value > 0 {
        digits[i] = (value % 10) as u8;
        value /= 10;
        i += 1;
    }
    for j in (0..i).rev() {
        buf.push(char::from(b'0' + digits[j]));
    }
}

/// Zero-allocation helper: Write @<decimal> instruction
fn push_at_decimal(buf: &mut String, value: u16) {
    buf.push('@');
    write_decimal(buf, value);
    buf.push('\n');
}

/// Zero-allocation helper: Write @<label> instruction
fn push_at_label(buf: &mut String, label: &str) {
    buf.push('@');
    buf.push_str(label);
    buf.push('\n');
}

/// Zero-allocation helper: Write @<filename>.<index> instruction
fn push_at_static(buf: &mut String, filename: &str, index: u16) {
    buf.push('@');
    buf.push_str(filename);
    buf.push('.');
    write_decimal(buf, index);
    buf.push('\n');
}

impl HackAssembly {
    pub fn new(filename: &str) -> Self {
        Self {
            label_counter: 0,
            static_filename: filename.to_string(),
        }
    }

    /// Generate a unique label for comparisons
    /// Note: Uses format! but only called once per comparison (low frequency)
    fn next_label(&mut self, prefix: &str) -> String {
        let label = format!("{}_{}", prefix, self.label_counter);
        self.label_counter += 1;
        label
    }

    /// Translate arithmetic/logical operations
    fn translate_arithmetic(&mut self, op: ArithmeticOp, buf: &mut String) {
        match op {
            // Binary operations: pop y, pop x, push (x op y)
            ArithmeticOp::Add => {
                buf.push_str("@SP\n");
                buf.push_str("AM=M-1\n"); // SP--, A points to y
                buf.push_str("D=M\n"); // D = y
                buf.push_str("A=A-1\n"); // A points to x
                buf.push_str("M=D+M\n"); // x + y
            }
            ArithmeticOp::Sub => {
                buf.push_str("@SP\n");
                buf.push_str("AM=M-1\n");
                buf.push_str("D=M\n"); // D = y
                buf.push_str("A=A-1\n");
                buf.push_str("M=M-D\n"); // x - y
            }
            ArithmeticOp::And => {
                buf.push_str("@SP\n");
                buf.push_str("AM=M-1\n");
                buf.push_str("D=M\n"); // D = y
                buf.push_str("A=A-1\n");
                buf.push_str("M=D&M\n"); // x & y
            }
            ArithmeticOp::Or => {
                buf.push_str("@SP\n");
                buf.push_str("AM=M-1\n");
                buf.push_str("D=M\n"); // D = y
                buf.push_str("A=A-1\n");
                buf.push_str("M=D|M\n"); // x | y
            }

            // Unary operations: pop x, push (op x)
            ArithmeticOp::Neg => {
                buf.push_str("@SP\n");
                buf.push_str("A=M-1\n"); // A = SP-1 (top of stack)
                buf.push_str("M=-M\n"); // negate in place
            }
            ArithmeticOp::Not => {
                buf.push_str("@SP\n");
                buf.push_str("A=M-1\n"); // A = SP-1 (top of stack)
                buf.push_str("M=!M\n"); // bitwise NOT in place
            }

            // Comparison operations: pop y, pop x, push (x cmp y ? -1 : 0)
            ArithmeticOp::Eq => self.translate_comparison("JEQ", buf),
            ArithmeticOp::Lt => self.translate_comparison("JLT", buf),
            ArithmeticOp::Gt => self.translate_comparison("JGT", buf),
        }
    }

    /// Translate comparison operations (eq, lt, gt)
    fn translate_comparison(&mut self, jump_cond: &str, buf: &mut String) {
        let true_label = self.next_label(&format!("{}_TRUE", jump_cond));
        let end_label = self.next_label(&format!("{}_END", jump_cond));

        // Pop y and x, compute x - y using fused SP decrement
        buf.push_str("@SP\n");
        buf.push_str("AM=M-1\n"); // SP--, A points to y
        buf.push_str("D=M\n"); // D = y
        buf.push_str("A=A-1\n"); // A points to x
        buf.push_str("D=M-D\n"); // D = x - y

        // Jump to true label if condition met (zero-allocation)
        push_at_label(buf, &true_label);
        buf.push_str("D;");
        buf.push_str(jump_cond);
        buf.push('\n');

        // False case: write 0 at SP-1 (result slot)
        buf.push_str("@SP\n");
        buf.push_str("A=M-1\n");
        buf.push_str("M=0\n");
        push_at_label(buf, &end_label);
        buf.push_str("0;JMP\n");

        // True case: write -1 at SP-1 (result slot)
        buf.push('(');
        buf.push_str(&true_label);
        buf.push_str(")\n");
        buf.push_str("@SP\n");
        buf.push_str("A=M-1\n");
        buf.push_str("M=-1\n");

        // End label
        buf.push('(');
        buf.push_str(&end_label);
        buf.push_str(")\n");
    }

    /// Translate push commands
    /// Type-safe: Uses SegmentAccess enum to eliminate .expect() calls
    /// Zero-allocation: Uses manual digit writing instead of format!
    fn translate_push(&mut self, segment: Segment, index: u16, buf: &mut String) {
        match segment_access(segment, index) {
            SegmentAccess::Direct { addr } => {
                // push constant/temp/pointer: direct addressing
                if matches!(segment, Segment::Constant) {
                    // push constant: load immediate value
                    push_at_decimal(buf, addr);
                    buf.push_str("D=A\n");
                } else {
                    // push temp/pointer: load from RAM[addr]
                    push_at_decimal(buf, addr);
                    buf.push_str("D=M\n");
                }
                buf.push_str("@SP\n");
                buf.push_str("A=M\n");
                buf.push_str("M=D\n");
                buf.push_str("@SP\n");
                buf.push_str("M=M+1\n");
            }
            SegmentAccess::Indirect { base } => {
                // push local/argument/this/that: indirect via base pointer
                // Compute address = base + index, load value
                push_at_decimal(buf, index);
                buf.push_str("D=A\n");
                push_at_label(buf, base);
                buf.push_str("A=D+M\n"); // A = base + index
                buf.push_str("D=M\n"); // D = *(base + index)

                // Push to stack
                buf.push_str("@SP\n");
                buf.push_str("A=M\n");
                buf.push_str("M=D\n");
                buf.push_str("@SP\n");
                buf.push_str("M=M+1\n");
            }
            SegmentAccess::Static { index } => {
                // push static: FileName.index
                push_at_static(buf, &self.static_filename, index);
                buf.push_str("D=M\n");
                buf.push_str("@SP\n");
                buf.push_str("A=M\n");
                buf.push_str("M=D\n");
                buf.push_str("@SP\n");
                buf.push_str("M=M+1\n");
            }
        }
    }

    /// Translate pop commands
    /// Type-safe: Uses SegmentAccess enum to eliminate .expect() calls
    /// Zero-allocation: Uses manual digit writing instead of format!
    fn translate_pop(&mut self, segment: Segment, index: u16, buf: &mut String) {
        match segment_access(segment, index) {
            SegmentAccess::Direct { addr } => {
                // pop temp/pointer: direct access to RAM[addr]
                buf.push_str("@SP\n");
                buf.push_str("AM=M-1\n");
                buf.push_str("D=M\n");
                push_at_decimal(buf, addr);
                buf.push_str("M=D\n");
            }
            SegmentAccess::Indirect { base } => {
                // pop local/argument/this/that: indirect via base pointer
                // Compute target address = base + index, store in R13
                push_at_decimal(buf, index);
                buf.push_str("D=A\n");
                push_at_label(buf, base);
                buf.push_str("D=D+M\n"); // D = base + index
                buf.push_str("@R13\n");
                buf.push_str("M=D\n"); // R13 = target address

                // Pop value from stack
                buf.push_str("@SP\n");
                buf.push_str("AM=M-1\n");
                buf.push_str("D=M\n"); // D = popped value

                // Store at target address
                buf.push_str("@R13\n");
                buf.push_str("A=M\n");
                buf.push_str("M=D\n");
            }
            SegmentAccess::Static { index } => {
                // pop static: FileName.index
                buf.push_str("@SP\n");
                buf.push_str("AM=M-1\n");
                buf.push_str("D=M\n");
                push_at_static(buf, &self.static_filename, index);
                buf.push_str("M=D\n");
            }
        }
    }
}

impl Backend for HackAssembly {
    fn translate_command(&mut self, cmd: &VMCommand, buf: &mut String) {
        match cmd {
            VMCommand::Arithmetic(op) => self.translate_arithmetic(*op, buf),
            VMCommand::Push { segment, index } => self.translate_push(*segment, *index, buf),
            VMCommand::Pop { segment, index } => self.translate_pop(*segment, *index, buf),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translate_add() {
        let mut codegen = HackAssembly::new("Test");
        let mut buf = String::new();
        codegen.translate_arithmetic(ArithmeticOp::Add, &mut buf);
        assert!(buf.contains("D+M"));
        assert!(buf.contains("@SP"));
    }

    #[test]
    fn test_translate_push_constant() {
        let mut codegen = HackAssembly::new("Test");
        let mut buf = String::new();
        codegen.translate_push(Segment::Constant, 7, &mut buf);
        assert!(buf.contains("@7"));
        assert!(buf.contains("D=A"));
        assert!(buf.contains("M=D"));
    }

    #[test]
    fn test_translate_pop_local() {
        let mut codegen = HackAssembly::new("Test");
        let mut buf = String::new();
        codegen.translate_pop(Segment::Local, 2, &mut buf);
        assert!(buf.contains("@2"));
        assert!(buf.contains("@LCL"));
        assert!(buf.contains("@R13"));
    }

    #[test]
    fn test_label_uniqueness() {
        let mut codegen = HackAssembly::new("Test");
        let mut buf1 = String::new();
        let mut buf2 = String::new();

        codegen.translate_arithmetic(ArithmeticOp::Eq, &mut buf1);
        codegen.translate_arithmetic(ArithmeticOp::Eq, &mut buf2);

        // Labels should contain unique counter values
        // First call should have counter 0 and 1 (true and end labels)
        // Second call should have counter 2 and 3
        assert!(buf1.contains("_0") || buf1.contains("_1"));
        assert!(buf2.contains("_2") || buf2.contains("_3"));
    }
}
