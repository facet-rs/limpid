//! Command-line interface handling

use anyhow::{anyhow, Result};
use std::path::PathBuf;

/// CLI configuration parsed from command-line arguments
#[derive(Debug, Clone)]
pub struct CliConfig {
    /// Generate markdown report to file
    pub markdown_output: Option<PathBuf>,
    /// Enable verbose logging
    pub verbose: bool,
}

impl CliConfig {
    /// Parse command-line arguments
    pub fn from_args() -> Result<Self> {
        let args: Vec<String> = std::env::args().collect();
        
        let mut markdown_output = None;
        let mut verbose = false;
        
        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--markdown" | "-m" => {
                    // Check if next argument is a path
                    if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                        markdown_output = Some(PathBuf::from(&args[i + 1]));
                        i += 1;
                    } else {
                        return Err(anyhow!(
                            "--markdown flag requires a file path argument\nUsage: {} --markdown <output-file>",
                            args[0]
                        ));
                    }
                    i += 1;
                }
                "--verbose" | "-v" => {
                    verbose = true;
                    i += 1;
                }
                "--help" | "-h" => {
                    print_help(&args[0]);
                    std::process::exit(0);
                }
                arg if arg.starts_with('-') => {
                    return Err(anyhow!("Unknown argument: {}", arg));
                }
                _ => i += 1,
            }
        }
        
        Ok(Self {
            markdown_output,
            verbose,
        })
    }
    
    /// Initialize logging based on verbose flag
    pub fn init_logging(&self) {
        if self.verbose {
            std::env::set_var("RUST_LOG", "debug");
        } else if std::env::var("RUST_LOG").is_err() {
            std::env::set_var("RUST_LOG", "info");
        }
        
        env_logger::init();
    }
    
    /// Check if we're in markdown mode
    pub fn is_markdown_mode(&self) -> bool {
        self.markdown_output.is_some()
    }
}

/// Print help message
fn print_help(program_name: &str) {
    println!("Usage: {} [OPTIONS]", program_name);
    println!();
    println!("OPTIONS:");
    println!("  -m, --markdown <file>  Generate markdown report to file");
    println!("  -v, --verbose          Enable verbose logging");
    println!("  -h, --help             Show this help message");
    println!();
    println!("DESCRIPTION:");
    println!("  Limpid analyzes binary size changes in the Facet serialization framework.");
    println!("  It compares the current branch against the main branch and generates");
    println!("  detailed reports about size changes, build times, and code generation.");
    println!();
    println!("EXAMPLES:");
    println!("  # Generate a CLI report");
    println!("  {}", program_name);
    println!();
    println!("  # Generate a markdown report");
    println!("  {} --markdown report.md", program_name);
    println!();
    println!("  # Enable verbose logging");
    println!("  {} --verbose", program_name);
}