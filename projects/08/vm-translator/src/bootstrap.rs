//! Bootstrap code generation for VM initialization.
//!
//! Generates the bootstrap code that initializes SP and calls Sys.init.

/// Generate VM bootstrap code.
///
/// The bootstrap code:
/// 1. Sets SP = 256
/// 2. Calls Sys.init with 0 arguments
/// 3. Halts with an infinite loop (defensive, in case Sys.init returns)
///
/// This is only needed for multi-file programs that have Sys.init.
pub fn generate_bootstrap() -> String {
    let mut buf = String::with_capacity(512);

    // SP = 256
    buf.push_str("@256\nD=A\n@SP\nM=D\n");

    // call Sys.init 0
    // Push return address
    buf.push_str("@Sys.init$ret.BOOTSTRAP\nD=A\n@SP\nA=M\nM=D\n@SP\nM=M+1\n");

    // Push LCL
    buf.push_str("@LCL\nD=M\n@SP\nA=M\nM=D\n@SP\nM=M+1\n");

    // Push ARG
    buf.push_str("@ARG\nD=M\n@SP\nA=M\nM=D\n@SP\nM=M+1\n");

    // Push THIS
    buf.push_str("@THIS\nD=M\n@SP\nA=M\nM=D\n@SP\nM=M+1\n");

    // Push THAT
    buf.push_str("@THAT\nD=M\n@SP\nA=M\nM=D\n@SP\nM=M+1\n");

    // ARG = SP - 0 - 5 = SP - 5
    buf.push_str("@SP\nD=M\n@5\nD=D-A\n@ARG\nM=D\n");

    // LCL = SP
    buf.push_str("@SP\nD=M\n@LCL\nM=D\n");

    // goto Sys.init
    buf.push_str("@Sys.init\n0;JMP\n");

    // Return label (never reached, but needed for structure)
    buf.push_str("(Sys.init$ret.BOOTSTRAP)\n");

    // Halt sentinel: infinite loop if Sys.init ever returns
    buf.push_str("(HALT)\n@HALT\n0;JMP\n");

    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bootstrap_sets_sp() {
        let code = generate_bootstrap();
        assert!(code.contains("@256"));
        assert!(code.contains("@SP\nM=D"));
    }

    #[test]
    fn test_bootstrap_calls_sys_init() {
        let code = generate_bootstrap();
        assert!(code.contains("@Sys.init\n0;JMP"));
    }

    #[test]
    fn test_bootstrap_pushes_frame() {
        let code = generate_bootstrap();
        // Should push LCL, ARG, THIS, THAT
        assert!(code.contains("@LCL\nD=M"));
        assert!(code.contains("@ARG\nD=M"));
        assert!(code.contains("@THIS\nD=M"));
        assert!(code.contains("@THAT\nD=M"));
    }

    #[test]
    fn test_bootstrap_has_return_label() {
        let code = generate_bootstrap();
        assert!(code.contains("(Sys.init$ret.BOOTSTRAP)"));
    }

    #[test]
    fn test_bootstrap_has_halt_sentinel() {
        let code = generate_bootstrap();
        assert!(code.contains("(HALT)\n@HALT\n0;JMP"));
    }
}
