//! Integration tests for the Full VM Translator (Project 08).
//!
//! Tests all 11 nand2tetris test programs (5 from P07 + 6 from P08).

use std::path::Path;
use vm_translator::{translate, translate_directory};

// =============================================================================
// In-Memory Tests (Always Run)
// =============================================================================

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
    assert!(asm_output.contains("@THIS\nM=D")); // pointer 0
    assert!(asm_output.contains("@Test.5")); // static 5
}

#[test]
fn test_all_branching_commands() {
    // Test all 3 branching commands
    let vm_code = "function Test.main 0\n\
                   label LOOP\n\
                   push constant 1\n\
                   if-goto END\n\
                   goto LOOP\n\
                   label END\n\
                   return";

    let asm_output = translate(vm_code, "Test").expect("Translation failed");

    // Verify branching commands
    assert!(
        asm_output.contains("(Test.main$LOOP)"),
        "Should contain LOOP label"
    );
    assert!(
        asm_output.contains("(Test.main$END)"),
        "Should contain END label"
    );
    assert!(
        asm_output.contains("@Test.main$LOOP\n0;JMP"),
        "Should contain goto LOOP"
    );
    assert!(
        asm_output.contains("@Test.main$END\nD;JNE"),
        "Should contain if-goto END"
    );
}

#[test]
fn test_all_function_commands() {
    // Test all 3 function commands
    let vm_code = "function Test.caller 1\n\
                   push constant 5\n\
                   call Test.callee 1\n\
                   pop local 0\n\
                   return\n\
                   function Test.callee 0\n\
                   push argument 0\n\
                   push constant 1\n\
                   add\n\
                   return";

    let asm_output = translate(vm_code, "Test").expect("Translation failed");

    // Verify function commands
    assert!(
        asm_output.contains("(Test.caller)"),
        "Should contain caller function"
    );
    assert!(
        asm_output.contains("(Test.callee)"),
        "Should contain callee function"
    );
    assert!(asm_output.contains("$ret."), "Should contain return label");
    assert!(
        asm_output.contains("@Test.callee\n0;JMP"),
        "Should jump to callee"
    );
    assert!(
        asm_output.contains("@R14\nA=M\n0;JMP"),
        "Should return via R14"
    );
}

#[test]
fn test_call_frame_structure() {
    // Test that call properly sets up the frame
    let vm_code = "function Test.main 0\n\
                   push constant 1\n\
                   push constant 2\n\
                   call Test.add 2\n\
                   return\n\
                   function Test.add 0\n\
                   push argument 0\n\
                   push argument 1\n\
                   add\n\
                   return";

    let asm_output = translate(vm_code, "Test").expect("Translation failed");

    // Verify call pushes all frame elements
    assert!(
        asm_output.contains("@LCL\nD=M\n@SP\nA=M\nM=D"),
        "Should push LCL"
    );
    assert!(
        asm_output.contains("@ARG\nD=M\n@SP\nA=M\nM=D"),
        "Should push ARG"
    );
    assert!(
        asm_output.contains("@THIS\nD=M\n@SP\nA=M\nM=D"),
        "Should push THIS"
    );
    assert!(
        asm_output.contains("@THAT\nD=M\n@SP\nA=M\nM=D"),
        "Should push THAT"
    );

    // Verify ARG repositioning (2 args + 5 frame = 7)
    assert!(
        asm_output.contains("@7\nD=D-A\n@ARG\nM=D"),
        "Should set ARG = SP - 7"
    );

    // Verify LCL = SP
    assert!(asm_output.contains("@LCL\nM=D"), "Should set LCL = SP");
}

#[test]
fn test_return_frame_restoration() {
    // Test that return properly restores the frame
    let vm_code = "function Test.main 0\nreturn";

    let asm_output = translate(vm_code, "Test").expect("Translation failed");

    // Verify return sequence
    assert!(
        asm_output.contains("@LCL\nD=M\n@R13\nM=D"),
        "Should save LCL to R13"
    );
    assert!(
        asm_output.contains("@5\nA=D-A\nD=M\n@R14\nM=D"),
        "Should save retAddr to R14"
    );
    assert!(
        asm_output.contains("@SP\nAM=M-1\nD=M\n@ARG\nA=M\nM=D"),
        "Should place return value"
    );
    assert!(
        asm_output.contains("@ARG\nD=M+1\n@SP\nM=D"),
        "Should set SP = ARG + 1"
    );
    assert!(
        asm_output.contains("@R13\nAM=M-1\nD=M\n@THAT\nM=D"),
        "Should restore THAT"
    );
    assert!(
        asm_output.contains("@R13\nAM=M-1\nD=M\n@THIS\nM=D"),
        "Should restore THIS"
    );
    assert!(
        asm_output.contains("@R13\nAM=M-1\nD=M\n@ARG\nM=D"),
        "Should restore ARG"
    );
    assert!(
        asm_output.contains("@R13\nAM=M-1\nD=M\n@LCL\nM=D"),
        "Should restore LCL"
    );
    assert!(
        asm_output.contains("@R14\nA=M\n0;JMP"),
        "Should jump to retAddr"
    );
}

