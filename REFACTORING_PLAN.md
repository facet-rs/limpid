# Limpid to Substance Refactoring Plan

## Overview

This document outlines the plan to refactor Limpid and Substance to better separate concerns:
- **Substance**: Binary analysis library with reporting capabilities
- **Limpid**: Facet-specific orchestration tool using Substance

Current state:
- Limpid: 2,242 lines (monolithic main.rs)
- Substance: ~2,000 lines (analysis library)

Target state:
- Limpid: ~500 lines (focused orchestration)
- Substance: ~3,000 lines (complete analysis toolkit)

## Phase 1: Extend Substance Library

### 1.1 Update Dependencies

Add to `substance/Cargo.toml`:
```toml
[features]
default = ["formatting"]
formatting = []
markdown = []
cli = ["owo-colors", "indicatif"]

[dependencies.owo-colors]
version = "4"
optional = true

[dependencies.indicatif]
version = "0.17"
optional = true
```

### 1.2 Create New Modules

- [x] **substance/src/formatting.rs**
  - [x] `format_bytes(bytes: u64) -> String`
  - [x] `format_size_diff(diff: i64) -> String`
  - [x] `format_percentage(value: f64) -> String`
  - [x] `format_duration(duration: Duration) -> String`

- [x] **substance/src/reporting.rs**
  - [x] Core types (see Report Structure section below)
  - [x] `SingleVersionReport` struct
  - [x] `Report` enum (Single/Comparison)
  - [x] `ReportConfig` and related configuration
  - [x] `to_markdown()`, `to_json()`, `to_plain_text()` methods

- [x] **substance/src/analysis_ext.rs**
  - [x] Extensions to `AnalysisResult`:
    - [x] `top_crates(n: usize) -> Vec<(String, u64, f64)>`
    - [x] `top_symbols(n: usize) -> Vec<(String, u64)>`
    - [x] `crate_sizes() -> HashMap<String, u64>`
  - [x] Extensions to `AnalysisComparison`:
    - [x] `significant_changes(threshold: f64) -> Vec<&CrateChange>`
    - [x] `build_time_changes(before, after) -> Vec<TimingChange>`

- [x] **substance/src/crate_name.rs** (enhance existing)
  - [x] Move `extract_crate_from_function()` from Limpid

- [x] **substance/src/llvm_ir.rs** (enhance existing)
  - [x] Add `LlvmComparison::from_summaries` method (via reporting.rs)
  - [x] Differential analysis implemented in reporting module
  - [x] Per-crate aggregation in reporting module

## Phase 2: Refactor Limpid

### 2.1 New Module Structure

- [x] **limpid/src/main.rs** (~275 lines)
  - [x] Main orchestration logic
  - [x] Use Substance APIs for analysis
  - [x] Route output based on CLI args

- [x] **limpid/src/git.rs** (~150 lines)
  - [x] `run_command()`
  - [x] `create_worktree()`
  - [x] `remove_worktree()`
  - [x] `create_comparison_workspace()`
  - [x] `find_git_root()`

- [x] **limpid/src/cli.rs** (~100 lines)
  - [x] Argument parsing with clap
  - [x] Help text
  - [x] Validation

- [x] **limpid/src/facet_specific.rs** (~50 lines)
  - [x] `KITCHENSINK_PATH` constant
  - [x] `KS_FACET_MANIFEST` constant
  - [x] `find_facet_workspace()`
  - [x] `verify_kitchensink_structure()`

### 2.2 Update Dependencies

```toml
[dependencies]
anyhow = "1.0"
substance = { version = "0.5", features = ["formatting", "markdown", "cli"] }
camino = "1.1"
clap = { version = "4", features = ["derive"] }
owo-colors = "4"
indicatif = "0.17"
env_logger = "0.11"
```

## Report Structure Documentation

### Report Sections (Comparison Mode)

1. **Header & Summary**
   - Title: "Comparing `main` branch with current commit `{hash}`"
   - Summary table: file size, text size, build time, LLVM IR lines

2. **Size Comparison Overview** 📊
   - File size: before → after (% change)
   - Text size: before → after (% change)
   - Build time: before → after (difference)

3. **Top Crate Size Changes** 📦
   - Table: Crate | Main | Current | Change | %
   - NEW crates marked with 🆕
   - REMOVED crates marked with 🗑️
   - Default limit: 20 crates

