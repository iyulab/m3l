//! Rename declarations — `@renamed_from(...)` (specification §2.2.6, §2.3.5).
//!
//! Five rules, each its own error code (§10.5.1):
//!
//! - `M3L-E028` a former name is a current name — the old name cannot be both gone and present;
//! - `M3L-E029` two elements claim the same former name — two elements cannot both have been it;
//! - `M3L-E030` one declaration lists a name twice, or lists the element's own name;
//! - `M3L-E031` the declaration sits where nothing is stored (a view or flow, its fields, a derived
//!   field without storage, an enum value);
//! - `M3L-E032` an argument that is not a name — empty, qualified, or not an identifier.
//!
//! Checked on the resolved AST, so a field's current names include what it inherits and what an
//! `::extend` block adds. An inherited field is copied into every model that inherits it; a problem
//! with its declaration is reported once, where it is written — interfaces are checked first, so the
//! report names the element that declares it.

use std::collections::{HashMap, HashSet};

use crate::types::*;

/// Reports a diagnostic once per declaration: `subject` is what the problem is about (a former name, an
/// enum value), so the same problem met again through an inherited copy is not reported again.
trait Report {
    fn report(&mut self, code: &str, file: &str, line: usize, subject: &str, message: String);
}

struct Reporter<'a> {
    errors: &'a mut Vec<Diagnostic>,
    seen: HashSet<(String, String, usize, String)>,
}

impl Report for Reporter<'_> {
    fn report(&mut self, code: &str, file: &str, line: usize, subject: &str, message: String) {
        if self.seen.insert((
            code.to_string(),
            file.to_string(),
            line,
            subject.to_string(),
        )) {
            self.errors.push(Diagnostic {
                code: code.to_string(),
                severity: DiagnosticSeverity::Error,
                file: file.to_string(),
                line,
                col: 1,
                message,
            });
        }
    }
}

pub(crate) fn validate_rename_declarations(ast: &M3lAst, errors: &mut Vec<Diagnostic>) {
    let mut out = Reporter {
        errors,
        seen: HashSet::new(),
    };

    // Models: their own names, and the namespace a former name must be unique in.
    let mut current: HashMap<(Option<&str>, &str), &ModelNode> = HashMap::new();
    for m in ast
        .models
        .iter()
        .chain(ast.views.iter())
        .chain(ast.interfaces.iter())
    {
        current
            .entry((m.namespace.as_deref(), m.name.as_str()))
            .or_insert(m);
    }
    let mut claimed: HashMap<(Option<&str>, String), &ModelNode> = HashMap::new();
    for model in &ast.models {
        let Some(names) = &model.renamed_from else {
            continue;
        };
        let what = format!("Model \"{}\"", model.name);
        for name in valid_names(names, &what, &model.source, model.line, &mut out) {
            if name == model.name {
                continue; // reported by check_history
            }
            if current.contains_key(&(model.namespace.as_deref(), name.as_str())) {
                out.report(
                    "M3L-E028",
                    &model.source,
                    model.line,
                    &name,
                    format!(
                        "{what} declares the former name \"{name}\", but a model of that name exists now. \
                         A former name is gone — remove the declaration, or rename the other model (§2.2.6)."
                    ),
                );
            }
            match claimed.get(&(model.namespace.as_deref(), name.clone())) {
                Some(other) if other.name != model.name => out.report(
                    "M3L-E029",
                    &model.source,
                    model.line,
                    &name,
                    format!(
                        "{what} and model \"{}\" both declare the former name \"{name}\" — two models \
                         cannot both have been it (§2.2.6).",
                        other.name
                    ),
                ),
                _ => {
                    claimed.insert((model.namespace.as_deref(), name), model);
                }
            }
        }
        check_history(
            names,
            &model.name,
            &what,
            &model.source,
            model.line,
            &mut out,
        );
    }

    // Headers where nothing is stored under the name.
    for (kind, list) in [("view", &ast.views), ("flow", &ast.flows)] {
        for node in list {
            if node.renamed_from.is_some() {
                out.report(
                    "M3L-E031",
                    &node.source,
                    node.line,
                    &node.name,
                    format!(
                        "A `::{kind}` stores no data under its name, so `@renamed_from` on \"{}\" has \
                         nothing to carry over — remove it (§2.2.6).",
                        node.name
                    ),
                );
            }
            for field in node.fields.iter().filter(|f| f.renamed_from.is_some()) {
                out.report(
                    "M3L-E031",
                    &field.loc.file,
                    field.loc.line,
                    &field.name,
                    format!(
                        "Field \"{}\" of the `::{kind}` \"{}\" is derived, so `@renamed_from` on it \
                         has nothing to carry over — remove it (§2.2.6).",
                        field.name, node.name
                    ),
                );
            }
        }
    }

    // Fields: interfaces first (a field declared there is reported there), then stored models, whose
    // fields include the inherited and `::extend`-added ones.
    for model in ast.interfaces.iter().chain(ast.models.iter()) {
        let field_names: HashSet<&str> = model.fields.iter().map(|f| f.name.as_str()).collect();
        let mut field_claimed: HashMap<String, &FieldNode> = HashMap::new();
        for field in &model.fields {
            check_enum_values(
                field.enum_values.iter().flatten(),
                &format!("field \"{}.{}\"", model.name, field.name),
                &field.loc.file,
                field.loc.line,
                &mut out,
            );
            let Some(names) = &field.renamed_from else {
                continue;
            };
            let what = format!("Field \"{}.{}\"", model.name, field.name);
            if !is_stored(field) {
                out.report(
                    "M3L-E031",
                    &field.loc.file,
                    field.loc.line,
                    &field.name,
                    format!(
                        "{what} is derived without storage, so `@renamed_from` on it has nothing to \
                         carry over — remove it, or persist the field (§2.2.6)."
                    ),
                );
                continue;
            }
            for name in valid_names(names, &what, &field.loc.file, field.loc.line, &mut out) {
                if name == field.name {
                    continue;
                }
                if field_names.contains(name.as_str()) {
                    out.report(
                        "M3L-E028",
                        &field.loc.file,
                        field.loc.line,
                        &name,
                        format!(
                            "{what} declares the former name \"{name}\", but \"{}\" has a field of that \
                             name now (its own, inherited, or added by `::extend`). A former name is gone \
                             (§2.2.6).",
                            model.name
                        ),
                    );
                }
                match field_claimed.get(&name) {
                    Some(other) if other.name != field.name => out.report(
                        "M3L-E029",
                        &field.loc.file,
                        field.loc.line,
                        &name,
                        format!(
                            "{what} and field \"{}\" both declare the former name \"{name}\" — two fields \
                             cannot both have been it (§2.2.6).",
                            other.name
                        ),
                    ),
                    _ => {
                        field_claimed.insert(name, field);
                    }
                }
            }
            check_history(
                names,
                &field.name,
                &what,
                &field.loc.file,
                field.loc.line,
                &mut out,
            );
        }
    }

    for e in &ast.enums {
        check_enum_values(
            e.values.iter(),
            &format!("enum \"{}\"", e.name),
            &e.source,
            e.line,
            &mut out,
        );
    }
}

