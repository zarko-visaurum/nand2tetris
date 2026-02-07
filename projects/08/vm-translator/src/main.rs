//! VM Translator CLI
//!
//! Translates VM bytecode to Hack assembly.
//!
//! # Usage
//!
//! ```bash
//! # Single file
//! vm-translator SimpleAdd.vm
//!
//! # Directory (with bootstrap)
//! vm-translator FibonacciElement/
//! ```

use std::env;
use std::fs;
use std::path::Path;
use std::process;
use std::time::Instant;

use vm_translator::{VMError, output_path, translate_directory, translate_file};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!(
            "VM Translator v{} - Full VM-to-Hack Translator",
            env!("CARGO_PKG_VERSION")
        );
        eprintln!();
        eprintln!("Usage: vm-translator <file.vm | directory> [-v]");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  -v, --verbose    Show detailed output");
        eprintln!();
        eprintln!("Examples:");
        eprintln!("  vm-translator SimpleAdd.vm          # Single file");
        eprintln!("  vm-translator FibonacciElement/     # Directory with bootstrap");
        process::exit(1);
    }

    let verbose = args.iter().any(|a| a == "-v" || a == "--verbose");
    let input_path = Path::new(&args[1]);

    if !input_path.exists() {
        eprintln!("Error: Path not found: {}", input_path.display());
        process::exit(1);
    }

    let start = Instant::now();

    let result = if input_path.is_dir() {
        translate_directory_mode(input_path, verbose)
    } else if input_path.extension().is_some_and(|ext| ext == "vm") {
        translate_file_mode(input_path, verbose)
    } else {
        Err(VMError::InvalidPath {
            path: input_path.display().to_string(),
        })
    };

    match result {
        Ok(output_file) => {
            let elapsed = start.elapsed();
            if verbose {
                println!(
                    "Translated -> {} ({:.2}ms)",
                    output_file.display(),
                    elapsed.as_secs_f64() * 1000.0
                );
            } else {
                println!("{}", output_file.display());
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}

fn translate_file_mode(input: &Path, verbose: bool) -> Result<std::path::PathBuf, VMError> {
    if verbose {
        eprintln!("Translating single file: {}", input.display());
    }

    let asm = translate_file(input)?;
    let output = output_path(input);

    fs::write(&output, &asm).map_err(|e| VMError::FileWrite {
        path: output.display().to_string(),
        source: e,
    })?;

    if verbose {
        let lines = asm.lines().count();
        eprintln!("Generated {} lines of assembly", lines);
    }

    Ok(output)
}

fn translate_directory_mode(input: &Path, verbose: bool) -> Result<std::path::PathBuf, VMError> {
    if verbose {
        eprintln!("Translating directory: {}", input.display());

        // List .vm files
        let vm_files: Vec<_> = fs::read_dir(input)
            .map_err(|e| VMError::FileRead {
                path: input.display().to_string(),
                source: e,
            })?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "vm"))
            .collect();

        eprintln!("Found {} .vm files:", vm_files.len());
        for f in &vm_files {
            eprintln!(
                "  - {}",
                f.file_name().unwrap_or_default().to_string_lossy()
            );
        }

        let sys_file = input.join("Sys.vm");
        if sys_file.exists() {
            eprintln!("Sys.vm found - generating bootstrap code");
        }
    }

    let asm = translate_directory(input)?;
    let output = output_path(input);

    fs::write(&output, &asm).map_err(|e| VMError::FileWrite {
        path: output.display().to_string(),
        source: e,
    })?;

    if verbose {
        let lines = asm.lines().count();
        eprintln!("Generated {} lines of assembly", lines);
    }

    Ok(output)
}
