//! VM command emitter for the Jack compiler.
//!
//! Generates VM commands as text with zero allocation during writes
//! by using pre-sized string buffers and manual digit conversion.

/// VM command writer with pre-allocated output buffer.
///
/// Uses direct string manipulation for minimal allocation overhead.
#[derive(Debug)]
pub struct VMWriter {
    output: String,
}

/// Write a u16 value to a string buffer without allocation.
#[inline]
fn write_u16(n: u16, buf: &mut String) {
    if n == 0 {
        buf.push('0');
        return;
    }
    let mut digits = [0u8; 5]; // Max 5 digits for u16 (65535)
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

impl VMWriter {
    /// Default initial capacity (8KB).
    const DEFAULT_CAPACITY: usize = 8192;

    /// Create a new VM writer with default capacity.
    pub fn new() -> Self {
        Self {
            output: String::with_capacity(Self::DEFAULT_CAPACITY),
        }
    }

    /// Create a new VM writer with specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            output: String::with_capacity(capacity),
        }
    }

    /// Write a push command.
    #[inline]
    pub fn write_push(&mut self, segment: &str, index: u16) {
        self.output.push_str("push ");
        self.output.push_str(segment);
        self.output.push(' ');
        write_u16(index, &mut self.output);
        self.output.push('\n');
    }

    /// Write a pop command.
    #[inline]
    pub fn write_pop(&mut self, segment: &str, index: u16) {
        self.output.push_str("pop ");
        self.output.push_str(segment);
        self.output.push(' ');
        write_u16(index, &mut self.output);
        self.output.push('\n');
    }

    /// Write an arithmetic/logical command.
    #[inline]
    pub fn write_arithmetic(&mut self, cmd: &str) {
        self.output.push_str(cmd);
        self.output.push('\n');
    }

    /// Write a label command.
    #[inline]
    pub fn write_label(&mut self, label: &str) {
        self.output.push_str("label ");
        self.output.push_str(label);
        self.output.push('\n');
    }

    /// Write a goto command.
    #[inline]
    pub fn write_goto(&mut self, label: &str) {
        self.output.push_str("goto ");
        self.output.push_str(label);
        self.output.push('\n');
    }

    /// Write an if-goto command.
    #[inline]
    pub fn write_if_goto(&mut self, label: &str) {
        self.output.push_str("if-goto ");
        self.output.push_str(label);
        self.output.push('\n');
    }

    /// Write a function declaration.
    #[inline]
    pub fn write_function(&mut self, name: &str, num_locals: u16) {
        self.output.push_str("function ");
        self.output.push_str(name);
        self.output.push(' ');
        write_u16(num_locals, &mut self.output);
        self.output.push('\n');
    }

    /// Write a function call.
    #[inline]
    pub fn write_call(&mut self, name: &str, num_args: u16) {
        self.output.push_str("call ");
        self.output.push_str(name);
        self.output.push(' ');
        write_u16(num_args, &mut self.output);
        self.output.push('\n');
    }

    /// Get mutable access to the output buffer (for direct writes).
    #[inline]
    pub fn output_mut(&mut self) -> &mut String {
        &mut self.output
    }

    /// Write a return command.
    #[inline]
    pub fn write_return(&mut self) {
        self.output.push_str("return\n");
    }

    /// Consume the writer and return the generated VM code.
    pub fn into_output(self) -> String {
        self.output
    }

    /// Get a reference to the generated VM code.
    pub fn as_str(&self) -> &str {
        &self.output
    }

    /// Get the current length of the output.
    pub fn len(&self) -> usize {
        self.output.len()
    }

    /// Check if the output is empty.
    pub fn is_empty(&self) -> bool {
        self.output.is_empty()
    }

    /// Clear the output buffer (retains capacity).
    pub fn clear(&mut self) {
        self.output.clear();
    }
}

