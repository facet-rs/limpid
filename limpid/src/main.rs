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
use git::{create_comparison_workspace, find_git_root, remove_worktree};

use crate::report::generate_reports;

fn main() -> Result<()> {
    let config = CliConfig::from_args()?;
    config.init_logging();

    let current_dir = Utf8PathBuf::from_path_buf(std::env::current_dir()?)
        .map_err(|_| anyhow::anyhow!("Current directory is not valid UTF-8"))?;

    println!("📍 current directory: {}", current_dir.bright_blue());

    // Find the limpid git repository root
    let limpid_root = find_git_root(&current_dir)?;
    println!("🌳 limpid repo root: {}", limpid_root.green());

    // Verify kitchensink structure
    let _ks_facet_manifest = verify_kitchensink_structure(&limpid_root)?;

    // Find the facet repository
    let facet_root = find_facet_workspace(&limpid_root)?;
    println!("🌊 facet repo root: {}", facet_root.green());

    // Create a temporary workspace for comparison
    let tmp_dir = if let Ok(env_tmp) = std::env::var("SUBSTANCE_TMP_DIR") {
        println!(
            "💾 Using SUBSTANCE_TMP_DIR from environment: {}",
            env_tmp.bright_blue()
        );
        Utf8PathBuf::from(env_tmp).into_std_path_buf()
    } else {
        let sys_tmp = std::env::temp_dir();
        println!(
            "💾 Using system temporary directory: {}",
            sys_tmp.display().to_string().bright_blue()
        );
        sys_tmp
    };
    let workspace_dir = Utf8PathBuf::from_path_buf(tmp_dir.join("limpid-workspace"))
        .expect("temp dir should be valid UTF-8");

    // Create comparison workspace — this creates worktrees of facet and limpid as sibling
    // directories into the temporary workspace directory.
    let (facet_worktree, limpid_worktree) =
        create_comparison_workspace(&facet_root, &limpid_root, &workspace_dir)?;

    // Perform comparison analysis
    let (baseline, current) = perform_comparison_analysis(&limpid_worktree, &limpid_root)?;

    // Clean up worktrees
    let _ = remove_worktree(&facet_root, &facet_worktree);
    let _ = remove_worktree(&limpid_root, &limpid_worktree);
    let _ = std::fs::remove_dir_all(&workspace_dir);

    let mut txt_output = String::new();
    let mut md_output = String::new();

    generate_reports(&baseline, &current, &mut txt_output, &mut md_output)?;

    println!("{}", txt_output);

    if let Some(markdown_output) = &config.markdown_output {
        std::fs::write(markdown_output, &md_output)?;
        println!(
            "📝 markdown report written to: {}",
            markdown_output.bright_blue()
        );
    }

    Ok(())
}

/// Perform comparison analysis between baseline and current versions
fn perform_comparison_analysis(
    limpid_baseline: &Utf8PathBuf,
    limpid_current: &Utf8PathBuf,
) -> Result<(BuildContext, BuildContext)> {
    let baseline_manifest = limpid_baseline
        .join("kitchensink")
        .join("ks-facet")
        .join("Cargo.toml");
    let baseline = build_and_analyze(&baseline_manifest)?;

    let current_manifest = limpid_current
        .join("kitchensink")
        .join("ks-facet")
        .join("Cargo.toml");
    let current_context = build_and_analyze(&current_manifest)?;

    Ok((baseline, current_context))
}

/// Build and analyze a manifest
fn build_and_analyze(manifest_path: &Utf8Path) -> Result<BuildContext> {
    // Create build runner with unique target directory
    let runner = BuildRunner::for_manifest(manifest_path)
        .arg("--bin")
        .arg("ks-facet")
        .arg("--release");

    println!("📦 Building {}...", manifest_path.parent().unwrap());

    // Run the build
    let context = runner
        .run()
        .map_err(|e| anyhow::anyhow!("Build failed: {:?}", e))?;

    Ok(context)
}
