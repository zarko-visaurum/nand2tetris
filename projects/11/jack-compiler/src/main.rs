//! Jack Compiler CLI - Compiles Jack files to VM code.
//!
//! Usage:
//!     JackCompiler <file.jack | directory>
//!     JackCompiler --no-optimize <file.jack | directory>

use clap::Parser as ClapParser;
use jack_compiler::{
    CompileOptions, compile_directory_with_options, compile_file_with_options, write_result,
};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(ClapParser, Debug)]
#[command(name = "JackCompiler")]
#[command(version = "1.0.0")]
#[command(about = "Jack to VM code compiler with optimizations")]
#[command(author = "nand2tetris")]
struct Args {
    /// Input file or directory
    #[arg(value_name = "INPUT")]
    input: PathBuf,

    /// Output directory (defaults to input directory)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Disable peephole optimization
    #[arg(long = "no-optimize")]
    no_optimize: bool,
}

fn main() -> ExitCode {
    let args = Args::parse();
    let options = CompileOptions {
        optimize: !args.no_optimize,
    };

    let (results, output_dir) = if args.input.is_file() {
        let result = compile_file_with_options(&args.input, options);
        let output_dir = args.output.unwrap_or_else(|| {
            args.input
                .parent()
                .unwrap_or(&PathBuf::from("."))
                .to_path_buf()
        });
        (vec![result], output_dir)
    } else if args.input.is_dir() {
        let results = compile_directory_with_options(&args.input, options);
        let output_dir = args.output.unwrap_or_else(|| args.input.clone());
        (results, output_dir)
    } else {
        eprintln!("Error: Input not found: {}", args.input.display());
        return ExitCode::from(2);
    };

    if results.is_empty() {
        eprintln!("Error: No .jack files found in {}", args.input.display());
        return ExitCode::from(2);
    }

    let mut has_errors = false;

    for result in &results {
        if result.is_ok() {
            match write_result(result, &output_dir) {
                Ok(()) => {
                    println!(
                        "Compiled {}.jack -> {}.vm",
                        result.filename, result.filename
                    );
                }
                Err(e) => {
                    eprintln!("Error writing {}.vm: {}", result.filename, e);
                    has_errors = true;
                }
            }
        } else {
            has_errors = true;
            for err in &result.errors {
                eprintln!("{}: {}", result.filename, err);
            }
        }
    }

    if has_errors {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
