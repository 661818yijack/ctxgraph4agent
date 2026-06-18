//! Dead-code audit test: ensures removed compression subsystem symbols do not
//! reappear in active source code.
//!
//! This test was added after the 2026-04-21 incident where `get_pattern_candidates`
//! called `get_compression_groups` on a non-existent table for 17 days, silently
//! returning empty results. See LESSONS-2026-04-21.md.
//!
//! Run with: cargo test --test dead_code_audit -- --nocapture

use std::process::Command;

/// List of symbols that belong to the removed compression subsystem.
/// If any of these appear in non-test, non-doc Rust source, the test fails.
const BANNED_SYMBOLS: &[&str] = &[
    "CompressionConfig",
    "CompressionResult",
    "CompressionGroupData",
    "run_batch_compression",
    "run_compression_if_needed",
    "get_compression_groups",
    "compress_episodes",
    "count_uncompressed_episodes",
    "maybe_compress",
];

/// Directories that contain only test or doc code and are allowed to reference
/// banned symbols (e.g. planning docs, old test fixtures).
const SKIP_DIRS: &[&str] = &["docs/", "tests/", "test/", "examples/"];

#[test]
fn test_no_dead_compression_symbols_in_source() {
    let repo_root = env!("CARGO_MANIFEST_DIR");
    let mut found = Vec::new();

    for symbol in BANNED_SYMBOLS {
        let output = Command::new("grep")
            .args([
                "-rn",
                symbol,
                "--include=*.rs",
                "--include=*.md",
                "--include=*.toml",
            ])
            .arg(format!("{}/../", repo_root))
            .output()
            .expect("failed to run grep");

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            // Skip if the path contains a test/doc directory
            let skip = SKIP_DIRS.iter().any(|d| line.contains(d));
            // Also skip the LESSONS file itself (it's documentation)
            let skip = skip || line.contains("LESSONS-2026-04-21.md");
            // Also skip this test file
            let skip = skip || line.contains("dead_code_audit.rs");
            if !skip {
                found.push(format!("{}: {}", symbol, line));
            }
        }
    }

    if !found.is_empty() {
        panic!(
            "Dead-code audit FAILED. The following banned compression symbols were found in active source:\n{}\n\n\
            These symbols belong to the removed compression subsystem. \
            If you are re-introducing compression intentionally, update BANNED_SYMBOLS in this test.",
            found.join("\n")
        );
    }
}

/// Verify that `get_pattern_candidates` does NOT reference compression tables
/// or functions in its implementation. This is a targeted check for the
/// specific bug that broke the learn pipeline for 17 days.
#[test]
fn test_pattern_candidates_no_compression_references() {
    let repo_root = env!("CARGO_MANIFEST_DIR");
    let sqlite_path = format!("{}/src/storage/sqlite.rs", repo_root);

    let content = std::fs::read_to_string(&sqlite_path).expect("failed to read sqlite.rs");

    // Find the get_pattern_candidates function body
    let fn_start = content
        .find("pub fn get_pattern_candidates(")
        .expect("get_pattern_candidates not found in sqlite.rs");

    // Extract until the next `pub fn` or end of impl block (heuristic: next "\n    pub fn")
    let rest = &content[fn_start..];
    let fn_end = rest[1..].find("\n    pub fn ").unwrap_or(rest.len());
    let fn_body = &rest[..fn_end];

    // The function docstring legitimately mentions 'compression' to explain the filter.
    // We only ban the *implementation* referencing dead helper symbols.
    let banned_in_fn = &["get_compression_groups", "CompressionGroup"];

    for term in banned_in_fn {
        assert!(
            !fn_body.contains(term),
            "get_pattern_candidates still references '{}' — dead helper from removed compression subsystem",
            term
        );
    }
}
