use std::process::Command;
use fs_err as fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // First, check where we are
    let current_dir = std::env::current_dir()?;
    println!("📍 Current directory: {}", current_dir.display());
    
    // Find the kitchensink directory
    let kitchensink_dir = current_dir.parent()
        .ok_or("No parent directory")?
        .join("kitchensink");
    
    if !kitchensink_dir.exists() {
        return Err(format!("Kitchensink directory not found at: {}", kitchensink_dir.display()).into());
    }
    
    let ks_facet_dir = kitchensink_dir.join("ks-facet");
    if !ks_facet_dir.exists() {
        return Err(format!("ks-facet directory not found at: {}", ks_facet_dir.display()).into());
    }
    
    println!("🚀 Building ks-facet in: {}", ks_facet_dir.display());
    
    // Build ks-facet binary
    let output = Command::new("cargo")
        .args(&["build", "--bin", "ks-facet"])
        .current_dir(&ks_facet_dir)
        .output()?;
    
    if !output.status.success() {
        eprintln!("Failed to build ks-facet:");
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        std::process::exit(1);
    }
    
    println!("✅ Build successful!");
    
    // Find the binary - cargo puts it in the workspace root target directory
    let workspace_root = current_dir.parent().ok_or("No parent directory")?;
    let binary_path = workspace_root.join("target/debug/ks-facet");
    
    // Check if binary exists
    if !binary_path.exists() {
        // Try to find it
        println!("⚠️  Binary not at expected location: {}", binary_path.display());
        println!("🔍 Looking for binary...");
        
        // List what's in the target directory
        let target_dir = workspace_root.join("target/debug");
        if target_dir.exists() {
            println!("Contents of {}:", target_dir.display());
            for entry in fs::read_dir(&target_dir)? {
                let entry = entry?;
                let name = entry.file_name();
                if name.to_string_lossy().contains("ks-") {
                    println!("  - {}", name.to_string_lossy());
                }
            }
        }
        
        return Err(format!("Binary not found at: {}", binary_path.display()).into());
    }
    
    println!("📊 Analyzing binary at: {}", binary_path.display());
    
    // Create a minimal context
    let context = substance::BuildContext {
        target_triple: std::env::var("TARGET").unwrap_or_else(|_| {
            // Default to current platform
            #[cfg(target_arch = "x86_64")]
            let arch = "x86_64";
            #[cfg(target_arch = "aarch64")]
            let arch = "aarch64";
            
            #[cfg(target_os = "macos")]
            let os = "apple-darwin";
            #[cfg(target_os = "linux")]
            let os = "unknown-linux-gnu";
            
            format!("{}-{}", arch, os)
        }),
        artifacts: vec![],
        std_crates: vec![
            "std".to_string(), 
            "core".to_string(), 
            "alloc".to_string(),
        ],
        dep_crates: vec![
            "facet".to_string(),
            "facet_core".to_string(),
        ],
        deps_symbols: Default::default(),
    };
    
    let config = substance::AnalysisConfig {
        symbols_section: None,
        split_std: false,
        analyze_llvm_ir: false,
        target_dir: None,
    };
    
    let result = substance::BloatAnalyzer::analyze_binary(&binary_path, &context, &config)?;
    
    println!("\n📈 Results:");
    println!("File size: {} bytes", result.file_size);
    println!("Text size: {} bytes", result.text_size);
    println!("Symbols:   {}", result.symbols.len());
    
    Ok(())
}