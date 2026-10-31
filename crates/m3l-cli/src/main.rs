mod commands;
mod reader;

use std::path::{Path, PathBuf};
use std::process;

use clap::{Parser, Subcommand};

use m3l_core::{parse_string, validate, ProjectInfo, ValidateOptions};
use reader::read_project;

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

    /// Compare two M3L files or directories and show what differs: exits 0 when they are the same, 1 when they
    /// differ, 2 when they cannot be compared
    Diff {
        /// First input file/directory
        left: PathBuf,

        /// Second input file/directory
        right: PathBuf,

        /// Output format: human (default) or json — a versioned document of structured changes (see the README)
        #[arg(long, default_value = "human")]
        format: String,
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
        // Like diff(1): 0 = the two sides are the same, 1 = they differ, 2 = the comparison could not be made.
        Commands::Diff {
            left,
            right,
            format,
        } => {
            if format != "human" && format != "json" {
                eprintln!("Error: unknown diff format '{format}' (expected human or json)");
                process::exit(2);
            }
            match commands::diff::run_diff(&left, &right) {
                Ok(report) => {
                    if format == "json" {
                        match serde_json::to_string_pretty(&report.json()) {
                            Ok(json) => println!("{json}"),
                            Err(e) => {
                                eprintln!("Error: JSON serialization error: {e}");
                                process::exit(2);
                            }
                        }
                    } else {
                        println!("{}", report.text);
                    }
                    if report.differs {
                        process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    process::exit(2);
                }
            }
        }
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
    build_ast_with(input_path, m3l_core::ResolveOptions::default())
}

pub fn build_ast_with(
    input_path: &Path,
    mut options: m3l_core::ResolveOptions,
) -> Result<m3l_core::M3lAst, String> {
    let input = read_project(input_path)?;
    for warning in &input.warnings {
        eprintln!("Warning: {warning}");
    }
    let files = input.files;

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

    let project_info = input.config.map(|c| ProjectInfo {
        name: c.name,
        version: c.version,
    });

    options.project_root = Some(input.root);
    // The files keep the paths they were given (the AST is portable); the root is absolute, so the resolver is told
    // which directory those paths are written from and compares them all in one frame.
    options.base_dir = std::env::current_dir()
        .ok()
        .map(|dir| dir.to_string_lossy().replace('\\', "/"));
    // A directory (or its m3l.config.yaml) is the whole set its imports may name — unless the project root lies above
    // it, then it is a part of the project; a single file is read on its own.
    options.complete_file_set = input_path.is_dir() && input.root_is_read_dir;
    let ast = m3l_core::resolve_with(&parsed_files, project_info, options);

    Ok(ast)
}

fn run_parse(input_path: &Path, output_file: Option<&Path>) -> Result<String, String> {
    let ast = build_ast(input_path)?;
    let json =
        serde_json::to_string_pretty(&ast).map_err(|e| format!("JSON serialization error: {e}"))?;

    if let Some(out_path) = output_file {
        std::fs::write(out_path, &json).map_err(|e| describe_write_error(out_path, &e))?;
        return Ok(format!("Written to {}", out_path.display()));
    }

    Ok(json)
}

/// A failed write in words: the missing directory or the permission, instead of the OS text alone.
fn describe_write_error(out_path: &Path, e: &std::io::Error) -> String {
    match e.kind() {
        std::io::ErrorKind::NotFound => {
            match out_path.parent().filter(|p| !p.as_os_str().is_empty()) {
                Some(dir) if !dir.exists() => format!(
                    "Failed to write {}: the directory {} does not exist — create it first",
                    out_path.display(),
                    dir.display()
                ),
                _ => format!("Failed to write {}: {e}", out_path.display()),
            }
        }
        std::io::ErrorKind::PermissionDenied => {
            format!("Failed to write {}: permission denied", out_path.display())
        }
        _ => format!("Failed to write {}: {e}", out_path.display()),
    }
}

fn run_validate(input_path: &Path, strict: bool, format: &str) -> Result<(String, usize), String> {
    // The same AST `parse` prints — the two commands used to build it separately, and the copy
    // here is where a resolver input added to one of them would have gone missing.
    let ast = build_ast(input_path)?;
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
