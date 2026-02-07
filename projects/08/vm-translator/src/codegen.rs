//! Hack assembly code generation for all 20 VM commands.
//!
//! Generates optimized assembly with zero-allocation hot paths.

use crate::memory::{SegmentAccess, pointer_symbol, segment_access, temp_address};
use crate::parser::{ArithmeticOp, Segment, VMCommand};

/// Code generator for Hack assembly.
pub struct CodeGenerator {
    /// Counter for unique comparison labels
    label_counter: usize,
    /// Counter for unique return address labels
    call_counter: usize,
    /// Current filename (without extension) for static variables
    static_filename: String,
    /// Current function name for label scoping
    current_function: String,
}

impl CodeGenerator {
    /// Create a new code generator.
    pub fn new() -> Self {
        Self {
            label_counter: 0,
            call_counter: 0,
            static_filename: String::new(),
            current_function: String::new(),
        }
    }

    /// Set the current filename for static variable naming.
    pub fn set_filename(&mut self, filename: &str) {
        self.static_filename = filename.to_string();
    }

    /// Set the current function for label scoping.
    pub fn set_function(&mut self, name: &str) {
        self.current_function = name.to_string();
    }

    /// Get the current function name.
    pub fn current_function(&self) -> &str {
        &self.current_function
    }

    /// Translate a VM command to Hack assembly.
    pub fn translate(&mut self, cmd: &VMCommand, buf: &mut String) {
        match cmd {
            VMCommand::Arithmetic(op) => self.translate_arithmetic(*op, buf),
            VMCommand::Push { segment, index } => self.translate_push(*segment, *index, buf),
            VMCommand::Pop { segment, index } => self.translate_pop(*segment, *index, buf),
            VMCommand::Label { name } => self.translate_label(name, buf),
            VMCommand::Goto { label } => self.translate_goto(label, buf),
            VMCommand::IfGoto { label } => self.translate_if_goto(label, buf),
            VMCommand::Function { name, num_locals } => {
                self.translate_function(name, *num_locals, buf)
            }
            VMCommand::Call { name, num_args } => self.translate_call(name, *num_args, buf),
            VMCommand::Return => self.translate_return(buf),
        }
    }

    // =========================================================================
    // Arithmetic/Logical Commands
    // =========================================================================

    fn translate_arithmetic(&mut self, op: ArithmeticOp, buf: &mut String) {
        match op {
            ArithmeticOp::Add => self.translate_binary_op("D+M", buf),
            ArithmeticOp::Sub => self.translate_binary_op("M-D", buf),
            ArithmeticOp::And => self.translate_binary_op("D&M", buf),
            ArithmeticOp::Or => self.translate_binary_op("D|M", buf),
            ArithmeticOp::Neg => self.translate_unary_op("-M", buf),
            ArithmeticOp::Not => self.translate_unary_op("!M", buf),
            ArithmeticOp::Eq => self.translate_comparison("JEQ", buf),
            ArithmeticOp::Lt => self.translate_comparison("JLT", buf),
            ArithmeticOp::Gt => self.translate_comparison("JGT", buf),
        }
    }

    fn translate_binary_op(&self, operation: &str, buf: &mut String) {
        // Pop y into D, then compute x op y
        buf.push_str("@SP\nAM=M-1\nD=M\nA=A-1\nM=");
        buf.push_str(operation);
        buf.push('\n');
    }

    fn translate_unary_op(&self, operation: &str, buf: &mut String) {
        // Apply operation to top of stack
        buf.push_str("@SP\nA=M-1\nM=");
        buf.push_str(operation);
        buf.push('\n');
    }

    fn translate_comparison(&mut self, jump: &str, buf: &mut String) {
        let counter = self.label_counter;
        self.label_counter += 1;

        // Pop y, compute x-y, conditional jump
        buf.push_str("@SP\nAM=M-1\nD=M\nA=A-1\nD=M-D\n@");
        self.write_comparison_label(jump, "TRUE", counter, buf);
        buf.push_str("\nD;");
        buf.push_str(jump);
        buf.push_str("\n@SP\nA=M-1\nM=0\n@");
        self.write_comparison_label(jump, "END", counter, buf);
        buf.push_str("\n0;JMP\n(");
        self.write_comparison_label(jump, "TRUE", counter, buf);
        buf.push_str(")\n@SP\nA=M-1\nM=-1\n(");
        self.write_comparison_label(jump, "END", counter, buf);
        buf.push_str(")\n");
    }

