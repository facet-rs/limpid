use itertools::Itertools;
use owo_colors::OwoColorize;
use std::{cmp, fmt::Write};
use substance::{AggregateSymbol, BuildContext, ByteSize};

/// Generate a text (with colors) and a markdown report comparing two builds
pub(crate) fn generate_reports(
    baseline: &BuildContext,
    current: &BuildContext,
    tx_w: &mut String,
    md_w: &mut String,
) -> anyhow::Result<()> {
    macro_rules! tx {
        ($($arg:tt)*) => {
            write!(tx_w, $($arg)*).unwrap();
        };
    }
    macro_rules! md {
        ($($arg:tt)*) => {
            write!(md_w, $($arg)*).unwrap();
        };
    }

    macro_rules! bytes_diff {
        ($old:expr, $new:expr) => {{
            let diff = $new as isize - $old as isize;
            use owo_colors::OwoColorize;
            if diff > 0 {
                tx!(
                    "{}",
                    format!(" (📈 +{})", format_bytes(diff as u64)).green()
                );
                md!(" (📈 +{})", format_bytes(diff as u64));
            } else if diff < 0 {
                tx!(
                    "{}",
                    format!(" (📉 -{})", format_bytes((-diff) as u64)).red()
                );
                md!(" (📉 -{})", format_bytes((-diff) as u64));
            } else {
                tx!("{}", " (➖ no change)".dimmed());
                md!(" (➖ no change)");
            }
        }};
    }

    macro_rules! unitless_diff {
        ($old:expr, $new:expr) => {{
            let diff = $new - $old;
            if diff > 0 {
                tx!(
                    "{}",
                    format!(" (📈 +{})", fmt_thousands(diff as isize)).green()
                );
                md!(" (📈 +{})", fmt_thousands(diff as isize));
            } else if diff < 0 {
                tx!(
                    "{}",
                    format!(" (📉 {})", fmt_thousands(diff as isize)).red()
                );
                md!(" (📉 {})", fmt_thousands(diff as isize));
            } else {
                tx!("{}", " (➖ no change)".dimmed());
                md!(" (➖ no change)");
            }
        }};
    }

    tx!("{}", "limpid text report\n".bright_blue());
    md!("# limpid markdown report\n\n");

    // ── General statistics ────────────────────────────────────────────────────
    tx!("====================================\n");
    tx!("📊 General statistics\n");
    tx!("====================================\n\n");
    md!("## General statistics\n\n");

    // Number of crates
    let current_num_crates = current.crates.len();
    let baseline_num_crates = baseline.crates.len();

    tx!("Number of crates: {}", current_num_crates.blue());
    md!("Number of crates: {}", current_num_crates);
    unitless_diff!(baseline_num_crates as isize, current_num_crates as isize);
    tx!("\n");
    md!("\n\n");

    // .text section size
    tx!(
        "Text section size: {}",
        format_bytes(current.text_size.value()).cyan()
    );
    md!(
        "Text section size: {}",
        format_bytes(current.text_size.value())
    );
    bytes_diff!(baseline.text_size.value(), current.text_size.value());
    tx!("\n");
    md!("\n\n");

    // Number of symbols
    let current_num_symbols = current.deps_symbols.len();
    let baseline_num_symbols = baseline.deps_symbols.len();

    tx!("Number of symbols: {}", current_num_symbols.blue());
    md!("Number of symbols: {}", current_num_symbols);
    unitless_diff!(baseline_num_symbols as isize, current_num_symbols as isize);
    tx!("\n");
    md!("\n\n");

    // Number of LLVM IR lines
    let current_llvm_lines = current.num_llvm_lines();
    let baseline_llvm_lines = baseline.num_llvm_lines();

    tx!(
        "Number of LLVM lines: {}",
        fmt_thousands(current_llvm_lines as isize).blue()
    );
    md!(
        "Number of LLVM lines: {}",
        fmt_thousands(current_llvm_lines as isize)
    );
    unitless_diff!(baseline_llvm_lines as isize, current_llvm_lines as isize);
    tx!("\n");
    md!("\n");

    // Now let's select interesting symbols: any in the top 20 largest symbols in baseline or in current.
    // Then we'll assign them a rank in baseline and a rank in current.
    // We'll show the old and new rank, or NEW or REMOVED.
    let current_sym_map = current.all_symbols();
    let current_syms_sorted: Vec<AggregateSymbol> = current_sym_map
        .values()
        .cloned()
        .sorted_by_key(|sym| cmp::Reverse(sym.total_size))
        .collect();
    let baseline_sym_map = baseline.all_symbols();
    let baseline_syms_sorted: Vec<AggregateSymbol> = baseline_sym_map
        .values()
        .cloned()
        .sorted_by_key(|sym| cmp::Reverse(sym.total_size))
        .collect();

    // Pick the top 20 symbols from both baseline and current, merge and dedup by name.
    let top_current: Vec<&AggregateSymbol> = current_syms_sorted.iter().take(20).collect();
    let top_baseline: Vec<&AggregateSymbol> = baseline_syms_sorted.iter().take(20).collect();

    struct ComparativeSymbol<'a> {
        old: Option<&'a AggregateSymbol>,
        new: Option<&'a AggregateSymbol>,
        // new size if available, otherwise old size
        size: ByteSize,
    }

    // Merge the top symbols from both baseline and current by name, deduped.
    use std::collections::BTreeSet;
    let mut symbol_names: BTreeSet<&str> = BTreeSet::new();
    for sym in top_baseline.iter().chain(top_current.iter()) {
        symbol_names.insert(sym.name.as_str());
    }
    // For each symbol name, create a ComparativeSymbol
    let mut comparative_syms: Vec<ComparativeSymbol> = Vec::new();
    for &name in &symbol_names {
        let old = baseline_sym_map.get(name);
        let new = current_sym_map.get(name);
        // size is new.size if present, else old.size, else 0
        let size = if let Some(sym) = new {
            sym.total_size
        } else if let Some(sym) = old {
            sym.total_size
        } else {
            unreachable!()
        };
        comparative_syms.push(ComparativeSymbol { old, new, size });
    }

    // Sort comparative_syms by size descending
    comparative_syms.sort_by_key(|sym| cmp::Reverse(sym.size));

    // Render a Markdown table comparing symbol sizes between baseline and current
    md!("## Top Symbol Size Comparison\n\n");
    md!("| Symbol | Baseline Size | Current Size | Change |\n");
    md!("|--------|---------------|--------------|--------|\n");
    for sym in &comparative_syms {
        let name = sym
            .new
            .map(|s| s.name.as_str())
            .or_else(|| sym.old.map(|s| s.name.as_str()))
            .unwrap_or("<unknown>");
        let baseline_size = sym.old.map(|s| s.total_size.value());
        let current_size = sym.new.map(|s| s.total_size.value());
        let (old_sz, new_sz) = (baseline_size.unwrap_or(0), current_size.unwrap_or(0));
        // Format sizes
        let baseline_fmt = if let Some(sz) = baseline_size {
            format_bytes(sz)
        } else {
            "—".to_string()
        };
        let current_fmt = if let Some(sz) = current_size {
            format_bytes(sz)
        } else {
            "—".to_string()
        };
        // Change column
        let change_str = if baseline_size.is_some() && current_size.is_some() {
            let diff = new_sz as isize - old_sz as isize;
            if diff > 0 {
                format!("📈 +{}", format_bytes(diff as u64))
            } else if diff < 0 {
                format!("📉 -{}", format_bytes((-diff) as u64))
            } else {
                "➖ no change".to_owned()
            }
        } else if baseline_size.is_none() && current_size.is_some() {
            "🆕 NEW".to_owned()
        } else if baseline_size.is_some() && current_size.is_none() {
            "🗑️ REMOVED".to_owned()
        } else {
            "—".to_owned()
        };
        md!(
            "| `{}` | {} | {} | {} |\n",
            name,
            baseline_fmt,
            current_fmt,
            change_str
        );
    }
    md!("\n");

    Ok(())
}

/// Format a byte count into a human-readable string (e.g., 1.2 MB)
fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else {
        format!("{:.1} {}", size, UNITS[unit])
    }
}

/// Format a number with thousand separators (e.g., 12,345)
fn fmt_thousands(n: isize) -> String {
    let negative = n < 0;
    let s = n.abs().to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3 + if negative { 1 } else { 0 });
    let chars: Vec<char> = s.chars().collect();
    for (i, ch) in chars.iter().rev().enumerate() {
        if i != 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(*ch);
    }
    let mut formatted = out.chars().rev().collect::<String>();
    if negative {
        formatted.insert(0, '-');
    }
    formatted
}
