//! Rule: naming-convention
//!
//! Checks that model names follow PascalCase and field names follow snake_case.

use m3l_core::types::M3lAst;

use crate::{LintDiagnostic, LintRule, LintSeverity};

pub struct NamingConventionRule;

impl LintRule for NamingConventionRule {
    fn id(&self) -> &str {
        "naming-convention"
    }

    fn description(&self) -> &str {
        "Model names should be PascalCase, field names should be snake_case"
    }

    fn default_severity(&self) -> LintSeverity {
        LintSeverity::Warning
    }

    fn check(&self, ast: &M3lAst) -> Vec<LintDiagnostic> {
        let mut diagnostics = Vec::new();

        // Check model names (PascalCase)
        for model in ast.models.iter().chain(ast.views.iter()) {
            if !is_pascal_case(&model.name) {
                diagnostics.push(LintDiagnostic {
                    rule: self.id().into(),
                    severity: self.default_severity(),
                    file: model.source.clone(),
                    line: model.line,
                    col: 1,
                    message: format!("Model name \"{}\" should be PascalCase", model.name),
                });
            }

            // Check field names (snake_case)
            check_field_names(&model.fields, &model.source, self, &mut diagnostics);
        }

        // Check interface names (PascalCase)
        for iface in &ast.interfaces {
            if !is_pascal_case(&iface.name) {
                diagnostics.push(LintDiagnostic {
                    rule: self.id().into(),
                    severity: self.default_severity(),
                    file: iface.source.clone(),
                    line: iface.line,
                    col: 1,
                    message: format!("Interface name \"{}\" should be PascalCase", iface.name),
                });
            }
            check_field_names(&iface.fields, &iface.source, self, &mut diagnostics);
        }

        // Check enum names (PascalCase)
        for e in &ast.enums {
            if !is_pascal_case(&e.name) {
                diagnostics.push(LintDiagnostic {
                    rule: self.id().into(),
                    severity: self.default_severity(),
                    file: e.source.clone(),
                    line: e.line,
                    col: 1,
                    message: format!("Enum name \"{}\" should be PascalCase", e.name),
                });
            }
        }

        diagnostics
    }
}

fn check_field_names(
    fields: &[m3l_core::types::FieldNode],
    source: &str,
    rule: &NamingConventionRule,
    diagnostics: &mut Vec<LintDiagnostic>,
) {
    for field in fields {
        if !is_snake_case(&field.name) {
            diagnostics.push(LintDiagnostic {
                rule: rule.id().into(),
                severity: rule.default_severity(),
                file: source.into(),
                line: field.loc.line,
                col: 1,
                message: format!("Field name \"{}\" should be snake_case", field.name),
            });
        }

        // Recurse into nested fields
        if let Some(ref sub_fields) = field.fields {
            check_field_names(sub_fields, source, rule, diagnostics);
        }
    }
}

/// Check if a name is PascalCase: starts with uppercase, no underscores.
fn is_pascal_case(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let first = name.chars().next().unwrap();
    if !first.is_ascii_uppercase() {
        return false;
    }
    // Allow dots for namespace-qualified names (e.g., Auth.User)
    !name.contains('_')
}

/// Check if a name is snake_case: all lowercase/digits/underscores, doesn't start with uppercase.
fn is_snake_case(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    name.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pascal_case_valid() {
        assert!(is_pascal_case("User"));
        assert!(is_pascal_case("OrderItem"));
        assert!(is_pascal_case("Auth"));
    }

    #[test]
    fn pascal_case_invalid() {
        assert!(!is_pascal_case("user"));
        assert!(!is_pascal_case("order_item"));
        assert!(!is_pascal_case("User_Name"));
    }

    #[test]
    fn snake_case_valid() {
        assert!(is_snake_case("user_name"));
        assert!(is_snake_case("id"));
        assert!(is_snake_case("created_at"));
    }

    #[test]
    fn snake_case_invalid() {
        assert!(!is_snake_case("userName"));
        assert!(!is_snake_case("UserName"));
        assert!(!is_snake_case("ID"));
    }

    #[test]
    fn rule_detects_bad_model_name() {
        let ast = m3l_core::parse_string("## user_model\n- id: identifier", "test.m3l.md");
        let resolved = m3l_core::resolve(&[ast], None);
        let rule = NamingConventionRule;
        let results = rule.check(&resolved);
        assert!(
            results.iter().any(|d| d.message.contains("PascalCase")),
            "Should detect non-PascalCase model name"
        );
    }

    #[test]
    fn rule_detects_bad_field_name() {
        let ast = m3l_core::parse_string("## User\n- UserName: string", "test.m3l.md");
        let resolved = m3l_core::resolve(&[ast], None);
        let rule = NamingConventionRule;
        let results = rule.check(&resolved);
        assert!(
            results.iter().any(|d| d.message.contains("snake_case")),
            "Should detect non-snake_case field name"
        );
    }

    #[test]
    fn rule_no_warnings_for_correct_names() {
        let ast = m3l_core::parse_string(
            "## User\n- user_name: string\n- id: identifier",
            "test.m3l.md",
        );
        let resolved = m3l_core::resolve(&[ast], None);
        let rule = NamingConventionRule;
        let results = rule.check(&resolved);
        assert!(
            results.is_empty(),
            "Should have no warnings for correct naming"
        );
    }
}
