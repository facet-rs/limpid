//! Git operations for managing worktrees and repositories

use anyhow::{ensure, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use owo_colors::OwoColorize;
use std::process::{Command, Output};

/// Run a command and print it with full arguments
pub fn run_command(cmd: &mut Command) -> Result<Output> {
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

    let output = cmd
        .output()
        .with_context(|| format!("Failed to execute command: {}", full_command))?;

    Ok(output)
}

/// Create a git worktree for the given repository
pub fn create_worktree(
    repo_path: &Utf8PathBuf,
    worktree_path: &Utf8PathBuf,
    branch: &str,
) -> Result<()> {
    // First, remove any existing worktree at this path
    if worktree_path.exists() {
        println!("🧹 Removing existing worktree at {}", worktree_path);
        std::fs::remove_dir_all(worktree_path)
            .with_context(|| format!("Failed to remove existing worktree at {}", worktree_path))?;
    }

    // Create parent directory if needed
    if let Some(parent) = worktree_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create parent directory for {}", worktree_path))?;
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
        "--force",
        "--detach",
        worktree_path.as_str(),
        branch,
    ])
    .current_dir(repo_path);

    let output = run_command(&mut cmd)?;

    ensure!(
        output.status.success(),
        "Failed to create worktree: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}

/// Remove a git worktree
pub fn remove_worktree(repo_path: &Utf8PathBuf, worktree_path: &Utf8PathBuf) -> Result<()> {
    println!("🧹 Removing worktree at {}", worktree_path);

    // Remove the worktree directory
    if worktree_path.exists() {
        std::fs::remove_dir_all(worktree_path)
            .with_context(|| format!("Failed to remove worktree directory at {}", worktree_path))?;
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
pub fn create_comparison_workspace(
    facet_repo: &Utf8PathBuf,
    limpid_repo: &Utf8PathBuf,
    workspace_dir: &Utf8PathBuf,
) -> Result<(Utf8PathBuf, Utf8PathBuf)> {
    println!("\n{} Creating comparison workspace...", "🏗️ ".bright_blue());

    // Create the workspace directory
    std::fs::create_dir_all(workspace_dir)
        .with_context(|| format!("Failed to create workspace directory at {}", workspace_dir))?;
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
    let limpid_head = std::str::from_utf8(&output.stdout)
        .context("Invalid UTF-8 in git output")?
        .trim();

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

/// Find the root of a git repository starting from the given path
pub fn find_git_root(start_path: &Utf8Path) -> Result<Utf8PathBuf> {
    let mut cmd = Command::new("git");
    cmd.args(["rev-parse", "--show-toplevel"])
        .current_dir(start_path);

    let output = run_command(&mut cmd)?;

    ensure!(
        output.status.success(),
        "Failed to find git root: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let path = std::str::from_utf8(&output.stdout)
        .context("Invalid UTF-8 in git output")?
        .trim();

    Ok(Utf8PathBuf::from(path))
}
