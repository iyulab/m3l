//! Keeps references that only make sense inside the maintainers' own workflow out of every file
//! this repository publishes.
//!
//! Everything tracked here is public: the sources, the specification, and the READMEs that ship
//! to crates.io, npm and NuGet. A comment such as "see item #142" or "(measured in cycle 93)"
//! points a reader at something they cannot open, so it explains nothing — the sentence has to
//! say what was learned, not where it was recorded.
//!
//! What is asserted is the *shape* of such a reference, never a list of specific names — naming
//! what must not appear would put those names in a public file, which is the very thing being
//! prevented.
//!
//! The guard lives in the CLI crate's tests only because they already resolve the workspace
//! root; it is about the repository, not about the CLI.

use regex::Regex;
use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent() // crates/
        .unwrap()
        .parent() // workspace root
        .unwrap()
        .to_path_buf()
}

/// Each shape, with the reason it means nothing to a public reader.
fn forbidden_shapes() -> Vec<(Regex, &'static str)> {
    vec![
        (
            Regex::new(r"(?i)\bdocket\b").unwrap(),
            "a reference into the maintainers' private issue queue",
        ),
        (
            Regex::new(r"(?i)\bcycle-\d+\b").unwrap(),
            "a work-log iteration number",
        ),
        (
            Regex::new(r"(?i)\bclaudedocs\b").unwrap(),
            "a path into untracked working notes",
        ),
        // This repository keeps no roadmap or handoff document; a reference to one is dead.
        (
            Regex::new(r"\b(?:ROADMAP|HANDOFF)\b").unwrap(),
            "a planning document this repository does not contain",
        ),
        (
            Regex::new(r"\b(?:ISSUE|TRIAGE|PLAN|PROPOSAL|DECISION)-[A-Za-z0-9.-]+-\d{8}").unwrap(),
            "a working-note file name",
        ),
    ]
}

/// Files allowed to contain the shapes, each for a stated reason. Paths are repository-relative
/// with forward slashes, as `git ls-files` prints them.
const EXEMPT: &[(&str, &str)] = &[
    // Untracking the notes directory is the policy working, not a reference to it.
    (
        ".gitignore",
        "excludes the working-notes directory from the repository",
    ),
    // The guard has to name the shapes it forbids.
    ("crates/m3l-cli/tests/public_text.rs", "this guard"),
];

/// The published set is exactly what git tracks — a directory walk would also read build output
/// and ignored local files.
fn tracked_files(root: &Path) -> Vec<String> {
    let output = Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(root)
        .output()
        .expect("failed to run `git ls-files`");
    assert!(
        output.status.success(),
        "`git ls-files` failed in {}: {}",
        root.display(),
        String::from_utf8_lossy(&output.stderr)
    );

    let files: Vec<String> = String::from_utf8(output.stdout)
        .expect("`git ls-files` printed non-UTF-8 paths")
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .collect();

    // A guard that stops finding its subject asserts nothing.
    assert!(files.iter().any(|f| f == "README.md"));
    assert!(files.iter().any(|f| f == "CHANGELOG.md"));
    files
}

#[test]
fn no_published_file_refers_to_the_maintainers_private_workflow() {
    let root = workspace_root();
    let shapes = forbidden_shapes();
    let mut hits = Vec::new();

    for file in tracked_files(&root) {
        if EXEMPT.iter().any(|(path, _)| *path == file) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(root.join(&file)) else {
            continue; // not UTF-8 text — nothing a reader would read as prose
        };
        for (index, line) in text.lines().enumerate() {
            for (pattern, why) in &shapes {
                if let Some(found) = pattern.find(line) {
                    hits.push(format!(
                        "{file}:{}: '{}' — {why}",
                        index + 1,
                        found.as_str()
                    ));
                }
            }
        }
    }

    assert!(
        hits.is_empty(),
        "Published files carry references a public reader cannot follow. Say what was learned \
         instead of where it was recorded (or drop the reference if the sentence stands without it):\n{}",
        hits.join("\n")
    );
}

#[test]
fn every_exemption_still_names_a_tracked_file() {
    let tracked = tracked_files(&workspace_root());
    let stale: Vec<&str> = EXEMPT
        .iter()
        .map(|(path, _)| *path)
        .filter(|path| !tracked.iter().any(|f| f == path))
        .collect();

    assert!(
        stale.is_empty(),
        "Exemptions outlived their files — remove them so the list cannot quietly cover a file \
         that later takes the same path:\n{}",
        stale.join("\n")
    );
}