    /// Write a comparison label without allocation: JUMP_SUFFIX_N
    #[inline]
    fn write_comparison_label(&self, jump: &str, suffix: &str, counter: usize, buf: &mut String) {
        buf.push_str(jump);
        buf.push('_');
        buf.push_str(suffix);
        buf.push('_');
        write_u16(counter as u16, buf);
    }

    // =========================================================================
    // Memory Access Commands
    // =========================================================================

    fn translate_push(&self, segment: Segment, index: u16, buf: &mut String) {
        match segment_access(segment) {
            SegmentAccess::Constant => {
                // @index, D=A, push D
                buf.push('@');
                write_u16(index, buf);
                buf.push_str("\nD=A\n@SP\nA=M\nM=D\n@SP\nM=M+1\n");
            }
            SegmentAccess::Indirect(base) => {
                // @index, D=A, @BASE, A=D+M, D=M, push D
                buf.push('@');
                write_u16(index, buf);
                buf.push_str("\nD=A\n@");
                buf.push_str(base);
                buf.push_str("\nA=D+M\nD=M\n@SP\nA=M\nM=D\n@SP\nM=M+1\n");
            }
            SegmentAccess::Direct => {
                if segment == Segment::Temp {
                    buf.push('@');
                    write_u16(temp_address(index), buf);
                    buf.push_str("\nD=M\n@SP\nA=M\nM=D\n@SP\nM=M+1\n");
                } else {
                    // Pointer
                    buf.push('@');
                    buf.push_str(pointer_symbol(index));
                    buf.push_str("\nD=M\n@SP\nA=M\nM=D\n@SP\nM=M+1\n");
                }
            }
            SegmentAccess::Static => {
                buf.push('@');
                buf.push_str(&self.static_filename);
                buf.push('.');
                write_u16(index, buf);
                buf.push_str("\nD=M\n@SP\nA=M\nM=D\n@SP\nM=M+1\n");
            }
        }
    }

    fn translate_pop(&self, segment: Segment, index: u16, buf: &mut String) {
        match segment_access(segment) {
            SegmentAccess::Constant => {
                // Parser validates this - dead code path
                // Debug builds catch if invariant is violated
                debug_assert!(false, "pop to constant should be caught by parser");
            }
            SegmentAccess::Indirect(base) => {
                // Calculate address, store in R13, pop into address
                buf.push('@');
                write_u16(index, buf);
                buf.push_str("\nD=A\n@");
                buf.push_str(base);
                buf.push_str("\nD=D+M\n@R13\nM=D\n@SP\nAM=M-1\nD=M\n@R13\nA=M\nM=D\n");
            }
            SegmentAccess::Direct => {
                if segment == Segment::Temp {
                    buf.push_str("@SP\nAM=M-1\nD=M\n@");
                    write_u16(temp_address(index), buf);
                    buf.push_str("\nM=D\n");
                } else {
                    // Pointer
                    buf.push_str("@SP\nAM=M-1\nD=M\n@");
                    buf.push_str(pointer_symbol(index));
                    buf.push_str("\nM=D\n");
                }
            }
            SegmentAccess::Static => {
                buf.push_str("@SP\nAM=M-1\nD=M\n@");
                buf.push_str(&self.static_filename);
                buf.push('.');
                write_u16(index, buf);
                buf.push_str("\nM=D\n");
            }
        }
    }

    // =========================================================================
    // Program Flow Commands
    // =========================================================================

    fn translate_label(&self, name: &str, buf: &mut String) {
        buf.push('(');
        self.write_scoped_label(name, buf);
        buf.push_str(")\n");
    }

    fn translate_goto(&self, label: &str, buf: &mut String) {
        buf.push('@');
        self.write_scoped_label(label, buf);
        buf.push_str("\n0;JMP\n");
    }

