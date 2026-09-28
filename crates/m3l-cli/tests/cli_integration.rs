use std::path::{Path, PathBuf};
use std::process::Command;

/// Get the workspace root (two levels up from CARGO_MANIFEST_DIR of m3l-cli)
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent() // crates/
        .unwrap()
        .parent() // workspace root
        .unwrap()
        .to_path_buf()
}

fn m3l_bin() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_m3l"));
    cmd.current_dir(workspace_root());
    cmd
}

#[test]
fn cli_help() {
    let output = m3l_bin().arg("--help").output().expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("M3L parser and validator"));
}

#[test]
fn cli_version() {
    let output = m3l_bin().arg("--version").output().expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn cli_parse_single_file() {
    let output = m3l_bin()
        .args(["parse", "samples/01-ecommerce.m3l.md"])
        .output()
        .expect("failed to run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);

    let ast: serde_json::Value = serde_json::from_str(&stdout).expect("invalid JSON output");
    assert_eq!(ast["parserVersion"], env!("CARGO_PKG_VERSION"));
    assert_eq!(ast["astVersion"], "1.0");
    assert!(ast["models"].is_array());
    assert!(!ast["models"].as_array().unwrap().is_empty());
}

#[test]
fn cli_parse_directory() {
    let output = m3l_bin()
        .args(["parse", "samples/multi/"])
        .output()
        .expect("failed to run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);

    let ast: serde_json::Value = serde_json::from_str(&stdout).expect("invalid JSON output");
    let sources = ast["sources"].as_array().expect("sources should be array");
    assert_eq!(sources.len(), 2); // base.m3l.md + inventory.m3l.md
}

#[test]
fn cli_parse_nonexistent() {
    let output = m3l_bin()
        .args(["parse", "nonexistent/path"])
        .output()
        .expect("failed to run");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Error:"));
}

#[test]
fn cli_validate_clean() {
    let output = m3l_bin()
        .args(["validate", "samples/01-ecommerce.m3l.md"])
        .output()
        .expect("failed to run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("0 errors"));
}

#[test]
fn cli_validate_json_format() {
    let output = m3l_bin()
        .args([
            "validate",
            "samples/01-ecommerce.m3l.md",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);

    let result: serde_json::Value = serde_json::from_str(&stdout).expect("invalid JSON output");
    assert_eq!(result["summary"]["errors"], 0);
    assert_eq!(result["summary"]["files"], 1);
}

#[test]
fn cli_validate_with_errors() {
    // Parsing all samples together causes duplicate name errors
    let output = m3l_bin()
        .args(["validate", "samples/"])
        .output()
        .expect("failed to run");
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("error"), "stdout: {stdout}");
    assert!(stdout.contains("M3L-E005"), "stdout: {stdout}");
}

#[test]
fn cli_parse_output_file() {
    let tmp = std::env::temp_dir().join("m3l-cli-test-output.json");
    let output = m3l_bin()
        .args([
            "parse",
            "samples/01-ecommerce.m3l.md",
            "-o",
            tmp.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let content = std::fs::read_to_string(&tmp).expect("output file should exist");
    let ast: serde_json::Value = serde_json::from_str(&content).expect("invalid JSON in file");
    assert_eq!(ast["parserVersion"], env!("CARGO_PKG_VERSION"));

    std::fs::remove_file(&tmp).ok();
}

// ── Lint tests ───────────────────────────────────────────────

#[test]
fn cli_lint_human() {
    let output = m3l_bin()
        .args(["lint", "samples/01-ecommerce.m3l.md"])
        .output()
        .expect("failed to run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("lint"));
}

#[test]
fn cli_lint_json() {
    let output = m3l_bin()
        .args(["lint", "samples/01-ecommerce.m3l.md", "--format", "json"])
        .output()
        .expect("failed to run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("invalid JSON");
    assert!(result["diagnostics"].is_array());
    assert!(result["summary"]["count"].is_number());
}

#[test]
fn cli_lint_sarif() {
    let output = m3l_bin()
        .args(["lint", "samples/01-ecommerce.m3l.md", "--format", "sarif"])
        .output()
        .expect("failed to run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let sarif: serde_json::Value = serde_json::from_str(&stdout).expect("invalid SARIF JSON");
    assert_eq!(sarif["version"], "2.1.0");
    assert!(sarif["runs"].is_array());
}

// ── Format tests ─────────────────────────────────────────────

#[test]
fn cli_format_single_file() {
    let output = m3l_bin()
        .args(["format", "samples/01-ecommerce.m3l.md"])
        .output()
        .expect("failed to run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should contain model headers (e.g. "## Customer : Timestampable")
    assert!(
        stdout.contains("##"),
        "expected model headers, got: {stdout}"
    );
}

#[test]
fn cli_format_directory() {
    let output = m3l_bin()
        .args(["format", "samples/multi/"])
        .output()
        .expect("failed to run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("##"),
        "expected model headers, got: {stdout}"
    );
}

// ── Diff tests ───────────────────────────────────────────────

#[test]
fn cli_diff_identical_files() {
    let output = m3l_bin()
        .args([
            "diff",
            "samples/01-ecommerce.m3l.md",
            "samples/01-ecommerce.m3l.md",
        ])
        .output()
        .expect("failed to run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Identical files should show no differences
    assert!(
        stdout.contains("No differences") || stdout.contains("0 added"),
        "expected no differences, got: {stdout}"
    );
}

#[test]
fn cli_diff_different_files() {
    let output = m3l_bin()
        .args([
            "diff",
            "samples/01-ecommerce.m3l.md",
            "samples/02-blog-cms.m3l.md",
        ])
        .output()
        .expect("failed to run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Different files should show additions and removals
    assert!(stdout.contains('+') || stdout.contains('-'));
}

// ── Analyze tests ────────────────────────────────────────────

#[test]
fn cli_analyze_mermaid() {
    let output = m3l_bin()
        .args(["analyze", "samples/01-ecommerce.m3l.md"])
        .output()
        .expect("failed to run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("graph LR"));
    assert!(stdout.contains("-->"));
    assert!(stdout.contains("nodes"));
    assert!(stdout.contains("edges"));
}

#[test]
fn cli_analyze_dot() {
    let output = m3l_bin()
        .args(["analyze", "samples/01-ecommerce.m3l.md", "--format", "dot"])
        .output()
        .expect("failed to run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("digraph M3L"));
    assert!(stdout.contains("->"));
    assert!(stdout.contains("rankdir=LR"));
}

#[test]
fn cli_analyze_directory() {
    let output = m3l_bin()
        .args(["analyze", "samples/multi/"])
        .output()
        .expect("failed to run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("graph LR"));
}

// ══════════════════════════════════════════════════════════════
// Validate — error codes (dedicated fixtures)
// ══════════════════════════════════════════════════════════════

#[test]
fn validate_e001_rollup_no_ref() {
    let output = m3l_bin()
        .args([
            "validate",
            "samples/test/validate/e001-rollup-no-ref.m3l.md",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run");
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("invalid JSON");
    assert!(
        result["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "M3L-E001"),
        "Expected M3L-E001 in diagnostics"
    );
}

#[test]
fn validate_e002_lookup_no_ref() {
    let output = m3l_bin()
        .args([
            "validate",
            "samples/test/validate/e002-lookup-no-ref.m3l.md",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run");
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("invalid JSON");
    assert!(
        result["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "M3L-E002"),
        "Expected M3L-E002 in diagnostics"
    );
}

#[test]
fn validate_e004_view_bad_source() {
    let output = m3l_bin()
        .args([
            "validate",
            "samples/test/validate/e004-view-bad-source.m3l.md",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run");
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("invalid JSON");
    assert!(
        result["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "M3L-E004"),
        "Expected M3L-E004 in diagnostics"
    );
}

#[test]
fn validate_e005_duplicate_name() {
    let output = m3l_bin()
        .args([
            "validate",
            "samples/test/validate/e005-duplicate-name.m3l.md",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run");
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("invalid JSON");
    assert!(
        result["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "M3L-E005"),
        "Expected M3L-E005 in diagnostics"
    );
}

#[test]
fn validate_e006_duplicate_field() {
    let output = m3l_bin()
        .args([
            "validate",
            "samples/test/validate/e006-duplicate-field.m3l.md",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run");
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("invalid JSON");
    assert!(
        result["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "M3L-E006"),
        "Expected M3L-E006 in diagnostics"
    );
}

#[test]
fn validate_e009_undefined_type() {
    let output = m3l_bin()
        .args([
            "validate",
            "samples/test/validate/e009-undefined-type.m3l.md",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run");
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("invalid JSON");
    assert!(
        result["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "M3L-E009"),
        "Expected M3L-E009 in diagnostics"
    );
}

#[test]
fn validate_e010_relations_no_ref() {
    let output = m3l_bin()
        .args([
            "validate",
            "samples/test/validate/e010-relations-no-ref.m3l.md",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run");
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("invalid JSON");
    assert!(
        result["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "M3L-E010"),
        "Expected M3L-E010 in diagnostics"
    );
}

// ══════════════════════════════════════════════════════════════
// Validate — warning codes (dedicated fixtures)
// ══════════════════════════════════════════════════════════════

#[test]
fn validate_w001_long_line_strict() {
    let output = m3l_bin()
        .args([
            "validate",
            "samples/test/validate/w001-long-line.m3l.md",
            "--strict",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("invalid JSON");
    assert!(
        result["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "M3L-W001"),
        "Expected M3L-W001 in diagnostics"
    );
}

#[test]
fn validate_w003_deprecated() {
    let output = m3l_bin()
        .args([
            "validate",
            "samples/test/validate/w003-deprecated.m3l.md",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("invalid JSON");
    let diags = result["diagnostics"].as_array().unwrap();
    let w003_count = diags.iter().filter(|d| d["code"] == "M3L-W003").count();
    assert!(
        w003_count >= 2,
        "Expected at least 2 M3L-W003 warnings (datetime + @cascade)"
    );
}

#[test]
fn validate_w004_long_lookup_strict() {
    let output = m3l_bin()
        .args([
            "validate",
            "samples/test/validate/w004-long-lookup.m3l.md",
            "--strict",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("invalid JSON");
    assert!(
        result["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "M3L-W004"),
        "Expected M3L-W004 in diagnostics"
    );
}

#[test]
fn validate_w005_type_mismatch() {
    let output = m3l_bin()
        .args([
            "validate",
            "samples/test/validate/w005-type-mismatch.m3l.md",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("invalid JSON");
    assert!(
        result["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "M3L-W005"),
        "Expected M3L-W005 in diagnostics"
    );
}

#[test]
fn validate_w006_range_violation() {
    let output = m3l_bin()
        .args([
            "validate",
            "samples/test/validate/w006-range-violation.m3l.md",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("invalid JSON");
    assert!(
        result["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "M3L-W006"),
        "Expected M3L-W006 in diagnostics"
    );
}

#[test]
fn validate_clean_fixture() {
    let output = m3l_bin()
        .args([
            "validate",
            "samples/test/validate/clean.m3l.md",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("invalid JSON");
    assert_eq!(result["summary"]["errors"], 0);
    assert_eq!(result["summary"]["warnings"], 0);
}

// ══════════════════════════════════════════════════════════════
// Lint — rule-specific fixtures
// ══════════════════════════════════════════════════════════════

#[test]
fn lint_naming_convention() {
    let output = m3l_bin()
        .args([
            "lint",
            "samples/test/lint/naming-bad.m3l.md",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("invalid JSON");
    assert!(
        result["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "naming-convention"),
        "Expected naming-convention rule hit"
    );
}

#[test]
fn lint_model_size() {
    let output = m3l_bin()
        .args([
            "lint",
            "samples/test/lint/large-model.m3l.md",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("invalid JSON");
    assert!(
        result["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "model-size"),
        "Expected model-size rule hit"
    );
}

#[test]
fn lint_similar_fields() {
    let output = m3l_bin()
        .args([
            "lint",
            "samples/test/lint/similar-fields.m3l.md",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("invalid JSON");
    assert!(
        result["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "similar-fields"),
        "Expected similar-fields rule hit"
    );
}

#[test]
fn lint_relation_complexity() {
    let output = m3l_bin()
        .args([
            "lint",
            "samples/test/lint/many-refs.m3l.md",
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("invalid JSON");
    assert!(
        result["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["rule"] == "relation-complexity"),
        "Expected relation-complexity rule hit"
    );
}

#[test]
fn lint_clean_fixture() {
    let output = m3l_bin()
        .args(["lint", "samples/test/lint/clean.m3l.md", "--format", "json"])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).expect("invalid JSON");
    assert_eq!(
        result["summary"]["count"], 0,
        "Expected 0 lint issues for clean fixture"
    );
}

// ══════════════════════════════════════════════════════════════
// Diff — dedicated fixtures
// ══════════════════════════════════════════════════════════════

#[test]
fn diff_known_changes() {
    let output = m3l_bin()
        .args([
            "diff",
            "samples/test/diff/v1.m3l.md",
            "samples/test/diff/v2.m3l.md",
        ])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("+ model NewModel"), "stdout: {stdout}");
    assert!(stdout.contains("- model OldModel"), "stdout: {stdout}");
    assert!(stdout.contains("+ Customer.age"), "stdout: {stdout}");
    assert!(stdout.contains("~ Customer.phone"), "stdout: {stdout}");
}

#[test]
fn diff_reverse() {
    let output = m3l_bin()
        .args([
            "diff",
            "samples/test/diff/v2.m3l.md",
            "samples/test/diff/v1.m3l.md",
        ])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Reversing swaps additions and removals
    assert!(stdout.contains("+ model OldModel"), "stdout: {stdout}");
    assert!(stdout.contains("- model NewModel"), "stdout: {stdout}");
    assert!(stdout.contains("- Customer.age"), "stdout: {stdout}");
}

#[test]
fn diff_summary_counts() {
    let output = m3l_bin()
        .args([
            "diff",
            "samples/test/diff/v1.m3l.md",
            "samples/test/diff/v2.m3l.md",
        ])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("2 added"), "stdout: {stdout}");
    assert!(stdout.contains("1 removed"), "stdout: {stdout}");
    assert!(stdout.contains("1 modified"), "stdout: {stdout}");
}

// ══════════════════════════════════════════════════════════════
// Analyze — dedicated fixtures
// ══════════════════════════════════════════════════════════════

#[test]
fn analyze_mermaid_edges() {
    let output = m3l_bin()
        .args(["analyze", "samples/test/analyze/graph.m3l.md"])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("graph LR"), "stdout: {stdout}");
    assert!(
        stdout.contains("-->|inherits|"),
        "Expected inheritance edge, stdout: {stdout}"
    );
    assert!(
        stdout.contains("-->|ref|"),
        "Expected reference edge, stdout: {stdout}"
    );
}

#[test]
fn analyze_dot_format() {
    let output = m3l_bin()
        .args([
            "analyze",
            "samples/test/analyze/graph.m3l.md",
            "--format",
            "dot",
        ])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("digraph M3L"), "stdout: {stdout}");
    assert!(stdout.contains("rankdir=LR"), "stdout: {stdout}");
    assert!(stdout.contains("->"), "stdout: {stdout}");
    assert!(
        stdout.contains("style=dashed"),
        "Expected inherits edge style, stdout: {stdout}"
    );
}

#[test]
fn analyze_isolated_node() {
    let output = m3l_bin()
        .args(["analyze", "samples/test/analyze/graph.m3l.md"])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Isolated node appears standalone without any edge
    assert!(stdout.contains("    Isolated"), "stdout: {stdout}");
    assert!(
        !stdout.contains("Isolated -->") && !stdout.contains("--> Isolated"),
        "Isolated node should have no edges, stdout: {stdout}"
    );
}

// ══════════════════════════════════════════════════════════════
// Format — dedicated fixtures
// ══════════════════════════════════════════════════════════════

#[test]
fn format_full_features() {
    let output = m3l_bin()
        .args(["format", "samples/test/format/full.m3l.md"])
        .output()
        .expect("failed to run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("# Namespace:"), "stdout: {stdout}");
    assert!(stdout.contains("::interface"), "stdout: {stdout}");
    assert!(stdout.contains("::enum"), "stdout: {stdout}");
    assert!(stdout.contains("::view"), "stdout: {stdout}");
    assert!(stdout.contains("## Customer"), "stdout: {stdout}");
}

#[test]
fn format_nested_fields() {
    let output = m3l_bin()
        .args(["format", "samples/test/format/full.m3l.md"])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Nested fields should be indented
    assert!(
        stdout.contains("  - bio:") || stdout.contains("  - avatar_url:"),
        "Expected indented nested fields, stdout: {stdout}"
    );
}

#[test]
fn format_roundtrip() {
    // Format → write to temp file → parse → should produce valid JSON AST
    let output = m3l_bin()
        .args(["format", "samples/test/format/full.m3l.md"])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let formatted = String::from_utf8_lossy(&output.stdout);

    let tmp = std::env::temp_dir().join("m3l-format-roundtrip.m3l.md");
    std::fs::write(&tmp, formatted.as_ref()).expect("write tmp");

    let parse_out = m3l_bin()
        .args(["parse", tmp.to_str().unwrap()])
        .output()
        .expect("failed to run");
    assert!(
        parse_out.status.success(),
        "Roundtrip parse failed, stderr: {}",
        String::from_utf8_lossy(&parse_out.stderr)
    );
    let stdout = String::from_utf8_lossy(&parse_out.stdout);
    let ast: serde_json::Value =
        serde_json::from_str(&stdout).expect("invalid JSON from roundtrip");
    assert!(
        ast["models"].is_array(),
        "Roundtrip should produce models array"
    );

    std::fs::remove_file(&tmp).ok();
}

#[test]
fn format_idempotent() {
    // format(input) should equal format(format(input))
    // i.e., formatting twice produces the same output
    let inputs = &[
        "samples/test/format/full.m3l.md",
        "spec/conformance/inputs/01-ecommerce.m3l.md",
        "spec/conformance/inputs/02-blog-cms.m3l.md",
        "spec/conformance/inputs/03-types-showcase.m3l.md",
    ];

    for input_path in inputs {
        // First format
        let output1 = m3l_bin()
            .args(["format", input_path])
            .output()
            .expect("failed to run");
        assert!(
            output1.status.success(),
            "[{input_path}] First format failed: {}",
            String::from_utf8_lossy(&output1.stderr)
        );
        let formatted1 = String::from_utf8_lossy(&output1.stdout);

        // Write first format to temp file
        let tmp = std::env::temp_dir().join("m3l-idempotent-test.m3l.md");
        std::fs::write(&tmp, formatted1.as_ref()).expect("write tmp");

        // Second format (format the already-formatted output)
        let output2 = m3l_bin()
            .args(["format", tmp.to_str().unwrap()])
            .output()
            .expect("failed to run");
        assert!(
            output2.status.success(),
            "[{input_path}] Second format failed: {}",
            String::from_utf8_lossy(&output2.stderr)
        );
        let formatted2 = String::from_utf8_lossy(&output2.stdout);

        // They should be identical
        assert_eq!(
            formatted1.as_ref(),
            formatted2.as_ref(),
            "[{input_path}] Format is not idempotent — second format produced different output"
        );

        // Also verify the formatted output parses correctly
        let parse_out = m3l_bin()
            .args(["parse", tmp.to_str().unwrap()])
            .output()
            .expect("failed to run");
        assert!(
            parse_out.status.success(),
            "[{input_path}] Formatted output failed to parse: {}",
            String::from_utf8_lossy(&parse_out.stderr)
        );

        std::fs::remove_file(&tmp).ok();
    }
}

#[test]
fn format_preserves_ast() {
    // parse(input) should produce same models/enums as parse(format(input))
    // We compare structural content (field names, types, counts) rather than
    // exact JSON equality, since loc/source fields will differ between files.
    let inputs = &[
        "samples/test/format/full.m3l.md",
        "spec/conformance/inputs/01-ecommerce.m3l.md",
        "spec/conformance/inputs/03-types-showcase.m3l.md",
    ];

    for input_path in inputs {
        // Parse original
        let parse1 = m3l_bin()
            .args(["parse", input_path])
            .output()
            .expect("failed to run");
        assert!(
            parse1.status.success(),
            "[{input_path}] Original parse failed"
        );
        let ast1: serde_json::Value =
            serde_json::from_str(&String::from_utf8_lossy(&parse1.stdout))
                .expect("invalid JSON from original parse");

        // Format then parse
        let format_out = m3l_bin()
            .args(["format", input_path])
            .output()
            .expect("failed to run");
        assert!(format_out.status.success(), "[{input_path}] Format failed");
        let tmp = std::env::temp_dir().join("m3l-ast-equiv-test.m3l.md");
        std::fs::write(&tmp, &format_out.stdout).expect("write tmp");

        let parse2 = m3l_bin()
            .args(["parse", tmp.to_str().unwrap()])
            .output()
            .expect("failed to run");
        assert!(
            parse2.status.success(),
            "[{input_path}] Formatted parse failed"
        );
        let ast2: serde_json::Value =
            serde_json::from_str(&String::from_utf8_lossy(&parse2.stdout))
                .expect("invalid JSON from formatted parse");

        // Compare model count
        let models1 = ast1["models"].as_array().expect("models1 array");
        let models2 = ast2["models"].as_array().expect("models2 array");
        assert_eq!(
            models1.len(),
            models2.len(),
            "[{input_path}] Model count differs after format"
        );

        // Compare each model's name, type, and field count/names
        for (m1, m2) in models1.iter().zip(models2.iter()) {
            assert_eq!(m1["name"], m2["name"], "[{input_path}] Model name differs");
            assert_eq!(
                m1["type"], m2["type"],
                "[{input_path}] Model type differs for {}",
                m1["name"]
            );

            let f1: Vec<&str> = m1["fields"]
                .as_array()
                .map(|a| a.iter().filter_map(|f| f["name"].as_str()).collect())
                .unwrap_or_default();
            let f2: Vec<&str> = m2["fields"]
                .as_array()
                .map(|a| a.iter().filter_map(|f| f["name"].as_str()).collect())
                .unwrap_or_default();
            assert_eq!(
                f1, f2,
                "[{input_path}] Field names differ for model {}",
                m1["name"]
            );
        }

        // Compare enum count and names
        let enums1 = ast1["enums"].as_array();
        let enums2 = ast2["enums"].as_array();
        if let (Some(e1), Some(e2)) = (enums1, enums2) {
            assert_eq!(
                e1.len(),
                e2.len(),
                "[{input_path}] Enum count differs after format"
            );
            for (en1, en2) in e1.iter().zip(e2.iter()) {
                assert_eq!(en1["name"], en2["name"], "[{input_path}] Enum name differs");
            }
        }

        std::fs::remove_file(&tmp).ok();
    }
}

// ══════════════════════════════════════════════════════════════
// Parse — edge cases
// ══════════════════════════════════════════════════════════════

#[test]
fn parse_file_with_errors() {
    // Even files with errors should produce an AST (with diagnostics)
    let output = m3l_bin()
        .args(["parse", "samples/test/validate/e009-undefined-type.m3l.md"])
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let ast: serde_json::Value = serde_json::from_str(&stdout).expect("invalid JSON");
    assert!(ast["models"].is_array());
    assert!(!ast["models"].as_array().unwrap().is_empty());
}

#[test]
fn parse_empty_dir() {
    let tmp = std::env::temp_dir().join("m3l-empty-dir-test");
    std::fs::create_dir_all(&tmp).ok();
    let output = m3l_bin()
        .args(["parse", tmp.to_str().unwrap()])
        .output()
        .expect("failed to run");
    assert!(
        !output.status.success(),
        "Expected failure for empty directory"
    );
    std::fs::remove_dir_all(&tmp).ok();
}

/// A scratch directory under the system temp dir, unique per test, removed on drop.
struct ScratchDir(PathBuf);

impl ScratchDir {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("m3l-cli-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        ScratchDir(dir)
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn cli_parse_directory_follows_project_config_sources() {
    let dir = ScratchDir::new("config-sources");
    std::fs::write(
        dir.0.join("m3l.config.yaml"),
        "name: demo\nversion: \"1.0\"\nsources:\n  - \"included.m3l.md\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.0.join("included.m3l.md"),
        "# Namespace: t\n\n## Included\n- id: identifier @pk\n",
    )
    .unwrap();
    std::fs::write(
        dir.0.join("excluded.m3l.md"),
        "# Namespace: t\n\n## Excluded\n- id: identifier @pk\n",
    )
    .unwrap();

    let output = m3l_bin()
        .arg("parse")
        .arg(&dir.0)
        .output()
        .expect("failed to run");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let ast: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("invalid JSON output");
    let names: Vec<&str> = ast["models"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Included"]);
}

#[test]
fn cli_parse_directory_reports_an_invalid_project_config() {
    let dir = ScratchDir::new("config-invalid");
    std::fs::write(dir.0.join("m3l.config.yaml"), "sources: [unclosed\n").unwrap();

    let output = m3l_bin()
        .arg("parse")
        .arg(&dir.0)
        .output()
        .expect("failed to run");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    // The message names the file; the parser's own text only says where in it.
    assert!(
        stderr.contains("Invalid project configuration"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("m3l.config.yaml"), "stderr: {stderr}");
}

/// A misspelt key used to be dropped without a word: `source:` left `sources` unset, the
/// directory scan it fell back to picked up every file, and the output looked like the setting
/// had worked. It is still ignored — a key this version does not know may be one a later version
/// does — but it is said out loud, on both paths that read the configuration.
#[test]
fn cli_warns_about_an_unknown_project_config_key() {
    let dir = ScratchDir::new("config-typo");
    std::fs::write(
        dir.0.join("m3l.config.yaml"),
        "name: demo
source:
  - \"only.m3l.md\"
",
    )
    .unwrap();
    std::fs::write(
        dir.0.join("only.m3l.md"),
        "# Namespace: t

## Only
- id: identifier @pk
",
    )
    .unwrap();

    for command in ["parse", "validate"] {
        let output = m3l_bin()
            .arg(command)
            .arg(&dir.0)
            .output()
            .expect("failed to run");
        assert!(
            output.status.success(),
            "{command} stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("unknown key 'source'"),
            "{command} stderr: {stderr}"
        );
        assert!(
            stderr.contains("did you mean 'sources'"),
            "{command} stderr: {stderr}"
        );
    }
}

#[test]
fn cli_is_silent_about_a_project_config_with_only_known_keys() {
    let dir = ScratchDir::new("config-clean");
    std::fs::write(
        dir.0.join("m3l.config.yaml"),
        "name: demo
version: \"1.0\"
",
    )
    .unwrap();
    std::fs::write(
        dir.0.join("a.m3l.md"),
        "# Namespace: t

## A
- id: identifier @pk
",
    )
    .unwrap();

    let output = m3l_bin()
        .arg("parse")
        .arg(&dir.0)
        .output()
        .expect("failed to run");
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("Warning"), "stderr: {stderr}");
    let ast: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("invalid JSON output");
    assert_eq!(ast["project"]["name"], "demo");
}

/// The directory scan reads every `.md` file, so a non-UTF-8 README in the tree stops the whole
/// run. The error names the file and how to keep it out, instead of the runtime's words alone.
#[test]
fn cli_names_a_non_utf8_file_the_directory_scan_picked_up() {
    let dir = ScratchDir::new("non-utf8-scan");
    std::fs::write(
        dir.0.join("model.m3l.md"),
        "# Namespace: t\n## A\n- id: identifier @pk\n",
    )
    .unwrap();
    std::fs::write(dir.0.join("README.md"), b"caf\xe9 latin-1 notes\n").unwrap();

    let output = m3l_bin()
        .arg("parse")
        .arg(&dir.0)
        .output()
        .expect("failed to run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("README.md"), "stderr: {stderr}");
    assert!(stderr.contains("is not UTF-8 text"), "stderr: {stderr}");
    assert!(stderr.contains("sources"), "stderr: {stderr}");
    assert!(stderr.contains("m3l.config.yaml"), "stderr: {stderr}");
}

#[test]
fn cli_says_to_re_save_a_non_utf8_file_named_directly() {
    let dir = ScratchDir::new("non-utf8-file");
    let file = dir.0.join("model.m3l.md");
    std::fs::write(&file, b"# Namespace: t\n## A\n- caf\xe9: string\n").unwrap();

    let output = m3l_bin()
        .arg("parse")
        .arg(&file)
        .output()
        .expect("failed to run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("is not UTF-8 text"), "stderr: {stderr}");
    assert!(stderr.contains("save it as UTF-8"), "stderr: {stderr}");
}

#[test]
fn cli_parse_output_into_a_missing_directory_says_so() {
    let dir = ScratchDir::new("parse-out-missing");
    let out = dir.0.join("nowhere").join("ast.json");

    let output = m3l_bin()
        .args(["parse", "samples/01-ecommerce.m3l.md", "-o"])
        .arg(&out)
        .output()
        .expect("failed to run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("does not exist"), "stderr: {stderr}");
    assert!(stderr.contains("nowhere"), "stderr: {stderr}");
    assert!(!stderr.contains("os error"), "stderr: {stderr}");
}
