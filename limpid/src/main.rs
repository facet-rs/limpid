use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use owo_colors::OwoColorize;
use substance::{BuildContext, BuildRunner};

mod cli;
mod facet_specific;
mod git;
mod report;

use cli::CliConfig;
use facet_specific::{find_facet_workspace, verify_kitchensink_structure};
use git::{create_comparison_workspace, find_git_root, get_current_commit, remove_worktree};

use crate::report::generate_reports;

fn main() -> Result<()> {
    let config = CliConfig::from_args()?;
    config.init_logging();

    let current_dir = Utf8PathBuf::from_path_buf(std::env::current_dir()?)
        .map_err(|_| anyhow::anyhow!("Current directory is not valid UTF-8"))?;

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

    // Verify kitchensink structure
    let _ks_facet_manifest = verify_kitchensink_structure(&limpid_root)?;

    // Find the facet repository
    let facet_root = find_facet_workspace(&limpid_root)?;
    println!(
        "{} {}",
        "🌊 Facet repo root:".bright_black(),
        facet_root.green()
    );

    // Create a temporary workspace for comparison
    let tmp_dir = std::env::temp_dir();
    let workspace_id = format!("limpid-workspace-{}", std::process::id());
    let workspace_dir = Utf8PathBuf::from_path_buf(tmp_dir.join(&workspace_id))
        .expect("temp dir should be valid UTF-8");

    // Create comparison workspace
    let (facet_worktree, limpid_worktree) =
        create_comparison_workspace(&facet_root, &limpid_root, &workspace_dir)?;

    // Perform comparison analysis
    let (baseline, current) =
        perform_comparison_analysis(&facet_worktree, &limpid_worktree, &limpid_root)?;

    // Clean up worktrees
    let _ = remove_worktree(&facet_root, &facet_worktree);
    let _ = remove_worktree(&limpid_root, &limpid_worktree);
    let _ = std::fs::remove_dir_all(&workspace_dir);

    let mut txt_output = String::new();
    let mut md_output = String::new();

    generate_reports(&baseline, &current, &mut txt_output, &mut md_output)?;

    println!("Text output:\n{}", txt_output);

    if let Some(markdown_output) = &config.markdown_output {
        std::fs::write(markdown_output, &md_output)?;
        println!(
            "{} {}",
            "📝 Markdown report written to:".bright_black(),
            markdown_output.bright_blue()
        );
    }

    Ok(())
}

/// Perform comparison analysis between baseline and current versions
fn perform_comparison_analysis(
    _facet_worktree: &Utf8PathBuf,
    limpid_worktree: &Utf8PathBuf,
    limpid_root: &Utf8PathBuf,
) -> Result<(BuildContext, BuildContext)> {
    let baseline_manifest = limpid_worktree
        .join("kitchensink")
        .join("ks-facet")
        .join("Cargo.toml");
    let baseline = build_and_analyze(&baseline_manifest)?;

    let current_manifest = limpid_root
        .join("kitchensink")
        .join("ks-facet")
        .join("Cargo.toml");
    let current_context = build_and_analyze(&current_manifest)?;

    Ok((baseline, current_context))
}

/// Build and analyze a manifest
fn build_and_analyze(manifest_path: &Utf8Path) -> Result<BuildContext> {
    // Create build runner with unique target directory
    let runner = BuildRunner::for_manifest(manifest_path).arg("--release");

    println!("📦 Building {}...", manifest_path.parent().unwrap());

    // Run the build
    let context = runner
        .run()
        .map_err(|e| anyhow::anyhow!("Build failed: {:?}", e))?;

    Ok(context)
}