4. **Top Crate Build Time Changes** ⏱️
   - Table: Crate | Main | Current | Change | %
   - Improvements marked with ⚡
   - Regressions marked with 🐌
   - Default limit: 15 crates

5. **Biggest Symbol Changes** 🔍
   - Expandable section
   - Table: Change | Before | After | Symbol
   - Default limit: 50 symbols

6. **Top Crates by Size (Current)** 📦
   - Table: Crate | Size | % of Total
   - Default limit: 15 crates

7. **Top Symbols by Size (Current)** 🔍
   - Expandable section
   - Table: Size | Symbol
   - Default limit: 30 symbols

8. **LLVM IR Analysis** 🔥
   - 8.1 Current Version Metrics
   - 8.2 Top Functions by LLVM IR Lines (expandable, top 30)
   - 8.3 Differential Analysis Summary
   - 8.4 Biggest Function Changes (expandable, top 50)
   - 8.5 Biggest Crate IR Changes (top 20)

### Report Sections (Single Version Mode)

1. **Header & Summary**
   - Commit hash
   - Basic metrics table

2. **Top Crates by Size** 📦
3. **LLVM IR Analysis** 🔥 (if available)
4. **Top Functions by LLVM IR Lines**

## Implementation Steps

### Step 1: Create Substance modules
- [x] Create formatting.rs with tests
- [x] Create reporting.rs with basic structures
- [x] Create analysis_ext.rs with extensions
- [ ] Update llvm_ir.rs with comparison methods

### Step 2: Implement report generation
- [ ] Implement markdown generation
- [ ] Implement JSON serialization
- [ ] Implement plain text output
- [ ] Add configuration options

### Step 3: Refactor Limpid
- [x] Extract git operations
- [x] Extract CLI parsing
- [x] Extract facet-specific logic
- [x] Update main.rs to use Substance APIs

### Step 4: Testing
- [ ] Unit tests for Substance modules
- [ ] Integration tests for report generation
- [ ] End-to-end tests for Limpid
- [ ] Regression tests for output format

### Step 5: Documentation
- [ ] Update Substance README
- [ ] Update Limpid README
- [ ] Add examples for Substance usage
- [ ] Document migration guide

## Success Criteria

1. Limpid reduced from 2,242 to ~500 lines
2. All analysis logic moved to Substance
3. Report generation is configurable and extensible
4. Existing functionality preserved
5. Tests pass with >80% coverage
6. Documentation complete

## Migration Checklist

- [ ] All functions moved to appropriate modules
- [ ] Error handling converted to anyhow in Limpid
- [ ] Substance API is backward compatible
- [ ] Report output matches current format
- [ ] Performance is not degraded
- [ ] Memory usage is reasonable

## Notes

- Keep Substance's existing `BloatError` for library API
- Use `anyhow::Result` in Limpid for better error messages
- Ensure all report sections are documented in code
- Make report configuration flexible for future needs

## Status

- **Started**: January 2025
- **Current Phase**: Phase 3 - Bug Fixes and Missing Features
- **Completed**:
  - ✅ All Substance modules created (formatting, reporting, analysis_ext)
  - ✅ Substance lib.rs updated with new modules
  - ✅ Substance Cargo.toml updated with optional dependencies
  - ✅ extract_crate_from_function moved to Substance
  - ✅ LLVM comparison methods added to reporting module (partial)
  - ✅ Limpid modules created (git, cli, facet_specific)
  - ✅ Limpid Cargo.toml updated with anyhow dependency
  - ✅ Limpid main.rs refactored to ~270 lines (down from 2,241)
  - ✅ Fixed worktree path issue (kitchensink in limpid, not facet)
- **Issues Found**:
  - ❌ Only 2 of 8 report sections are being generated
  - ❌ Missing: Top Crate Size Changes, Symbol Changes, Top Crates/Symbols, LLVM Analysis
  - ❌ Symbol comparison not implemented (empty vec in from_reports)
  - ❌ Verbose option doesn't do anything useful
  - ❌ LLVM IR analysis might not be working correctly
- **Remaining**:
  - 🔧 Fix missing report sections
  - 🔧 Implement symbol comparison
  - 🔧 Fix LLVM IR analysis
  - 🔧 Make verbose option useful or remove it
  - 📝 Add unit tests for new modules
  - 📝 Add integration tests
  - 📝 Update documentation
- **Blockers**: Missing data in report generation
- **Next Steps**: Debug why report sections are missing