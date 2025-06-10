use camino::Utf8PathBuf;
use substance::{AnalysisConfig, ArtifactKind, BloatAnalyzer, BuildRunner, BuildType, BuildOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();
    
    // Get the workspace root (parent of limpid dir)
    let current_dir = Utf8PathBuf::from_path_buf(std::env::current_dir()?)
        .map_err(|_| "Current directory is not valid UTF-8")?;
    
    println!("📍 Current directory: {}", current_dir);
    
    // We're in limpid/limpid, so go up two levels to workspace root
    let workspace_root = current_dir
        .parent()
        .and_then(|p| p.parent())
        .ok_or("Could not find workspace root")?;
    
    println!("📁 Workspace root: {}", workspace_root);
    
    // Verify kitchensink exists
    let kitchensink_dir = workspace_root.join("limpid/kitchensink");
    if !kitchensink_dir.exists() {
        return Err(format!("Kitchensink directory not found at: {}", kitchensink_dir).into());
    }
    
    let ks_facet_manifest = kitchensink_dir.join("ks-facet/Cargo.toml");
    if !ks_facet_manifest.exists() {
        return Err(format!("ks-facet manifest not found at: {}", ks_facet_manifest).into());
    }
    
    println!("🚀 Building ks-facet...");
    
    // Use a temporary target directory to avoid workspace issues
    let temp_dir = Utf8PathBuf::from_path_buf(std::env::temp_dir())
        .map_err(|_| "Temp dir is not valid UTF-8")?;
    let target_dir = temp_dir.join(format!("limpid-build-{}", std::process::id()));
    
    // Create the directory if it doesn't exist
    std::fs::create_dir_all(&target_dir)?;
    
    println!("📁 Using temporary target directory: {}", target_dir);
    
    // Build using substance's BuildRunner
    // Configure to build the binary (not examples)
    let mut build_options = BuildOptions::default();
    build_options.build_bin = Some("ks-facet".to_string());
    
    let build_result = match BuildRunner::new(
        ks_facet_manifest.as_std_path(),
        target_dir.as_std_path(),
        BuildType::Debug,
    )
    .with_options(build_options)
    .run() {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Build failed: {:?}", e);
            return Err(e.into());
        }
    };
    
    println!("🔍 Found {} artifacts", build_result.context.artifacts.len());
    
    // Calculate total build time from timing data
    let total_build_time: f64 = build_result.timing_data.iter()
        .map(|t| t.duration)
        .sum();
    
    println!("✅ Build completed in {:.2}s", total_build_time);
    
    // Find the ks-facet binary in the artifacts
    // Note: cargo normalizes names by replacing hyphens with underscores
    let ks_facet_binary = build_result
        .context
        .artifacts
        .iter()
        .find(|a| a.kind == ArtifactKind::Binary && a.name == "ks_facet")
        .ok_or("ks-facet binary not found in build artifacts")?;
    
    println!("📊 Analyzing binary at: {}", ks_facet_binary.path.display());
    
    // Analyze the binary
    let config = AnalysisConfig::default();
    let analysis = BloatAnalyzer::analyze_binary(
        &ks_facet_binary.path,
        &build_result.context,
        &config,
    )?;
    
    println!("\n📈 Results:");
    println!("File size:    {} bytes", analysis.file_size);
    println!("Text size:    {} bytes", analysis.text_size);
    println!("Symbols:      {}", analysis.symbols.len());
    println!("Build time:   {:.2}s", total_build_time);
    
    // Show top crates by size
    let mut crate_sizes = std::collections::HashMap::new();
    for symbol in &analysis.symbols {
        let (crate_name, _) = substance::crate_name::from_sym(
            &build_result.context,
            config.split_std,
            &symbol.name,
        );
        *crate_sizes.entry(crate_name).or_insert(0) += symbol.size;
    }
    
    // Sort crates by size
    let mut crate_list: Vec<(&String, &u64)> = crate_sizes.iter().collect();
    crate_list.sort_by_key(|(_name, &size)| std::cmp::Reverse(size));
    
    println!("\n📦 Top 10 Crates by Size:");
    for (crate_name, &size) in crate_list.iter().take(10) {
        let percent = size as f64 / analysis.text_size as f64 * 100.0;
        println!("  {:>6} bytes ({:>5.1}%) {}", size, percent, crate_name);
    }
    
    // Clean up temporary directory
    if target_dir.exists() {
        println!("\n🧹 Cleaning up temporary directory...");
        std::fs::remove_dir_all(&target_dir)?;
    }
    
    Ok(())
}