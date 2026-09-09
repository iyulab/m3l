use std::collections::{HashMap, HashSet};

use crate::catalogs::TYPE_CATALOG;
use crate::types::*;

/// Deprecated cascade attribute names (spec §3.2.1.1)
static DEPRECATED_CASCADE_ATTRS: &[&str] = &["cascade", "no_action", "set_null", "restrict"];

/// Validate a resolved M3L AST for semantic errors and style warnings.
pub fn validate(ast: &M3lAst, options: &ValidateOptions) -> ValidateResult {
    let mut errors: Vec<Diagnostic> = ast.errors.clone();
    let mut warnings: Vec<Diagnostic> = ast.warnings.clone();

    let all_models: Vec<&ModelNode> = ast
        .models
        .iter()
        .chain(ast.views.iter())
        .chain(ast.flows.iter())
        .collect();
    let model_map: HashMap<&str, &ModelNode> =
        all_models.iter().map(|m| (m.name.as_str(), *m)).collect();

    // M3L-E001: @rollup FK missing @reference
    for model in &all_models {
        for field in &model.fields {
            if field.kind == FieldKind::Rollup {
                if let Some(ref rollup) = field.rollup {
                    validate_rollup_reference(field, model, rollup, &model_map, &mut errors);
                }
            }
        }
    }

    // M3L-E002: @lookup path FK missing @reference
    for model in &all_models {
        for field in &model.fields {
            if field.kind == FieldKind::Lookup {
                if let Some(ref lookup) = field.lookup {
                    validate_lookup_reference(field, model, lookup, &mut errors);
                }
            }
        }
    }

    // M3L-E004: View source_def.from references undefined model
    for view in &ast.views {
        if let Some(ref sd) = view.source_def {
            if let Some(ref from) = sd.from {
                if !model_map.contains_key(from.as_str()) {
                    errors.push(Diagnostic {
                        code: "M3L-E004".into(),
                        severity: DiagnosticSeverity::Error,
                        file: view.source.clone(),
                        line: view.line,
                        col: 1,
                        message: format!(
                            "View \"{}\" references model \"{}\" which is not defined",
                            view.name, from
                        ),
                    });
                }
            }
        }
    }

    // M3L-E006: Duplicate field names (redundant re-check)
    for model in &all_models {
        let mut seen: HashSet<&str> = HashSet::new();
        for field in &model.fields {
            if seen.contains(field.name.as_str()) {
                let model_type = match &model.model_type {
                    ModelType::Model => "model",
                    ModelType::View => "view",
                    ModelType::Interface => "interface",
                    ModelType::Enum => "enum",
                    ModelType::Flow => "flow",
                    ModelType::Extension(s) => s.as_str(),
                };
                errors.push(Diagnostic {
                    code: "M3L-E006".into(),
                    severity: DiagnosticSeverity::Error,
                    file: field.loc.file.clone(),
                    line: field.loc.line,
                    col: 1,
                    message: format!(
                        "Duplicate field name \"{}\" in {} \"{}\"",
                        field.name, model_type, model.name
                    ),
                });
            }
            seen.insert(&field.name);
        }
    }

    // Build defined names set for E009
    let mut defined_names: HashSet<&str> = HashSet::new();
    for m in &all_models {
        defined_names.insert(&m.name);
    }
    for e in &ast.enums {
        defined_names.insert(&e.name);
    }
    for i in &ast.interfaces {
        defined_names.insert(&i.name);
    }

    // M3L-E009: Undefined type reference
    for model in &all_models {
        validate_field_types(&model.fields, model, &defined_names, &mut errors);
    }

    // M3L-W003: Deprecated syntax warning
    for model in &all_models {
        check_deprecated_syntax(&model.fields, &mut warnings);
    }

    // M3L-E010: Relations entry without @reference
    for model in &all_models {
        validate_relations_references(model, &mut errors);
    }

    // M3L-W009: Relationship notation written in the field list
    for model in &all_models {
        validate_relation_notation_placement(model, &mut warnings);
    }

    // M3L-W005/W006/W007: Attribute registry value validation
    if !ast.attribute_registry.is_empty() {
        let registry_map: HashMap<&str, &AttributeRegistryEntry> = ast
            .attribute_registry
            .iter()
            .map(|r| (r.name.as_str(), r))
            .collect();

        for model in &all_models {
            validate_registry_attrs(&model.fields, model, &registry_map, &mut warnings);
            validate_model_registry_attrs(model, &registry_map, &mut warnings);
        }
    }

    // Strict mode warnings
    if options.strict {
        for model in &all_models {
            for field in &model.fields {
                // M3L-W001: Field line length > 80 chars
                check_field_line_length(field, &mut warnings);

                // M3L-W004: Lookup chain > 3 hops
                if field.kind == FieldKind::Lookup {
                    if let Some(ref lookup) = field.lookup {
                        let hops = lookup.path.split('.').count();
                        if hops > 3 {
                            warnings.push(Diagnostic {
                                code: "M3L-W004".into(),
                                severity: DiagnosticSeverity::Warning,
                                file: field.loc.file.clone(),
                                line: field.loc.line,
                                col: 1,
                                message: format!(
                                    "Lookup chain \"{}\" exceeds 3 hops ({} hops)",
                                    lookup.path, hops
                                ),
                            });
                        }
                    }
                }
            }

            // M3L-W002: Object nesting > 3 levels
            check_nesting_depth(&model.fields, 1, model, &mut warnings);
        }
    }

    ValidateResult { errors, warnings }
}

