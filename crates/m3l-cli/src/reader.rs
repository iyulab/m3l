use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// A file with its path and content.
pub struct M3lFile {
    pub path: String,
    pub content: String,
}

/// Project configuration from m3l.config.yaml.
#[derive(Debug, Deserialize)]
pub struct M3lConfig {
    pub name: Option<String>,
    pub version: Option<String>,
    pub sources: Option<Vec<String>>,
    /// Keys the configuration does not define. Collected rather than refused, and reported as
    /// warnings: a misspelt `source:` used to be dropped without a word, and the directory scan it
    /// fell back to looked like the setting had worked.
    #[serde(flatten)]
    unknown: std::collections::BTreeMap<String, yaml_serde::Value>,
}

const CONFIG_FILE: &str = "m3l.config.yaml";
const CONFIG_KEYS: [&str; 3] = ["name", "version", "sources"];

/// What a path reads as: its M3L files, the project configuration that selected them (when the
/// path is a directory carrying one), and warnings about that configuration.
pub struct ProjectInput {
    pub files: Vec<M3lFile>,
    pub config: Option<M3lConfig>,
    pub warnings: Vec<String>,
}

/// The path recorded as a file's `source` in the AST, with `/` separators on every platform.
///
/// The AST is a portable artefact — the same files must produce the same output wherever they are
/// parsed — and Windows paths would otherwise carry `\`. Only the platform separator is
/// rewritten: on Unix a backslash is an ordinary file-name character.
fn source_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    if std::path::MAIN_SEPARATOR == '\\' {
        text.replace('\\', "/")
    } else {
        text.into_owned()
    }
}

/// Read M3L files from a path (file or directory), with the project configuration that selected
/// them.
///
/// The configuration is read once, here, and handed back with the files. It used to be read twice —
/// once to select the files, which reported a malformed file, and once more for the project name,
/// which discarded any error — so the same file could be both refused and silently ignored
/// depending on which read looked at it.
pub fn read_project(input_path: &Path) -> Result<ProjectInput, String> {
    if !input_path.exists() {
        return Err(format!("Path does not exist: {}", input_path.display()));
    }

    if input_path.is_file() {
        let content = read_m3l_text(input_path, Selected::Named)?;
        return Ok(ProjectInput {
            files: vec![M3lFile {
                path: source_path(input_path),
                content,
            }],
            config: None,
            warnings: Vec::new(),
        });
    }

    if input_path.is_dir() {
        let config_path = input_path.join(CONFIG_FILE);
        if config_path.exists() {
            let config = read_config(&config_path)?;
            let warnings = unknown_key_warnings(&config_path, &config);
            let files = read_from_config(&config, input_path)?;
            return Ok(ProjectInput {
                files,
                config: Some(config),
                warnings,
            });
        }

        // Default: scan for *.m3l.md and *.m3l files
        return Ok(ProjectInput {
            files: scan_directory(input_path)?,
            config: None,
            warnings: Vec::new(),
        });
    }

    Err(format!(
        "Path is neither a file nor a directory: {}",
        input_path.display()
    ))
}

/// Parses the configuration file, naming the file in the error — the parser's own message says
/// where in the file, not which file.
fn read_config(config_path: &Path) -> Result<M3lConfig, String> {
    let yaml = fs::read_to_string(config_path)
        .map_err(|e| format!("Failed to read {}: {}", config_path.display(), e))?;
    yaml_serde::from_str(&yaml).map_err(|e| {
        format!(
            "Invalid project configuration {}: {}",
            config_path.display(),
            e
        )
    })
}

fn unknown_key_warnings(config_path: &Path, config: &M3lConfig) -> Vec<String> {
    config
        .unknown
        .keys()
        .map(|key| {
            let hint = CONFIG_KEYS
                .iter()
                .find(|known| is_near(key, known))
                .map(|known| format!(" (did you mean '{known}'?)"))
                .unwrap_or_default();
            format!(
                "{}: unknown key '{key}' is ignored{hint}; known keys are {}",
                config_path.display(),
                CONFIG_KEYS.join(", ")
            )
        })
        .collect()
}

