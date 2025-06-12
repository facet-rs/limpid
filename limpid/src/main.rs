use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use owo_colors::OwoColorize;
use substance::{
    AnalysisConfig, AnalysisResult, 
    BloatAnalyzer, BuildContext, BuildRunner, BuildType,
    formatting::*, reporting::*,
};
use std::time::Instant;

mod cli;
mod facet_specific;
mod git;
mod debug_report;

use cli::CliConfig;
use facet_specific::{find_facet_workspace, verify_kitchensink_structure, get_ks_facet_manifest_in_worktree};
use git::{create_comparison_workspace, find_git_root, get_current_commit, remove_worktree};

fn main() -> Result<()> {
    // Parse CLI arguments
    let config = CliConfig::from_args()?;
    config.init_logging();

    // Print header
    if !config.is_markdown_mode() {
        println!("{}", "🌊 Limpid - Binary Size Analyzer".blue().bold());
        println!("{}", "─".repeat(40).bright_black());
    }

    // Get current directory
    let current_dir = Utf8PathBuf::from_path_buf(std::env::current_dir()?)
        .map_err(|_| anyhow::anyhow!("Current directory is not valid UTF-8"))?;

    if !config.is_markdown_mode() {
        println!(
            "{} {}",
            "📍 Current directory:".bright_black(),
            current_dir.bright_blue()
        );
    }

    // Find the limpid git repository root
    let limpid_root = find_git_root(&current_dir)?;
    if !config.is_markdown_mode() {
        println!(
            "{} {}",
            "🌳 Limpid repo root:".bright_black(),
            limpid_root.green()
        );
    }

    // Verify kitchensink structure
    let _ks_facet_manifest = verify_kitchensink_structure(&limpid_root)?;

    // Find the facet repository
    let facet_root = find_facet_workspace(&limpid_root)?;
    if !config.is_markdown_mode() {
        println!(
            "{} {}",
            "🌊 Facet repo root:".bright_black(),
            facet_root.green()
        );
    }

    // Create a temporary workspace for comparison
    let tmp_dir = std::env::temp_dir();
    let workspace_id = format!("limpid-workspace-{}", std::process::id());
    let workspace_dir = Utf8PathBuf::from_path_buf(tmp_dir.join(&workspace_id))
        .expect("temp dir should be valid UTF-8");

    // Create comparison workspace
    let (facet_worktree, limpid_worktree) = 
        create_comparison_workspace(&facet_root, &limpid_root, &workspace_dir)?;

    // Perform comparison analysis
    let comparison_result = perform_comparison_analysis(
        &facet_worktree,
        &limpid_worktree,
        &limpid_root,
        config.is_markdown_mode(),
    );

    // Clean up worktrees
    let _ = remove_worktree(&facet_root, &facet_worktree);
    let _ = remove_worktree(&limpid_root, &limpid_worktree);
    let _ = std::fs::remove_dir_all(&workspace_dir);

    // Handle results
    match comparison_result {
        Ok((baseline, current, comparison)) => {
            // Create report
            let report = Report::Comparison {
                baseline,
                current,
                comparison,
            };

            // Generate output
            if let Some(markdown_path) = config.markdown_output {
                let markdown = report.to_markdown(&ReportConfig::default());
                std::fs::write(&markdown_path, markdown)
                    .with_context(|| format!("Failed to write markdown to {:?}", markdown_path))?;
                println!("📝 Markdown report written to: {}", markdown_path.display());
            } else {
                // Print CLI report
                print_cli_report(&report);
            }
        }
        Err(e) => {
            eprintln!("❌ Analysis failed: {:#}", e);
            return Err(e);
        }
    }

    Ok(())
}

/// Perform comparison analysis between baseline and current versions
fn perform_comparison_analysis(
    _facet_worktree: &Utf8PathBuf,
    limpid_worktree: &Utf8PathBuf,
    limpid_root: &Utf8PathBuf,
    markdown_mode: bool,
) -> Result<(SingleVersionReport, SingleVersionReport, ComparisonData)> {
    // Create unique temporary directories for each build
    let temp_dir = tempfile::tempdir()
        .context("Failed to create temporary directory")?;
    let temp_path = Utf8PathBuf::from_path_buf(temp_dir.path().to_path_buf())
        .expect("temp dir should be valid UTF-8");
    
    let baseline_target_dir = temp_path.join("target-baseline");
    let current_target_dir = temp_path.join("target-current");
    
    if !markdown_mode {
        println!("\n{} Analyzing baseline (main branch)...", "1️⃣".bright_blue());
    }

    // Build and analyze baseline
    let baseline_start = Instant::now();
    let baseline_manifest = get_ks_facet_manifest_in_worktree(limpid_worktree);
    let baseline_result = build_and_analyze(&baseline_manifest, markdown_mode, &baseline_target_dir)?;
    let baseline_wall_time = baseline_start.elapsed();
    let baseline_report = create_single_version_report(baseline_result, "main", baseline_wall_time);

    if !markdown_mode {
        println!("\n{} Analyzing current version...", "2️⃣".bright_blue());
    }

    // Build and analyze current
    let current_start = Instant::now();
    let current_manifest = limpid_root.join("kitchensink").join("ks-facet").join("Cargo.toml");
    let current_result = build_and_analyze(&current_manifest, markdown_mode, &current_target_dir)?;
    let current_wall_time = current_start.elapsed();
    let current_commit = get_current_commit(limpid_root)?;
    let current_report = create_single_version_report(current_result, &current_commit[..8], current_wall_time);

    // Create comparison
    let comparison = ComparisonData::from_reports(&baseline_report, &current_report);
    
    // Clean up is automatic when temp_dir is dropped

    Ok((baseline_report, current_report, comparison))
}