#[test]
fn test_local_variable_initialization() {
    // Test that function initializes local variables to 0
    let vm_code = "function Test.main 5\nreturn";

    let asm_output = translate(vm_code, "Test").expect("Translation failed");

    // Should have 5 local variable initializations
    let init_count = asm_output.matches("M=0\n@SP\nM=M+1").count();
    assert_eq!(init_count, 5, "Should initialize 5 local variables");
}

#[test]
fn test_comparison_label_uniqueness() {
    // Test that multiple comparisons generate unique labels
    let vm_code = "push constant 1\npush constant 2\neq\n\
                   push constant 3\npush constant 4\neq\n\
                   push constant 5\npush constant 6\neq";

    let asm_output = translate(vm_code, "Test").expect("Translation failed");

    // Should have unique labels for each comparison
    assert!(asm_output.contains("JEQ_TRUE_0"), "Should have label 0");
    assert!(asm_output.contains("JEQ_TRUE_1"), "Should have label 1");
    assert!(asm_output.contains("JEQ_TRUE_2"), "Should have label 2");
}

#[test]
fn test_static_variable_naming() {
    // Test that static variables use correct file prefix
    let vm_code = "push static 0\npush static 5\npush static 10";

    let asm_output = translate(vm_code, "MyFile").expect("Translation failed");

    assert!(asm_output.contains("@MyFile.0"), "Should have MyFile.0");
    assert!(asm_output.contains("@MyFile.5"), "Should have MyFile.5");
    assert!(asm_output.contains("@MyFile.10"), "Should have MyFile.10");
}

#[test]
fn test_label_scoping_within_function() {
    // Test that labels are scoped to their function
    let vm_code = "function Foo.bar 0\n\
                   label LOOP\n\
                   goto LOOP\n\
                   return";

    let asm_output = translate(vm_code, "Foo").expect("Translation failed");

    assert!(
        asm_output.contains("(Foo.bar$LOOP)"),
        "Label should be scoped to function"
    );
    assert!(
        asm_output.contains("@Foo.bar$LOOP"),
        "Goto should use scoped label"
    );
}

#[test]
fn test_multiple_functions() {
    // Test multiple functions in one file
    let vm_code = "function Class.method1 2\n\
                   push local 0\n\
                   return\n\
                   function Class.method2 1\n\
                   push local 0\n\
                   return";

    let asm_output = translate(vm_code, "Class").expect("Translation failed");

    assert!(
        asm_output.contains("(Class.method1)"),
        "Should have method1"
    );
    assert!(
        asm_output.contains("(Class.method2)"),
        "Should have method2"
    );
}

#[test]
fn test_recursive_call() {
    // Test recursive function call
    let vm_code = "function Test.recurse 1\n\
                   push argument 0\n\
                   push constant 0\n\
                   eq\n\
                   if-goto BASE\n\
                   push argument 0\n\
                   push constant 1\n\
                   sub\n\
                   call Test.recurse 1\n\
                   return\n\
                   label BASE\n\
                   push constant 1\n\
                   return";

    let asm_output = translate(vm_code, "Test").expect("Translation failed");

    assert!(
        asm_output.contains("(Test.recurse)"),
        "Should have function label"
    );
    assert!(
        asm_output.contains("@Test.recurse\n0;JMP"),
        "Should call itself"
    );
    assert!(
        asm_output.contains("(Test.recurse$BASE)"),
        "Should have BASE label"
    );
}

// =============================================================================
// Project 07 File-Based Tests (Backward Compatibility)
// =============================================================================

