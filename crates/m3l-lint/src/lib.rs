//! M3L schema linter — configurable rules for schema quality.
//!
//! Provides a trait-based rule framework for analyzing M3L ASTs
//! and reporting lint diagnostics.

mod rules;

use m3l_core::types::M3lAst;
pub use rules::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Lint severity (separate from parser diagnostics)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LintSeverity {
    Error,
    Warning,
    Info,
}

// ---------------------------------------------------------------------------
// Lint diagnostic
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LintDiagnostic {
    pub rule: String,
    pub severity: LintSeverity,
    pub file: String,
    pub line: usize,
    pub col: usize,
    pub message: String,
}

// ---------------------------------------------------------------------------
// Lint rule trait
// ---------------------------------------------------------------------------

/// Trait that all lint rules must implement.
pub trait LintRule: Send + Sync {
    /// Unique rule identifier (e.g., "naming-convention").
    fn id(&self) -> &str;

    /// Human-readable description.
    fn description(&self) -> &str;

    /// Default severity.
    fn default_severity(&self) -> LintSeverity;

    /// Run the rule against an AST and return diagnostics.
    fn check(&self, ast: &M3lAst) -> Vec<LintDiagnostic>;
}

// ---------------------------------------------------------------------------
// Lint configuration
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleLevel {
    Off,
    #[default]
    Warn,
    Error,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LintConfig {
    #[serde(default)]
    pub rules: HashMap<String, RuleLevel>,
}

impl LintConfig {
    /// Check if a rule is enabled (not off).
    pub fn is_enabled(&self, rule_id: &str) -> bool {
        !matches!(self.rules.get(rule_id), Some(RuleLevel::Off))
    }

    /// Get the configured severity for a rule, or its default.
    pub fn severity_for(&self, rule: &dyn LintRule) -> LintSeverity {
        match self.rules.get(rule.id()) {
            Some(RuleLevel::Error) => LintSeverity::Error,
            Some(RuleLevel::Warn) => LintSeverity::Warning,
            Some(RuleLevel::Off) => LintSeverity::Warning,
            None => rule.default_severity(),
        }
    }
}

// ---------------------------------------------------------------------------
// Linter engine
// ---------------------------------------------------------------------------

pub struct Linter {
    rules: Vec<Box<dyn LintRule>>,
    config: LintConfig,
}

impl Linter {
    /// Create a new linter with all built-in rules.
    pub fn new(config: LintConfig) -> Self {
        Self {
            rules: builtin_rules(),
            config,
        }
    }

    /// Get a reference to the registered rules.
    pub fn rules(&self) -> &[Box<dyn LintRule>] {
        &self.rules
    }

    /// Run all enabled rules against the AST.
    pub fn lint(&self, ast: &M3lAst) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();

        for rule in &self.rules {
            if self.config.is_enabled(rule.id()) {
                let severity = self.config.severity_for(rule.as_ref());
                let mut results = rule.check(ast);
                for d in &mut results {
                    d.severity = severity.clone();
                }
                diagnostics.extend(results);
            }
        }

        diagnostics
    }
}

impl Default for Linter {
    fn default() -> Self {
        Self::new(LintConfig::default())
    }
}

/// Return all built-in lint rules.
fn builtin_rules() -> Vec<Box<dyn LintRule>> {
    vec![
        Box::new(NamingConventionRule),
        Box::new(ModelSizeRule::default()),
        Box::new(SimilarFieldsRule),
        Box::new(RelationComplexityRule::default()),
    ]
}

// ---------------------------------------------------------------------------
// FFI helper — full pipeline: parse → resolve → lint → JSON
// ---------------------------------------------------------------------------

