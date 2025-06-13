use anyhow::{anyhow, Result};
use camino::Utf8PathBuf;
use pico_args::Arguments;

/// CLI configuration parsed from command-line arguments
#[derive(Debug, Clone)]
pub struct CliConfig {
    /// Generate markdown report to file
    pub markdown_output: Option<Utf8PathBuf>,
    /// Enable verbose logging
    pub verbose: bool,
}

impl CliConfig {
    /// Parse command-line arguments
    pub fn from_args() -> Result<Self> {
        let mut pargs = Arguments::from_env();

        if pargs.contains(["-h", "--help"]) {
            // pico-args does not have a prog_name() method, so use std::env::args()
            let program_name = std::env::args()
                .next()
                .unwrap_or_else(|| "prog".to_string());
            print_help(&program_name);
            std::process::exit(0);
        }

        let markdown_output: Option<Utf8PathBuf> =
            pargs.opt_value_from_os_str(["-m", "--markdown"], |s| {
                s.to_str()
                    .ok_or_else(|| anyhow!("Non-UTF8 path for markdown"))
                    .map(Utf8PathBuf::from)
            })?;

        let verbose = pargs.contains(["-v", "--verbose"]);

        // Any argument left means an unrecognized argument.
        let rest = pargs.finish();
        if !rest.is_empty() {
            return Err(anyhow!(
                "Unknown argument(s): {}",
                rest.iter()
                    .map(|a| a.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(" ")
            ));
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
