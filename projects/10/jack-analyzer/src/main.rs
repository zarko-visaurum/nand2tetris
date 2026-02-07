//! JackAnalyzer CLI - Syntax analyzer for the Jack programming language.

use clap::Parser as ClapParser;
use jack_analyzer::error::format_errors;
use jack_analyzer::{analyze_directory, analyze_file, write_results};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(ClapParser, Debug)]
#[command(name = "JackAnalyzer")]
#[command(author = "nand2tetris")]
#[command(version = "1.0.0")]
#[command(about = "Syntax analyzer for the Jack programming language")]
struct Args {
    /// Input file (.jack) or directory containing .jack files
    #[arg(value_name = "INPUT")]
    input: PathBuf,

    /// Output directory (defaults to input directory)
    #[arg(short, long)]
    output: Option<PathBuf>,
}

fn main() -> ExitCode {
    let args = Args::parse();

    let (results, output_dir) = if args.input.is_file() {
        let result = analyze_file(&args.input);
        let output_dir = args
            .output
            .unwrap_or_else(|| args.input.parent().unwrap_or(&args.input).to_path_buf());
        (vec![result], output_dir)
    } else if args.input.is_dir() {
        let results = analyze_directory(&args.input);
        let output_dir = args.output.unwrap_or_else(|| args.input.clone());
        (results, output_dir)
    } else {
        eprintln!("Error: Input path does not exist: {}", args.input.display());
        return ExitCode::from(2);
    };

    if results.is_empty() {
        eprintln!("Error: No .jack files found in {}", args.input.display());
        return ExitCode::from(2);
    }

    let mut has_errors = false;

    for result in &results {
        if !result.errors.is_empty() {
            has_errors = true;
            eprint!(
                "{}",
                format_errors(&result.errors, &result.source, &result.filename)
            );
        } else if let Err(e) = write_results(result, &output_dir) {
            eprintln!("Error writing output for {}: {}", result.filename, e);
            has_errors = true;
        }
    }

    if has_errors {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
