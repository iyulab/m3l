use std::path::Path;

use m3l_lint::{LintConfig, Linter};

use crate::build_ast;

pub fn run_lint(input_path: &Path, format: &str) -> Result<String, String> {
    let ast = build_ast(input_path)?;

    let config = LintConfig::default();
    let linter = Linter::new(config);
    let results = linter.lint(&ast);

    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&serde_json::json!({
                "diagnostics": results,
                "summary": {
                    "count": results.len(),
                    "files": ast.sources.len(),
                }
            }))
            .map_err(|e| format!("JSON serialization error: {e}"))?;
            Ok(json)
        }
        "sarif" => {
            let sarif = build_sarif(&results, &linter);
            serde_json::to_string_pretty(&sarif)
                .map_err(|e| format!("SARIF serialization error: {e}"))
        }
        _ => {
            // Human-readable format
            let mut lines: Vec<String> = Vec::new();

            for d in &results {
                let severity = match d.severity {
                    m3l_lint::LintSeverity::Error => "error",
                    m3l_lint::LintSeverity::Warning => "warning",
                    m3l_lint::LintSeverity::Info => "info",
                };
                lines.push(format!(
                    "{}:{}:{} {}[{}]: {}",
                    d.file, d.line, d.col, severity, d.rule, d.message
                ));
            }

            let count = results.len();
            let file_count = ast.sources.len();
            let issue_word = if count == 1 { "issue" } else { "issues" };
            let file_word = if file_count == 1 { "file" } else { "files" };
            lines.push(format!(
                "{count} lint {issue_word} in {file_count} {file_word}."
            ));

            Ok(lines.join("\n"))
        }
    }
}

fn build_sarif(results: &[m3l_lint::LintDiagnostic], linter: &Linter) -> serde_json::Value {
    let rule_descriptors: Vec<serde_json::Value> = linter
        .rules()
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id(),
                "shortDescription": { "text": r.description() },
                "defaultConfiguration": {
                    "level": match r.default_severity() {
                        m3l_lint::LintSeverity::Error => "error",
                        m3l_lint::LintSeverity::Warning => "warning",
                        m3l_lint::LintSeverity::Info => "note",
                    }
                }
            })
        })
        .collect();

    let sarif_results: Vec<serde_json::Value> = results
        .iter()
        .map(|d| {
            let level = match d.severity {
                m3l_lint::LintSeverity::Error => "error",
                m3l_lint::LintSeverity::Warning => "warning",
                m3l_lint::LintSeverity::Info => "note",
            };
            serde_json::json!({
                "ruleId": d.rule,
                "level": level,
                "message": { "text": d.message },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": { "uri": d.file },
                        "region": {
                            "startLine": d.line,
                            "startColumn": d.col
                        }
                    }
                }]
            })
        })
        .collect();

    serde_json::json!({
        "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/main/sarif-2.1/schema/sarif-schema-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "m3l-lint",
                    "version": env!("CARGO_PKG_VERSION"),
                    "rules": rule_descriptors
                }
            },
            "results": sarif_results
        }]
    })
}
