use camino::{Utf8Path, Utf8PathBuf};
use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use std::process::{Command, Output};
use substance::{
    AnalysisComparison, AnalysisConfig, ArtifactKind, BloatAnalyzer, BuildContext, BuildOptions,
    BuildRunner, BuildType,
};

/// Run a command and print it with full arguments
fn run_command(cmd: &mut Command) -> Result<Output, Box<dyn std::error::Error>> {
    // Build the command string for display
    let program = cmd.get_program().to_string_lossy();
    let args: Vec<String> = cmd.get_args().map(|s| s.to_string_lossy().to_string()).collect();
    let full_command = format!("{} {}", program, args.join(" "));
    
    println!("{} {}", "🔧 Running:".bright_black(), full_command.bright_blue());
    
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
    cmd.args(["worktree", "prune"])
        .current_dir(repo_path);
    
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
    println!("  {} Created workspace at {}", "✅".green(), workspace_dir.bright_blue());
    
    // Create facet worktree at main branch
    let facet_worktree = workspace_dir.join("facet");
    println!("\n  {} Creating facet worktree at main branch...", "1️⃣ ".bright_black());
    create_worktree(facet_repo, &facet_worktree, "main")?;
    
    // Get current HEAD of limpid for the worktree
    let mut cmd = Command::new("git");
    cmd.args(["rev-parse", "HEAD"])
        .current_dir(limpid_repo);
    let output = run_command(&mut cmd)?;
    let limpid_head = std::str::from_utf8(&output.stdout)?.trim();
    
    // Create limpid worktree at current HEAD
    let limpid_worktree = workspace_dir.join("limpid");
    println!("\n  {} Creating limpid worktree at HEAD ({})...", "2️⃣ ".bright_black(), &limpid_head[..8].yellow());
    create_worktree(limpid_repo, &limpid_worktree, limpid_head)?;
    
    println!("\n  {} Workspace created successfully!", "🎉".bright_green());
    println!("    {} Facet:  {}", "•".bright_black(), facet_worktree.bright_blue());
    println!("    {} Limpid: {}", "•".bright_black(), limpid_worktree.bright_blue());
    
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

/// Get the default target triple from rustc
fn get_default_target() -> Result<String, Box<dyn std::error::Error>> {
    let mut cmd = Command::new("rustc");
    cmd.args(["--print", "target-libdir"]);
    
    let output = run_command(&mut cmd)?;

    let path = std::str::from_utf8(&output.stdout)?.trim();
    // Extract target from path like: /path/to/rustlib/aarch64-apple-darwin/lib
    let components: Vec<&str> = path.split('/').collect();

    // Find the component after "rustlib"
    for i in 0..components.len() {
        if components[i] == "rustlib" && i + 1 < components.len() {
            return Ok(components[i + 1].to_owned());
        }
    }

    Err("Failed to detect target triple from rustc output".into())
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

/// Build and analyze a specific version of ks-facet
fn build_and_analyze(
    ks_facet_manifest: &Utf8PathBuf,
    target_dir: &Utf8PathBuf,
    version_name: &str,
) -> Result<substance::AnalysisResult, Box<dyn std::error::Error>> {
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

    let build_result = BuildRunner::new(
        ks_facet_manifest.as_std_path(),
        target_dir.as_std_path(),
        BuildType::Debug,
    )
    .with_options(build_options)
    .run()?;

    spinner.finish_and_clear();

    // Calculate total build time
    let total_build_time: f64 = build_result.timing_data.iter().map(|t| t.duration).sum();

    println!(
        "{} {} build completed in {:.2}s",
        "✅".green(),
        version_name.cyan(),
        total_build_time.to_string().yellow()
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

    let config = AnalysisConfig::default();
    let analysis =
        BloatAnalyzer::analyze_binary(&ks_facet_binary.path, &build_result.context, &config)?;

    analysis_spinner.finish_and_clear();

    println!(
        "  {} {} (text: {})",
        "📊 Size:".bright_black(),
        format_bytes(analysis.file_size).yellow().bold(),
        format_bytes(analysis.text_size).yellow()
    );

    Ok(analysis)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging with default level if not set
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

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
    let current_analysis = build_and_analyze(&ks_facet_manifest, &target_dir, "current")?;

    // Show top crates by size
    println!("\n{} Analyzing crate sizes...", "📊".bright_black());

    let pb = ProgressBar::new(current_analysis.symbols.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
            )
            .unwrap()
            .progress_chars("#>-"),
    );
    pb.set_message("Processing symbols");

    let config = AnalysisConfig::default();
    let mut crate_sizes = std::collections::HashMap::new();

    let build_context = BuildContext {
        target_triple: get_default_target()?,
        artifacts: vec![],
        std_crates: vec![],
        dep_crates: vec![],
        deps_symbols: Default::default(),
    };

    for symbol in &current_analysis.symbols {
        let (crate_name, _) =
            substance::crate_name::from_sym(&build_context, config.split_std, &symbol.name);
        *crate_sizes.entry(crate_name).or_insert(0) += symbol.size;
        pb.inc(1);
    }

    pb.finish_with_message("Symbol analysis complete");

    // Sort crates by size
    println!("{} Sorting crates by size...", "🔍".bright_black());
    let mut crate_list: Vec<(&String, &u64)> = crate_sizes.iter().collect();
    crate_list.sort_by_key(|(_name, &size)| std::cmp::Reverse(size));

    println!("\n{}", "📦 Top 10 Crates by Size:".white().bold());
    println!("{}", "─".repeat(50).bright_black());
    for (crate_name, &size) in crate_list.iter().take(10) {
        let percent = size as f64 / current_analysis.text_size as f64 * 100.0;
        let percent_str = format!("{:5.1}%", percent);
        println!(
            "  {:>10} ({}) {}",
            format_bytes(size).yellow(),
            percent_str.bright_cyan(),
            crate_name.bright_white()
        );
    }

    // Clean up temporary directory
    if target_dir.exists() {
        println!(
            "\n{} Cleaning up temporary directory...",
            "🧹".bright_black()
        );
        std::fs::remove_dir_all(&target_dir)?;
    }

    // Compare with main branch if facet repo exists
    let facet_root = workspace_root.join("facet");
    if facet_root.exists() && facet_root.join(".git").exists() {
        println!(
            "\n{}",
            "🔄 Comparing with main branch...".bright_magenta().bold()
        );
        println!("{}", "─".repeat(40).bright_black());

        // Get current commit hash
        let mut cmd = Command::new("git");
        cmd.args(["rev-parse", "HEAD"])
            .current_dir(&facet_root);
        
        let current_commit = run_command(&mut cmd)?;
        let current_hash = std::str::from_utf8(&current_commit.stdout)?.trim();
        println!(
            "{} {} ({})",
            "📌 Current commit:".bright_black(),
            (&current_hash[..8]).yellow(),
            current_hash.bright_black()
        );

        // Create worktree for main branch
        let worktree_path = temp_dir.join(format!("facet-main-{}", std::process::id()));
        create_worktree(&facet_root, &worktree_path, "main")?;

        // Build ks-facet from main branch
        let main_ks_facet_manifest = worktree_path.join("limpid/kitchensink/ks-facet/Cargo.toml");
        if main_ks_facet_manifest.exists() {
            let main_target_dir =
                temp_dir.join(format!("limpid-ks-facet-main-{}", std::process::id()));
            std::fs::create_dir_all(&main_target_dir)?;

            let main_analysis =
                build_and_analyze(&main_ks_facet_manifest, &main_target_dir, "main")?;

            // Compare analyses
            println!("\n{}", "📊 Size Comparison:".white().bold());
            println!("{}", "─".repeat(50).bright_black());

            let size_diff = current_analysis.file_size as i64 - main_analysis.file_size as i64;
            let text_diff = current_analysis.text_size as i64 - main_analysis.text_size as i64;

            println!(
                "  {} {}",
                "File size:".bright_black(),
                format!(
                    "{} → {} ({})",
                    format_bytes(main_analysis.file_size).yellow(),
                    format_bytes(current_analysis.file_size).yellow(),
                    format_size_diff(size_diff)
                )
                .white()
            );

            println!(
                "  {} {}",
                "Text size:".bright_black(),
                format!(
                    "{} → {} ({})",
                    format_bytes(main_analysis.text_size).yellow(),
                    format_bytes(current_analysis.text_size).yellow(),
                    format_size_diff(text_diff)
                )
                .white()
            );

            // Analyze changes using Substance's comparison API
            let comparison = AnalysisComparison::compare(&main_analysis, &current_analysis)?;

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

            if significant_crate_changes.len() > 15 {
                println!(
                    "  {} ... and {} more changes",
                    " ".repeat(10).bright_black(),
                    (significant_crate_changes.len() - 15)
                        .to_string()
                        .bright_cyan()
                );
            }

            // Show top symbol changes (only those with significant relative change)
            let mut symbol_changes = comparison.symbol_changes.clone();
            symbol_changes.sort_by(|a, b| {
                let a_pct = a.percent_change().map(|p| p.abs()).unwrap_or(0.0);
                let b_pct = b.percent_change().map(|p| p.abs()).unwrap_or(0.0);
                b_pct.partial_cmp(&a_pct).unwrap()
            });

            let significant_symbol_changes: Vec<_> = symbol_changes
                .into_iter()
                .filter(|s| {
                    match (s.size_before, s.size_after) {
                        (Some(before), Some(after)) => {
                            before != after
                                && s.percent_change().map(|p| p.abs() > 5.0).unwrap_or(false)
                        }
                        _ => true, // Include new or removed symbols
                    }
                })
                .collect();

            if !significant_symbol_changes.is_empty() {
                println!("\n{}", "🔍 Top Symbol Changes (>5% change):".white().bold());
                println!("{}", "─".repeat(60).bright_black());

                for change in significant_symbol_changes.iter().take(20) {
                    match (change.size_before, change.size_after) {
                        (Some(before), Some(after)) => {
                            let pct = change.percent_change().unwrap();
                            println!(
                                "  {:+6.1}% {:>10} → {:>10} {}",
                                pct,
                                format_bytes(before).yellow(),
                                format_bytes(after).yellow(),
                                change.demangled.bright_white()
                            );
                        }
                        (None, Some(after)) => {
                            println!(
                                "    NEW   {:>10}              {}",
                                format_bytes(after).yellow(),
                                change.demangled.bright_white()
                            );
                        }
                        (Some(before), None) => {
                            println!(
                                " REMOVED {:>10}              {}",
                                format_bytes(before).yellow(),
                                change.demangled.bright_white()
                            );
                        }
                        _ => {}
                    }
                }

                if significant_symbol_changes.len() > 20 {
                    println!(
                        "  {} ... and {} more significant changes",
                        " ".repeat(10).bright_black(),
                        (significant_symbol_changes.len() - 20)
                            .to_string()
                            .bright_cyan()
                    );
                }
            }

            // Clean up main branch build directory
            if main_target_dir.exists() {
                std::fs::remove_dir_all(&main_target_dir)?;
            }

            // Remove worktree
            remove_worktree(&facet_root, &worktree_path)?;
        } else {
            println!(
                "⚠️  {} ks-facet not found in main branch",
                "Warning:".yellow()
            );
        }
    } else {
        println!(
            "\n{} Facet repository not found at {}. Skipping comparison.",
            "ℹ️ ".bright_blue(),
            facet_root.to_string().bright_black()
        );
    }

    Ok(())
}
