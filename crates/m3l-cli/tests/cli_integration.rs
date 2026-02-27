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
    assert!(stdout.contains("0.4.0"));
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
    assert_eq!(ast["parserVersion"], "0.4.0");
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
    assert_eq!(ast["parserVersion"], "0.4.0");

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
