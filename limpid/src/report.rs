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

    Ok(())
}