/// Build and analyze a manifest
fn build_and_analyze(
    manifest_path: &Utf8PathBuf,
    markdown_mode: bool,
    target_dir: &Utf8PathBuf,
) -> Result<(AnalysisResult, BuildContext, Vec<substance::TimingInfo>)> {
    let start = Instant::now();

    // Create build runner with unique target directory
    let build_runner = BuildRunner::new(
        manifest_path.as_std_path().to_path_buf(),
        target_dir.as_std_path().to_path_buf(),
        BuildType::Release,
    );

    if !markdown_mode {
        println!("📦 Building {}...", manifest_path.parent().unwrap());
    }

    // Run the build
    let build_result = build_runner.run()
        .map_err(|e| anyhow::anyhow!("Build failed: {:?}", e))?;
    let build_duration = start.elapsed();

    if !markdown_mode {
        println!(
            "✅ Build completed in {}",
            format_duration(&build_duration).green()
        );
    }

    // Find the binary artifact
    let binary = build_result.context.artifacts
        .iter()
        .find(|a| a.kind == substance::ArtifactKind::Binary)
        .ok_or_else(|| anyhow::anyhow!("No binary artifact found"))?;

    // Analyze the binary
    let analysis_result = BloatAnalyzer::analyze_binary(
        &binary.path,
        &build_result.context,
        &AnalysisConfig {
            analyze_llvm_ir: true,
            split_std: true,
            target_dir: Some(target_dir.as_std_path().to_path_buf()),
            build_type: Some(BuildType::Release),
            ..Default::default()
        },
    )?;

    Ok((analysis_result, build_result.context, build_result.timing_data))
}

/// Create a single version report from analysis results
fn create_single_version_report(
    (analysis_result, build_context, timing_info): (AnalysisResult, BuildContext, Vec<substance::TimingInfo>),
    version: &str,
    wall_time: std::time::Duration,
) -> SingleVersionReport {
    SingleVersionReport::from_analysis(
        &analysis_result,
        version.to_string(),
        build_context,
        timing_info,
        wall_time,
    )
}

/// Print CLI report
fn print_cli_report(report: &Report) {
    match report {
        Report::Single(single) => {
            println!("\n📊 Analysis Results");
            println!("   File size: {}", format_bytes(single.metrics.file_size.value()));
            println!("   Text size: {}", format_bytes(single.metrics.text_size.value()));
            println!("   Build time: {:.2}s", single.build_time.total_cpu_time);
            if let Some(llvm) = &single.llvm_analysis {
                println!("   LLVM IR lines: {}", llvm.total_lines);
            }
        }
        Report::Comparison { baseline, current, comparison } => {
            println!("\n📊 Size Comparison");
            println!("   File size: {} → {} ({})",
                format_bytes(baseline.metrics.file_size.value()),
                format_bytes(current.metrics.file_size.value()),
                format_size_diff(comparison.size_changes.file_size_diff)
            );
            println!("   Text size: {} → {} ({})",
                format_bytes(baseline.metrics.text_size.value()),
                format_bytes(current.metrics.text_size.value()),
                format_size_diff(comparison.size_changes.text_size_diff)
            );
            println!("   Build time: {:.2}s → {:.2}s ({:+.2}s)",
                baseline.build_time.total_cpu_time,
                current.build_time.total_cpu_time,
                current.build_time.total_cpu_time - baseline.build_time.total_cpu_time
            );

            // Print top changes
            if !comparison.crate_changes.is_empty() {
                println!("\n📦 Top Crate Changes:");
                for (i, change) in comparison.crate_changes.iter().take(5).enumerate() {
                    let size_before = change.size_before.unwrap_or(0);
                    let size_after = change.size_after.unwrap_or(0);
                    let diff = size_after as i64 - size_before as i64;
                    
                    println!("   {}. {} ({} → {} | {})",
                        i + 1,
                        change.name,
                        format_bytes(size_before),
                        format_bytes(size_after),
                        format_size_diff(diff)
                    );
                }
            }
        }
    }
}