/// Lint M3L content and return results as JSON.
///
/// Input: M3L markdown text + optional config JSON
/// Output: JSON string with `{ success: bool, data?: LintResult, error?: string }`
///
/// The `config_json` parameter accepts a JSON object matching [`LintConfig`].
/// If empty or `"{}"`, default configuration is used.
pub fn lint_to_json(content: &str, config_json: &str) -> String {
    let config: LintConfig = if config_json.is_empty() || config_json == "{}" {
        LintConfig::default()
    } else {
        match serde_json::from_str(config_json) {
            Ok(c) => c,
            Err(e) => {
                return serde_json::to_string(&FfiLintResult {
                    success: false,
                    data: None,
                    error: Some(format!("Invalid config JSON: {e}")),
                })
                .unwrap();
            }
        }
    };

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let parsed = m3l_core::parse_string(content, "input.m3l.md");
        let ast = m3l_core::resolve(&[parsed], None);
        let linter = Linter::new(config);
        let diagnostics = linter.lint(&ast);
        LintResultData {
            diagnostics,
            file_count: ast.sources.len(),
        }
    }));

    match result {
        Ok(data) => {
            let ffi_result = FfiLintResult {
                success: true,
                data: Some(data),
                error: None,
            };
            serde_json::to_string(&ffi_result).unwrap_or_else(|e| {
                serde_json::to_string(&FfiLintResult {
                    success: false,
                    data: None,
                    error: Some(format!("JSON serialization error: {e}")),
                })
                .unwrap()
            })
        }
        Err(_) => serde_json::to_string(&FfiLintResult {
            success: false,
            data: None,
            error: Some("Internal linter panic".to_string()),
        })
        .unwrap(),
    }
}

#[derive(Debug, Serialize)]
struct FfiLintResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<LintResultData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LintResultData {
    pub diagnostics: Vec<LintDiagnostic>,
    pub file_count: usize,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linter_default_no_rules() {
        let linter = Linter::default();
        let ast = M3lAst {
            parser_version: "0.4.0".into(),
            ast_version: "1.0".into(),
            project: m3l_core::types::ProjectInfo {
                name: None,
                version: None,
            },
            sources: vec![],
            models: vec![],
            enums: vec![],
            interfaces: vec![],
            views: vec![],
            attribute_registry: vec![],
            errors: vec![],
            warnings: vec![],
        };
        let result = linter.lint(&ast);
        assert!(result.is_empty());
    }

    #[test]
    fn config_rule_off() {
        let mut config = LintConfig::default();
        config.rules.insert("test-rule".into(), RuleLevel::Off);
        assert!(!config.is_enabled("test-rule"));
        assert!(config.is_enabled("other-rule"));
    }

    #[test]
    fn config_severity_override() {
        struct TestRule;
        impl LintRule for TestRule {
            fn id(&self) -> &str {
                "test-rule"
            }
            fn description(&self) -> &str {
                "test"
            }
            fn default_severity(&self) -> LintSeverity {
                LintSeverity::Warning
            }
            fn check(&self, _ast: &M3lAst) -> Vec<LintDiagnostic> {
                vec![]
            }
        }

        let mut config = LintConfig::default();
        config.rules.insert("test-rule".into(), RuleLevel::Error);
        assert_eq!(config.severity_for(&TestRule), LintSeverity::Error);
    }

    #[test]
    fn lint_to_json_empty_content() {
        let result = lint_to_json("", "{}");
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["success"], true);
        assert!(parsed["data"]["diagnostics"].is_array());
    }

    #[test]
    fn lint_to_json_default_config() {
        let content = "# TestModel\n\n- name: string\n";
        let result = lint_to_json(content, "");
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["success"], true);
        assert!(parsed["data"]["diagnostics"].is_array());
        assert!(parsed["data"]["file_count"].is_number());
    }

    #[test]
    fn lint_to_json_invalid_config() {
        let result = lint_to_json("# Model\n", "not-json");
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["success"], false);
        assert!(parsed["error"]
            .as_str()
            .unwrap()
            .contains("Invalid config JSON"));
    }

    #[test]
    fn lint_to_json_with_custom_config() {
        let content = "# test_model\n\n- Name: string\n";
        let config = r#"{"rules":{"naming-convention":"off"}}"#;
        let result = lint_to_json(content, config);
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["success"], true);
        // naming-convention is off, so no naming diagnostics
        let diagnostics = parsed["data"]["diagnostics"].as_array().unwrap();
        for d in diagnostics {
            assert_ne!(d["rule"].as_str().unwrap(), "naming-convention");
        }
    }
}