    fn translate_if_goto(&self, label: &str, buf: &mut String) {
        buf.push_str("@SP\nAM=M-1\nD=M\n@");
        self.write_scoped_label(label, buf);
        buf.push_str("\nD;JNE\n");
    }

    /// Write a function-scoped label without allocation.
    #[inline]
    fn write_scoped_label(&self, label: &str, buf: &mut String) {
        if !self.current_function.is_empty() {
            buf.push_str(&self.current_function);
            buf.push('$');
        } else if !self.static_filename.is_empty() {
            buf.push_str(&self.static_filename);
            buf.push('$');
        }
        buf.push_str(label);
    }

    // =========================================================================
    // Function Commands
    // =========================================================================

    fn translate_function(&mut self, name: &str, num_locals: u16, buf: &mut String) {
        // Set current function for label scoping
        self.set_function(name);

        // Function entry label
        buf.push('(');
        buf.push_str(name);
        buf.push_str(")\n");

        // Initialize local variables to 0
        for _ in 0..num_locals {
            buf.push_str("@SP\nA=M\nM=0\n@SP\nM=M+1\n");
        }
    }

    fn translate_call(&mut self, name: &str, num_args: u16, buf: &mut String) {
        let counter = self.call_counter;
        self.call_counter += 1;

        // Push return address
        buf.push('@');
        self.write_return_label(counter, buf);
        buf.push_str("\nD=A\n@SP\nA=M\nM=D\n@SP\nM=M+1\n");

        // Push LCL
        buf.push_str("@LCL\nD=M\n@SP\nA=M\nM=D\n@SP\nM=M+1\n");

        // Push ARG
        buf.push_str("@ARG\nD=M\n@SP\nA=M\nM=D\n@SP\nM=M+1\n");

        // Push THIS
        buf.push_str("@THIS\nD=M\n@SP\nA=M\nM=D\n@SP\nM=M+1\n");

        // Push THAT
        buf.push_str("@THAT\nD=M\n@SP\nA=M\nM=D\n@SP\nM=M+1\n");

        // ARG = SP - num_args - 5
        buf.push_str("@SP\nD=M\n@");
        write_u16(num_args + 5, buf);
        buf.push_str("\nD=D-A\n@ARG\nM=D\n");

        // LCL = SP
        buf.push_str("@SP\nD=M\n@LCL\nM=D\n");

        // goto function
        buf.push('@');
        buf.push_str(name);
        buf.push_str("\n0;JMP\n");

        // Return label
        buf.push('(');
        self.write_return_label(counter, buf);
        buf.push_str(")\n");
    }

    /// Write a return label without allocation: prefix$ret.N
    #[inline]
    fn write_return_label(&self, counter: usize, buf: &mut String) {
        let prefix = if self.current_function.is_empty() {
            &self.static_filename
        } else {
            &self.current_function
        };
        buf.push_str(prefix);
        buf.push_str("$ret.");
        write_u16(counter as u16, buf);
    }

    fn translate_return(&self, buf: &mut String) {
        // frame = LCL (store in R13)
        buf.push_str("@LCL\nD=M\n@R13\nM=D\n");

        // retAddr = *(frame - 5) (store in R14)
        buf.push_str("@5\nA=D-A\nD=M\n@R14\nM=D\n");

        // *ARG = pop()
        buf.push_str("@SP\nAM=M-1\nD=M\n@ARG\nA=M\nM=D\n");

        // SP = ARG + 1
        buf.push_str("@ARG\nD=M+1\n@SP\nM=D\n");

        // THAT = *(frame - 1)
        buf.push_str("@R13\nAM=M-1\nD=M\n@THAT\nM=D\n");

        // THIS = *(frame - 2)
        buf.push_str("@R13\nAM=M-1\nD=M\n@THIS\nM=D\n");

        // ARG = *(frame - 3)
        buf.push_str("@R13\nAM=M-1\nD=M\n@ARG\nM=D\n");

        // LCL = *(frame - 4)
        buf.push_str("@R13\nAM=M-1\nD=M\n@LCL\nM=D\n");

        // goto retAddr
        buf.push_str("@R14\nA=M\n0;JMP\n");
    }
}