fn validate_field_types(
    fields: &[FieldNode],
    model: &ModelNode,
    defined_names: &HashSet<&str>,
    errors: &mut Vec<Diagnostic>,
) {
    let model_type = match &model.model_type {
        ModelType::Model => "model",
        ModelType::View => "view",
        ModelType::Interface => "interface",
        ModelType::Enum => "enum",
        ModelType::Flow => "flow",
        ModelType::Extension(s) => s.as_str(),
    };

    for field in fields {
        if let Some(ref type_name) = field.field_type {
            if !type_name.is_empty() && !is_known_type(type_name, defined_names) {
                errors.push(Diagnostic {
                    code: "M3L-E009".into(),
                    severity: DiagnosticSeverity::Error,
                    file: field.loc.file.clone(),
                    line: field.loc.line,
                    col: 1,
                    message: format!(
                        "Undefined type \"{}\" in field \"{}\" of {} \"{}\"",
                        type_name, field.name, model_type, model.name
                    ),
                });
            }
        }

        // Recurse into nested fields
        if let Some(ref sub_fields) = field.fields {
            validate_field_types(sub_fields, model, defined_names, errors);
        }
    }
}

fn check_deprecated_syntax(fields: &[FieldNode], warnings: &mut Vec<Diagnostic>) {
    for field in fields {
        // W003: datetime → timestamp
        if field.field_type.as_deref() == Some("datetime") {
            warnings.push(Diagnostic {
                code: "M3L-W003".into(),
                severity: DiagnosticSeverity::Warning,
                file: field.loc.file.clone(),
                line: field.loc.line,
                col: 1,
                message: format!(
                    "Deprecated type \"datetime\" in field \"{}\" — use \"timestamp\" instead",
                    field.name
                ),
            });
        }

        // W003: deprecated cascade attributes
        for attr in &field.attributes {
            if DEPRECATED_CASCADE_ATTRS.contains(&attr.name.as_str()) {
                warnings.push(Diagnostic {
                    code: "M3L-W003".into(),
                    severity: DiagnosticSeverity::Warning,
                    file: field.loc.file.clone(),
                    line: field.loc.line,
                    col: 1,
                    message: format!(
                        "Deprecated attribute \"@{}\" in field \"{}\" — use @reference symbol suffix (!/?/!!) or extended format instead",
                        attr.name, field.name
                    ),
                });
            }
        }

        // Recurse
        if let Some(ref sub_fields) = field.fields {
            check_deprecated_syntax(sub_fields, warnings);
        }
    }
}

