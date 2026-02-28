mod commands;
mod reader;

use std::path::{Path, PathBuf};
use std::process;

use clap::{Parser, Subcommand};

use m3l_core::{parse_string, resolve, validate, ProjectInfo, ValidateOptions};
use reader::{read_m3l_files, read_project_config};

#[derive(Parser)]
#[command(
    name = "m3l",
    version,
    about = "M3L parser and validator — parse .m3l, .m3l.md, .md files into JSON AST"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse M3L files and output JSON AST
    Parse {
        /// Input path (file or directory, defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Write output to file instead of stdout
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Analyze model dependencies and output a graph
    Analyze {
        /// Input path (file or directory, defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Output format: mermaid (default) or dot
        #[arg(long, default_value = "mermaid")]
        format: String,
    },

    /// Compare two M3L files and show differences
    Diff {
        /// First input file/directory
        left: PathBuf,

        /// Second input file/directory
        right: PathBuf,
    },

    /// Format M3L files into standardized output
    Format {
        /// Input path (file or directory, defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Lint M3L files for style and quality issues
    Lint {
        /// Input path (file or directory, defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Output format: human (default) or json
        #[arg(long, default_value = "human")]
        format: String,
    },

    /// Validate M3L files and report diagnostics
    Validate {
        /// Input path (file or directory, defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Enable strict style guidelines
        #[arg(long)]
        strict: bool,

        /// Output format: human (default) or json
        #[arg(long, default_value = "human")]
        format: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Parse { path, output } => match run_parse(&path, output.as_deref()) {
            Ok(json) => {
                if output.is_none() {
                    println!("{json}");
                }
            }
            Err(e) => {
                eprintln!("Error: {e}");
                process::exit(1);
            }
        },
        Commands::Analyze { path, format } => {
            match commands::analyze::run_analyze(&path, &format) {
                Ok(output) => {
                    println!("{output}");
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    process::exit(1);
                }
            }
        }
        Commands::Diff { left, right } => match run_diff(&left, &right) {
            Ok(output) => {
                println!("{output}");
            }
            Err(e) => {
                eprintln!("Error: {e}");
                process::exit(1);
            }
        },
        Commands::Format { path } => match commands::format::run_format(&path) {
            Ok(output) => {
                println!("{output}");
            }
            Err(e) => {
                eprintln!("Error: {e}");
                process::exit(1);
            }
        },
        Commands::Lint { path, format } => match commands::lint::run_lint(&path, &format) {
            Ok(output) => {
                println!("{output}");
            }
            Err(e) => {
                eprintln!("Error: {e}");
                process::exit(1);
            }
        },
        Commands::Validate {
            path,
            strict,
            format,
        } => match run_validate(&path, strict, &format) {
            Ok((output, error_count)) => {
                println!("{output}");
                if error_count > 0 {
                    process::exit(1);
                }
            }
            Err(e) => {
                eprintln!("Error: {e}");
                process::exit(1);
            }
        },
    }
}

pub fn build_ast(input_path: &Path) -> Result<m3l_core::M3lAst, String> {
    let files = read_m3l_files(input_path)?;

    if files.is_empty() {
        return Err(format!(
            "No M3L files (.m3l, .m3l.md, .md) found at: {}",
            input_path.display()
        ));
    }

    let parsed_files: Vec<_> = files
        .iter()
        .map(|f| parse_string(&f.content, &f.path))
        .collect();

    // Read project config if input is a directory
    let project_info = if input_path.is_dir() {
        read_project_config(input_path).map(|c| ProjectInfo {
            name: c.name,
            version: c.version,
        })
    } else {
        None
    };

    let ast = resolve(&parsed_files, project_info);

    Ok(ast)
}

fn run_parse(input_path: &Path, output_file: Option<&Path>) -> Result<String, String> {
    let ast = build_ast(input_path)?;
    let json =
        serde_json::to_string_pretty(&ast).map_err(|e| format!("JSON serialization error: {e}"))?;

    if let Some(out_path) = output_file {
        std::fs::write(out_path, &json)
            .map_err(|e| format!("Failed to write {}: {e}", out_path.display()))?;
        return Ok(format!("Written to {}", out_path.display()));
    }

    Ok(json)
}

fn run_diff(left_path: &Path, right_path: &Path) -> Result<String, String> {
    let left_ast = build_ast(left_path)?;
    let right_ast = build_ast(right_path)?;

    let mut lines: Vec<String> = Vec::new();

    // Build name maps
    let left_models: std::collections::HashMap<&str, &m3l_core::ModelNode> = left_ast
        .models
        .iter()
        .chain(left_ast.views.iter())
        .map(|m| (m.name.as_str(), m))
        .collect();
    let right_models: std::collections::HashMap<&str, &m3l_core::ModelNode> = right_ast
        .models
        .iter()
        .chain(right_ast.views.iter())
        .map(|m| (m.name.as_str(), m))
        .collect();

    // Added models
    for name in right_models.keys() {
        if !left_models.contains_key(name) {
            lines.push(format!("+ model {name}"));
        }
    }

    // Removed models
    for name in left_models.keys() {
        if !right_models.contains_key(name) {
            lines.push(format!("- model {name}"));
        }
    }

    // Changed models (field-level diff)
    for (name, left_model) in &left_models {
        if let Some(right_model) = right_models.get(name) {
            let left_fields: std::collections::HashMap<&str, &m3l_core::FieldNode> = left_model
                .fields
                .iter()
                .map(|f| (f.name.as_str(), f))
                .collect();
            let right_fields: std::collections::HashMap<&str, &m3l_core::FieldNode> = right_model
                .fields
                .iter()
                .map(|f| (f.name.as_str(), f))
                .collect();

            for fname in right_fields.keys() {
                if !left_fields.contains_key(fname) {
                    lines.push(format!("+ {name}.{fname}"));
                }
            }
            for fname in left_fields.keys() {
                if !right_fields.contains_key(fname) {
                    lines.push(format!("- {name}.{fname}"));
                }
            }
            for (fname, lf) in &left_fields {
                if let Some(rf) = right_fields.get(fname) {
                    let mut changes = Vec::new();
                    if lf.field_type != rf.field_type {
                        changes.push(format!(
                            "type: {} → {}",
                            lf.field_type.as_deref().unwrap_or("none"),
                            rf.field_type.as_deref().unwrap_or("none")
                        ));
                    }
                    if lf.nullable != rf.nullable {
                        changes.push(format!("nullable: {} → {}", lf.nullable, rf.nullable));
                    }
                    if lf.array != rf.array {
                        changes.push(format!("array: {} → {}", lf.array, rf.array));
                    }
                    if !changes.is_empty() {
                        lines.push(format!("~ {name}.{fname}: {}", changes.join(", ")));
                    }
                }
            }
        }
    }

    // Enum diff
    let left_enums: std::collections::HashMap<&str, &m3l_core::EnumNode> = left_ast
        .enums
        .iter()
        .map(|e| (e.name.as_str(), e))
        .collect();
    let right_enums: std::collections::HashMap<&str, &m3l_core::EnumNode> = right_ast
        .enums
        .iter()
        .map(|e| (e.name.as_str(), e))
        .collect();

    for name in right_enums.keys() {
        if !left_enums.contains_key(name) {
            lines.push(format!("+ enum {name}"));
        }
    }
    for name in left_enums.keys() {
        if !right_enums.contains_key(name) {
            lines.push(format!("- enum {name}"));
        }
    }

    if lines.is_empty() {
        lines.push("No differences found.".into());
    } else {
        lines.sort();
        let add_count = lines.iter().filter(|l| l.starts_with('+')).count();
        let rem_count = lines.iter().filter(|l| l.starts_with('-')).count();
        let mod_count = lines.iter().filter(|l| l.starts_with('~')).count();
        lines.push(format!(
            "\n{} added, {} removed, {} modified",
            add_count, rem_count, mod_count
        ));
    }

    Ok(lines.join("\n"))
}

fn run_validate(input_path: &Path, strict: bool, format: &str) -> Result<(String, usize), String> {
    let files = read_m3l_files(input_path)?;

    if files.is_empty() {
        return Err(format!(
            "No M3L files (.m3l, .m3l.md, .md) found at: {}",
            input_path.display()
        ));
    }

    let parsed_files: Vec<_> = files
        .iter()
        .map(|f| parse_string(&f.content, &f.path))
        .collect();

    let project_info = if input_path.is_dir() {
        read_project_config(input_path).map(|c| ProjectInfo {
            name: c.name,
            version: c.version,
        })
    } else {
        None
    };

    let ast = resolve(&parsed_files, project_info);
    let result = validate(&ast, &ValidateOptions { strict });

    // ValidateResult already includes resolver diagnostics (cloned from AST)
    let error_count = result.errors.len();
    let warning_count = result.warnings.len();
    let file_count = ast.sources.len();

    if format == "json" {
        let diagnostics: Vec<&m3l_core::Diagnostic> =
            result.errors.iter().chain(result.warnings.iter()).collect();
        let output = serde_json::json!({
            "diagnostics": diagnostics,
            "summary": {
                "errors": error_count,
                "warnings": warning_count,
                "files": file_count,
            }
        });
        let json = serde_json::to_string_pretty(&output)
            .map_err(|e| format!("JSON serialization error: {e}"))?;
        return Ok((json, error_count));
    }

    // Human-readable format
    let mut lines: Vec<String> = Vec::new();

    for d in result.errors.iter().chain(result.warnings.iter()) {
        let severity = match d.severity {
            m3l_core::DiagnosticSeverity::Error => "error",
            m3l_core::DiagnosticSeverity::Warning => "warning",
        };
        lines.push(format!(
            "{}:{}:{} {}[{}]: {}",
            d.file, d.line, d.col, severity, d.code, d.message
        ));
    }

    let error_word = if error_count == 1 { "error" } else { "errors" };
    let warning_word = if warning_count == 1 {
        "warning"
    } else {
        "warnings"
    };
    let file_word = if file_count == 1 { "file" } else { "files" };
    lines.push(format!(
        "{error_count} {error_word}, {warning_count} {warning_word} in {file_count} {file_word}."
    ));

    Ok((lines.join("\n"), error_count))
}