#[test]
fn test_simple_add_file() {
    let vm_source =
        std::fs::read_to_string("../SimpleAdd/SimpleAdd.vm").expect("Failed to read SimpleAdd.vm");

    let asm_output = translate(&vm_source, "SimpleAdd").expect("Translation failed");

    assert!(asm_output.contains("@7"), "Should contain constant 7");
    assert!(asm_output.contains("@8"), "Should contain constant 8");
    assert!(asm_output.contains("D+M"), "Should contain add operation");

    // Write output for verification
    std::fs::write("../SimpleAdd/SimpleAdd.asm", &asm_output).expect("Failed to write output");
}

#[test]
fn test_stack_test_file() {
    let vm_source =
        std::fs::read_to_string("../StackTest/StackTest.vm").expect("Failed to read StackTest.vm");

    let asm_output = translate(&vm_source, "StackTest").expect("Translation failed");

    // Verify all 9 arithmetic/logical operations
    assert!(asm_output.contains("D+M"), "Should contain add");
    assert!(asm_output.contains("M-D"), "Should contain sub");
    assert!(asm_output.contains("D&M"), "Should contain and");
    assert!(asm_output.contains("D|M"), "Should contain or");
    assert!(asm_output.contains("M=-M"), "Should contain neg");
    assert!(asm_output.contains("M=!M"), "Should contain not");
    assert!(asm_output.contains("JEQ"), "Should contain eq comparison");
    assert!(asm_output.contains("JLT"), "Should contain lt comparison");
    assert!(asm_output.contains("JGT"), "Should contain gt comparison");

    std::fs::write("../StackTest/StackTest.asm", &asm_output).expect("Failed to write output");
}

#[test]
fn test_basic_test_file() {
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

    std::fs::write("../BasicTest/BasicTest.asm", &asm_output).expect("Failed to write output");
}

#[test]
fn test_pointer_test_file() {
    let vm_source = std::fs::read_to_string("../PointerTest/PointerTest.vm")
        .expect("Failed to read PointerTest.vm");

    let asm_output = translate(&vm_source, "PointerTest").expect("Translation failed");

    // Verify pointer segment access (THIS and THAT)
    assert!(
        asm_output.contains("@THIS"),
        "Should access pointer 0 (THIS)"
    );
    assert!(
        asm_output.contains("@THAT"),
        "Should access pointer 1 (THAT)"
    );

    std::fs::write("../PointerTest/PointerTest.asm", &asm_output).expect("Failed to write output");
}

#[test]
fn test_static_test_file() {
    let vm_source = std::fs::read_to_string("../StaticTest/StaticTest.vm")
        .expect("Failed to read StaticTest.vm");

    let asm_output = translate(&vm_source, "StaticTest").expect("Translation failed");

    // Verify static variable naming
    assert!(
        asm_output.contains("@StaticTest."),
        "Should contain static variables with file prefix"
    );

    std::fs::write("../StaticTest/StaticTest.asm", &asm_output).expect("Failed to write output");
}

// =============================================================================
// Project 08 File-Based Tests - Branching
// =============================================================================

#[test]
fn test_basic_loop_file() {
    let vm_source = std::fs::read_to_string("../ProgramFlow/BasicLoop/BasicLoop.vm")
        .expect("Failed to read BasicLoop.vm");

    let asm_output = translate(&vm_source, "BasicLoop").expect("Translation failed");

    // Verify branching commands
    assert!(
        asm_output.contains("$LOOP_START"),
        "Should contain LOOP_START label"
    );
    assert!(asm_output.contains("D;JNE"), "Should contain if-goto (JNE)");

    std::fs::write("../ProgramFlow/BasicLoop/BasicLoop.asm", &asm_output)
        .expect("Failed to write output");
}

#[test]
fn test_fibonacci_series_file() {
    let vm_source = std::fs::read_to_string("../ProgramFlow/FibonacciSeries/FibonacciSeries.vm")
        .expect("Failed to read FibonacciSeries.vm");

    let asm_output = translate(&vm_source, "FibonacciSeries").expect("Translation failed");

    // Verify branching commands
    assert!(
        asm_output.contains("$MAIN_LOOP_START"),
        "Should contain MAIN_LOOP_START"
    );
    assert!(
        asm_output.contains("$COMPUTE_ELEMENT"),
        "Should contain COMPUTE_ELEMENT"
    );
    assert!(
        asm_output.contains("$END_PROGRAM"),
        "Should contain END_PROGRAM"
    );
    assert!(asm_output.contains("D;JNE"), "Should contain if-goto");
    assert!(asm_output.contains("0;JMP"), "Should contain goto");

    std::fs::write(
        "../ProgramFlow/FibonacciSeries/FibonacciSeries.asm",
        &asm_output,
    )
    .expect("Failed to write output");
}