fn is_known_type(type_name: &str, defined_names: &HashSet<&str>) -> bool {
    if TYPE_CATALOG.contains(type_name) || defined_names.contains(type_name) {
        return true;
    }
    // Support qualified references like "Namespace.ModelName"
    if let Some(dot_pos) = type_name.rfind('.') {
        let simple_name = &type_name[dot_pos + 1..];
        if defined_names.contains(simple_name) {
            return true;
        }
    }
    false
}

/// M3L-W009 — 관계 표기(`>x`·`<x`·`<>x`)가 필드 목록에 쓰였다.
///
/// 파서는 그것을 필드로 만들지 않고 관계로 구조화한다(§3.2.4). 다만 명세가 정본 위치로
/// 두는 곳은 `### Relations` 절이므로(§3.2.3), 구조화는 하되 그 사실은 알린다 —
/// 오류가 아니라 경고인 이유는 **읽어 낸 뜻이 명확하고 빌드를 막을 이유가 없기** 때문이다.
fn validate_relation_notation_placement(model: &ModelNode, warnings: &mut Vec<Diagnostic>) {
    for rel in &model.sections.relations {
        if rel.get("declaredIn").and_then(|v| v.as_str()) != Some("fields") {
            continue;
        }
        let raw = rel.get("raw").and_then(|v| v.as_str()).unwrap_or("");
        let (file, line, col) = rel
            .get("loc")
            .map(|l| {
                (
                    l.get("file")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    l.get("line").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
                    l.get("col").and_then(|v| v.as_u64()).unwrap_or(1) as usize,
                )
            })
            .unwrap_or_default();

        warnings.push(Diagnostic {
            code: "M3L-W009".to_string(),
            severity: DiagnosticSeverity::Warning,
            file,
            line,
            col,
            message: format!(
                "Relationship notation \"{}\" in model \"{}\" is written among the fields — it is read as a relationship, not a field, but \"### Relations\" is where it belongs",
                raw, model.name
            ),
        });
    }
}

fn validate_relations_references(model: &ModelNode, errors: &mut Vec<Diagnostic>) {
    for rel in &model.sections.relations {
        // Skip directive-type entries
        if rel.get("type").and_then(|v| v.as_str()) == Some("directive") {
            continue;
        }

        // Only outgoing relations put the foreign key on *this* model, so only
        // they can be checked against its fields. The direction is a parsed fact
        // (specification 3.2.4) -- looking for a '>' in the source line mistook
        // `<>` for `>`, and let a '>' anywhere in a description turn an incoming
        // relation into an outgoing one. Both were reachable, and both reported
        // M3L-E010 against a model that never owned the key.
        //
        // An entry the notation could not classify carries no `direction` at
        // all -- a `### Relations` item written without one of the leading
        // spellings. It is left undiagnosed rather than guessed at: the same
        // stance the parser takes toward what follows a cardinality colon.
        if rel.get("direction").and_then(|v| v.as_str()) != Some("to") {
            continue;
        }

        // `from` is a parsed fact (specification 3.2.4): a nested `- from:` or a
        // trailing `via <field>` on the notation itself, whichever the entry
        // used -- the parser resolves both to this one key.
        let from_field = match rel.get("from").and_then(|v| v.as_str()) {
            Some(f) => f.to_string(),
            None => continue,
        };

        // Find FK field
        let fk_field = model.fields.iter().find(|f| f.name == from_field);
        let fk_field = match fk_field {
            Some(f) => f,
            None => continue,
        };

        let has_reference = fk_field
            .attributes
            .iter()
            .any(|a| a.name == "reference" || a.name == "fk");

        if !has_reference {
            let loc = rel.get("loc");
            let (file, line) = match loc {
                Some(l) => (
                    l.get("file")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&model.source)
                        .to_string(),
                    l.get("line")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(model.line as u64) as usize,
                ),
                None => (model.source.clone(), model.line),
            };

            let model_type = match &model.model_type {
                ModelType::Model => "model",
                ModelType::View => "view",
                ModelType::Interface => "interface",
                ModelType::Enum => "enum",
                ModelType::Flow => "flow",
                ModelType::Extension(s) => s.as_str(),
            };

            errors.push(Diagnostic {
                code: "M3L-E010".into(),
                severity: DiagnosticSeverity::Error,
                file,
                line,
                col: 1,
                message: format!(
                    "Relations entry references FK \"{}\" which has no @reference or @fk attribute in {} \"{}\"",
                    from_field, model_type, model.name
                ),
            });
        }
    }
}

