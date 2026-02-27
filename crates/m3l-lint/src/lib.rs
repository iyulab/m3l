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
}
