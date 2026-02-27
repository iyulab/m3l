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
}

/// Read M3L files from a path (file or directory).
pub fn read_m3l_files(input_path: &Path) -> Result<Vec<M3lFile>, String> {
    if !input_path.exists() {
        return Err(format!("Path does not exist: {}", input_path.display()));
    }

    if input_path.is_file() {
        let content = fs::read_to_string(input_path)
            .map_err(|e| format!("Failed to read {}: {}", input_path.display(), e))?;
        return Ok(vec![M3lFile {
            path: input_path.to_string_lossy().to_string(),
            content,
        }]);
    }

    if input_path.is_dir() {
        // Check for m3l.config.yaml
        let config_path = input_path.join("m3l.config.yaml");
        if config_path.exists() {
            return read_from_config(&config_path, input_path);
        }

        // Default: scan for *.m3l.md and *.m3l files
        return scan_directory(input_path);
    }

    Err(format!(
        "Path is neither a file nor a directory: {}",
        input_path.display()
    ))
}

/// Read project config from m3l.config.yaml if it exists.
pub fn read_project_config(dir_path: &Path) -> Option<M3lConfig> {
    let config_path = dir_path.join("m3l.config.yaml");
    if !config_path.exists() {
        return None;
    }

    let content = fs::read_to_string(&config_path).ok()?;
    serde_yaml::from_str(&content).ok()
}

fn scan_directory(dir_path: &Path) -> Result<Vec<M3lFile>, String> {
    let pattern_md = dir_path.join("**/*.m3l.md");
    let pattern_plain = dir_path.join("**/*.m3l");

    let mut paths: Vec<PathBuf> = Vec::new();

    for pattern in [&pattern_md, &pattern_plain] {
        let pattern_str = pattern.to_string_lossy().replace('\\', "/");
        let entries =
            glob::glob(&pattern_str).map_err(|e| format!("Invalid glob pattern: {}", e))?;

        for entry in entries {
            match entry {
                Ok(path) => {
                    // Skip .m3l.md matches from the .m3l pattern
                    if path.extension().is_some_and(|e| e == "m3l")
                        && path.to_string_lossy().ends_with(".m3l.md")
                    {
                        continue;
                    }
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
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        files.push(M3lFile {
            path: path.to_string_lossy().to_string(),
            content,
        });
    }

    Ok(files)
}

fn read_from_config(config_path: &Path, base_dir: &Path) -> Result<Vec<M3lFile>, String> {
    let yaml_content =
        fs::read_to_string(config_path).map_err(|e| format!("Failed to read config: {}", e))?;

    let config: M3lConfig =
        serde_yaml::from_str(&yaml_content).map_err(|e| format!("Invalid YAML config: {}", e))?;

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
            let content = fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
            files.push(M3lFile {
                path: path.to_string_lossy().to_string(),
                content,
            });
        }
    }

    Ok(files)
}