/// A likely misspelling: equal ignoring case, one a prefix of the other, or one edit apart.
fn is_near(key: &str, known: &str) -> bool {
    let (a, b) = (key.to_ascii_lowercase(), known.to_ascii_lowercase());
    if a == b || a.starts_with(&b) || b.starts_with(&a) {
        return true;
    }
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cur = row[j + 1];
            row[j + 1] = if ca == cb {
                prev
            } else {
                1 + prev.min(row[j]).min(row[j + 1])
            };
            prev = cur;
        }
    }
    row[b.len()] <= 1
}

/// How a file came to be read — which decides what the reader is told to do when it cannot be.
#[derive(Clone, Copy)]
enum Selected {
    /// Named on the command line or matched by the configuration's `sources`.
    Named,
    /// Picked up by the directory scan, which reads every `.md` file under the directory.
    ByScan,
}

/// Reads a model file as UTF-8 text. A file that is not UTF-8 is named with what to do about it:
/// re-save it, or — when the directory scan picked it up (a README, notes) — keep it out of the
/// model by listing the model files under `sources`.
fn read_m3l_text(path: &Path, selected: Selected) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| match e.kind() {
        std::io::ErrorKind::InvalidData => match selected {
            Selected::Named => format!(
                "{} is not UTF-8 text — M3L files are read as UTF-8; save it as UTF-8",
                path.display()
            ),
            Selected::ByScan => format!(
                "{} is not UTF-8 text — M3L files are read as UTF-8. The directory scan reads every .md \
                 file; if this one is not part of the model, list the model files under `sources` in {}",
                path.display(),
                CONFIG_FILE
            ),
        },
        std::io::ErrorKind::PermissionDenied => {
            format!("Failed to read {}: permission denied", path.display())
        }
        _ => format!("Failed to read {}: {}", path.display(), e),
    })
}

fn scan_directory(dir_path: &Path) -> Result<Vec<M3lFile>, String> {
    // Scan *.m3l.md, *.m3l, and *.md — all three extensions are valid M3L files.
    let patterns = [
        dir_path.join("**/*.m3l.md"),
        dir_path.join("**/*.m3l"),
        dir_path.join("**/*.md"),
    ];

    let mut paths: Vec<PathBuf> = Vec::new();

    for pattern in &patterns {
        let pattern_str = pattern.to_string_lossy().replace('\\', "/");
        let entries =
            glob::glob(&pattern_str).map_err(|e| format!("Invalid glob pattern: {}", e))?;

        for entry in entries {
            match entry {
                Ok(path) => {
                    // Deduplicate: .m3l.md files match both *.m3l and *.md patterns
                    if !paths.contains(&path) {
                        paths.push(path);
                    }
                }
                Err(e) => {
                    return Err(format!("Glob error: {}", e));
                }
            }
        }
    }

    paths.sort();

    let mut files = Vec::new();
    for path in paths {
        let content = read_m3l_text(&path, Selected::ByScan)?;
        files.push(M3lFile {
            path: source_path(&path),
            content,
        });
    }

    Ok(files)
}

fn read_from_config(config: &M3lConfig, base_dir: &Path) -> Result<Vec<M3lFile>, String> {
    let source_patterns = match config.sources {
        Some(ref s) if !s.is_empty() => s.clone(),
        _ => return scan_directory(base_dir),
    };

    let mut files: Vec<M3lFile> = Vec::new();
    let mut seen: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();

    for pattern in &source_patterns {
        let full_pattern = base_dir.join(pattern);
        let pattern_str = full_pattern.to_string_lossy().replace('\\', "/");
        let entries = glob::glob(&pattern_str)
            .map_err(|e| format!("Invalid glob pattern '{}': {}", pattern, e))?;

        let mut matched: Vec<PathBuf> = Vec::new();
        for entry in entries {
            match entry {
                Ok(path) => {
                    if !seen.contains(&path) {
                        seen.insert(path.clone());
                        matched.push(path);
                    }
                }
                Err(e) => return Err(format!("Glob error: {}", e)),
            }
        }
        matched.sort();

        for path in matched {
            let content = read_m3l_text(&path, Selected::Named)?;
            files.push(M3lFile {
                path: source_path(&path),
                content,
            });
        }
    }

    Ok(files)
}
