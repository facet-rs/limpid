use camino::{Utf8Path, Utf8PathBuf};
use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use std::process::Command;
use substance::{AnalysisConfig, AnalysisComparison, ArtifactKind, BloatAnalyzer, BuildContext, BuildRunner, BuildType, BuildOptions};

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
    
    println!("🌳 Creating worktree for {} at {}", repo_path, worktree_path);
    
    // Create the worktree
    let output = Command::new("git")
        .args(&["worktree", "add", "--detach", worktree_path.as_str(), branch])
        .current_dir(repo_path)
        .output()?;
    
    if !output.status.success() {
        return Err(format!(
            "Failed to create worktree: {}",
            String::from_utf8_lossy(&output.stderr)
        ).into());
    }
    
    Ok(())
}

/// Remove a git worktree
fn remove_worktree(
    repo_path: &Utf8PathBuf,
    worktree_path: &Utf8PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🧹 Removing worktree at {}", worktree_path);
    
    // Remove the worktree
    let output = Command::new("git")
        .args(&["worktree", "remove", "--force", worktree_path.as_str()])
        .current_dir(repo_path)
        .output()?;
    
    if !output.status.success() {
        // If it fails, try to clean up manually
        if worktree_path.exists() {
            std::fs::remove_dir_all(worktree_path)?;
        }
    }
    
    Ok(())
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

/// Get the default target triple from rustc
fn get_default_target() -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("rustc")
        .args(&["--print", "target-libdir"])
        .output()?;
    
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
    let output = Command::new("git")
        .args(&["rev-parse", "--show-toplevel"])
        .current_dir(start_path)
        .output()?;
    
    if !output.status.success() {
        return Err(format!(
            "Failed to find git root: {}",
            String::from_utf8_lossy(&output.stderr)
        ).into());
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
    println!("\n{} Building {} version...", "🚀".yellow(), version_name.cyan().bold());
    println!("  {} {}", "📄 Manifest:".bright_black(), ks_facet_manifest.bright_blue());
    println!("  {} {}", "📁 Target dir:".bright_black(), target_dir.bright_blue());
    
    // Configure to build the binary
    let mut build_options = BuildOptions::default();
    build_options.build_bin = Some("ks-facet".to_string());
    
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
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
    let total_build_time: f64 = build_result.timing_data.iter()
        .map(|t| t.duration)
        .sum();
    
    println!("{} {} build completed in {:.2}s", "✅".green(), version_name.cyan(), total_build_time.to_string().yellow());
    
    // Find the ks-facet binary
    let ks_facet_binary = build_result
        .context
        .artifacts
        .iter()
        .find(|a| a.kind == ArtifactKind::Binary && a.name == "ks_facet")
        .ok_or("ks-facet binary not found in build artifacts")?;
    
    println!("  {} {}", "📦 Binary found:".bright_black(), ks_facet_binary.path.display().to_string().bright_blue());
    
    // Analyze the binary
    let analysis_spinner = ProgressBar::new_spinner();
    analysis_spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
    );
    analysis_spinner.set_message("Computing binary analysis...");
    analysis_spinner.enable_steady_tick(std::time::Duration::from_millis(100));
    
    let config = AnalysisConfig::default();
    let analysis = BloatAnalyzer::analyze_binary(
        &ks_facet_binary.path,
        &build_result.context,
        &config,
    )?;
    
    analysis_spinner.finish_and_clear();
    
    println!("  {} {} (text: {})", 
             "📊 Size:".bright_black(),
             format_bytes(analysis.file_size).yellow().bold(), 
             format_bytes(analysis.text_size).yellow());
    
    Ok(analysis)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();
    
    println!("{}", "🌊 Limpid - Binary Size Analyzer".blue().bold());
    println!("{}", "─".repeat(40).bright_black());
    
    // Get current directory
    let current_dir = Utf8PathBuf::from_path_buf(std::env::current_dir()?)
        .map_err(|_| "Current directory is not valid UTF-8")?;
    
    println!("{} {}", "📍 Current directory:".bright_black(), current_dir.bright_blue());
    
    // Find the limpid git repository root
    let limpid_root = find_git_root(&current_dir)?;
    println!("{} {}", "🌳 Limpid repo root:".bright_black(), limpid_root.green());
    
    // Find the parent directory where facet should be
    let workspace_root = limpid_root.parent()
        .ok_or("Could not find parent of limpid repository")?;
    println!("{} {}", "📁 Workspace root:".bright_black(), workspace_root.green());
    
    // Verify kitchensink exists
    let kitchensink_dir = limpid_root.join("kitchensink");
    if !kitchensink_dir.exists() {
        return Err(format!("Kitchensink directory not found at: {}", kitchensink_dir.red()).into());
    }
    println!("{} {}", "✅ Found kitchensink:".bright_black(), kitchensink_dir.green());
    
    let ks_facet_manifest = kitchensink_dir.join("ks-facet/Cargo.toml");
    if !ks_facet_manifest.exists() {
        return Err(format!("ks-facet manifest not found at: {}", ks_facet_manifest.red()).into());
    }
    println!("{} {}", "✅ Found ks-facet manifest:".bright_black(), ks_facet_manifest.green());
    
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
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("#>-")
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
        let (crate_name, _) = substance::crate_name::from_sym(
            &build_context,
            config.split_std,
            &symbol.name,
        );
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
        println!("  {:>10} ({}) {}", 
                 format_bytes(size).yellow(), 
                 percent_str.bright_cyan(),
                 crate_name.bright_white());
    }
    
    // Clean up temporary directory
    if target_dir.exists() {
        println!("\n{} Cleaning up temporary directory...", "🧹".bright_black());
        std::fs::remove_dir_all(&target_dir)?;
    }
    
    // TODO: Next steps for worktree comparison
    println!("\n{}", "📋 Next steps (not implemented yet):".bright_magenta().bold());
    println!("{}", "─".repeat(40).bright_black());
    println!("  {} Create worktree of facet repo at main branch", "1.".bright_black());
    println!("  {} Build ks-facet from worktree/limpid/kitchensink/ks-facet", "2.".bright_black());
    println!("  {} Compare current vs main branch analysis", "3.".bright_black());
    println!("  {} Show size differences and changed symbols", "4.".bright_black());
    
    Ok(())
}