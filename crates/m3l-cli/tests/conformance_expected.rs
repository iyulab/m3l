//! The shared conformance suite, `spec/conformance/`: every test in `spec.json` names an input and
//! the AST `m3l parse` must print for it. This is the check that makes the suite executable — the
//! expected files are compared on every `cargo test`, not merely kept alongside the inputs.
//!
//! `parserVersion` is the one field left out of the comparison: it changes with every release and
//! says nothing about what a document means.
//!
//! After an intended change to the AST, regenerate the expected files and review the diff:
//!
//! ```text
//! M3L_BLESS=1 cargo test -p m3l-cli --test conformance_expected
//! ```

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn suite_dir() -> PathBuf {
    workspace_root().join("spec/conformance")
}

fn manifest() -> Vec<Value> {
    let text = std::fs::read_to_string(suite_dir().join("spec.json")).unwrap();
    let spec: Value = serde_json::from_str(&text).unwrap();
    spec["tests"].as_array().unwrap().clone()
}

/// `m3l parse` on a path relative to the workspace root, as the expected files were produced —
/// the printed text (fields in the order the parser serialises them) and its parsed value.
fn parse(relative: &str) -> (String, Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_m3l"))
        .current_dir(workspace_root())
        .args(["parse", relative])
        .output()
        .expect("failed to run m3l");
    assert!(
        output.status.success(),
        "m3l parse {relative} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).expect("m3l parse printed invalid UTF-8");
    let value = serde_json::from_str(&text).expect("m3l parse printed invalid JSON");
    (text, value)
}

fn without_version(mut v: Value) -> Value {
    if let Some(obj) = v.as_object_mut() {
        obj.remove("parserVersion");
    }
    v
}

fn diff(path: &str, a: &Value, b: &Value, out: &mut Vec<String>) {
    if a == b {
        return;
    }
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            let keys: std::collections::BTreeSet<&String> = x.keys().chain(y.keys()).collect();
            for k in keys {
                let (l, r) = (
                    x.get(k).unwrap_or(&Value::Null),
                    y.get(k).unwrap_or(&Value::Null),
                );
                diff(&format!("{path}.{k}"), l, r, out);
            }
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => {
            for (i, (l, r)) in x.iter().zip(y).enumerate() {
                diff(&format!("{path}[{i}]"), l, r, out);
            }
        }
        _ => out.push(format!("{path}: parser {a} / expected {b}")),
    }
}

#[test]
fn every_input_is_in_the_manifest_and_every_entry_has_its_files() {
    let tests = manifest();
    let listed: Vec<String> = tests
        .iter()
        .filter_map(|t| {
            t["input"]
                .as_str()
                .or(t["input_dir"].as_str())
                .map(str::to_string)
        })
        .collect();

    let mut problems = Vec::new();
    for entry in std::fs::read_dir(suite_dir().join("inputs")).unwrap() {
        let path = entry.unwrap().path();
        let name = format!("inputs/{}", path.file_name().unwrap().to_string_lossy());
        let is_input = path.is_dir() || name.ends_with(".m3l.md");
        if is_input && !listed.contains(&name) {
            problems.push(format!("{name} is not listed in spec.json"));
        }
    }
    for t in &tests {
        for key in ["input", "input_dir", "expected"] {
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
fn every_input_produces_its_expected_ast() {
    let bless = std::env::var_os("M3L_BLESS").is_some();
    let mut failures = Vec::new();

    for t in manifest() {
        let name = t["name"].as_str().unwrap();
        let input = t["input"].as_str().or(t["input_dir"].as_str()).unwrap();
        let expected_path = suite_dir().join(t["expected"].as_str().unwrap());

        let (text, actual) = parse(&format!("spec/conformance/{input}"));
        if bless {
            std::fs::write(&expected_path, text).unwrap();
            continue;
        }

        let expected: Value =
            serde_json::from_str(&std::fs::read_to_string(&expected_path).unwrap()).unwrap();
        let mut out = Vec::new();
        diff(
            "",
            &without_version(actual),
            &without_version(expected),
            &mut out,
        );
        if !out.is_empty() {
            failures.push(format!("{name}:\n  {}", out.join("\n  ")));
        }
    }

    assert!(
        failures.is_empty(),
        "the parser no longer produces the expected AST (regenerate with M3L_BLESS=1 only if the \
         change is intended):\n{}",
        failures.join("\n")
    );
}

/// The printed AST is the same text on every run. The comparison above is by value and would not
/// notice keys changing order, but a consumer that diffs or caches the output would — maps in the
/// AST are kept in a sorted order, not in whatever order a hash map iterates that process.
#[test]
fn parse_output_is_the_same_text_on_every_run() {
    let input = "spec/conformance/inputs/01-ecommerce.m3l.md";
    let (first, _) = parse(input);
    for _ in 0..4 {
        let (again, _) = parse(input);
        assert_eq!(
            again, first,
            "m3l parse printed a different text for the same input"
        );
    }
}