fn validate_rollup_reference(
    field: &FieldNode,
    _model: &ModelNode,
    rollup: &RollupDef,
    model_map: &HashMap<&str, &ModelNode>,
    errors: &mut Vec<Diagnostic>,
) {
    let target_model = match model_map.get(rollup.target.as_str()) {
        Some(m) => m,
        None => return,
    };

    let fk_field = match target_model.fields.iter().find(|f| f.name == rollup.fk) {
        Some(f) => f,
        None => return,
    };

    let has_reference = fk_field
        .attributes
        .iter()
        .any(|a| a.name == "reference" || a.name == "fk");

    if !has_reference {
        errors.push(Diagnostic {
            code: "M3L-E001".into(),
            severity: DiagnosticSeverity::Error,
            file: field.loc.file.clone(),
            line: field.loc.line,
            col: 1,
            message: format!(
                "@rollup on \"{}\" targets \"{}.{}\" which has no @reference or @fk attribute",
                field.name, rollup.target, rollup.fk
            ),
        });
    }
}

fn validate_lookup_reference(
    field: &FieldNode,
    model: &ModelNode,
    lookup: &LookupDef,
    errors: &mut Vec<Diagnostic>,
) {
    let segments: Vec<&str> = lookup.path.split('.').collect();
    if segments.len() < 2 {
        return;
    }

    let fk_field_name = segments[0];
    let fk_field = match model.fields.iter().find(|f| f.name == fk_field_name) {
        Some(f) => f,
        None => return,
    };

    let has_reference = fk_field
        .attributes
        .iter()
        .any(|a| a.name == "reference" || a.name == "fk");

    if !has_reference {
        errors.push(Diagnostic {
            code: "M3L-E002".into(),
            severity: DiagnosticSeverity::Error,
            file: field.loc.file.clone(),
            line: field.loc.line,
            col: 1,
            message: format!(
                "@lookup on \"{}\" references FK \"{}\" which has no @reference or @fk attribute",
                field.name, fk_field_name
            ),
        });
    }
}

fn validate_registry_attrs(
    fields: &[FieldNode],
    model: &ModelNode,
    registry_map: &HashMap<&str, &AttributeRegistryEntry>,
    warnings: &mut Vec<Diagnostic>,
) {
    let model_type = match &model.model_type {
        ModelType::Model => "model",
        ModelType::View => "view",
        ModelType::Interface => "interface",
        ModelType::Enum => "enum",
        ModelType::Flow => "flow",
        ModelType::Extension(s) => s.as_str(),
    };

    for field in fields {
        for attr in &field.attributes {
            if let Some(reg) = registry_map.get(attr.name.as_str()) {
                let subject = format!(
                    "field \"{}\" of {} \"{}\"",
                    field.name, model_type, model.name
                );
                check_registry_attr(attr, reg, "field", &field.loc, &subject, warnings);
            }
        }

        // Recurse into nested fields
        if let Some(ref sub_fields) = field.fields {
            validate_registry_attrs(sub_fields, model, registry_map, warnings);
        }
    }
}

