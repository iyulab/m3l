//! Rule: similar-fields
//!
//! Detects fields with very similar names within the same model,
//! which may indicate naming inconsistencies or accidental duplication.

use m3l_core::types::M3lAst;

use crate::{LintDiagnostic, LintRule, LintSeverity};

pub struct SimilarFieldsRule;

impl LintRule for SimilarFieldsRule {
    fn id(&self) -> &str {
        "similar-fields"
    }

    fn description(&self) -> &str {
        "Detects fields with very similar names that may be confusing"
    }

    fn default_severity(&self) -> LintSeverity {
        LintSeverity::Info
    }

    fn check(&self, ast: &M3lAst) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();

        for model in ast.models.iter().chain(ast.views.iter()) {
            let names: Vec<&str> = model.fields.iter().map(|f| f.name.as_str()).collect();

            for i in 0..names.len() {
                for j in (i + 1)..names.len() {
                    if are_similar(names[i], names[j]) {
                        diagnostics.push(LintDiagnostic {
                            rule: self.id().into(),
                            severity: self.default_severity(),
                            file: model.source.clone(),
                            line: model.fields[j].loc.line,
                            col: 1,
                            message: format!(
                                "Field \"{}\" is very similar to \"{}\" in model \"{}\"",
                                names[j], names[i], model.name
                            ),
                        });
                    }
                }
            }
        }

        diagnostics
    }
}

/// Check if two field names are "similar" — differ only by a common suffix pattern.
fn are_similar(a: &str, b: &str) -> bool {
    if a == b {
        return false; // exact duplicates are caught by E006
    }

    let na = normalize(a);
    let nb = normalize(b);

    if na == nb {
        return true;
    }

    // Check if one is a prefix/suffix variation of the other
    // e.g., "user_id" vs "userid", "created_date" vs "create_date"
    let la = a.replace('_', "").to_lowercase();
    let lb = b.replace('_', "").to_lowercase();
    la == lb
}

/// Normalize a field name by removing common suffixes for comparison.
fn normalize(name: &str) -> String {
    let lower = name.to_lowercase();
    let suffixes = ["_id", "_at", "_on", "_by", "_name", "_date", "_time"];
    for suffix in &suffixes {
        if let Some(stripped) = lower.strip_suffix(suffix) {
            if !stripped.is_empty() {
                return stripped.to_string();
            }
        }
    }
    lower
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn similar_names_detected() {
        assert!(are_similar("user_name", "username"));
        assert!(are_similar("created_at", "createdat"));
    }

    #[test]
    fn different_names_not_similar() {
        assert!(!are_similar("user_name", "email"));
        assert!(!are_similar("id", "name"));
    }

    #[test]
    fn rule_detects_similar_fields() {
        let input = "## User\n- user_name: string\n- username: string";
        let parsed = m3l_core::parse_string(input, "test.m3l.md");
        let ast = m3l_core::resolve(&[parsed], None);
        let rule = SimilarFieldsRule;
        let results = rule.check(&ast);
        assert!(!results.is_empty(), "Should detect similar field names");
    }

    #[test]
    fn rule_no_warning_distinct_fields() {
        let input = "## User\n- name: string\n- email: string\n- age: integer";
        let parsed = m3l_core::parse_string(input, "test.m3l.md");
        let ast = m3l_core::resolve(&[parsed], None);
        let rule = SimilarFieldsRule;
        let results = rule.check(&ast);
        assert!(results.is_empty());
    }
}
