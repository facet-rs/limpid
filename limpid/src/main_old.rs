use camino::{Utf8Path, Utf8PathBuf};
use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::{OwoColorize, Style};
use std::fmt::Write;
use std::process::{Command, Output};
use substance::{
    AnalysisComparison, AnalysisConfig, ArtifactKind, BloatAnalyzer, 
    BuildOptions, BuildRunner, BuildType,
};

/// Run a command and print it with full arguments
fn run_command(cmd: &mut Command) -> Result<Output, Box<dyn std::error::Error>> {
    // Build the command string for display
    let program = cmd.get_program().to_string_lossy();
    let args: Vec<String> = cmd
        .get_args()
        .map(|s| s.to_string_lossy().to_string())
        .collect();
    let full_command = format!("{} {}", program, args.join(" "));

    println!(
        "{} {}",
        "🔧 Running:".bright_black(),
        full_command.bright_blue()
    );

    let output = cmd.output()?;
    Ok(output)
}

/// Create a git worktree for the given repository
fn create_worktree(
    repo_path: &Utf8PathBuf,
    worktree_path: &Utf8PathBuf,
    branch: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // First, remove any existing worktree at this path
    if worktree_path.exists() {
        println!("🧹 Removing existing worktree at {}", worktree_path);
        std::fs::remove_dir_all(worktree_path)?;
    }

    // Create parent directory if needed
    if let Some(parent) = worktree_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    println!(
        "🌳 Creating worktree for {} at {}",
        repo_path, worktree_path
    );

    // Create the worktree
    let mut cmd = Command::new("git");
    cmd.args([
        "worktree",
        "add",
        "--detach",
        worktree_path.as_str(),
        branch,
    ])
    .current_dir(repo_path);

    let output = run_command(&mut cmd)?;

    if !output.status.success() {
        return Err(format!(
            "Failed to create worktree: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    Ok(())
}

/// Remove a git worktree
fn remove_worktree(
    repo_path: &Utf8PathBuf,
    worktree_path: &Utf8PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🧹 Removing worktree at {}", worktree_path);

    // Remove the worktree directory
    if worktree_path.exists() {
        std::fs::remove_dir_all(worktree_path)?;
    }

    // Run git worktree prune to clean up
    let mut cmd = Command::new("git");
    cmd.args(["worktree", "prune"]).current_dir(repo_path);

    let output = run_command(&mut cmd)?;

    if !output.status.success() {
        eprintln!(
            "Warning: git worktree prune failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

/// Create a comparison workspace with both facet and limpid worktrees
fn create_comparison_workspace(
    facet_repo: &Utf8PathBuf,
    limpid_repo: &Utf8PathBuf,
    workspace_dir: &Utf8PathBuf,
) -> Result<(Utf8PathBuf, Utf8PathBuf), Box<dyn std::error::Error>> {
    println!("\n{} Creating comparison workspace...", "🏗️ ".bright_blue());

    // Create the workspace directory
    std::fs::create_dir_all(workspace_dir)?;
    println!(
        "  {} Created workspace at {}",
        "✅".green(),
        workspace_dir.bright_blue()
    );

    // Create facet worktree at main branch
    let facet_worktree = workspace_dir.join("facet");
    println!(
        "\n  {} Creating facet worktree at main branch...",
        "1️⃣ ".bright_black()
    );
    create_worktree(facet_repo, &facet_worktree, "origin/main")?;

    // Get current HEAD of limpid for the worktree
    let mut cmd = Command::new("git");
    cmd.args(["rev-parse", "HEAD"]).current_dir(limpid_repo);
    let output = run_command(&mut cmd)?;
    let limpid_head = std::str::from_utf8(&output.stdout)?.trim();

    // Create limpid worktree at current HEAD
    let limpid_worktree = workspace_dir.join("limpid");
    println!(
        "\n  {} Creating limpid worktree at HEAD ({})...",
        "2️⃣ ".bright_black(),
        (&limpid_head[..8]).yellow()
    );
    create_worktree(limpid_repo, &limpid_worktree, limpid_head)?;

    println!(
        "\n  {} Workspace created successfully!",
        "🎉".bright_green()
    );
    println!(
        "    {} Facet:  {}",
        "•".bright_black(),
        facet_worktree.bright_blue()
    );
    println!(
        "    {} Limpid: {}",
        "•".bright_black(),
        limpid_worktree.bright_blue()
    );

    Ok((facet_worktree, limpid_worktree))
}

/// Format bytes into human-readable units
fn format_bytes(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = 1024 * KIB;
    const GIB: u64 = 1024 * MIB;

    if bytes >= GIB {
        format!("{:.2} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.2} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.2} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Format size difference with appropriate color and sign
fn format_size_diff(diff: i64) -> String {
    let abs_diff = diff.abs();
    let formatted = format_bytes(abs_diff.try_into().unwrap_or_default());

    if diff > 0 {
        format!("+{}", formatted).red().to_string()
    } else if diff < 0 {
        format!("-{}", formatted).green().to_string()
    } else {
        "no change".bright_black().to_string()
    }
}

/// Format size difference for markdown (no colors)
fn format_size_diff_md(diff: i64) -> String {
    let abs_diff = diff.abs();
    let formatted = format_bytes(abs_diff.try_into().unwrap_or_default());

    if diff > 0 {
        format!("+{}", formatted)
    } else if diff < 0 {
        format!("-{}", formatted)
    } else {
        "no change".to_string()
    }
}

/// Extract crate name from a function name (similar to substance's logic)
fn extract_crate_from_function(func_name: &str) -> String {
    // Handle generic implementations and trait bounds
    let cleaned = if func_name.starts_with("<") {
        // For functions like "<T as alloc::vec::Vec>::method", extract after "as"
        if let Some(as_pos) = func_name.find(" as ") {
            let after_as = &func_name[as_pos + 4..];
            if let Some(end) = after_as.find(">::") {
                after_as[..end].to_string()
            } else if let Some(end) = after_as.find(">") {
                after_as[..end].to_string()
            } else {
                after_as.to_string()
            }
        } else if let Some(space_pos) = func_name.find(" ") {
            // Handle other generic patterns
            func_name[space_pos + 1..].to_string()
        } else {
            func_name.to_string()
        }
    } else {
        func_name.to_string()
    };

    // Extract the crate name from the cleaned function name
    let parts: Vec<&str> = cleaned.split("::").collect();
    if parts.is_empty() {
        return "unknown".to_string();
    }

    let first_part = parts[0];
    
    // Common Rust standard library crates
    let std_crates = ["core", "alloc", "std", "proc_macro", "test"];
    if std_crates.contains(&first_part) {
        return first_part.to_string();
    }

    // If it's a known crate pattern, return it
    if !first_part.is_empty() 
        && !first_part.starts_with('<')
        && !first_part.starts_with('_')
        && !first_part.chars().all(|c| c.is_numeric())
        && first_part.chars().all(|c| c.is_alphanumeric() || c == '_')
    {
        return first_part.to_string();
    }

    // For complex functions, try to find a crate name in the path
    for part in parts {
        if !part.is_empty()
            && !part.starts_with('<')
            && !part.starts_with('_')
            && !part.chars().all(|c| c.is_numeric())
            && part.chars().all(|c| c.is_alphanumeric() || c == '_')
        {
            // Check if this looks like a crate name (not a type or function)
            if !part.chars().next().map_or(false, |c| c.is_uppercase()) {
                return part.to_string();
            }
        }
    }

    // Default to unknown
    "unknown".to_string()
}

/// Find the root of a git repository starting from the given path
fn find_git_root(start_path: &Utf8Path) -> Result<Utf8PathBuf, Box<dyn std::error::Error>> {
    let mut cmd = Command::new("git");
    cmd.args(["rev-parse", "--show-toplevel"])
        .current_dir(start_path);

    let output = run_command(&mut cmd)?;

    if !output.status.success() {
        return Err(format!(
            "Failed to find git root: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let path = std::str::from_utf8(&output.stdout)?.trim();
    Ok(Utf8PathBuf::from(path))
}

/// Result of build and analysis
struct BuildAnalysisResult {
    analysis: substance::AnalysisResult,
    timing_data: Vec<substance::TimingInfo>,
    wall_time: std::time::Duration,
    build_context: substance::BuildContext,
}


/// Analysis report for a single version
#[derive(Debug)]
struct SingleVersionReport {
    commit_hash: String,
    file_size: u64,
    text_size: u64,
    wall_time: std::time::Duration,
    top_crates: Vec<(String, u64, f64)>, // (name, size, percentage)
    top_symbols: Vec<(String, u64)>, // (name, size)
    llvm_analysis: Option<substance::LlvmIrAnalysis>,
    build_context: substance::BuildContext,
    timing_data: Vec<substance::TimingInfo>,
}

/// Full report that can represent either a single analysis or comparison
#[derive(Debug)]
enum Report {
    Single(SingleVersionReport),
    Comparison {
        main: SingleVersionReport,
        current: SingleVersionReport,
        comparison: substance::AnalysisComparison,
        crate_time_changes: Vec<(String, Option<f64>, Option<f64>)>,
    },
}

impl SingleVersionReport {
    /// Create a report from build analysis results
    fn from_build_result(
        result: &BuildAnalysisResult,
        commit_hash: String,
        config: &AnalysisConfig,
    ) -> Self {
        // Calculate crate sizes
        let mut crate_sizes = std::collections::HashMap::new();
        for symbol in &result.analysis.symbols {
            let (crate_name, _) = substance::crate_name::from_sym(
                &result.build_context,
                config.split_std,
                &symbol.name,
            );
            *crate_sizes.entry(crate_name).or_insert(0) += symbol.size;
        }
        
        // Sort crates by size and calculate percentages
        let mut crate_list: Vec<(String, u64)> = crate_sizes.into_iter().collect();
        crate_list.sort_by_key(|(_, size)| std::cmp::Reverse(*size));
        
        let top_crates: Vec<(String, u64, f64)> = crate_list
            .into_iter()
            .take(15)
            .map(|(name, size)| {
                let percentage = size as f64 / result.analysis.text_size as f64 * 100.0;
                (name, size, percentage)
            })
            .collect();
        
        // Get top symbols
        let mut symbol_list: Vec<(String, u64)> = result.analysis.symbols
            .iter()
            .map(|s| (s.name.trimmed.clone(), s.size))
            .collect();
        symbol_list.sort_by_key(|(_, size)| std::cmp::Reverse(*size));
        let top_symbols = symbol_list.into_iter().take(30).collect();
        
        Self {
            commit_hash,
            file_size: result.analysis.file_size,
            text_size: result.analysis.text_size,
            wall_time: result.wall_time,
            top_crates,
            top_symbols,
            llvm_analysis: result.analysis.llvm_ir_data.clone(),
            build_context: result.build_context.clone(),
            timing_data: result.timing_data.clone(),
        }
    }
}


/// Build and analyze a specific version of ks-facet
fn build_and_analyze(
    ks_facet_manifest: &Utf8PathBuf,
    target_dir: &Utf8PathBuf,
    version_name: &str,
) -> Result<BuildAnalysisResult, Box<dyn std::error::Error>> {
    println!(
        "\n{} Building {} version...",
        "🚀".yellow(),
        version_name.cyan().bold()
    );
    println!(
        "  {} {}",
        "📄 Manifest:".bright_black(),
        ks_facet_manifest.bright_blue()
    );
    println!(
        "  {} {}",
        "📁 Target dir:".bright_black(),
        target_dir.bright_blue()
    );

    // Configure to build the binary
    let build_options = BuildOptions {
        build_bin: Some("ks-facet".to_string()),
        ..Default::default()
    };

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
    );
    spinner.set_message("Building with cargo...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    let build_start = std::time::Instant::now();
    let build_result = BuildRunner::new(
        ks_facet_manifest.as_std_path(),
        target_dir.as_std_path(),
        BuildType::Release,
    )
    .with_options(build_options)
    .run()?;

    let actual_build_time = build_start.elapsed();
    spinner.finish_and_clear();

    // Calculate total build time (sum of all crates - parallel time would be less)
    let total_crate_time: f64 = build_result.timing_data.iter().map(|t| t.duration).sum();

    println!(
        "{} {} build completed in {:.2}s (wall time: {:.2}s)",
        "✅".green(),
        version_name.cyan(),
        total_crate_time.to_string().yellow(),
        actual_build_time.as_secs_f64()
    );

    // Find the ks-facet binary
    let ks_facet_binary = build_result
        .context
        .artifacts
        .iter()
        .find(|a| a.kind == ArtifactKind::Binary && a.name == "ks_facet")
        .ok_or("ks-facet binary not found in build artifacts")?;

    println!(
        "  {} {}",
        "📦 Binary found:".bright_black(),
        ks_facet_binary.path.display().to_string().bright_blue()
    );

    // Analyze the binary
    let analysis_spinner = ProgressBar::new_spinner();
    analysis_spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
    );
    analysis_spinner.set_message("Computing binary analysis...");
    analysis_spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    let config = AnalysisConfig {
        analyze_llvm_ir: true,
        target_dir: Some(target_dir.as_std_path().to_owned()),
        build_type: Some(BuildType::Release),
        ..Default::default()
    };
    let analysis =
        BloatAnalyzer::analyze_binary(&ks_facet_binary.path, &build_result.context, &config)?;

    analysis_spinner.finish_and_clear();

    println!(
        "  {} {} (text: {})",
        "📊 Size:".bright_black(),
        format_bytes(analysis.file_size).yellow().bold(),
        format_bytes(analysis.text_size).yellow()
    );

    Ok(BuildAnalysisResult {
        analysis,
        timing_data: build_result.timing_data,
        wall_time: actual_build_time,
        build_context: build_result.context,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments first to check for verbose flag
    let args: Vec<String> = std::env::args().collect();
    let mut markdown_mode = false;
    let mut markdown_output_path: Option<String> = None;
    let mut verbose_mode = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--markdown" | "-m" => {
                markdown_mode = true;
                // Check if next argument is a path
                if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                    markdown_output_path = Some(args[i + 1].clone());
                    i += 1;
                }
                i += 1;
            }
            "--verbose" | "-v" => {
                verbose_mode = true;
                i += 1;
            }
            "--help" | "-h" => {
                println!("Usage: {} [OPTIONS]", args[0]);
                println!();
                println!("OPTIONS:");
                println!("  -m, --markdown <file>  Generate markdown report to file");
                println!("  -v, --verbose          Enable verbose logging");
                println!("  -h, --help             Show this help message");
                std::process::exit(0);
            }
            _ => i += 1,
        }
    }

    // Validate output modes
    if markdown_mode && markdown_output_path.is_none() {
        eprintln!("Error: --markdown flag requires a file path argument");
        eprintln!("Usage: {} --markdown <output-file>", args[0]);
        std::process::exit(1);
    }
    
    // Set logging level based on verbose flag
    if verbose_mode {
        std::env::set_var("RUST_LOG", "debug");
    } else if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    
    // Initialize logging after setting the level
    env_logger::init();

    // Always print the header regardless of markdown mode
    println!("{}", "🌊 Limpid - Binary Size Analyzer".blue().bold());
    println!("{}", "─".repeat(40).bright_black());

    // Get current directory
    let current_dir = Utf8PathBuf::from_path_buf(std::env::current_dir()?)
        .map_err(|_| "Current directory is not valid UTF-8")?;

    println!(
        "{} {}",
        "📍 Current directory:".bright_black(),
        current_dir.bright_blue()
    );

    // Find the limpid git repository root
    let limpid_root = find_git_root(&current_dir)?;
    println!(
        "{} {}",
        "🌳 Limpid repo root:".bright_black(),
        limpid_root.green()
    );

    // Find the parent directory where facet should be
    let workspace_root = limpid_root
        .parent()
        .ok_or("Could not find parent of limpid repository")?;
    println!(
        "{} {}",
        "📁 Workspace root:".bright_black(),
        workspace_root.green()
    );

    // Verify kitchensink exists
    let kitchensink_dir = limpid_root.join("kitchensink");
    if !kitchensink_dir.exists() {
        return Err(format!(
            "Kitchensink directory not found at: {}",
            kitchensink_dir.red()
        )
        .into());
    }
    println!(
        "{} {}",
        "✅ Found kitchensink:".bright_black(),
        kitchensink_dir.green()
    );

    let ks_facet_manifest = kitchensink_dir.join("ks-facet/Cargo.toml");
    if !ks_facet_manifest.exists() {
        return Err(format!(
            "ks-facet manifest not found at: {}",
            ks_facet_manifest.red()
        )
        .into());
    }
    println!(
        "{} {}",
        "✅ Found ks-facet manifest:".bright_black(),
        ks_facet_manifest.green()
    );

    // Use a temporary target directory to avoid workspace issues
    let temp_dir = Utf8PathBuf::from_path_buf(std::env::temp_dir())
        .map_err(|_| "Temp dir is not valid UTF-8")?;
    let target_dir = temp_dir.join(format!("limpid-ks-facet-build-{}", std::process::id()));

    // Create the directory if it doesn't exist
    std::fs::create_dir_all(&target_dir)?;

    // Build and analyze the current version
    let current_result = build_and_analyze(&ks_facet_manifest, &target_dir, "current")?;
    
    // Get current commit hash if possible
    let current_commit_hash = if limpid_root.join(".git").exists() {
        let mut cmd = Command::new("git");
        cmd.args(["rev-parse", "HEAD"]).current_dir(&limpid_root);
        match run_command(&mut cmd) {
            Ok(output) => std::str::from_utf8(&output.stdout).unwrap_or("unknown").trim().to_string(),
            Err(_) => "unknown".to_string(),
        }
    } else {
        "unknown".to_string()
    };

    // Check if we should do comparison or single analysis
    let facet_root = workspace_root.join("facet");
    if facet_root.exists() && facet_root.join(".git").exists() {
        println!(
            "\n{}",
            "🔄 Comparing with main branch...".bright_magenta().bold()
        );
        println!("{}", "─".repeat(40).bright_black());

        // Get current commit hash
        let mut cmd = Command::new("git");
        cmd.args(["rev-parse", "HEAD"]).current_dir(&facet_root);

        let current_commit = run_command(&mut cmd)?;
        let current_hash = std::str::from_utf8(&current_commit.stdout)?.trim();
        println!(
            "{} {} ({})",
            "📌 Current commit:".bright_black(),
            (&current_hash[..8]).yellow(),
            current_hash.bright_black()
        );

        // Create comparison workspace with both worktrees
        let workspace_dir = temp_dir.join(format!("limpid-main-workspace-{}", std::process::id()));
        let (facet_worktree, limpid_worktree) =
            create_comparison_workspace(&facet_root, &limpid_root, &workspace_dir)?;

        // Build ks-facet from main branch worktree
        let main_ks_facet_manifest = limpid_worktree.join("kitchensink/ks-facet/Cargo.toml");
        if main_ks_facet_manifest.exists() {
            let main_target_dir =
                temp_dir.join(format!("limpid-ks-facet-main-{}", std::process::id()));
            std::fs::create_dir_all(&main_target_dir)?;

            let main_result = build_and_analyze(&main_ks_facet_manifest, &main_target_dir, "main")?;

            // Compare analyses
            println!("\n{}", "📊 Size Comparison:".white().bold());
            println!("{}", "─".repeat(50).bright_black());

            let size_diff =
                current_result.analysis.file_size as i64 - main_result.analysis.file_size as i64;
            let text_diff =
                current_result.analysis.text_size as i64 - main_result.analysis.text_size as i64;

            println!(
                "  {} {}",
                "File size:".bright_black(),
                format!(
                    "{} → {} ({})",
                    format_bytes(main_result.analysis.file_size).yellow(),
                    format_bytes(current_result.analysis.file_size).yellow(),
                    format_size_diff(size_diff)
                )
                .white()
            );

            println!(
                "  {} {}",
                "Text size:".bright_black(),
                format!(
                    "{} → {} ({})",
                    format_bytes(main_result.analysis.text_size).yellow(),
                    format_bytes(current_result.analysis.text_size).yellow(),
                    format_size_diff(text_diff)
                )
                .white()
            );

            // Add build time comparison
            println!(
                "  {} {}",
                "Build time:".bright_black(),
                format!(
                    "{:.2}s → {:.2}s ({:+.2}s)",
                    main_result.wall_time.as_secs_f64(),
                    current_result.wall_time.as_secs_f64(),
                    current_result.wall_time.as_secs_f64() - main_result.wall_time.as_secs_f64()
                )
                .white()
            );

            // Analyze changes using Substance's comparison API
            let comparison =
                AnalysisComparison::compare(&main_result.analysis, &current_result.analysis)?;

            // Prepare crate time changes data (needed for both markdown and CLI output)
            let mut main_crate_times: std::collections::HashMap<String, f64> =
                std::collections::HashMap::new();
            let mut current_crate_times: std::collections::HashMap<String, f64> =
                std::collections::HashMap::new();

            for timing in &main_result.timing_data {
                main_crate_times.insert(timing.crate_name.clone(), timing.duration);
            }
            for timing in &current_result.timing_data {
                current_crate_times.insert(timing.crate_name.clone(), timing.duration);
            }

            // Combine and sort by absolute time difference
            let mut all_crate_names = std::collections::HashSet::new();
            all_crate_names.extend(main_crate_times.keys().cloned());
            all_crate_names.extend(current_crate_times.keys().cloned());

            let mut crate_time_changes: Vec<(String, Option<f64>, Option<f64>)> = all_crate_names
                .into_iter()
                .map(|name| {
                    (
                        name.clone(),
                        main_crate_times.get(&name).copied(),
                        current_crate_times.get(&name).copied(),
                    )
                })
                .collect();

            crate_time_changes.sort_by(|a, b| {
                let a_diff = match (a.1, a.2) {
                    (Some(before), Some(after)) => (after - before).abs(),
                    (None, Some(after)) => after,
                    (Some(before), None) => before,
                    _ => 0.0,
                };
                let b_diff = match (b.1, b.2) {
                    (Some(before), Some(after)) => (after - before).abs(),
                    (None, Some(after)) => after,
                    (Some(before), None) => before,
                    _ => 0.0,
                };
                b_diff.partial_cmp(&a_diff).unwrap()
            });

            // All display code removed - will be handled by Report at the end
            let mut crate_changes = comparison.crate_changes.clone();
            crate_changes.sort_by(|a, b| {
                let a_change = a.absolute_change().map(|c| c.abs()).unwrap_or(0);
                let b_change = b.absolute_change().map(|c| c.abs()).unwrap_or(0);
                b_change.cmp(&a_change)
            });

            println!("\n{}", "📦 Crate Size Changes:".white().bold());
            println!("{}", "─".repeat(60).bright_black());

            let significant_crate_changes: Vec<_> = crate_changes
                .iter()
                .filter(|c| {
                    c.absolute_change()
                        .map(|change| change != 0)
                        .unwrap_or(true)
                })
                .collect();

            for change in significant_crate_changes.iter().take(15) {
                match (change.size_before, change.size_after) {
                    (Some(before), Some(after)) => {
                        let abs_change = change.absolute_change().unwrap();
                        let pct = change.percent_change().unwrap();
                        println!(
                            "  {:>10} → {:>10} ({:>10}) {:+5.1}%  {}",
                            format_bytes(before).yellow(),
                            format_bytes(after).yellow(),
                            format_size_diff(abs_change),
                            pct,
                            change.name.bright_white()
                        );
                    }
                    (None, Some(after)) => {
                        println!(
                            "  {:>10}   {:>10} ({:>10})   NEW   {}",
                            " ".bright_black(),
                            format_bytes(after).yellow(),
                            format!("+{}", format_bytes(after)).red(),
                            change.name.bright_white()
                        );
                    }
                    (Some(before), None) => {
                        println!(
                            "  {:>10} → {:>10} ({:>10}) REMOVED {}",
                            format_bytes(before).yellow(),
                            "0 B".bright_black(),
                            format!("-{}", format_bytes(before)).green(),
                            change.name.bright_white()
                        );
                    }
                    _ => {}
                }
            }

            if significant_crate_changes.len() > 15 {
                println!(
                    "  {} ... and {} more changes",
                    " ".repeat(10).bright_black(),
                    (significant_crate_changes.len() - 15)
                        .to_string()
                        .bright_cyan()
                );
            }

            // Show crate build time changes
            println!("\n{}", "⏱️  Crate Build Time Changes:".white().bold());
            println!("{}", "─".repeat(60).bright_black());

            // Create maps for timing data
            let mut main_crate_times: std::collections::HashMap<String, f64> =
                std::collections::HashMap::new();
            let mut current_crate_times: std::collections::HashMap<String, f64> =
                std::collections::HashMap::new();

            for timing in &main_result.timing_data {
                main_crate_times.insert(timing.crate_name.clone(), timing.duration);
            }
            for timing in &current_result.timing_data {
                current_crate_times.insert(timing.crate_name.clone(), timing.duration);
            }

            // Combine and sort by absolute time difference
            let mut all_crate_names = std::collections::HashSet::new();
            all_crate_names.extend(main_crate_times.keys().cloned());
            all_crate_names.extend(current_crate_times.keys().cloned());

            let mut crate_time_changes: Vec<(String, Option<f64>, Option<f64>)> = all_crate_names
                .into_iter()
                .map(|name| {
                    (
                        name.clone(),
                        main_crate_times.get(&name).copied(),
                        current_crate_times.get(&name).copied(),
                    )
                })
                .collect();

            crate_time_changes.sort_by(|a, b| {
                let a_diff = match (a.1, a.2) {
                    (Some(before), Some(after)) => (after - before).abs(),
                    (None, Some(after)) => after,
                    (Some(before), None) => before,
                    _ => 0.0,
                };
                let b_diff = match (b.1, b.2) {
                    (Some(before), Some(after)) => (after - before).abs(),
                    (None, Some(after)) => after,
                    (Some(before), None) => before,
                    _ => 0.0,
                };
                b_diff.partial_cmp(&a_diff).unwrap()
            });

            for (crate_name, before, after) in crate_time_changes.iter().take(15) {
                match (before, after) {
                    (Some(before), Some(after)) => {
                        let diff = after - before;
                        let pct = (diff / before) * 100.0;

                        // Prepare the formatted diff string and its style separately
                        let (diff_str, style) = if diff > 0.0 {
                            (format!("+{:.2}s", diff), Style::new().green())
                        } else {
                            (format!("{:.2}s", diff), Style::new().red())
                        };

                        println!(
                            "  {:>7.2}s → {:>7.2}s ({}) {:+5.1}%  {}",
                            before,
                            after,
                            diff_str.style(style),
                            pct,
                            crate_name.bright_white()
                        );
                    }
                    (None, Some(after)) => {
                        println!(
                            "  {:>7}   {:>7.2}s ({})   NEW   {}",
                            " ",
                            after,
                            format!("+{:.2}s", after).red(),
                            crate_name.bright_white()
                        );
                    }
                    (Some(before), None) => {
                        println!(
                            "  {:>7.2}s → {:>7}   ({}) REMOVED {}",
                            before,
                            "0s",
                            format!("-{:.2}s", before).green(),
                            crate_name.bright_white()
                        );
                    }
                    _ => {}
                }
            }

            // First section: Biggest changes by absolute size
            let mut changed_symbols: Vec<_> = comparison
                .symbol_changes
                .iter()
                .filter_map(|s| match (s.size_before, s.size_after) {
                    (Some(before), Some(after)) if before != after => {
                        let change = after as i64 - before as i64;
                        Some((s, change))
                    }
                    (None, Some(after)) => Some((s, after as i64)),
                    (Some(before), None) => Some((s, -(before as i64))),
                    _ => None,
                })
                .collect();

            changed_symbols.sort_by_key(|(_, change)| -change.abs());

            if !changed_symbols.is_empty() {
                println!(
                    "\n{}",
                    "📈 Biggest Symbol Changes (by size):".white().bold()
                );
                println!("{}", "─".repeat(70).bright_black());

                for (change, size_change) in changed_symbols.iter().take(20) {
                    match (change.size_before, change.size_after) {
                        (Some(before), Some(after)) => {
                            println!(
                                "  {:>10} {:>10} → {:>10}  {}",
                                format_size_diff(*size_change),
                                format_bytes(before).yellow(),
                                format_bytes(after).yellow(),
                                change.demangled.bright_white()
                            );
                        }
                        (None, Some(after)) => {
                            println!(
                                "  {:>10} {:>10}   {:>10}  {}",
                                format!("+({})", format_bytes(after)).red(),
                                "NEW".green(),
                                format_bytes(after).yellow(),
                                change.demangled.bright_white()
                            );
                        }
                        (Some(before), None) => {
                            println!(
                                "  {:>10} {:>10}   {:>10}  {}",
                                format!("-{}", format_bytes(before)).green(),
                                format_bytes(before).yellow(),
                                "REMOVED".red(),
                                change.demangled.bright_white()
                            );
                        }
                        _ => {}
                    }
                }

                if changed_symbols.len() > 20 {
                    println!(
                        "  {} ... and {} more changes",
                        " ".repeat(10).bright_black(),
                        (changed_symbols.len() - 20).to_string().bright_cyan()
                    );
                }
            }

            // Second section: Largest symbols in current (HEAD) version
            let mut current_symbols: Vec<_> = comparison
                .symbol_changes
                .iter()
                .filter_map(|s| s.size_after.map(|size| (s, size)))
                .collect();

            current_symbols.sort_by_key(|(_, size)| std::cmp::Reverse(*size));

            println!(
                "\n{}",
                "🏆 Largest Symbols in Current Version:".white().bold()
            );
            println!("{}", "─".repeat(70).bright_black());

            for (symbol, current_size) in current_symbols.iter().take(20) {
                match symbol.size_before {
                    Some(before) if before != *current_size => {
                        let change = *current_size as i64 - before as i64;
                        println!(
                            "  {:>10} ({:>10})  {}",
                            format_bytes(*current_size).yellow(),
                            format_size_diff(change),
                            symbol.demangled.bright_white()
                        );
                    }
                    None => {
                        println!(
                            "  {:>10} ({:>10})  {}",
                            format_bytes(*current_size).yellow(),
                            "NEW".green(),
                            symbol.demangled.bright_white()
                        );
                    }
                    _ => {
                        println!(
                            "  {:>10}               {}",
                            format_bytes(*current_size).yellow(),
                            symbol.demangled.bright_white()
                        );
                    }
                }
            }

            // Create and display the comparison report
            let main_report = SingleVersionReport::from_build_result(
                &main_result,
                "main".to_string(),
                &AnalysisConfig::default(),
            );
            let current_report = SingleVersionReport::from_build_result(
                &current_result,
                current_hash.to_string(),
                &AnalysisConfig::default(),
            );
            
            let report = Report::Comparison {
                main: main_report,
                current: current_report,
                comparison,
                crate_time_changes,
            };
            
            // Generate output based on mode
            if markdown_mode {
                let markdown = report.to_markdown();
                let output_path = std::path::Path::new(markdown_output_path.as_ref().unwrap());
                std::fs::write(output_path, &markdown)?;
                eprintln!("📝 Markdown report written to: {}", output_path.display());
            } else {
                report.display_cli();
            }
            
            // Clean up main branch build directory
            if main_target_dir.exists() {
                std::fs::remove_dir_all(&main_target_dir)?;
            }

            // Clean up workspace (both worktrees)
            println!("\n{} Cleaning up workspace...", "🧹".bright_black());
            if workspace_dir.exists() {
                std::fs::remove_dir_all(&workspace_dir)?;
            }
            remove_worktree(&facet_root, &facet_worktree)?;
            remove_worktree(&limpid_root, &limpid_worktree)?;
        } else {
            println!(
                "⚠️  {} ks-facet not found in main branch",
                "Warning:".yellow()
            );
        }
    } else {
        // No comparison, just show single version report
        println!(
            "\n{} Facet repository not found at {}. Showing single version analysis.",
            "ℹ️ ".bright_blue(),
            facet_root.to_string().bright_black()
        );
        
        let report = SingleVersionReport::from_build_result(
            &current_result,
            current_commit_hash,
            &AnalysisConfig::default(),
        );
        
        let full_report = Report::Single(report);
        
        // Generate output based on mode
        if markdown_mode {
            let markdown = full_report.to_markdown();
            let output_path = std::path::Path::new(markdown_output_path.as_ref().unwrap());
            std::fs::write(output_path, &markdown)?;
            eprintln!("📝 Markdown report written to: {}", output_path.display());
        } else {
            full_report.display_cli();
        }
    }

    // Clean up temporary directory
    if target_dir.exists() {
        println!(
            "\n{} Cleaning up temporary directory...",
            "🧹".bright_black()
        );
        std::fs::remove_dir_all(&target_dir)?;
    }

    Ok(())
}

impl Report {
    /// Display the report in CLI format
    fn display_cli(&self) {
        match self {
            Report::Single(report) => {
                println!("\n{}", "📊 Binary Analysis Report".white().bold());
                println!("{}", "─".repeat(50).bright_black());
                
                // Basic metrics
                println!("  {} {}", "Commit:".bright_black(), report.commit_hash);
                println!("  {} {}", "File size:".bright_black(), format_bytes(report.file_size).yellow());
                println!("  {} {}", "Text size:".bright_black(), format_bytes(report.text_size).yellow());
                println!("  {} {:.2}s", "Build time:".bright_black(), report.wall_time.as_secs_f64());
                
                // Top crates
                println!("\n{}", "📦 Top Crates by Size:".white().bold());
                println!("{}", "─".repeat(50).bright_black());
                for (crate_name, size, percent) in report.top_crates.iter().take(10) {
                    let percent_str = format!("{:5.1}%", percent);
                    println!(
                        "  {:>10} ({}) {}",
                        format_bytes(*size).yellow(),
                        percent_str.bright_cyan(),
                        crate_name.bright_white()
                    );
                }
                
                // LLVM IR analysis
                if let Some(llvm_analysis) = &report.llvm_analysis {
                    println!("\n{}", "🔥 LLVM IR Analysis:".white().bold());
                    println!("{}", "─".repeat(50).bright_black());
                    println!(
                        "  {} {}",
                        "Total LLVM IR lines:".bright_black(),
                        llvm_analysis.total_lines.to_string().yellow()
                    );
                    println!(
                        "  {} {}",
                        "Total instantiations:".bright_black(),
                        llvm_analysis.total_copies.to_string().yellow()
                    );
                    println!(
                        "  {} {}",
                        "Analyzed .ll files:".bright_black(),
                        llvm_analysis.analyzed_files.len().to_string().yellow()
                    );
                    
                    // Top functions by LLVM IR lines
                    let mut functions: Vec<(&String, &substance::llvm_ir::LlvmInstantiations)> =
                        llvm_analysis.instantiations.iter().collect();
                    functions.sort_by_key(|(_, stats)| std::cmp::Reverse(stats.total_lines));
                    
                    println!(
                        "\n{}",
                        "🔍 Top 10 Functions by LLVM IR Lines:".white().bold()
                    );
                    println!("{}", "─".repeat(70).bright_black());
                    for (rank, (func_name, stats)) in functions.iter().take(10).enumerate() {
                        let percent = stats.total_lines as f64 / llvm_analysis.total_lines as f64 * 100.0;
                        println!(
                            "{:2}. {:>6} lines ({:>5.1}%) {} instantiations - {}",
                            rank + 1,
                            stats.total_lines.to_string().yellow(),
                            percent,
                            stats.copies.to_string().bright_cyan(),
                            func_name.bright_white()
                        );
                    }
                } else {
                    println!("\n{}", "⚠️  No LLVM IR analysis available".yellow());
                    println!("{}", "    Make sure to build with RUSTFLAGS='--emit=llvm-ir'".bright_black());
                }
            }
            Report::Comparison { main, current, comparison, crate_time_changes } => {
                // Use existing comparison display logic but with the new data structure
                self.display_comparison_cli(main, current, comparison, crate_time_changes);
            }
        }
    }
    
    /// Helper method to display comparison in CLI format
    fn display_comparison_cli(
        &self,
        main: &SingleVersionReport,
        current: &SingleVersionReport,
        comparison: &substance::AnalysisComparison,
        crate_time_changes: &[(String, Option<f64>, Option<f64>)],
    ) {
        println!("\n{}", "📊 Size Comparison:".white().bold());
        println!("{}", "─".repeat(50).bright_black());

        let size_diff = current.file_size as i64 - main.file_size as i64;
        let text_diff = current.text_size as i64 - main.text_size as i64;

        println!(
            "  {} {}",
            "File size:".bright_black(),
            format!(
                "{} → {} ({})",
                format_bytes(main.file_size).yellow(),
                format_bytes(current.file_size).yellow(),
                format_size_diff(size_diff)
            )
            .white()
        );

        println!(
            "  {} {}",
            "Text size:".bright_black(),
            format!(
                "{} → {} ({})",
                format_bytes(main.text_size).yellow(),
                format_bytes(current.text_size).yellow(),
                format_size_diff(text_diff)
            )
            .white()
        );

        // Add build time comparison
        println!(
            "  {} {}",
            "Build time:".bright_black(),
            format!(
                "{:.2}s → {:.2}s ({:+.2}s)",
                main.wall_time.as_secs_f64(),
                current.wall_time.as_secs_f64(),
                current.wall_time.as_secs_f64() - main.wall_time.as_secs_f64()
            )
            .white()
        );
        
        // Show crate-level changes
        let mut crate_changes = comparison.crate_changes.clone();
        crate_changes.sort_by(|a, b| {
            let a_change = a.absolute_change().map(|c| c.abs()).unwrap_or(0);
            let b_change = b.absolute_change().map(|c| c.abs()).unwrap_or(0);
            b_change.cmp(&a_change)
        });

        println!("\n{}", "📦 Crate Size Changes:".white().bold());
        println!("{}", "─".repeat(60).bright_black());

        let significant_crate_changes: Vec<_> = crate_changes
            .iter()
            .filter(|c| {
                c.absolute_change()
                    .map(|change| change != 0)
                    .unwrap_or(true)
            })
            .collect();

        for change in significant_crate_changes.iter().take(15) {
            match (change.size_before, change.size_after) {
                (Some(before), Some(after)) => {
                    let abs_change = change.absolute_change().unwrap();
                    let pct = change.percent_change().unwrap();
                    println!(
                        "  {:>10} → {:>10} ({:>10}) {:+5.1}%  {}",
                        format_bytes(before).yellow(),
                        format_bytes(after).yellow(),
                        format_size_diff(abs_change),
                        pct,
                        change.name.bright_white()
                    );
                }
                (None, Some(after)) => {
                    println!(
                        "  {:>10}   {:>10} ({:>10})   NEW   {}",
                        " ".bright_black(),
                        format_bytes(after).yellow(),
                        format!("+{}", format_bytes(after)).red(),
                        change.name.bright_white()
                    );
                }
                (Some(before), None) => {
                    println!(
                        "  {:>10} → {:>10} ({:>10}) REMOVED {}",
                        format_bytes(before).yellow(),
                        "0 B".bright_black(),
                        format!("-{}", format_bytes(before)).green(),
                        change.name.bright_white()
                    );
                }
                _ => {}
            }
        }
        
        // Show crate build time changes
        if !crate_time_changes.is_empty() {
            println!("\n{}", "⏱️  Crate Build Time Changes:".white().bold());
            println!("{}", "─".repeat(60).bright_black());
            
            for (crate_name, before, after) in crate_time_changes.iter().take(15) {
                match (before, after) {
                    (Some(before), Some(after)) => {
                        let diff = after - before;
                        let pct = (diff / before) * 100.0;
                        
                        let (diff_str, style) = if diff > 0.0 {
                            (format!("+{:.2}s", diff), Style::new().red())
                        } else {
                            (format!("{:.2}s", diff), Style::new().green())
                        };
                        
                        println!(
                            "  {:>7.2}s → {:>7.2}s ({}) {:+5.1}%  {}",
                            before,
                            after,
                            diff_str.style(style),
                            pct,
                            crate_name.bright_white()
                        );
                    }
                    (None, Some(after)) => {
                        println!(
                            "  {:>7}   {:>7.2}s ({})   NEW   {}",
                            " ",
                            after,
                            format!("+{:.2}s", after).red(),
                            crate_name.bright_white()
                        );
                    }
                    (Some(before), None) => {
                        println!(
                            "  {:>7.2}s → {:>7}   ({}) REMOVED {}",
                            before,
                            "0s",
                            format!("-{:.2}s", before).green(),
                            crate_name.bright_white()
                        );
                    }
                    _ => {}
                }
            }
        }
        
        // Show biggest symbol changes
        let mut changed_symbols: Vec<_> = comparison
            .symbol_changes
            .iter()
            .filter_map(|s| match (s.size_before, s.size_after) {
                (Some(before), Some(after)) if before != after => {
                    let change = after as i64 - before as i64;
                    Some((s, change))
                }
                (None, Some(after)) => Some((s, after as i64)),
                (Some(before), None) => Some((s, -(before as i64))),
                _ => None,
            })
            .collect();
        
        changed_symbols.sort_by_key(|(_, change)| -change.abs());
        
        if !changed_symbols.is_empty() {
            println!("\n{}", "📈 Biggest Symbol Changes (by size):".white().bold());
            println!("{}", "─".repeat(70).bright_black());
            
            for (change, size_change) in changed_symbols.iter().take(20) {
                match (change.size_before, change.size_after) {
                    (Some(before), Some(after)) => {
                        println!(
                            "  {:>10} {:>10} → {:>10}  {}",
                            format_size_diff(*size_change),
                            format_bytes(before).yellow(),
                            format_bytes(after).yellow(),
                            change.demangled.bright_white()
                        );
                    }
                    (None, Some(after)) => {
                        println!(
                            "  {:>10} {:>10}   {:>10}  {}",
                            format!("+{}", format_bytes(after)).red(),
                            "NEW".green(),
                            format_bytes(after).yellow(),
                            change.demangled.bright_white()
                        );
                    }
                    (Some(before), None) => {
                        println!(
                            "  {:>10} {:>10}   {:>10}  {}",
                            format!("-{}", format_bytes(before)).green(),
                            format_bytes(before).yellow(),
                            "REMOVED".red(),
                            change.demangled.bright_white()
                        );
                    }
                    _ => {}
                }
            }
        }
        
        // Show top crates by size in current version
        println!("\n{}", "📦 Top Crates by Size (Current Version):".white().bold());
        println!("{}", "─".repeat(50).bright_black());
        for (crate_name, size, percent) in current.top_crates.iter().take(10) {
            let percent_str = format!("{:5.1}%", percent);
            println!(
                "  {:>10} ({}) {}",
                format_bytes(*size).yellow(),
                percent_str.bright_cyan(),
                crate_name.bright_white()
            );
        }
        
        // Show top symbols by size in current version
        println!("\n{}", "🔍 Top Symbols by Size (Current Version):".white().bold());
        println!("{}", "─".repeat(70).bright_black());
        for (symbol_name, size) in current.top_symbols.iter().take(20) {
            println!(
                "  {:>10}  {}",
                format_bytes(*size).yellow(),
                symbol_name.bright_white()
            );
        }

        // Show LLVM IR differential analysis
        if let (Some(main_llvm), Some(current_llvm)) = (&main.llvm_analysis, &current.llvm_analysis) {
            println!("\n{}", "🔥 LLVM IR Differential Analysis:".white().bold());
            println!("{}", "─".repeat(60).bright_black());
            
            let lines_diff = current_llvm.total_lines as i64 - main_llvm.total_lines as i64;
            let copies_diff = current_llvm.total_copies as i64 - main_llvm.total_copies as i64;
            
            println!(
                "  {} {} → {} ({})",
                "Total LLVM IR lines:".bright_black(),
                main_llvm.total_lines.to_string().yellow(),
                current_llvm.total_lines.to_string().yellow(),
                if lines_diff > 0 {
                    format!("+{}", lines_diff).red().to_string()
                } else if lines_diff < 0 {
                    format!("{}", lines_diff).green().to_string()
                } else {
                    "no change".bright_black().to_string()
                }
            );
            
            println!(
                "  {} {} → {} ({})",
                "Total instantiations:".bright_black(),
                main_llvm.total_copies.to_string().yellow(),
                current_llvm.total_copies.to_string().yellow(),
                if copies_diff > 0 {
                    format!("+{}", copies_diff).red().to_string()
                } else if copies_diff < 0 {
                    format!("{}", copies_diff).green().to_string()
                } else {
                    "no change".bright_black().to_string()
                }
            );
            
            // Find functions with biggest LLVM IR changes
            let mut function_changes: Vec<(String, i64, usize, usize, i64)> = Vec::new();
            
            // Create a combined set of all function names
            let mut all_functions = std::collections::HashSet::new();
            all_functions.extend(main_llvm.instantiations.keys().cloned());
            all_functions.extend(current_llvm.instantiations.keys().cloned());
            
            for func_name in all_functions {
                let main_stats = main_llvm.instantiations.get(&func_name);
                let current_stats = current_llvm.instantiations.get(&func_name);
                
                let main_lines = main_stats.map(|s| s.total_lines).unwrap_or(0);
                let current_lines = current_stats.map(|s| s.total_lines).unwrap_or(0);
                let lines_change = current_lines as i64 - main_lines as i64;
                
                let main_copies = main_stats.map(|s| s.copies).unwrap_or(0);
                let current_copies = current_stats.map(|s| s.copies).unwrap_or(0);
                let copies_change = current_copies as i64 - main_copies as i64;
                
                if lines_change != 0 || copies_change != 0 {
                    function_changes.push((func_name, lines_change, main_lines, current_lines, copies_change));
                }
            }
            
            // Sort by absolute line change
            function_changes.sort_by_key(|(_, lines, _, _, _)| -lines.abs());
            
            if !function_changes.is_empty() {
                println!("\n{}", "🔥 Biggest LLVM IR Changes per Function:".white().bold());
                println!("{}", "─".repeat(90).bright_black());
                println!("  {:>8} {:>8} → {:>8} (copies    ) Function", "Main", "Current", "Change");
                println!("{}", "─".repeat(90).bright_black());
                
                for (func_name, lines_change, main_lines, current_lines, copies_change) in function_changes.iter().take(20) {
                    let lines_str = if *lines_change > 0 {
                        format!("+{}", lines_change).red().to_string()
                    } else if *lines_change < 0 {
                        format!("{}", lines_change).green().to_string()
                    } else {
                        "0".bright_black().to_string()
                    };
                    
                    let copies_str = if *copies_change > 0 {
                        format!("+{}", copies_change).red().to_string()
                    } else if *copies_change < 0 {
                        format!("{}", copies_change).green().to_string()
                    } else {
                        "0".bright_black().to_string()
                    };
                    
                    println!("  {:>8} {:>8} → {:>8} ({:>8}) {}", 
                        main_lines, current_lines, lines_str, copies_str, func_name.bright_white());
                }
                
                if function_changes.len() > 20 {
                    println!(
                        "  {} ... and {} more functions with changes",
                        " ".repeat(10).bright_black(),
                        (function_changes.len() - 20).to_string().bright_cyan()
                    );
                }
            }
            
            // LLVM IR changes per crate
            println!("\n{}", "📦 Biggest LLVM IR Changes per Crate:".white().bold());
            println!("{}", "─".repeat(70).bright_black());
            println!("  {:>10} {:>10} → {:>10} Crate", "Main", "Current", "Change");
            println!("{}", "─".repeat(70).bright_black());
            
            // Group functions by crate
            let mut main_crate_lines: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
            let mut current_crate_lines: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
            
            for (func_name, stats) in &main_llvm.instantiations {
                let crate_name = extract_crate_from_function(func_name);
                *main_crate_lines.entry(crate_name).or_insert(0) += stats.total_lines;
            }
            
            for (func_name, stats) in &current_llvm.instantiations {
                let crate_name = extract_crate_from_function(func_name);
                *current_crate_lines.entry(crate_name).or_insert(0) += stats.total_lines;
            }
            
            // Combine and calculate changes
            let mut all_crates = std::collections::HashSet::new();
            all_crates.extend(main_crate_lines.keys().cloned());
            all_crates.extend(current_crate_lines.keys().cloned());
            
            let mut crate_changes: Vec<(String, i64, usize, usize)> = Vec::new();
            for crate_name in all_crates {
                let main_lines = main_crate_lines.get(&crate_name).copied().unwrap_or(0);
                let current_lines = current_crate_lines.get(&crate_name).copied().unwrap_or(0);
                let lines_change = current_lines as i64 - main_lines as i64;
                
                if lines_change != 0 {
                    crate_changes.push((crate_name, lines_change, main_lines, current_lines));
                }
            }
            
            // Sort by absolute change
            crate_changes.sort_by_key(|(_, change, _, _)| -change.abs());
            
            for (crate_name, lines_change, main_lines, current_lines) in crate_changes.iter().take(15) {
                let change_str = if *lines_change > 0 {
                    format!("+{}", lines_change).red().to_string()
                } else {
                    format!("{}", lines_change).green().to_string()
                };
                
                println!("  {:>10} {:>10} → {:>10}  {}", 
                    main_lines, current_lines, change_str, crate_name.bright_white());
            }
        }
    }
    
    /// Generate markdown output for the report
    fn to_markdown(&self) -> String {
        match self {
            Report::Single(report) => {
                let mut md = String::new();
                writeln!(&mut md, "# 🌊 Limpid Binary Size Analysis Report").unwrap();
                writeln!(&mut md).unwrap();
                writeln!(&mut md, "Analyzing commit `{}`", report.commit_hash).unwrap();
                writeln!(&mut md).unwrap();
                
                // Size metrics
                writeln!(&mut md, "## 📊 Size Metrics").unwrap();
                writeln!(&mut md).unwrap();
                writeln!(&mut md, "| Metric | Value |").unwrap();
                writeln!(&mut md, "|--------|-------|").unwrap();
                writeln!(
                    &mut md,
                    "| File size | {} |",
                    format_bytes(report.file_size)
                )
                .unwrap();
                writeln!(
                    &mut md,
                    "| Text size | {} |",
                    format_bytes(report.text_size)
                )
                .unwrap();
                writeln!(
                    &mut md,
                    "| Build time | {:.2}s |",
                    report.wall_time.as_secs_f64()
                )
                .unwrap();
                writeln!(&mut md).unwrap();
                
                // Top crates
                writeln!(&mut md, "## 📦 Top Crates by Size").unwrap();
                writeln!(&mut md).unwrap();
                writeln!(&mut md, "| Crate | Size | % of Total |").unwrap();
                writeln!(&mut md, "|-------|------|------------|").unwrap();
                for (crate_name, size, percent) in report.top_crates.iter().take(15) {
                    writeln!(
                        &mut md,
                        "| {} | {} | {:.1}% |",
                        crate_name,
                        format_bytes(*size),
                        percent
                    )
                    .unwrap();
                }
                writeln!(&mut md).unwrap();
                
                // LLVM IR Analysis
                if let Some(llvm_analysis) = &report.llvm_analysis {
                    writeln!(&mut md, "## 🔥 LLVM IR Analysis").unwrap();
                    writeln!(&mut md).unwrap();
                    writeln!(&mut md, "| Metric | Value |").unwrap();
                    writeln!(&mut md, "|--------|-------|").unwrap();
                    writeln!(
                        &mut md,
                        "| Total LLVM IR lines | {} |",
                        llvm_analysis.total_lines
                    )
                    .unwrap();
                    writeln!(
                        &mut md,
                        "| Total instantiations | {} |",
                        llvm_analysis.total_copies
                    )
                    .unwrap();
                    writeln!(
                        &mut md,
                        "| Analyzed .ll files | {} |",
                        llvm_analysis.analyzed_files.len()
                    )
                    .unwrap();
                    writeln!(&mut md).unwrap();
                    
                    // Top functions
                    writeln!(&mut md, "### 🔍 Top Functions by LLVM IR Lines").unwrap();
                    writeln!(&mut md).unwrap();
                    writeln!(&mut md, "<details>").unwrap();
                    writeln!(
                        &mut md,
                        "<summary>Top 30 most complex functions (click to expand)</summary>"
                    )
                    .unwrap();
                    writeln!(&mut md).unwrap();
                    writeln!(&mut md, "| Lines | % | Copies | Function |").unwrap();
                    writeln!(&mut md, "|-------|---|--------|----------|").unwrap();
                    
                    let mut functions: Vec<(&String, &substance::llvm_ir::LlvmInstantiations)> =
                        llvm_analysis.instantiations.iter().collect();
                    functions.sort_by_key(|(_, stats)| std::cmp::Reverse(stats.total_lines));
                    
                    for (func_name, stats) in functions.iter().take(30) {
                        let percent = stats.total_lines as f64 / llvm_analysis.total_lines as f64 * 100.0;
                        writeln!(
                            &mut md,
                            "| {} | {:.1}% | {} | `{}` |",
                            stats.total_lines, percent, stats.copies, func_name
                        )
                        .unwrap();
                    }
                    
                    writeln!(&mut md).unwrap();
                    writeln!(&mut md, "</details>").unwrap();
                    writeln!(&mut md).unwrap();
                } else {
                    writeln!(&mut md, "_💡 Tip: LLVM IR analysis data not available. Make sure to build with RUSTFLAGS='--emit=llvm-ir'._").unwrap();
                    writeln!(&mut md).unwrap();
                }
                
                // Footer
                writeln!(&mut md, "---").unwrap();
                writeln!(
                    &mut md,
                    "_Generated by [Limpid](https://github.com/facet-rs/limpid)_"
                )
                .unwrap();
                
                md
            }
            Report::Comparison { main, current, comparison, crate_time_changes } => {
                // Generate markdown for comparison report
                let mut md = String::new();
                
                // Header
                writeln!(&mut md, "# 🌊 Limpid Binary Size Analysis Report").unwrap();
                writeln!(&mut md).unwrap();
                writeln!(
                    &mut md,
                    "Comparing `main` branch with current commit `{}`",
                    &current.commit_hash[..8.min(current.commit_hash.len())]
                )
                .unwrap();
                writeln!(&mut md).unwrap();
                
                // Size comparison summary
                writeln!(&mut md, "## 📊 Size Comparison").unwrap();
                writeln!(&mut md).unwrap();
                
                let size_diff = current.file_size as i64 - main.file_size as i64;
                let text_diff = current.text_size as i64 - main.text_size as i64;
                let time_diff = current.wall_time.as_secs_f64() - main.wall_time.as_secs_f64();
                
                writeln!(&mut md, "| Metric | Main | Current | Change |").unwrap();
                writeln!(&mut md, "|--------|------|---------|--------|").unwrap();
                writeln!(
                    &mut md,
                    "| File size | {} | {} | {} {} |",
                    format_bytes(main.file_size),
                    format_bytes(current.file_size),
                    if size_diff > 0 {
                        "📈"
                    } else if size_diff < 0 {
                        "📉"
                    } else {
                        "➖"
                    },
                    format_size_diff_md(size_diff)
                )
                .unwrap();
                writeln!(
                    &mut md,
                    "| Text size | {} | {} | {} {} |",
                    format_bytes(main.text_size),
                    format_bytes(current.text_size),
                    if text_diff > 0 {
                        "📈"
                    } else if text_diff < 0 {
                        "📉"
                    } else {
                        "➖"
                    },
                    format_size_diff_md(text_diff)
                )
                .unwrap();
                writeln!(
                    &mut md,
                    "| Build time | {:.2}s | {:.2}s | {} {:+.2}s |",
                    main.wall_time.as_secs_f64(),
                    current.wall_time.as_secs_f64(),
                    if time_diff < 0.0 {
                        "⚡"
                    } else if time_diff > 0.0 {
                        "🐌"
                    } else {
                        "➖"
                    },
                    time_diff
                )
                .unwrap();
                writeln!(&mut md).unwrap();
                
                // Top crate size changes
                writeln!(&mut md, "## 📦 Top Crate Size Changes").unwrap();
                writeln!(&mut md).unwrap();
                writeln!(&mut md, "| Crate | Main | Current | Change | % |").unwrap();
                writeln!(&mut md, "|-------|------|---------|--------|---|").unwrap();
                
                let mut crate_changes = comparison.crate_changes.clone();
                crate_changes.sort_by(|a, b| {
                    let a_change = a.absolute_change().map(|c| c.abs()).unwrap_or(0);
                    let b_change = b.absolute_change().map(|c| c.abs()).unwrap_or(0);
                    b_change.cmp(&a_change)
                });
                
                let significant_crate_changes: Vec<_> = crate_changes
                    .iter()
                    .filter(|c| {
                        c.absolute_change()
                            .map(|change| change != 0)
                            .unwrap_or(true)
                    })
                    .take(20)
                    .collect();
                
                for change in &significant_crate_changes {
                    match (change.size_before, change.size_after) {
                        (Some(before), Some(after)) => {
                            let abs_change = change.absolute_change().unwrap();
                            let pct = change.percent_change().unwrap();
                            let emoji = if abs_change > 0 {
                                "📈"
                            } else if abs_change < 0 {
                                "📉"
                            } else {
                                "➖"
                            };
                            writeln!(
                                &mut md,
                                "| {} | {} | {} | {} {} | {:+.1}% |",
                                change.name,
                                format_bytes(before),
                                format_bytes(after),
                                emoji,
                                format_size_diff_md(abs_change),
                                pct
                            )
                            .unwrap();
                        }
                        (None, Some(after)) => {
                            writeln!(
                                &mut md,
                                "| {} | - | {} | 🆕 {} | NEW |",
                                change.name,
                                format_bytes(after),
                                format!("+{}", format_bytes(after))
                            )
                            .unwrap();
                        }
                        (Some(before), None) => {
                            writeln!(
                                &mut md,
                                "| {} | {} | - | 🗑️ {} | REMOVED |",
                                change.name,
                                format_bytes(before),
                                format!("-{}", format_bytes(before))
                            )
                            .unwrap();
                        }
                        _ => {}
                    }
                }
                writeln!(&mut md).unwrap();
                
                // Top crate build time changes
                writeln!(&mut md, "## ⏱️ Top Crate Build Time Changes").unwrap();
                writeln!(&mut md).unwrap();
                writeln!(&mut md, "| Crate | Main | Current | Change | % |").unwrap();
                writeln!(&mut md, "|-------|------|---------|--------|---|").unwrap();
                
                for (crate_name, before, after) in crate_time_changes.iter().take(15) {
                    match (before, after) {
                        (Some(before), Some(after)) => {
                            let diff = after - before;
                            let pct = (diff / before) * 100.0;
                            let emoji = if diff < 0.0 {
                                "⚡"
                            } else if diff > 0.0 {
                                "🐌"
                            } else {
                                "➖"
                            };
                            writeln!(
                                &mut md,
                                "| {} | {:.2}s | {:.2}s | {} {:+.2}s | {:+.1}% |",
                                crate_name, before, after, emoji, diff, pct
                            )
                            .unwrap();
                        }
                        (None, Some(after)) => {
                            writeln!(
                                &mut md,
                                "| {} | - | {:.2}s | 🆕 +{:.2}s | NEW |",
                                crate_name, after, after
                            )
                            .unwrap();
                        }
                        (Some(before), None) => {
                            writeln!(
                                &mut md,
                                "| {} | {:.2}s | - | 🗑️ -{:.2}s | REMOVED |",
                                crate_name, before, before
                            )
                            .unwrap();
                        }
                        _ => {}
                    }
                }
                writeln!(&mut md).unwrap();
                
                // Biggest symbol changes
                writeln!(&mut md, "## 🔍 Biggest Symbol Changes").unwrap();
                writeln!(&mut md).unwrap();
                writeln!(&mut md, "<details>").unwrap();
                writeln!(
                    &mut md,
                    "<summary>Top 50 symbol size changes (click to expand)</summary>"
                )
                .unwrap();
                writeln!(&mut md).unwrap();
                writeln!(&mut md, "| Change | Before | After | Symbol |").unwrap();
                writeln!(&mut md, "|--------|--------|-------|--------|").unwrap();
                
                let mut changed_symbols: Vec<_> = comparison
                    .symbol_changes
                    .iter()
                    .filter_map(|s| match (s.size_before, s.size_after) {
                        (Some(before), Some(after)) if before != after => {
                            let change = after as i64 - before as i64;
                            Some((s, change))
                        }
                        (None, Some(after)) => Some((s, after as i64)),
                        (Some(before), None) => Some((s, -(before as i64))),
                        _ => None,
                    })
                    .collect();
                
                changed_symbols.sort_by_key(|(_, change)| -change.abs());
                
                for (symbol, change) in changed_symbols.iter().take(50) {
                    match (symbol.size_before, symbol.size_after) {
                        (Some(before), Some(after)) => {
                            let emoji = if *change > 0 { "📈" } else { "📉" };
                            writeln!(
                                &mut md,
                                "| {} {} | {} | {} | `{}` |",
                                emoji,
                                format_size_diff_md(*change),
                                format_bytes(before),
                                format_bytes(after),
                                symbol.demangled
                            )
                            .unwrap();
                        }
                        (None, Some(after)) => {
                            writeln!(
                                &mut md,
                                "| 🆕 +{} | NEW | {} | `{}` |",
                                format_bytes(after),
                                format_bytes(after),
                                symbol.demangled
                            )
                            .unwrap();
                        }
                        (Some(before), None) => {
                            writeln!(
                                &mut md,
                                "| 🗑️ -{} | {} | REMOVED | `{}` |",
                                format_bytes(before),
                                format_bytes(before),
                                symbol.demangled
                            )
                            .unwrap();
                        }
                        _ => {}
                    }
                }
                
                writeln!(&mut md).unwrap();
                writeln!(&mut md, "</details>").unwrap();
                writeln!(&mut md).unwrap();
                
                // Top crates by size in current version
                if !current.top_crates.is_empty() {
                    writeln!(&mut md, "## 📦 Top Crates by Size (Current Version)").unwrap();
                    writeln!(&mut md).unwrap();
                    writeln!(&mut md, "| Crate | Size | % of Total |").unwrap();
                    writeln!(&mut md, "|-------|------|------------|").unwrap();
                    
                    for (crate_name, size, percent) in current.top_crates.iter().take(15) {
                        writeln!(
                            &mut md,
                            "| {} | {} | {:.1}% |",
                            crate_name,
                            format_bytes(*size),
                            percent
                        )
                        .unwrap();
                    }
                    writeln!(&mut md).unwrap();
                }
                
                // Top symbols by size in current version
                if !current.top_symbols.is_empty() {
                    writeln!(&mut md, "## 🔍 Top Symbols by Size (Current Version)").unwrap();
                    writeln!(&mut md).unwrap();
                    writeln!(&mut md, "<details>").unwrap();
                    writeln!(
                        &mut md,
                        "<summary>Top 30 largest symbols (click to expand)</summary>"
                    )
                    .unwrap();
                    writeln!(&mut md).unwrap();
                    writeln!(&mut md, "| Size | Symbol |").unwrap();
                    writeln!(&mut md, "|------|--------|").unwrap();
                    
                    for (symbol_name, size) in current.top_symbols.iter().take(30) {
                        writeln!(
                            &mut md,
                            "| {} | `{}` |",
                            format_bytes(*size),
                            symbol_name
                        )
                        .unwrap();
                    }
                    
                    writeln!(&mut md).unwrap();
                    writeln!(&mut md, "</details>").unwrap();
                    writeln!(&mut md).unwrap();
                }
                
                // LLVM IR Analysis (if available)
                if let Some(llvm_analysis) = &current.llvm_analysis {
                    writeln!(&mut md, "## 🔥 LLVM IR Analysis (Current Version)").unwrap();
                    writeln!(&mut md).unwrap();
                    writeln!(&mut md, "| Metric | Value |").unwrap();
                    writeln!(&mut md, "|--------|-------|").unwrap();
                    writeln!(
                        &mut md,
                        "| Total LLVM IR lines | {} |",
                        llvm_analysis.total_lines
                    )
                    .unwrap();
                    writeln!(
                        &mut md,
                        "| Total instantiations | {} |",
                        llvm_analysis.total_copies
                    )
                    .unwrap();
                    writeln!(
                        &mut md,
                        "| Analyzed .ll files | {} |",
                        llvm_analysis.analyzed_files.len()
                    )
                    .unwrap();
                    writeln!(&mut md).unwrap();
                    
                    // Top functions by LLVM IR lines
                    writeln!(&mut md, "### 🔍 Top Functions by LLVM IR Lines").unwrap();
                    writeln!(&mut md).unwrap();
                    writeln!(&mut md, "<details>").unwrap();
                    writeln!(
                        &mut md,
                        "<summary>Top 30 most complex functions (click to expand)</summary>"
                    )
                    .unwrap();
                    writeln!(&mut md).unwrap();
                    writeln!(&mut md, "| Lines | % | Copies | Function |").unwrap();
                    writeln!(&mut md, "|-------|---|--------|----------|").unwrap();
                    
                    let mut functions: Vec<(&String, &substance::llvm_ir::LlvmInstantiations)> =
                        llvm_analysis.instantiations.iter().collect();
                    functions.sort_by_key(|(_, stats)| std::cmp::Reverse(stats.total_lines));
                    
                    for (func_name, stats) in functions.iter().take(30) {
                        let percent = stats.total_lines as f64 / llvm_analysis.total_lines as f64 * 100.0;
                        writeln!(
                            &mut md,
                            "| {} | {:.1}% | {} | `{}` |",
                            stats.total_lines, percent, stats.copies, func_name
                        )
                        .unwrap();
                    }
                    
                    writeln!(&mut md).unwrap();
                    writeln!(&mut md, "</details>").unwrap();
                    writeln!(&mut md).unwrap();
                    
                    // Differential LLVM IR analysis
                    if let Some(main_llvm) = &main.llvm_analysis {
                        writeln!(&mut md, "### 📊 LLVM IR Differential Analysis").unwrap();
                        writeln!(&mut md).unwrap();
                        
                        let lines_diff = llvm_analysis.total_lines as i64 - main_llvm.total_lines as i64;
                        let copies_diff = llvm_analysis.total_copies as i64 - main_llvm.total_copies as i64;
                        
                        writeln!(&mut md, "| Metric | Main | Current | Change |").unwrap();
                        writeln!(&mut md, "|--------|------|---------|--------|").unwrap();
                        writeln!(
                            &mut md,
                            "| Total LLVM IR lines | {} | {} | {} |",
                            main_llvm.total_lines,
                            llvm_analysis.total_lines,
                            if lines_diff > 0 {
                                format!("+{}", lines_diff)
                            } else if lines_diff < 0 {
                                format!("{}", lines_diff)
                            } else {
                                "no change".to_string()
                            }
                        )
                        .unwrap();
                        writeln!(
                            &mut md,
                            "| Total instantiations | {} | {} | {} |",
                            main_llvm.total_copies,
                            llvm_analysis.total_copies,
                            if copies_diff > 0 {
                                format!("+{}", copies_diff)
                            } else if copies_diff < 0 {
                                format!("{}", copies_diff)
                            } else {
                                "no change".to_string()
                            }
                        )
                        .unwrap();
                        writeln!(&mut md).unwrap();
                        
                        // Biggest LLVM IR changes per function
                        writeln!(&mut md, "#### 🔥 Biggest LLVM IR Changes per Function").unwrap();
                        writeln!(&mut md).unwrap();
                        writeln!(&mut md, "<details>").unwrap();
                        writeln!(&mut md, "<summary>Top 50 function LLVM IR changes (click to expand)</summary>").unwrap();
                        writeln!(&mut md).unwrap();
                        writeln!(&mut md, "| Change | Main | Current | Copies | Function |").unwrap();
                        writeln!(&mut md, "|--------|------|---------|--------|----------|").unwrap();
                        
                        // Calculate function changes
                        let mut function_changes: Vec<(String, i64, usize, usize, usize, usize)> = Vec::new();
                        let mut all_functions = std::collections::HashSet::new();
                        all_functions.extend(main_llvm.instantiations.keys().cloned());
                        all_functions.extend(llvm_analysis.instantiations.keys().cloned());
                        
                        for func_name in all_functions {
                            let main_stats = main_llvm.instantiations.get(&func_name);
                            let current_stats = llvm_analysis.instantiations.get(&func_name);
                            
                            let main_lines = main_stats.map(|s| s.total_lines).unwrap_or(0);
                            let current_lines = current_stats.map(|s| s.total_lines).unwrap_or(0);
                            let main_copies = main_stats.map(|s| s.copies).unwrap_or(0);
                            let current_copies = current_stats.map(|s| s.copies).unwrap_or(0);
                            let lines_change = current_lines as i64 - main_lines as i64;
                            
                            if lines_change != 0 {
                                function_changes.push((func_name, lines_change, main_lines, current_lines, main_copies, current_copies));
                            }
                        }
                        
                        // Sort by absolute line change
                        function_changes.sort_by_key(|(_, lines, _, _, _, _)| -lines.abs());
                        
                        for (func_name, lines_change, main_lines, current_lines, main_copies, current_copies) in function_changes.iter().take(50) {
                            let emoji = if *lines_change > 0 { "📈" } else { "📉" };
                            let copies_str = format!("{} → {}", main_copies, current_copies);
                            writeln!(
                                &mut md,
                                "| {} {:+} | {} | {} | {} | `{}` |",
                                emoji,
                                lines_change,
                                main_lines,
                                current_lines,
                                copies_str,
                                func_name
                            )
                            .unwrap();
                        }
                        
                        writeln!(&mut md).unwrap();
                        writeln!(&mut md, "</details>").unwrap();
                        writeln!(&mut md).unwrap();
                        
                        // Biggest LLVM IR changes per crate
                        writeln!(&mut md, "#### 📦 Biggest LLVM IR Changes per Crate").unwrap();
                        writeln!(&mut md).unwrap();
                        writeln!(&mut md, "| Change | Main | Current | Crate |").unwrap();
                        writeln!(&mut md, "|--------|------|---------|-------|").unwrap();
                        
                        // Group functions by crate
                        let mut main_crate_lines: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
                        let mut current_crate_lines: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
                        
                        for (func_name, stats) in &main_llvm.instantiations {
                            let crate_name = extract_crate_from_function(func_name);
                            *main_crate_lines.entry(crate_name).or_insert(0) += stats.total_lines;
                        }
                        
                        for (func_name, stats) in &llvm_analysis.instantiations {
                            let crate_name = extract_crate_from_function(func_name);
                            *current_crate_lines.entry(crate_name).or_insert(0) += stats.total_lines;
                        }
                        
                        // Combine and calculate changes
                        let mut all_crates = std::collections::HashSet::new();
                        all_crates.extend(main_crate_lines.keys().cloned());
                        all_crates.extend(current_crate_lines.keys().cloned());
                        
                        let mut crate_changes: Vec<(String, i64, usize, usize)> = Vec::new();
                        for crate_name in all_crates {
                            let main_lines = main_crate_lines.get(&crate_name).copied().unwrap_or(0);
                            let current_lines = current_crate_lines.get(&crate_name).copied().unwrap_or(0);
                            let lines_change = current_lines as i64 - main_lines as i64;
                            
                            if lines_change != 0 {
                                crate_changes.push((crate_name, lines_change, main_lines, current_lines));
                            }
                        }
                        
                        // Sort by absolute change
                        crate_changes.sort_by_key(|(_, change, _, _)| -change.abs());
                        
                        for (crate_name, lines_change, main_lines, current_lines) in crate_changes.iter().take(20) {
                            let emoji = if *lines_change > 0 { "📈" } else { "📉" };
                            writeln!(
                                &mut md,
                                "| {} {:+} | {} | {} | {} |",
                                emoji,
                                lines_change,
                                main_lines,
                                current_lines,
                                crate_name
                            )
                            .unwrap();
                        }
                        
                        writeln!(&mut md).unwrap();
                    }
                } else {
                    writeln!(&mut md, "_💡 Tip: LLVM IR analysis data not available. This is likely due to missing .ll files in the build output._").unwrap();
                    writeln!(&mut md).unwrap();
                }
                
                // Footer
                writeln!(&mut md, "---").unwrap();
                writeln!(
                    &mut md,
                    "_Generated by [Limpid](https://github.com/facet-rs/limpid)_"
                )
                .unwrap();
                
                md
            }
        }
    }
}