impl Default for CodeGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Write a u16 to the buffer without allocation.
#[inline]
fn write_u16(n: u16, buf: &mut String) {
    if n == 0 {
        buf.push('0');
        return;
    }

    let mut digits = [0u8; 5];
    let mut i = 0;
    let mut num = n;

    while num > 0 {
        digits[i] = (num % 10) as u8;
        num /= 10;
        i += 1;
    }

    while i > 0 {
        i -= 1;
        buf.push((b'0' + digits[i]) as char);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translate_add() {
        let cgen = CodeGenerator::new();
        let mut buf = String::new();
        cgen.translate_binary_op("D+M", &mut buf);
        assert!(buf.contains("AM=M-1"));
        assert!(buf.contains("M=D+M"));
    }

    #[test]
    fn test_translate_push_constant() {
        let cgen = CodeGenerator::new();
        let mut buf = String::new();
        cgen.translate_push(Segment::Constant, 7, &mut buf);
        assert!(buf.contains("@7"));
        assert!(buf.contains("D=A"));
        assert!(buf.contains("M=M+1"));
    }

    #[test]
    fn test_translate_push_local() {
        let cgen = CodeGenerator::new();
        let mut buf = String::new();
        cgen.translate_push(Segment::Local, 2, &mut buf);
        assert!(buf.contains("@2"));
        assert!(buf.contains("@LCL"));
        assert!(buf.contains("A=D+M"));
    }

    #[test]
    fn test_translate_pop_local() {
        let cgen = CodeGenerator::new();
        let mut buf = String::new();
        cgen.translate_pop(Segment::Local, 3, &mut buf);
        assert!(buf.contains("@3"));
        assert!(buf.contains("@LCL"));
        assert!(buf.contains("@R13"));
    }

    #[test]
    fn test_translate_label() {
        let mut cgen = CodeGenerator::new();
        cgen.set_function("Foo.bar");
        let mut buf = String::new();
        cgen.translate_label("LOOP", &mut buf);
        assert!(buf.contains("(Foo.bar$LOOP)"));
    }

    #[test]
    fn test_translate_goto() {
        let mut cgen = CodeGenerator::new();
        cgen.set_function("Foo.bar");
        let mut buf = String::new();
        cgen.translate_goto("END", &mut buf);
        assert!(buf.contains("@Foo.bar$END"));
        assert!(buf.contains("0;JMP"));
    }

    #[test]
    fn test_translate_if_goto() {
        let mut cgen = CodeGenerator::new();
        cgen.set_function("Foo.bar");
        let mut buf = String::new();
        cgen.translate_if_goto("LOOP", &mut buf);
        assert!(buf.contains("@Foo.bar$LOOP"));
        assert!(buf.contains("D;JNE"));
    }

    #[test]
    fn test_translate_function() {
        let mut cgen = CodeGenerator::new();
        let mut buf = String::new();
        cgen.translate_function("SimpleFunction.test", 2, &mut buf);
        assert!(buf.contains("(SimpleFunction.test)"));
        assert_eq!(buf.matches("M=0").count(), 2); // 2 local vars
    }

    #[test]
    fn test_translate_call() {
        let mut cgen = CodeGenerator::new();
        cgen.set_function("Main.main");
        let mut buf = String::new();
        cgen.translate_call("Foo.bar", 2, &mut buf);
        assert!(buf.contains("@Main.main$ret.0"));
        assert!(buf.contains("@7")); // num_args + 5
        assert!(buf.contains("@Foo.bar"));
        assert!(buf.contains("0;JMP"));
    }

    #[test]
    fn test_translate_return() {
        let cgen = CodeGenerator::new();
        let mut buf = String::new();
        cgen.translate_return(&mut buf);
        assert!(buf.contains("@R13"));
        assert!(buf.contains("@R14"));
        assert!(buf.contains("@ARG"));
        assert!(buf.contains("A=M\n0;JMP"));
    }

    #[test]
    fn test_write_u16() {
        let mut buf = String::new();
        write_u16(0, &mut buf);
        assert_eq!(buf, "0");

        buf.clear();
        write_u16(42, &mut buf);
        assert_eq!(buf, "42");

        buf.clear();
        write_u16(65535, &mut buf);
        assert_eq!(buf, "65535");
    }
}
