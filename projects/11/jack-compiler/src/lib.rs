//! Jack Compiler - Full Jack to VM code compiler with optimizations.
//!
//! This crate compiles Jack source code to VM code for the nand2tetris
//! virtual machine. It supports:
//!
//! - Complete Jack language compilation
//! - Constant folding optimization
//! - Strength reduction (power-of-2 multiplications use shift instead of Math.multiply)
//! - Peephole optimization of generated VM code
//! - Parallel file processing
//!
//! # Usage
//!
//! ```no_run
//! use jack_compiler::{compile_file, compile_directory, compile_file_with_options, CompileOptions};
//! use std::path::Path;
//!
//! // Compile a single file
//! let result = compile_file(Path::new("Main.jack"));
//!
//! // Compile a directory with optimization
//! let results = compile_directory(Path::new("Square/"));
//!
//! // Compile without optimization
//! let options = CompileOptions { optimize: false };
//! let result = compile_file_with_options(Path::new("Main.jack"), options);
//! ```

pub mod codegen;
pub mod error;
pub mod optimizer;
pub mod symbol_table;
pub mod vm_writer;

use rayon::prelude::*;
use std::fs;
use std::path::Path;

// Re-export key types
pub use codegen::CodeGenerator;
pub use error::CompileError;
pub use optimizer::{ConstantFolder, PeepholeOptimizer, StrengthReduction};
pub use symbol_table::{Symbol, SymbolKind, SymbolTable};
pub use vm_writer::VMWriter;

/// Result of compiling a single Jack file.
#[derive(Debug)]
pub struct CompileResult {
    /// The filename that was compiled.
    pub filename: String,
    /// The generated VM code (empty if errors occurred).
    pub vm_code: String,
    /// Any errors encountered during compilation.
    pub errors: Vec<CompileError>,
}

impl CompileResult {
    /// Check if the compilation was successful (no errors).
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Compilation options.
#[derive(Debug, Clone, Copy)]
pub struct CompileOptions {
    /// Enable peephole optimization (default: true).
    pub optimize: bool,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self { optimize: true }
    }
}

/// Compile a single Jack file.
pub fn compile_file(path: &Path) -> CompileResult {
    compile_file_with_options(path, CompileOptions::default())
}

/// Compile a single Jack file with custom options.
pub fn compile_file_with_options(path: &Path, options: CompileOptions) -> CompileResult {
    let filename = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            return CompileResult {
                filename,
                vm_code: String::new(),
                errors: vec![CompileError::io(path, e)],
            };
        }
    };

    compile_source_with_options(&source, &filename, options)
}

/// Compile Jack source code directly.
pub fn compile_source(source: &str, filename: &str) -> CompileResult {
    compile_source_with_options(source, filename, CompileOptions::default())
}

/// Compile Jack source code with custom options.
pub fn compile_source_with_options(
    source: &str,
    filename: &str,
    options: CompileOptions,
) -> CompileResult {
    // Tokenize
    let tokenizer = jack_analyzer::tokenizer::JackTokenizer::new(source);
    let tokens = match tokenizer.tokenize() {
        Ok(tokens) => tokens,
        Err(errors) => {
            return CompileResult {
                filename: filename.to_string(),
                vm_code: String::new(),
                errors: errors.into_iter().map(CompileError::from).collect(),
            };
        }
    };

    // Parse
    let parser = jack_analyzer::parser::Parser::new(&tokens);
    let class = match parser.parse() {
        Ok(class) => class,
        Err(errors) => {
            return CompileResult {
                filename: filename.to_string(),
                vm_code: String::new(),
                errors: errors.into_iter().map(CompileError::from).collect(),
            };
        }
    };

    // Compile to VM code (pass optimize flag for constant folding)
    match CodeGenerator::compile_with_options(&class, options.optimize) {
        Ok(vm_code) => {
            // Apply peephole optimization if enabled
            let vm_code = if options.optimize {
                PeepholeOptimizer::optimize(&vm_code)
            } else {
                vm_code
            };

            CompileResult {
                filename: filename.to_string(),
                vm_code,
                errors: Vec::new(),
            }
        }
        Err(errors) => CompileResult {
            filename: filename.to_string(),
            vm_code: String::new(),
            errors,
        },
    }
}

/// Compile all Jack files in a directory.
pub fn compile_directory(dir: &Path) -> Vec<CompileResult> {
    compile_directory_with_options(dir, CompileOptions::default())
}

/// Compile all Jack files in a directory with custom options.
pub fn compile_directory_with_options(dir: &Path, options: CompileOptions) -> Vec<CompileResult> {
    let jack_files: Vec<_> = match fs::read_dir(dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|ext| ext == "jack"))
            .collect(),
        Err(e) => {
            return vec![CompileResult {
                filename: dir.to_string_lossy().to_string(),
                vm_code: String::new(),
                errors: vec![CompileError::io(dir, e)],
            }];
        }
    };

    if jack_files.is_empty() {
        return Vec::new();
    }

    // Parallel compilation
    jack_files
        .par_iter()
        .map(|path| compile_file_with_options(path, options))
        .collect()
}

/// Write a compile result to an output file.
pub fn write_result(result: &CompileResult, output_dir: &Path) -> Result<(), CompileError> {
    let vm_path = output_dir.join(format!("{}.vm", result.filename));
    fs::write(&vm_path, &result.vm_code).map_err(|e| CompileError::io(&vm_path, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_source_simple() {
        let source = r#"
class Main {
    function void main() {
        return;
    }
}
"#;
        let result = compile_source(source, "Main");
        assert!(result.is_ok());
        assert!(result.vm_code.contains("function Main.main 0"));
        assert!(result.vm_code.contains("return"));
    }

    #[test]
    fn test_compile_source_with_error() {
        let source = r#"
class Main {
    function void main() {
        let x = 5;
        return;
    }
}
"#;
        let result = compile_source(source, "Main");
        assert!(!result.is_ok());
    }

    #[test]
    fn test_compile_with_optimization() {
        let source = r#"
class Main {
    function void main() {
        var int x;
        let x = ~~5;
        return;
    }
}
"#;
        let result = compile_source_with_options(source, "Main", CompileOptions { optimize: true });
        assert!(result.is_ok());

        // Double not should be optimized away
        let not_count = result.vm_code.matches("\nnot\n").count();
        assert_eq!(not_count, 0, "Double not should be eliminated");
    }

    #[test]
    fn test_compile_without_optimization() {
        let source = r#"
class Main {
    function void main() {
        var int x;
        let x = ~~5;
        return;
    }
}
"#;
        let result =
            compile_source_with_options(source, "Main", CompileOptions { optimize: false });
        assert!(result.is_ok());

        // Without optimization, double not should remain
        let not_count = result.vm_code.matches("not\n").count();
        assert_eq!(
            not_count, 2,
            "Without optimization, both nots should remain"
        );
    }

    #[test]
    fn test_default_options() {
        let options = CompileOptions::default();
        assert!(options.optimize);
    }
}