// =============================================================================
// Project 08 File-Based Tests - Function Commands
// =============================================================================

#[test]
fn test_simple_function_file() {
    let vm_source = std::fs::read_to_string("../FunctionCalls/SimpleFunction/SimpleFunction.vm")
        .expect("Failed to read SimpleFunction.vm");

    let asm_output = translate(&vm_source, "SimpleFunction").expect("Translation failed");

    // Verify function declaration
    assert!(
        asm_output.contains("(SimpleFunction.test)"),
        "Should contain function label"
    );

    // Verify local variable initialization (2 locals)
    assert_eq!(
        asm_output.matches("M=0\n@SP\nM=M+1").count(),
        2,
        "Should initialize 2 locals"
    );

    // Verify return code
    assert!(asm_output.contains("@R13"), "Should use R13 for frame");
    assert!(asm_output.contains("@R14"), "Should use R14 for retAddr");

    std::fs::write(
        "../FunctionCalls/SimpleFunction/SimpleFunction.asm",
        &asm_output,
    )
    .expect("Failed to write output");
}

#[test]
fn test_nested_call_file() {
    let dir_path = Path::new("../FunctionCalls/NestedCall");
    let asm_output = translate_directory(dir_path).expect("Translation failed");

    // Verify bootstrap code
    assert!(
        asm_output.starts_with("@256\nD=A\n@SP\nM=D"),
        "Should start with SP=256"
    );
    assert!(
        asm_output.contains("@Sys.init\n0;JMP"),
        "Should call Sys.init"
    );

    // Verify all functions
    assert!(
        asm_output.contains("(Sys.init)"),
        "Should contain Sys.init function"
    );
    assert!(
        asm_output.contains("(Sys.main)"),
        "Should contain Sys.main function"
    );
    assert!(
        asm_output.contains("(Sys.add12)"),
        "Should contain Sys.add12 function"
    );

    // Verify call frame setup
    assert!(asm_output.contains("$ret."), "Should contain return labels");

    std::fs::write("../FunctionCalls/NestedCall/NestedCall.asm", &asm_output)
        .expect("Failed to write output");
}

#[test]
fn test_fibonacci_element_file() {
    let dir_path = Path::new("../FunctionCalls/FibonacciElement");
    let asm_output = translate_directory(dir_path).expect("Translation failed");

    // Verify bootstrap code
    assert!(
        asm_output.starts_with("@256\nD=A\n@SP\nM=D"),
        "Should start with SP=256"
    );
    assert!(
        asm_output.contains("@Sys.init\n0;JMP"),
        "Should call Sys.init"
    );

    // Verify all functions present
    assert!(asm_output.contains("(Sys.init)"), "Should contain Sys.init");
    assert!(
        asm_output.contains("(Main.fibonacci)"),
        "Should contain Main.fibonacci"
    );

    // Verify recursive call
    assert!(
        asm_output.contains("@Main.fibonacci\n0;JMP"),
        "Should call Main.fibonacci recursively"
    );

    std::fs::write(
        "../FunctionCalls/FibonacciElement/FibonacciElement.asm",
        &asm_output,
    )
    .expect("Failed to write output");
}

#[test]
fn test_statics_test_file() {
    let dir_path = Path::new("../FunctionCalls/StaticsTest");
    let asm_output = translate_directory(dir_path).expect("Translation failed");

    // Verify bootstrap code
    assert!(
        asm_output.starts_with("@256\nD=A\n@SP\nM=D"),
        "Should start with SP=256"
    );

    // Verify static variables with file prefixes
    assert!(
        asm_output.contains("@Class1."),
        "Should contain Class1 static variables"
    );
    assert!(
        asm_output.contains("@Class2."),
        "Should contain Class2 static variables"
    );

    // Verify all classes are present
    assert!(
        asm_output.contains("(Class1.set)"),
        "Should contain Class1.set"
    );
    assert!(
        asm_output.contains("(Class1.get)"),
        "Should contain Class1.get"
    );
    assert!(
        asm_output.contains("(Class2.set)"),
        "Should contain Class2.set"
    );
    assert!(
        asm_output.contains("(Class2.get)"),
        "Should contain Class2.get"
    );

    std::fs::write("../FunctionCalls/StaticsTest/StaticsTest.asm", &asm_output)
        .expect("Failed to write output");
}
