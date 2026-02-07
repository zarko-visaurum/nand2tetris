use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::time::Instant;

use hack_assembler::assemble;

fn print_usage() {
    eprintln!("Hack Assembler v{}", env!("CARGO_PKG_VERSION"));
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("    hack-assembler <file.asm> [options]");
    eprintln!("    hack-assembler <file1.asm> <file2.asm> ... [options]");
    eprintln!();
    eprintln!("OPTIONS:");
    eprintln!("    -v, --verbose    Show detailed output");
    eprintln!("    -h, --help       Show this help message");
    eprintln!();
    eprintln!("EXAMPLES:");
    eprintln!("    hack-assembler Add.asm");
    eprintln!("    hack-assembler prog1.asm prog2.asm -v");
}

fn assemble_file(input_path: &Path, verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();

    // Read source
    let source = fs::read_to_string(input_path)?;

    if verbose {
        eprintln!("Assembling: {}", input_path.display());
    }

    // Assemble
    let output = assemble(&source)?;

    // Write output
    let output_path = input_path.with_extension("hack");
    fs::write(&output_path, output)?;

    let elapsed = start.elapsed();

    if verbose {
        let lines = source.lines().count();
        eprintln!(
            "  ✓ {} lines assembled in {:.2}ms",
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
        if let Err(e) = assemble_file(&file, verbose) {
            eprintln!("Error processing {}: {}", file.display(), e);
            errors += 1;
        }
    }

    if errors > 0 {
        process::exit(1);
    }
}