/// A field whose value is stored under its name: a stored field, or a computed one that is persisted.
fn is_stored(field: &FieldNode) -> bool {
    match field.kind {
        FieldKind::Stored => true,
        FieldKind::Computed => field.attributes.iter().any(|a| a.name == "persisted"),
        FieldKind::Lookup | FieldKind::Rollup => false,
    }
}

/// An enum value is data: renaming one rewrites stored rows, which is not what this declaration says.
/// A value carries no location of its own, so it is reported at its enum (or its inline-enum field).
fn check_enum_values<'a>(
    values: impl Iterator<Item = &'a EnumValue>,
    owner: &str,
    file: &str,
    line: usize,
    out: &mut impl Report,
) {
    for value in values.filter(|v| {
        v.attributes
            .iter()
            .flatten()
            .any(|a| a.name == "renamed_from")
    }) {
        out.report(
            "M3L-E031",
            file,
            line,
            &value.name,
            format!(
                "Value \"{}\" of {owner}: renaming an enum value rewrites the rows that hold it — a data \
                 change, not a rename — so `@renamed_from` does not apply (§2.2.6).",
                value.name
            ),
        );
    }
}

/// The arguments that are names; each one that is not is reported (`M3L-E032`).
fn valid_names(
    names: &[String],
    what: &str,
    file: &str,
    line: usize,
    out: &mut impl Report,
) -> Vec<String> {
    if names.is_empty() {
        out.report(
            "M3L-E032",
            file,
            line,
            "",
            format!("{what}: `@renamed_from` names no former name (§2.2.6)."),
        );
        return Vec::new();
    }
    let mut valid = Vec::new();
    for name in names {
        let problem = if name.is_empty() {
            Some("is empty")
        } else if name.contains('.') {
            Some("is qualified — a rename stays within its namespace")
        } else if !is_identifier(name) {
            Some("is not an identifier")
        } else {
            None
        };
        match problem {
            Some(problem) => out.report(
                "M3L-E032",
                file,
                line,
                name,
                format!("{what}: the former name \"{name}\" {problem} (§2.2.6)."),
            ),
            None => valid.push(name.clone()),
        }
    }
    valid
}

/// `M3L-E030`: a history that names one name twice, or names the element's current name.
fn check_history(
    names: &[String],
    current: &str,
    what: &str,
    file: &str,
    line: usize,
    out: &mut impl Report,
) {
    let mut seen = HashSet::new();
    for name in names {
        if name == current {
            out.report(
                "M3L-E030",
                file,
                line,
                name,
                format!("{what} lists its own name \"{name}\" as a former name (§2.2.6)."),
            );
        } else if !seen.insert(name.as_str()) {
            out.report(
                "M3L-E030",
                file,
                line,
                name,
                format!("{what} lists the former name \"{name}\" twice (§2.2.6)."),
            );
        }
    }
}

fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(c) if c.is_alphabetic() || c == '_')
        && chars.all(|c| c.is_alphanumeric() || c == '_')
}