/// M3L-W005/W006/W007 for attributes attached directly to a model header
/// (`## Name @attr`), the model-level counterpart to `validate_registry_attrs`'
/// field-level checks. A registry entry's `target` can name `model` as well as
/// `field` (`m3l-core/src/parser.rs::finalize_attr_def`), so a model-level usage
/// is just as checkable as a field-level one — this was previously unvalidated
/// entirely, not merely missing the target check.
fn validate_model_registry_attrs(
    model: &ModelNode,
    registry_map: &HashMap<&str, &AttributeRegistryEntry>,
    warnings: &mut Vec<Diagnostic>,
) {
    let model_type = match &model.model_type {
        ModelType::Model => "model",
        ModelType::View => "view",
        ModelType::Interface => "interface",
        ModelType::Enum => "enum",
        ModelType::Flow => "flow",
        ModelType::Extension(s) => s.as_str(),
    };

    for attr in &model.attributes {
        if let Some(reg) = registry_map.get(attr.name.as_str()) {
            let subject = format!("{} \"{}\"", model_type, model.name);
            check_registry_attr(attr, reg, "model", &model.loc, &subject, warnings);
        }
    }
}

/// Shared M3L-W005 (type mismatch) / M3L-W006 (range violation) / M3L-W007
/// (used outside its declared `target`) / M3L-W008 (required attribute used
/// without an explicit argument) checks for one attribute usage against its
/// registry entry. `context` is `"field"` or `"model"` — the same vocabulary
/// `AttributeRegistryEntry::target` uses, so a `target: [field]` attribute used
/// on a model header (or vice versa) is caught the same way a type/range
/// mismatch is.
fn check_registry_attr(
    attr: &FieldAttribute,
    reg: &AttributeRegistryEntry,
    context: &str,
    loc: &SourceLocation,
    subject: &str,
    warnings: &mut Vec<Diagnostic>,
) {
    if !reg.target.iter().any(|t| t == context) {
        warnings.push(Diagnostic {
            code: "M3L-W007".into(),
            severity: DiagnosticSeverity::Warning,
            file: loc.file.clone(),
            line: loc.line,
            col: 1,
            message: format!(
                "Attribute \"@{}\" is declared for target [{}] but used on {}",
                attr.name,
                reg.target.join(", "),
                subject
            ),
        });
    }

    // M3L-W008: `required` means an explicit argument is mandatory whenever the
    // attribute is used — `default` is the value assumed for a non-required
    // attribute used bare, not a silent stand-in for a required one (§10.8.7).
    let Some(ref args) = attr.args else {
        if reg.required {
            warnings.push(Diagnostic {
                code: "M3L-W008".into(),
                severity: DiagnosticSeverity::Warning,
                file: loc.file.clone(),
                line: loc.line,
                col: 1,
                message: format!(
                    "Attribute \"@{}\" is required but used without an explicit argument on {}",
                    attr.name, subject
                ),
            });
        }
        return;
    };

    for arg in args {
        let arg_kind = match arg {
            AttrArgValue::String(_) => "string",
            AttrArgValue::Number(_) => "number",
            AttrArgValue::Bool(_) => "boolean",
        };
        if reg.attr_type != arg_kind
            && matches!(reg.attr_type.as_str(), "string" | "number" | "boolean")
        {
            warnings.push(Diagnostic {
                code: "M3L-W005".into(),
                severity: DiagnosticSeverity::Warning,
                file: loc.file.clone(),
                line: loc.line,
                col: 1,
                message: format!(
                    "Attribute \"@{}\" expects {} argument but got {} on {}",
                    attr.name, reg.attr_type, arg_kind, subject
                ),
            });
        }

        // Range check for number types
        if let Some((min, max)) = reg.range {
            if let AttrArgValue::Number(n) = arg {
                if *n < min || *n > max {
                    warnings.push(Diagnostic {
                        code: "M3L-W006".into(),
                        severity: DiagnosticSeverity::Warning,
                        file: loc.file.clone(),
                        line: loc.line,
                        col: 1,
                        message: format!(
                            "Attribute \"@{}\" argument {} is outside range [{}, {}] on {}",
                            attr.name, n, min, max, subject
                        ),
                    });
                }
            }
        }
    }
}

