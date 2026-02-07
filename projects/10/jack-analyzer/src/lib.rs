//! Jack Analyzer - Syntax analyzer for the Jack programming language.
//!
//! This crate provides lexical analysis (tokenization) and syntactic analysis
//! (parsing) for the Jack language, producing XML output as specified by the
//! nand2tetris course Project 10.
//!
//! # Usage
//!
//! ```no_run
//! use jack_analyzer::{analyze_file, analyze_directory};
//! use std::path::Path;
//!
//! // Analyze a single file
//! let result = analyze_file(Path::new("Main.jack"));
//!
//! // Analyze a directory (parallel processing)
//! let results = analyze_directory(Path::new("Square/"));
//! ```

pub mod ast;
pub mod error;
pub mod parser;
pub mod token;
pub mod tokenizer;
pub mod xml;

use error::JackError;
use parser::Parser;
use rayon::prelude::*;
use std::fs;
use std::path::Path;
use tokenizer::JackTokenizer;

/// Result of analyzing a single Jack file.
#[derive(Debug)]
pub struct AnalysisResult {
    /// The filename that was analyzed.
    pub filename: String,
    /// The original source code (retained for diagnostic formatting).
    pub source: String,
    /// Token XML output (for *T.xml file).
    pub token_xml: String,
    /// Parse tree XML output (for *.xml file).
    pub parse_xml: String,
    /// Any errors encountered during analysis.
    pub errors: Vec<JackError>,
}

impl AnalysisResult {
    /// Check if the analysis was successful (no errors).
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Analyze a single Jack file.
///
/// Returns an `AnalysisResult` containing the token XML, parse tree XML,
/// and any errors encountered.
pub fn analyze_file(path: &Path) -> AnalysisResult {
    let filename = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Read the source file
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            return AnalysisResult {
                filename,
                source: String::new(),
                token_xml: String::new(),
                parse_xml: String::new(),
                errors: vec![JackError::io(path, e)],
            };
        }
    };

    analyze_source(&source, &filename)
}

/// Analyze Jack source code directly.
///
/// This is useful for testing or when the source is already in memory.
pub fn analyze_source(source: &str, filename: &str) -> AnalysisResult {
    // Tokenize
    let tokenizer = JackTokenizer::new(source);
    let tokens = match tokenizer.tokenize() {
        Ok(tokens) => tokens,
        Err(errors) => {
            return AnalysisResult {
                filename: filename.to_string(),
                source: source.to_string(),
                token_xml: String::new(),
                parse_xml: String::new(),
                errors,
            };
        }
    };

    // Generate token XML
    let token_xml = xml::tokens_to_xml(&tokens);

    // Parse
    let parser = Parser::new(&tokens);
    let class = match parser.parse() {
        Ok(class) => class,
        Err(errors) => {
            return AnalysisResult {
                filename: filename.to_string(),
                source: source.to_string(),
                token_xml,
                parse_xml: String::new(),
                errors,
            };
        }
    };

    // Generate parse tree XML
    let parse_xml = xml::XmlWriter::new().write_class(&class, &tokens);

    AnalysisResult {
        filename: filename.to_string(),
        source: source.to_string(),
        token_xml,
        parse_xml,
        errors: Vec::new(),
    }
}

/// Analyze all Jack files in a directory.
///
/// Uses parallel processing via Rayon to analyze multiple files concurrently.
pub fn analyze_directory(dir: &Path) -> Vec<AnalysisResult> {
    let jack_files: Vec<_> = match fs::read_dir(dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|ext| ext == "jack"))
            .collect(),
        Err(e) => {
            return vec![AnalysisResult {
                filename: dir.to_string_lossy().to_string(),
                source: String::new(),
                token_xml: String::new(),
                parse_xml: String::new(),
                errors: vec![JackError::io(dir, e)],
            }];
        }
    };

    if jack_files.is_empty() {
        return Vec::new();
    }

    // Parallel analysis
    jack_files
        .par_iter()
        .map(|path| analyze_file(path))
        .collect()
}

/// Write analysis results to output files.
///
/// Creates *T.xml (tokens) and *.xml (parse tree) files.
pub fn write_results(result: &AnalysisResult, output_dir: &Path) -> Result<(), JackError> {
    let stem = result
        .filename
        .strip_suffix(".jack")
        .unwrap_or(&result.filename);

    // Write token XML
    let token_path = output_dir.join(format!("{}T.xml", stem));
    fs::write(&token_path, &result.token_xml).map_err(|e| JackError::io(&token_path, e))?;

    // Write parse tree XML
    let parse_path = output_dir.join(format!("{}.xml", stem));
    fs::write(&parse_path, &result.parse_xml).map_err(|e| JackError::io(&parse_path, e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_source() {
        let source = "class Main { function void main() { return; } }";
        let result = analyze_source(source, "Main.jack");

        assert!(result.is_ok());
        assert!(!result.token_xml.is_empty());
        assert!(!result.parse_xml.is_empty());
        assert!(result.token_xml.contains("<tokens>"));
        assert!(result.parse_xml.contains("<class>"));
    }

    #[test]
    fn test_analyze_source_with_error() {
        let source = "class Main { function void main() { let x = ; return; } }";
        let result = analyze_source(source, "Main.jack");

        assert!(!result.is_ok());
        assert!(!result.errors.is_empty());
    }
}
