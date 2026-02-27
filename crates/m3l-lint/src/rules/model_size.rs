//! Rule: model-size
//!
//! Warns when a model has too many fields (default threshold: 20).

use m3l_core::types::M3lAst;

use crate::{LintDiagnostic, LintRule, LintSeverity};

const DEFAULT_MAX_FIELDS: usize = 20;

pub struct ModelSizeRule {
    pub max_fields: usize,
}

impl Default for ModelSizeRule {
    fn default() -> Self {
        Self {
            max_fields: DEFAULT_MAX_FIELDS,
        }
    }
}

impl LintRule for ModelSizeRule {
    fn id(&self) -> &str {
        "model-size"
    }

    fn description(&self) -> &str {
        "Models should not have too many fields"
    }

    fn default_severity(&self) -> LintSeverity {
        LintSeverity::Warning
    }

    fn check(&self, ast: &M3lAst) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();

        for model in ast.models.iter().chain(ast.views.iter()) {
            if model.fields.len() > self.max_fields {
                diagnostics.push(LintDiagnostic {
                    rule: self.id().into(),
                    severity: self.default_severity(),
                    file: model.source.clone(),
                    line: model.line,
                    col: 1,
                    message: format!(
                        "Model \"{}\" has {} fields (max {}). Consider splitting into smaller models",
                        model.name,
                        model.fields.len(),
                        self.max_fields
                    ),
                });
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_large_model(field_count: usize) -> String {
        let mut s = "## LargeModel\n".to_string();
        for i in 0..field_count {
            s.push_str(&format!("- field_{}: string\n", i));
        }
        s
    }

    #[test]
    fn detects_large_model() {
        let input = make_large_model(25);
        let parsed = m3l_core::parse_string(&input, "test.m3l.md");
        let ast = m3l_core::resolve(&[parsed], None);
        let rule = ModelSizeRule::default();
        let results = rule.check(&ast);
        assert_eq!(results.len(), 1);
        assert!(results[0].message.contains("25 fields"));
    }

    #[test]
    fn no_warning_for_small_model() {
        let input = "## User\n- id: identifier\n- name: string";
        let parsed = m3l_core::parse_string(input, "test.m3l.md");
        let ast = m3l_core::resolve(&[parsed], None);
        let rule = ModelSizeRule::default();
        let results = rule.check(&ast);
        assert!(results.is_empty());
    }

    #[test]
    fn custom_threshold() {
        let input = make_large_model(5);
        let parsed = m3l_core::parse_string(&input, "test.m3l.md");
        let ast = m3l_core::resolve(&[parsed], None);
        let rule = ModelSizeRule { max_fields: 3 };
        let results = rule.check(&ast);
        assert_eq!(results.len(), 1);
    }
}
