use hack_assembler::assemble;
use std::fs;

fn test_file(name: &str) {
    let asm_path = format!("tests/{}.asm", name);
    let hack_path = format!("tests/{}.hack", name);

    let source =
        fs::read_to_string(&asm_path).unwrap_or_else(|_| panic!("Failed to read {}", asm_path));

    let expected =
        fs::read_to_string(&hack_path).unwrap_or_else(|_| panic!("Failed to read {}", hack_path));

    let result = assemble(&source).unwrap_or_else(|e| panic!("Failed to assemble {}: {}", name, e));

    assert_eq!(
        result.trim(),
        expected.trim(),
        "Output mismatch for {}",
        name
    );
}

#[test]
fn test_add() {
    test_file("Add");
}

#[test]
fn test_max() {
    test_file("Max");
}

#[test]
fn test_rect() {
    test_file("Rect");
}

#[test]
fn test_pong() {
    test_file("Pong");
}
