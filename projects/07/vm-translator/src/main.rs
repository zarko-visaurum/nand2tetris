use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::time::Instant;

use vm_translator::translate;

fn print_usage() {
    eprintln!("VM Translator v1.0.0");
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("    vm-translator <file.vm> [options]");
    eprintln!("    vm-translator <file1.vm> <file2.vm> ... [options]");
    eprintln!();
    eprintln!("OPTIONS:");
    eprintln!("    -v, --verbose    Show detailed output");
    eprintln!("    -h, --help       Show this help message");
    eprintln!();
    eprintln!("EXAMPLES:");
    eprintln!("    vm-translator SimpleAdd.vm");
    eprintln!("    vm-translator prog1.vm prog2.vm -v");
}

fn translate_file(input_path: &Path, verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();

    // Read source
    let source = fs::read_to_string(input_path)?;

    // Extract filename without extension for static variables
    let filename = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("Invalid filename")?;

    if verbose {
        eprintln!("Translating: {}", input_path.display());
    }

    // Translate VM to assembly
    let output = translate(&source, filename)?;

    // Write output
    let output_path = input_path.with_extension("asm");
    fs::write(&output_path, output)?;

    let elapsed = start.elapsed();

    if verbose {
        let lines = source.lines().count();
        eprintln!(
            "  ✓ {} lines translated in {:.2}ms",
            lines,
            elapsed.as_secs_f64() * 1000.0
        );
        eprintln!("  Output: {}", output_path.display());
    } else {
        println!("{} -> {}", input_path.display(), output_path.display());
    }

    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    let mut files = Vec::new();
    let mut verbose = false;

    for arg in &args[1..] {
        match arg.as_str() {
            "-v" | "--verbose" => verbose = true,
            "-h" | "--help" => {
                print_usage();
                process::exit(0);
            }
            _ if arg.starts_with('-') => {
                eprintln!("Error: Unknown option: {}", arg);
                print_usage();
                process::exit(1);
            }
            _ => files.push(PathBuf::from(arg)),
        }
    }

    if files.is_empty() {
        eprintln!("Error: No input files specified");
        print_usage();
        process::exit(1);
    }

    let mut errors = 0;

    for file in files {
        if let Err(e) = translate_file(&file, verbose) {
            eprintln!("Error processing {}: {}", file.display(), e);
            errors += 1;
        }
    }

    if errors > 0 {
        process::exit(1);
    }
}