fn check_field_line_length(field: &FieldNode, warnings: &mut Vec<Diagnostic>) {
    let mut len = 2 + field.name.len();
    if let Some(ref label) = field.label {
        len += label.len() + 2;
    }
    if let Some(ref t) = field.field_type {
        len += 2 + t.len();
    }
    if field.nullable {
        len += 1;
    }
    if let Some(ref dv) = field.default_value {
        len += 3 + dv.len();
    }
    for attr in &field.attributes {
        len += 2 + attr.name.len();
    }
    if let Some(ref desc) = field.description {
        len += 3 + desc.len();
    }

    if len > 80 {
        warnings.push(Diagnostic {
            code: "M3L-W001".into(),
            severity: DiagnosticSeverity::Warning,
            file: field.loc.file.clone(),
            line: field.loc.line,
            col: 1,
            message: format!(
                "Field \"{}\" line length (~{} chars) exceeds 80 character guideline",
                field.name, len
            ),
        });
    }
}

fn check_nesting_depth(
    fields: &[FieldNode],
    depth: usize,
    model: &ModelNode,
    warnings: &mut Vec<Diagnostic>,
) {
    for field in fields {
        if let Some(ref sub_fields) = field.fields {
            if !sub_fields.is_empty() {
                if depth >= 3 {
                    warnings.push(Diagnostic {
                        code: "M3L-W002".into(),
                        severity: DiagnosticSeverity::Warning,
                        file: field.loc.file.clone(),
                        line: field.loc.line,
                        col: 1,
                        message: format!(
                            "Object nesting depth exceeds 3 levels at field \"{}\" in \"{}\"",
                            field.name, model.name
                        ),
                    });
                }
                check_nesting_depth(sub_fields, depth + 1, model, warnings);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_string;
    use crate::resolver;

    fn parse_and_validate(input: &str) -> ValidateResult {
        let parsed = parse_string(input, "test.m3l.md");
        let ast = resolver::resolve(&[parsed], None);
        validate(&ast, &ValidateOptions::default())
    }

    #[test]
    fn validate_clean() {
        let result = parse_and_validate("## User\n- id: identifier @pk\n- name: string");
        assert!(result.errors.is_empty());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn validate_e009_undefined_type() {
        let result = parse_and_validate("## User\n- id: UnknownType");
        assert!(result.errors.iter().any(|e| e.code == "M3L-E009"));
    }

    #[test]
    fn validate_e009_defined_model_ref() {
        let result = parse_and_validate("## Address\n- city: string\n\n## User\n- addr: Address");
        assert!(
            !result.errors.iter().any(|e| e.code == "M3L-E009"),
            "Should not report E009 for a defined model reference"
        );
    }

    #[test]
    fn validate_w003_datetime() {
        let result = parse_and_validate("## User\n- ts: datetime");
        assert!(result.warnings.iter().any(|w| w.code == "M3L-W003"));
    }

    #[test]
    fn validate_w003_deprecated_attr() {
        let result = parse_and_validate("## User\n- ref_id: identifier @reference(Other) @cascade");
        assert!(result
            .warnings
            .iter()
            .any(|w| w.code == "M3L-W003" && w.message.contains("cascade")));
    }

    #[test]
    fn validate_strict_w004_long_lookup() {
        let input =
            "## A\n- fk: identifier @reference(B)\n### Lookup\n- x: string @lookup(fk.B.C.D.name)";
        let parsed = parse_string(input, "test.m3l.md");
        let ast = resolver::resolve(&[parsed], None);
        let result = validate(&ast, &ValidateOptions { strict: true });
        assert!(result.warnings.iter().any(|w| w.code == "M3L-W004"));
    }

    #[test]
    fn validate_w005_attr_type_mismatch() {
        // Register a number-type attribute via ::attribute syntax, then use with string arg
        let input = "## rating ::attribute\n- type: number\n- target: field\n\n## Product\n- score: integer @rating(high)";
        let result = parse_and_validate(input);
        assert!(
            result.warnings.iter().any(|w| w.code == "M3L-W005"),
            "Should warn about type mismatch: expected number got string"
        );
    }

    #[test]
    fn validate_w006_attr_range_violation() {
        let input = "## priority ::attribute\n- type: number\n- range: 1..10\n- target: field\n\n## Task\n- level: integer @priority(15)";
        let result = parse_and_validate(input);
        assert!(
            result.warnings.iter().any(|w| w.code == "M3L-W006"),
            "Should warn about value outside range [1, 10]"
        );
    }

    #[test]
    fn validate_no_w005_correct_type() {
        let input = "## priority ::attribute\n- type: number\n- target: field\n\n## Task\n- level: integer @priority(5)";
        let result = parse_and_validate(input);
        assert!(
            !result.warnings.iter().any(|w| w.code == "M3L-W005"),
            "Should not warn when type matches"
        );
    }

    #[test]
    fn validate_no_w006_in_range() {
        let input = "## priority ::attribute\n- type: number\n- range: 1..10\n- target: field\n\n## Task\n- level: integer @priority(5)";
        let result = parse_and_validate(input);
        assert!(
            !result.warnings.iter().any(|w| w.code == "M3L-W006"),
            "Should not warn when value is in range"
        );
    }

    #[test]
    fn validate_w007_field_only_attr_used_on_model() {
        let input = "## ledger ::attribute\n- type: boolean\n- target: field\n\n## StockMove @ledger\n- id: identifier";
        let result = parse_and_validate(input);
        assert!(
            result.warnings.iter().any(|w| w.code == "M3L-W007"),
            "Should warn when a field-only attribute is used on a model header"
        );
    }

    #[test]
    fn validate_w007_model_only_attr_used_on_field() {
        let input = "## audited ::attribute\n- type: boolean\n- target: model\n\n## Task\n- flag: boolean @audited";
        let result = parse_and_validate(input);
        assert!(
            result.warnings.iter().any(|w| w.code == "M3L-W007"),
            "Should warn when a model-only attribute is used on a field"
        );
    }

    #[test]
    fn validate_no_w007_when_target_matches() {
        let input = "## custom_flag ::attribute\n- type: boolean\n- target: [field, model]\n\n## Product @custom_flag\n- active: boolean @custom_flag";
        let result = parse_and_validate(input);
        assert!(
            !result.warnings.iter().any(|w| w.code == "M3L-W007"),
            "Should not warn when the usage target is declared by the registry entry"
        );
    }

    #[test]
    fn validate_w005_model_level_type_mismatch() {
        // Registry attribute usages on a model header (`## Name @attr`) previously went
        // entirely unchecked — only field-level usages were validated.
        let input = "## rating ::attribute\n- type: number\n- target: model\n\n## Product @rating(high)\n- id: identifier";
        let result = parse_and_validate(input);
        assert!(
            result.warnings.iter().any(|w| w.code == "M3L-W005"),
            "Should warn about type mismatch on a model-level attribute usage"
        );
    }

    #[test]
    fn validate_w006_model_level_range_violation() {
        let input = "## priority ::attribute\n- type: number\n- range: 1..10\n- target: model\n\n## Task @priority(15)\n- id: identifier";
        let result = parse_and_validate(input);
        assert!(
            result.warnings.iter().any(|w| w.code == "M3L-W006"),
            "Should warn about a range violation on a model-level attribute usage"
        );
    }

    #[test]
    fn validate_w005_boolean_type_mismatch() {
        // Only ("number", String) and ("string", Number) were checked before —
        // a "boolean"-typed registry entry with a non-boolean argument went unchecked.
        let input = "## flag ::attribute\n- type: boolean\n- target: field\n\n## Task\n- x: integer @flag(5)";
        let result = parse_and_validate(input);
        assert!(
            result.warnings.iter().any(|w| w.code == "M3L-W005"),
            "Should warn when a boolean-typed attribute gets a non-boolean argument"
        );
    }

    #[test]
    fn validate_no_w005_boolean_correct_type() {
        let input = "## flag ::attribute\n- type: boolean\n- target: field\n\n## Task\n- x: integer @flag(true)";
        let result = parse_and_validate(input);
        assert!(
            !result.warnings.iter().any(|w| w.code == "M3L-W005"),
            "Should not warn when a boolean-typed attribute gets a boolean argument"
        );
    }

    #[test]
    fn validate_w008_required_attr_used_without_argument() {
        let input = "## priority ::attribute\n- type: number\n- required: true\n- target: field\n\n## Task\n- level: integer @priority";
        let result = parse_and_validate(input);
        assert!(
            result.warnings.iter().any(|w| w.code == "M3L-W008"),
            "Should warn when a required attribute is used bare (no explicit argument)"
        );
    }

    #[test]
    fn validate_w008_required_attr_used_without_argument_on_model() {
        let input = "## audited ::attribute\n- type: boolean\n- required: true\n- target: model\n\n## Task @audited\n- id: identifier";
        let result = parse_and_validate(input);
        assert!(
            result.warnings.iter().any(|w| w.code == "M3L-W008"),
            "Should warn when a required attribute is used bare on a model header"
        );
    }

    #[test]
    fn validate_no_w008_when_argument_given() {
        let input = "## priority ::attribute\n- type: number\n- required: true\n- target: field\n\n## Task\n- level: integer @priority(5)";
        let result = parse_and_validate(input);
        assert!(
            !result.warnings.iter().any(|w| w.code == "M3L-W008"),
            "Should not warn when a required attribute is used with an explicit argument"
        );
    }

    #[test]
    fn validate_no_w008_when_not_required() {
        let input = "## priority ::attribute\n- type: number\n- target: field\n\n## Task\n- level: integer @priority";
        let result = parse_and_validate(input);
        assert!(
            !result.warnings.iter().any(|w| w.code == "M3L-W008"),
            "Should not warn when a non-required attribute is used bare — default_value may apply"
        );
    }

    #[test]
    fn validate_w007_and_w008_fire_together_on_same_misuse() {
        // A required, field-only attribute used bare on a model header hits both checks:
        // wrong target (W007) and missing argument (W008) — the two are independent, not
        // mutually exclusive.
        let input = "## ledger ::attribute\n- type: boolean\n- required: true\n- target: field\n\n## StockMove @ledger\n- id: identifier";
        let result = parse_and_validate(input);
        assert!(
            result.warnings.iter().any(|w| w.code == "M3L-W007"),
            "Should still warn about the target mismatch"
        );
        assert!(
            result.warnings.iter().any(|w| w.code == "M3L-W008"),
            "Should also warn about the missing required argument"
        );
    }

    #[test]
    fn validate_qualified_namespace_ref() {
        let input = "## User\n- id: identifier\n\n## Order\n- buyer: Auth.User";
        let result = parse_and_validate(input);
        assert!(
            !result.errors.iter().any(|e| e.code == "M3L-E009"),
            "Should not report E009 for qualified namespace reference Auth.User when User is defined"
        );
    }

    #[test]
    fn validate_qualified_ref_unknown() {
        let input = "## Order\n- buyer: Auth.Customer";
        let result = parse_and_validate(input);
        assert!(
            result.errors.iter().any(|e| e.code == "M3L-E009"),
            "Should report E009 for Auth.Customer when Customer is not defined"
        );
    }

    #[test]
    fn validate_no_false_positives() {
        let input = "## Status ::enum\n- Active\n- Inactive\n\n## User\n- id: identifier @pk\n- status: Status";
        let result = parse_and_validate(input);
        assert!(result.errors.is_empty());
    }
}
