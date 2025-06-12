use substance::reporting::*;

pub fn debug_report(report: &Report) {
    match report {
        Report::Comparison { baseline, current, comparison } => {
            println!("\n=== DEBUG: Report Data ===");
            
            // Check baseline data
            println!("\nBaseline Report:");
            println!("  - top_crates: {} items", baseline.top_crates.len());
            println!("  - top_symbols: {} items", baseline.top_symbols.len());
            println!("  - llvm_analysis: {}", baseline.llvm_analysis.is_some());
            
            // Check current data
            println!("\nCurrent Report:");
            println!("  - top_crates: {} items", current.top_crates.len());
            println!("  - top_symbols: {} items", current.top_symbols.len());
            println!("  - llvm_analysis: {}", current.llvm_analysis.is_some());
            
            // Check comparison data
            println!("\nComparison Data:");
            println!("  - crate_changes: {} items", comparison.crate_changes.len());
            println!("  - symbol_changes: {} items", comparison.symbol_changes.len());
            println!("  - build_time_changes: {} items", comparison.build_time_changes.len());
            println!("  - llvm_comparison: {}", comparison.llvm_comparison.is_some());
            
            println!("\n========================\n");
        }
        _ => {}
    }
}