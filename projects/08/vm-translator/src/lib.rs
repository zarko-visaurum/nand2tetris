//! VM Translator - Full Stack VM to Hack Assembly Translator
//!
//! Translates VM bytecode (.vm) to Hack assembly (.asm) for the nand2tetris computer.
//! Supports all 20 VM commands including branching and function calls.
//!
//! # Usage Modes
//!
//! - Single file: `translate("source", "filename")` - No bootstrap
//! - Directory: `translate_directory(path)` - With bootstrap if Sys.vm exists

pub mod bootstrap;
pub mod codegen;
pub mod error;
pub mod memory;
pub mod parser;

use std::fs;
use std::path::Path;

use crate::bootstrap::generate_bootstrap;
use crate::codegen::CodeGenerator;
pub use crate::error::{Result, VMError};
use crate::parser::parse_line;

/// Translate a single VM source string to Hack assembly.
///
/// This is the backward-compatible single-file mode (no bootstrap).
pub fn translate(source: &str, filename: &str) -> Result<String> {
    let mut codegen = CodeGenerator::new();
    codegen.set_filename(filename);

    let estimated_size = source.lines().count() * 50;
    let mut output = String::with_capacity(estimated_size);

    for (line_num, line) in source.lines().enumerate() {
        if let Some(cmd) = parse_line(line, line_num + 1, filename)? {
            codegen.translate(&cmd, &mut output);
        }
    }

    Ok(output)
}

/// Translate a single .vm file to Hack assembly.
pub fn translate_file(path: &Path) -> Result<String> {
    let filename = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown");

    let source = fs::read_to_string(path).map_err(|e| VMError::FileRead {
        path: path.display().to_string(),
        source: e,
    })?;

    translate(&source, filename)
}

/// Translate a .vm file using the given code generator.
///
/// This allows sharing state across multiple files (e.g., call counter).
fn translate_file_with_codegen(path: &Path, codegen: &mut CodeGenerator) -> Result<String> {
    let filename = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown");

    codegen.set_filename(filename);

    let source = fs::read_to_string(path).map_err(|e| VMError::FileRead {
        path: path.display().to_string(),
        source: e,
    })?;

    let estimated_size = source.lines().count() * 50;
    let mut output = String::with_capacity(estimated_size);

    for (line_num, line) in source.lines().enumerate() {
        if let Some(cmd) = parse_line(line, line_num + 1, filename)? {
            codegen.translate(&cmd, &mut output);
        }
    }

    Ok(output)
}

/// Translate all .vm files in a directory to a single .asm file.
///
/// - Generates bootstrap code if Sys.vm exists
/// - Processes Sys.vm first, then other files alphabetically
/// - Returns the combined assembly output
pub fn translate_directory(dir_path: &Path) -> Result<String> {
    // Find all .vm files
    let mut vm_files: Vec<_> = fs::read_dir(dir_path)
        .map_err(|e| VMError::FileRead {
            path: dir_path.display().to_string(),
            source: e,
        })?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "vm"))
        .collect();

    if vm_files.is_empty() {
        return Err(VMError::NoVmFiles {
            path: dir_path.display().to_string(),
        });
    }

    // Sort files alphabetically
    vm_files.sort();

    // Check if Sys.vm exists
    let sys_file = dir_path.join("Sys.vm");
    let has_sys = sys_file.exists();

    // Estimate output size
    let total_lines: usize = vm_files
        .iter()
        .map(|f| {
            fs::read_to_string(f)
                .map(|s| s.lines().count())
                .unwrap_or(0)
        })
        .sum();
    let mut output = String::with_capacity(total_lines * 50 + 512);

    let mut codegen = CodeGenerator::new();

    // Generate bootstrap if Sys.vm exists
    if has_sys {
        output.push_str(&generate_bootstrap());
    }

    // Process Sys.vm first if it exists
    if has_sys {
        let asm = translate_file_with_codegen(&sys_file, &mut codegen)?;
        output.push_str(&asm);
        // Remove Sys.vm from the list
        vm_files.retain(|f| f.file_name() != Some(std::ffi::OsStr::new("Sys.vm")));
    }

    // Process remaining files in alphabetical order
    for vm_file in vm_files {
        let asm = translate_file_with_codegen(&vm_file, &mut codegen)?;
        output.push_str(&asm);
    }

    Ok(output)
}

/// Determine the output filename for a given input.
///
/// - Single file: Input.vm -> Input.asm
/// - Directory: dir/ -> dir/dir.asm
pub fn output_path(input: &Path) -> std::path::PathBuf {
    if input.is_dir() {
        let dir_name = input
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        input.join(format!("{}.asm", dir_name))
    } else {
        input.with_extension("asm")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translate_simple_add() {
        let source = "push constant 7\npush constant 8\nadd";
        let asm = translate(source, "SimpleAdd").unwrap();
        assert!(asm.contains("@7"));
        assert!(asm.contains("@8"));
        assert!(asm.contains("D+M"));
    }

    #[test]
    fn test_translate_with_comments() {
        let source = "// This is a comment\npush constant 5 // inline\n// another comment";
        let asm = translate(source, "Test").unwrap();
        assert!(asm.contains("@5"));
        assert!(!asm.contains("comment"));
    }

    #[test]
    fn test_translate_branching() {
        let source = "label LOOP\ngoto LOOP\nif-goto LOOP";
        let asm = translate(source, "Test").unwrap();
        assert!(asm.contains("(Test$LOOP)"));
        assert!(asm.contains("@Test$LOOP"));
        assert!(asm.contains("0;JMP"));
        assert!(asm.contains("D;JNE"));
    }

    #[test]
    fn test_translate_function() {
        let source = "function Foo.bar 2\nreturn";
        let asm = translate(source, "Foo").unwrap();
        assert!(asm.contains("(Foo.bar)"));
        assert_eq!(asm.matches("M=0").count(), 2);
        assert!(asm.contains("@R14\nA=M\n0;JMP"));
    }

    #[test]
    fn test_translate_call() {
        let source = "function Main.main 0\ncall Foo.bar 2\nreturn";
        let asm = translate(source, "Main").unwrap();
        assert!(asm.contains("@Main.main$ret.0"));
        assert!(asm.contains("@Foo.bar\n0;JMP"));
        assert!(asm.contains("(Main.main$ret.0)"));
    }

    #[test]
    fn test_output_path_file() {
        let path = Path::new("Test.vm");
        assert_eq!(output_path(path), Path::new("Test.asm"));
    }
}