impl Default for VMWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_writer_is_empty() {
        let writer = VMWriter::new();
        assert!(writer.is_empty());
        assert_eq!(writer.len(), 0);
    }

    #[test]
    fn test_write_push() {
        let mut writer = VMWriter::new();
        writer.write_push("constant", 7);
        assert_eq!(writer.as_str(), "push constant 7\n");
    }

    #[test]
    fn test_write_push_various_segments() {
        let mut writer = VMWriter::new();
        writer.write_push("constant", 0);
        writer.write_push("local", 1);
        writer.write_push("argument", 2);
        writer.write_push("this", 3);
        writer.write_push("that", 4);
        writer.write_push("static", 5);
        writer.write_push("temp", 6);
        writer.write_push("pointer", 0);

        let expected = "\
push constant 0
push local 1
push argument 2
push this 3
push that 4
push static 5
push temp 6
push pointer 0
";
        assert_eq!(writer.as_str(), expected);
    }

    #[test]
    fn test_write_pop() {
        let mut writer = VMWriter::new();
        writer.write_pop("local", 0);
        assert_eq!(writer.as_str(), "pop local 0\n");
    }

    #[test]
    fn test_write_pop_various_segments() {
        let mut writer = VMWriter::new();
        writer.write_pop("local", 0);
        writer.write_pop("argument", 1);
        writer.write_pop("this", 2);
        writer.write_pop("that", 3);
        writer.write_pop("static", 4);
        writer.write_pop("temp", 5);
        writer.write_pop("pointer", 1);

        let expected = "\
pop local 0
pop argument 1
pop this 2
pop that 3
pop static 4
pop temp 5
pop pointer 1
";
        assert_eq!(writer.as_str(), expected);
    }

    #[test]
    fn test_write_arithmetic() {
        let mut writer = VMWriter::new();
        writer.write_arithmetic("add");
        writer.write_arithmetic("sub");
        writer.write_arithmetic("neg");
        writer.write_arithmetic("eq");
        writer.write_arithmetic("gt");
        writer.write_arithmetic("lt");
        writer.write_arithmetic("and");
        writer.write_arithmetic("or");
        writer.write_arithmetic("not");

        let expected = "add\nsub\nneg\neq\ngt\nlt\nand\nor\nnot\n";
        assert_eq!(writer.as_str(), expected);
    }

    #[test]
    fn test_write_label() {
        let mut writer = VMWriter::new();
        writer.write_label("LOOP_START");
        assert_eq!(writer.as_str(), "label LOOP_START\n");
    }

    #[test]
    fn test_write_goto() {
        let mut writer = VMWriter::new();
        writer.write_goto("LOOP_START");
        assert_eq!(writer.as_str(), "goto LOOP_START\n");
    }

    #[test]
    fn test_write_if_goto() {
        let mut writer = VMWriter::new();
        writer.write_if_goto("IF_FALSE");
        assert_eq!(writer.as_str(), "if-goto IF_FALSE\n");
    }

    #[test]
    fn test_write_function() {
        let mut writer = VMWriter::new();
        writer.write_function("Main.main", 0);
        assert_eq!(writer.as_str(), "function Main.main 0\n");
    }

    #[test]
    fn test_write_function_with_locals() {
        let mut writer = VMWriter::new();
        writer.write_function("Square.new", 3);
        assert_eq!(writer.as_str(), "function Square.new 3\n");
    }

    #[test]
    fn test_write_call() {
        let mut writer = VMWriter::new();
        writer.write_call("Math.multiply", 2);
        assert_eq!(writer.as_str(), "call Math.multiply 2\n");
    }

    #[test]
    fn test_write_return() {
        let mut writer = VMWriter::new();
        writer.write_return();
        assert_eq!(writer.as_str(), "return\n");
    }

    #[test]
    fn test_complex_function() {
        let mut writer = VMWriter::new();

        // function Main.main 1
        writer.write_function("Main.main", 1);
        // push constant 7
        writer.write_push("constant", 7);
        // pop local 0
        writer.write_pop("local", 0);
        // push local 0
        writer.write_push("local", 0);
        // call Output.printInt 1
        writer.write_call("Output.printInt", 1);
        // pop temp 0
        writer.write_pop("temp", 0);
        // push constant 0
        writer.write_push("constant", 0);
        // return
        writer.write_return();

        let expected = "\
function Main.main 1
push constant 7
pop local 0
push local 0
call Output.printInt 1
pop temp 0
push constant 0
return
";
        assert_eq!(writer.as_str(), expected);
    }

    #[test]
    fn test_while_loop() {
        let mut writer = VMWriter::new();

        writer.write_label("WHILE_EXP0");
        writer.write_push("local", 0);
        writer.write_push("constant", 10);
        writer.write_arithmetic("lt");
        writer.write_arithmetic("not");
        writer.write_if_goto("WHILE_END0");
        // loop body
        writer.write_push("local", 0);
        writer.write_push("constant", 1);
        writer.write_arithmetic("add");
        writer.write_pop("local", 0);
        writer.write_goto("WHILE_EXP0");
        writer.write_label("WHILE_END0");

        let expected = "\
label WHILE_EXP0
push local 0
push constant 10
lt
not
if-goto WHILE_END0
push local 0
push constant 1
add
pop local 0
goto WHILE_EXP0
label WHILE_END0
";
        assert_eq!(writer.as_str(), expected);
    }

    #[test]
    fn test_if_else() {
        let mut writer = VMWriter::new();

        writer.write_push("local", 0);
        writer.write_arithmetic("not");
        writer.write_if_goto("IF_FALSE0");
        // then branch
        writer.write_push("constant", 1);
        writer.write_pop("local", 1);
        writer.write_goto("IF_END0");
        writer.write_label("IF_FALSE0");
        // else branch
        writer.write_push("constant", 2);
        writer.write_pop("local", 1);
        writer.write_label("IF_END0");

        let expected = "\
push local 0
not
if-goto IF_FALSE0
push constant 1
pop local 1
goto IF_END0
label IF_FALSE0
push constant 2
pop local 1
label IF_END0
";
        assert_eq!(writer.as_str(), expected);
    }

    #[test]
    fn test_into_output() {
        let mut writer = VMWriter::new();
        writer.write_push("constant", 42);
        let output = writer.into_output();
        assert_eq!(output, "push constant 42\n");
    }

    #[test]
    fn test_clear() {
        let mut writer = VMWriter::new();
        writer.write_push("constant", 42);
        assert!(!writer.is_empty());
        writer.clear();
        assert!(writer.is_empty());
    }

    #[test]
    fn test_large_index() {
        let mut writer = VMWriter::new();
        writer.write_push("constant", 32767);
        assert_eq!(writer.as_str(), "push constant 32767\n");
    }

    #[test]
    fn test_with_capacity() {
        let writer = VMWriter::with_capacity(1024);
        assert!(writer.is_empty());
    }
}
