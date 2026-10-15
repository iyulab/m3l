//! The validate half of the conformance suite, `spec/conformance/validate.json`: every test names an
//! input and the diagnostics `m3l validate --format json` must report for it — code, line and
//! severity. `m3l parse` output (`conformance_expected.rs`) carries only what resolution reports, so
//! without this suite a diagnostic the validator raises (`M3L-E002`, `M3L-E022`, `M3L-E027`, …) would
//! be pinned by the core's own tests alone, which bindings and other implementations cannot run.
//!
//! Messages are not compared: the code is the contract, the wording is for people.
//!
//! After an intended change, regenerate the expected files and review the diff:
//!
//! ```text
//! M3L_BLESS=1 cargo test -p m3l-cli --test conformance_validate
//! ```

use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn suite_dir() -> PathBuf {
    workspace_root().join("spec/conformance")
}

fn manifest() -> Vec<Value> {
    let text = std::fs::read_to_string(suite_dir().join("validate.json")).unwrap();
    let spec: Value = serde_json::from_str(&text).unwrap();
    spec["tests"].as_array().unwrap().clone()
}

/// `m3l validate --format json` on a path relative to the suite, reduced to what the suite pins.
fn validate(relative: &str, strict: bool) -> Value {
    let mut args = vec!["validate", "--format", "json"];
    if strict {
        args.push("--strict");
    }
    let input = format!("spec/conformance/{relative}");
    args.push(&input);
    let output = Command::new(env!("CARGO_BIN_EXE_m3l"))
        .current_dir(workspace_root())
        .args(&args)
        .output()
        .expect("failed to run m3l");
    let text = String::from_utf8(output.stdout).expect("m3l validate printed invalid UTF-8");
    let report: Value = serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("m3l validate {relative} printed invalid JSON ({e}): {text}"));
    let diagnostics: Vec<Value> = report["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| json!({ "code": d["code"], "line": d["line"], "severity": d["severity"] }))
        .collect();
    json!({ "diagnostics": diagnostics })
}

#[test]
fn every_validate_input_is_in_the_manifest_and_every_entry_has_its_files() {
    let tests = manifest();
    let listed: Vec<&str> = tests.iter().filter_map(|t| t["input"].as_str()).collect();

    let mut problems = Vec::new();
    for entry in std::fs::read_dir(suite_dir().join("validate/inputs")).unwrap() {
        let path = entry.unwrap().path();
        let name = format!(
            "validate/inputs/{}",
            path.file_name().unwrap().to_string_lossy()
        );
        if name.ends_with(".m3l.md") && !listed.contains(&name.as_str()) {
            problems.push(format!("{name} is not listed in validate.json"));
        }
    }
    for t in &tests {
        for key in ["input", "expected"] {
            if let Some(rel) = t[key].as_str() {
                if !suite_dir().join(rel).exists() {
                    problems.push(format!("{}: {key} {rel} does not exist", t["name"]));
                }
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn every_validate_input_reports_its_expected_diagnostics() {
    let bless = std::env::var_os("M3L_BLESS").is_some();
    let mut failures = Vec::new();

    for t in manifest() {
        let name = t["name"].as_str().unwrap();
        let input = t["input"].as_str().unwrap();
        let expected_path = suite_dir().join(t["expected"].as_str().unwrap());
        let actual = validate(input, t["strict"].as_bool().unwrap_or(false));

        if bless {
            let text = serde_json::to_string_pretty(&actual).unwrap() + "\n";
            std::fs::write(&expected_path, text).unwrap();
            continue;
        }

        let expected: Value =
            serde_json::from_str(&std::fs::read_to_string(&expected_path).unwrap()).unwrap();
        if actual != expected {
            failures.push(format!(
                "{name}: validate reported {actual}, expected {expected}"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
