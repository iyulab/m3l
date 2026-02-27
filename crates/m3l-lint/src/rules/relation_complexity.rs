//! Rule: relation-complexity
//!
//! Warns when a model has too many reference/FK relationships (default threshold: 5).

use m3l_core::types::M3lAst;

use crate::{LintDiagnostic, LintRule, LintSeverity};

const DEFAULT_MAX_RELATIONS: usize = 5;

pub struct RelationComplexityRule {
    pub max_relations: usize,
}

impl Default for RelationComplexityRule {
    fn default() -> Self {
        Self {
            max_relations: DEFAULT_MAX_RELATIONS,
        }
    }
}

impl LintRule for RelationComplexityRule {
    fn id(&self) -> &str {
        "relation-complexity"
    }

    fn description(&self) -> &str {
        "Models should not have too many outgoing references"
    }

    fn default_severity(&self) -> LintSeverity {
        LintSeverity::Warning
    }

    fn check(&self, ast: &M3lAst) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();

        for model in ast.models.iter().chain(ast.views.iter()) {
            let ref_count = model
                .fields
                .iter()
                .filter(|f| {
                    f.attributes
                        .iter()
                        .any(|a| a.name == "reference" || a.name == "fk")
                })
                .count();

            if ref_count > self.max_relations {
                diagnostics.push(LintDiagnostic {
                    rule: self.id().into(),
                    severity: self.default_severity(),
                    file: model.source.clone(),
                    line: model.line,
                    col: 1,
                    message: format!(
                        "Model \"{}\" has {} reference fields (max {}). Consider decomposing",
                        model.name, ref_count, self.max_relations
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

    #[test]
    fn detects_many_relations() {
        let mut input = "## Complex\n".to_string();
        for i in 0..6 {
            input.push_str(&format!(
                "- ref_{}: identifier @reference(Target{})\n",
                i, i
            ));
        }
        let parsed = m3l_core::parse_string(&input, "test.m3l.md");
        let ast = m3l_core::resolve(&[parsed], None);
        let rule = RelationComplexityRule::default();
        let results = rule.check(&ast);
        assert_eq!(results.len(), 1);
        assert!(results[0].message.contains("6 reference fields"));
    }

    #[test]
    fn no_warning_few_relations() {
        let input = "## User\n- org_id: identifier @reference(Org)\n- name: string";
        let parsed = m3l_core::parse_string(input, "test.m3l.md");
        let ast = m3l_core::resolve(&[parsed], None);
        let rule = RelationComplexityRule::default();
        let results = rule.check(&ast);
        assert!(results.is_empty());
    }
}
