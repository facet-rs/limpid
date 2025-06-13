use owo_colors::OwoColorize;
use std::fmt::Write;
use substance::BuildContext;

/// Generate a text (with colors) and a markdown report comparing two builds
pub(crate) fn generate_reports(
    baseline: &BuildContext,
    current: &BuildContext,
    tx: &mut String,
    md: &mut String,
) -> anyhow::Result<()> {
    writeln!(tx, "{}", "limpid text report".bright_blue())?;
    writeln!(md, "# limpid markdown report")?;

    let current_num_crates = current.crates.len();
    let baseline_num_crates = baseline.crates.len();
    let diff = current_num_crates as isize - baseline_num_crates as isize;

    writeln!(tx, "General statistics\n")?;
    writeln!(md, "## General stats\n")?;

    write!(tx, "Number of crates: {}", current_num_crates.blue())?;
    write!(md, "Number of crates: {current_num_crates}")?;
    if diff > 0 {
        write!(tx, "{}", format!(" (📈 +{diff})").green())?;
        write!(md, " (📈  +{diff})")?;
    } else if diff < 0 {
        write!(tx, "{}", format!(" (📉 {diff})").red())?;
        write!(md, " (📉 {diff})")?;
    } else {
        write!(tx, "{}", " (➖ no change)".dimmed())?;
        write!(md, " (➖ no change)")?;
    }

    write!(
        tx,
        "\nText section size: {}",
        format_bytes(current.text_size.value()).cyan()
    )?;
    write!(
        md,
        "\nText section size: {}",
        format_bytes(current.text_size.value())
    )?;

    let text_diff = current.text_size.value() as isize - baseline.text_size.value() as isize;
    if text_diff > 0 {
        writeln!(
            tx,
            "{}",
            format!(" (📈 +{})", format_bytes(text_diff as u64)).green()
        )?;
        writeln!(md, " (📈 +{})", format_bytes(text_diff as u64))?;
    } else if text_diff < 0 {
        writeln!(
            tx,
            "{}",
            format!(" (📉 -{})", format_bytes((-text_diff) as u64)).red()
        )?;
        writeln!(md, " (📉 -{})", format_bytes((-text_diff) as u64))?;
    } else {
        writeln!(tx, "{}", " (➖ no change)".dimmed())?;
        writeln!(md, " (➖ no change)")?;
    }

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
