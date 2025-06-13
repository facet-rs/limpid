//! Facet-specific paths and configuration

use anyhow::{anyhow, Result};
use camino::{Utf8Path, Utf8PathBuf};
use owo_colors::OwoColorize;

/// Path to the kitchensink directory relative to limpid root
pub const KITCHENSINK_PATH: &str = "kitchensink";

/// Path to the ks-facet manifest relative to kitchensink
pub const KS_FACET_MANIFEST: &str = "ks-facet/Cargo.toml";

/// Find the Facet workspace given the Limpid repository root
pub fn find_facet_workspace(limpid_root: &Utf8Path) -> Result<Utf8PathBuf> {
    // Facet should be in the parent directory of limpid
    let workspace_root = limpid_root
        .parent()
        .ok_or_else(|| anyhow!("Could not find parent of limpid repository"))?;

    let facet_root = workspace_root.join("facet");

    if facet_root.exists() && facet_root.join(".git").exists() {
        Ok(facet_root)
    } else {
        Err(anyhow!(
            "Facet repository not found at {}. Expected directory structure:\n\
             workspace/\n\
             ├── facet/\n\
             └── limpid/",
            facet_root
        ))
    }
}

/// Verify that the kitchensink structure exists and is valid
pub fn verify_kitchensink_structure(limpid_root: &Utf8Path) -> Result<Utf8PathBuf> {
    let kitchensink_dir = limpid_root.join(KITCHENSINK_PATH);

    if !kitchensink_dir.exists() {
        return Err(anyhow!(
            "Kitchensink directory not found at: {}",
            kitchensink_dir.red()
        ));
    }

    println!(
        "{} {}",
        "✅ Found kitchensink:".bright_black(),
        kitchensink_dir.green()
    );

    let ks_facet_manifest = kitchensink_dir.join(KS_FACET_MANIFEST);
    if !ks_facet_manifest.exists() {
        return Err(anyhow!(
            "ks-facet manifest not found at: {}",
            ks_facet_manifest.red()
        ));
    }

    println!(
        "{} {}",
        "✅ Found ks-facet manifest:".bright_black(),
        ks_facet_manifest.green()
    );

    Ok(ks_facet_manifest)
}
