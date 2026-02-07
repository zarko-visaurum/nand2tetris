use vm_translator::translate;

#[test]
fn test_simple_add() {
    let vm_source =
        std::fs::read_to_string("../SimpleAdd/SimpleAdd.vm").expect("Failed to read SimpleAdd.vm");

    let asm_output = translate(&vm_source, "SimpleAdd").expect("Translation failed");

    // Verify key assembly patterns exist
    assert!(asm_output.contains("@7"), "Should contain constant 7");
    assert!(asm_output.contains("@8"), "Should contain constant 8");
    assert!(asm_output.contains("D+M"), "Should contain add operation");

    // Write output for manual verification
    std::fs::write("../SimpleAdd/SimpleAdd.asm", &asm_output).expect("Failed to write output");
}

#[test]
fn test_stack_test() {
    let vm_source =
        std::fs::read_to_string("../StackTest/StackTest.vm").expect("Failed to read StackTest.vm");

    let asm_output = translate(&vm_source, "StackTest").expect("Translation failed");

    // Verify arithmetic operations
    assert!(asm_output.contains("D+M"), "Should contain add");
    assert!(asm_output.contains("M-D"), "Should contain sub");
    assert!(asm_output.contains("D&M"), "Should contain and");
    assert!(asm_output.contains("D|M"), "Should contain or");
    assert!(asm_output.contains("M=-M"), "Should contain neg");
    assert!(asm_output.contains("M=!M"), "Should contain not");

    // Verify comparison operations with labels
    assert!(asm_output.contains("JEQ"), "Should contain eq comparison");
    assert!(asm_output.contains("JLT"), "Should contain lt comparison");
    assert!(asm_output.contains("JGT"), "Should contain gt comparison");

    std::fs::write("../StackTest/StackTest.asm", &asm_output).expect("Failed to write output");
}

#[test]
fn test_basic_test() {
    let vm_source =
        std::fs::read_to_string("../BasicTest/BasicTest.vm").expect("Failed to read BasicTest.vm");

    let asm_output = translate(&vm_source, "BasicTest").expect("Translation failed");

    // Verify memory segment access
    assert!(asm_output.contains("@LCL"), "Should access local segment");
    assert!(
        asm_output.contains("@ARG"),
        "Should access argument segment"
    );
    assert!(asm_output.contains("@THIS"), "Should access this segment");
    assert!(asm_output.contains("@THAT"), "Should access that segment");
    assert!(asm_output.contains("@R13"), "Should use temp register R13");

    std::fs::write("../BasicTest/BasicTest.asm", &asm_output).expect("Failed to write output");
}

#[test]
fn test_pointer_test() {
    let vm_source = std::fs::read_to_string("../PointerTest/PointerTest.vm")
        .expect("Failed to read PointerTest.vm");

    let asm_output = translate(&vm_source, "PointerTest").expect("Translation failed");

    // Verify pointer segment access (RAM[3] and RAM[4])
    assert!(asm_output.contains("@3"), "Should access pointer 0 (THIS)");
    assert!(asm_output.contains("@4"), "Should access pointer 1 (THAT)");

    std::fs::write("../PointerTest/PointerTest.asm", &asm_output).expect("Failed to write output");
}

#[test]
fn test_static_test() {
    let vm_source = std::fs::read_to_string("../StaticTest/StaticTest.vm")
        .expect("Failed to read StaticTest.vm");

    let asm_output = translate(&vm_source, "StaticTest").expect("Translation failed");

    // Verify static variable naming (StaticTest.0, StaticTest.1, etc.)
    assert!(
        asm_output.contains("@StaticTest."),
        "Should contain static variables with file prefix"
    );

    std::fs::write("../StaticTest/StaticTest.asm", &asm_output).expect("Failed to write output");
}

#[test]
fn test_all_arithmetic_operations() {
    // Test that all 9 arithmetic operations work
    let vm_code = "push constant 10\npush constant 5\nadd\n\
                   push constant 10\npush constant 5\nsub\n\
                   push constant 10\nneg\n\
                   push constant 10\npush constant 5\neq\n\
                   push constant 10\npush constant 5\nlt\n\
                   push constant 10\npush constant 5\ngt\n\
                   push constant 10\npush constant 5\nand\n\
                   push constant 10\npush constant 5\nor\n\
                   push constant 10\nnot";

    let asm_output = translate(vm_code, "Test").expect("Translation failed");

    // Verify all operations are present
    assert!(asm_output.contains("D+M"));
    assert!(asm_output.contains("M-D"));
    assert!(asm_output.contains("M=-M"));
    assert!(asm_output.contains("JEQ"));
    assert!(asm_output.contains("JLT"));
    assert!(asm_output.contains("JGT"));
    assert!(asm_output.contains("D&M"));
    assert!(asm_output.contains("D|M"));
    assert!(asm_output.contains("M=!M"));
}

#[test]
fn test_all_memory_segments() {
    // Test all memory segments (except constant which can't be popped)
    let vm_code = "push constant 10\npop local 0\n\
                   push constant 20\npop argument 1\n\
                   push constant 30\npop this 2\n\
                   push constant 40\npop that 3\n\
                   push constant 50\npop temp 4\n\
                   push constant 3030\npop pointer 0\n\
                   push constant 60\npop static 5";

    let asm_output = translate(vm_code, "Test").expect("Translation failed");

    // Verify all segments are accessed
    assert!(asm_output.contains("@LCL"));
    assert!(asm_output.contains("@ARG"));
    assert!(asm_output.contains("@THIS"));
    assert!(asm_output.contains("@THAT"));
    assert!(asm_output.contains("@9")); // temp 4 = RAM[5+4] = RAM[9]
    assert!(asm_output.contains("@3")); // pointer 0 = RAM[3]
    assert!(asm_output.contains("@Test.5")); // static 5
}
