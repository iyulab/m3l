//! `m3l diff` — compare two schemas.
//!
//! Every declaration (model, view, interface, flow, extension block, enum) is compared as the JSON the parser
//! writes for it, so a column the AST gains later is compared without this file learning about it. Four things
//! are left out of the comparison because they say where something was written, not what it declares: the
//! `loc` of every node, the `line` and `source` file of a declaration, the `source` of the `::extend` blocks
//! merged into it, and the `raw` text an entry of a section was written as.
//!
//! Declarations and fields are matched by name — and by `@renamed_from`: a declaration that lists the name of one
//! on the other side is the same declaration under a new name, shown as one `~` entry, not as a removal plus an
//! addition. Lists of named items (fields, enum values, named indexes) are matched by name and lists of attributes
//! as a set, so reordering them changes nothing.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::Path;

use serde_json::Value;

use crate::build_ast;

/// What `m3l diff` found: the text to print, the same findings as structured entries, and whether the two sides
/// differ.
pub struct DiffReport {
    pub text: String,
    pub differs: bool,
    entries: Vec<Entry>,
}

/// The version of the JSON document `--format json` prints. Fields are only ever added within a version; a change
/// to the meaning of a field or the removal of one is a new version.
pub const JSON_VERSION: u32 = 1;

impl DiffReport {
    /// The findings as the JSON document of `--format json`: `version`, `differs`, `summary`
    /// (`added` / `removed` / `modified` / `renamed` counts of entries), and `changes` in the order the text lists them.
    ///
    /// An entry is `{"op", "scope", …}`. `op` is `added`, `removed`, `modified` or `renamed`; `scope` is
    /// `declaration`, `field`, `project` or `attributeRegistry`. A declaration carries `kind` and `name`; a field
    /// carries `declaration` and `name`; a rename carries `from` and `to`. A `modified` entry (and a rename that also
    /// changed something) carries `details`, each `{"path", "change", "before"?, "after"?}` where `change` is
    /// `added`, `removed` or `changed` and `before` / `after` hold the AST's own JSON for the value.
    pub fn json(&self) -> Value {
        let count = |op: &str| {
            self.entries
                .iter()
                .filter(|e| e.json.get("op").and_then(Value::as_str) == Some(op))
                .count()
        };
        serde_json::json!({
            "version": JSON_VERSION,
            "differs": self.differs,
            "summary": {
                "added": count("added"),
                "removed": count("removed"),
                "modified": count("modified"),
                "renamed": count("renamed"),
            },
            "changes": self.entries.iter().map(|e| e.json.clone()).collect::<Vec<_>>(),
        })
    }
}

/// One finding: the line the text lists, and the same finding as JSON.
struct Entry {
    text: String,
    json: Value,
}

/// One change inside a declaration, a field, the project or the attribute registry.
struct Detail {
    text: String,
    json: Value,
}

impl Detail {
    /// `text` is how the change is worded in the listing; the JSON is derived from what changed: a value only on the
    /// right was `added`, one only on the left `removed`, one on both sides `changed`.
    fn new(text: String, path: &str, before: Option<&Value>, after: Option<&Value>) -> Detail {
        let change = match (before, after) {
            (Some(_), Some(_)) => "changed",
            (None, Some(_)) => "added",
            _ => "removed",
        };
        let mut json =
            serde_json::json!({ "path": path.trim_start_matches('.'), "change": change });
        if let Some(b) = before {
            json["before"] = b.clone();
        }
        if let Some(a) = after {
            json["after"] = a.clone();
        }
        Detail { text, json }
    }
}

/// An `added` or `removed` entry for a whole declaration or field.
fn presence(op: &str, text: String, mut json: Value) -> Entry {
    json["op"] = Value::from(op);
    Entry { text, json }
}

/// A `modified` entry: `prefix` then the details joined as the listing words them.
fn modified(prefix: String, mut json: Value, details: Vec<Detail>) -> Entry {
    let text = format!(
        "{prefix}{}",
        details
            .iter()
            .map(|d| d.text.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    json["op"] = Value::from("modified");
    json["details"] = Value::Array(details.into_iter().map(|d| d.json).collect());
    Entry { text, json }
}

pub fn run_diff(left_path: &Path, right_path: &Path) -> Result<DiffReport, String> {
    let left = build_ast(left_path)?;
    let right = build_ast(right_path)?;
    diff_asts(&left, &right)
}

/// One declaration as it is compared.
struct Decl {
    /// `model`, `view`, `interface`, `flow`, `enum`, or the name of an extension block kind.
    kind: String,
    name: String,
    namespace: Option<String>,
    renamed_from: Vec<String>,
    /// The name it is shown under (see `diff_asts`).
    shown: String,
    value: Value,
}

pub fn diff_asts(left: &m3l_core::M3lAst, right: &m3l_core::M3lAst) -> Result<DiffReport, String> {
    let mut left_decls = declarations(left)?;
    let mut right_decls = declarations(right)?;

    // Each declaration by the name it is shown under: its short name, or `namespace.Name` when the short name is
    // declared in more than one namespace on either side — one namespace is one scope, so two declarations of one
    // short name are two entries, not one replacing the other.
    let mut shared: HashSet<String> = HashSet::new();
    for decls in [&left_decls, &right_decls] {
        let mut seen = HashSet::new();
        for d in decls.iter() {
            if !seen.insert(d.name.clone()) {
                shared.insert(d.name.clone());
            }
        }
    }
    for d in left_decls.iter_mut().chain(right_decls.iter_mut()) {
        d.shown = match &d.namespace {
            Some(ns) if shared.contains(&d.name) => format!("{ns}.{}", d.name),
            _ => d.name.clone(),
        };
    }

    // One entry per kind family and shown name. A model, a view and an interface of one name are the same slot —
    // changing which of them it is, is a change of that declaration.
    let key = |d: &Decl| match d.kind.as_str() {
        "model" | "view" | "interface" | "flow" => format!("model\u{0}{}", d.shown),
        other => format!("{other}\u{0}{}", d.shown),
    };
    let left_by_key: HashMap<String, usize> = left_decls
        .iter()
        .enumerate()
        .map(|(i, d)| (key(d), i))
        .collect();
    let right_by_key: HashMap<String, usize> = right_decls
        .iter()
        .enumerate()
        .map(|(i, d)| (key(d), i))
        .collect();

    // Renamed declarations: an unmatched right-hand declaration listing the name of an unmatched left-hand one.
    let mut renamed: HashMap<usize, usize> = HashMap::new(); // right index → left index
    let mut taken: HashSet<usize> = HashSet::new();
    for (ri, r) in right_decls.iter().enumerate() {
        if left_by_key.contains_key(&key(r)) || r.renamed_from.is_empty() {
            continue;
        }
        // Newest former name first: the declaration the list ends in is the one that was there last.
        for former in r.renamed_from.iter().rev() {
            let found = left_decls.iter().enumerate().find(|(li, l)| {
                !taken.contains(li)
                    && !right_by_key.contains_key(&key(l))
                    && l.name == *former
                    && l.namespace == r.namespace
                    && l.kind == r.kind
            });
            if let Some((li, _)) = found {
                taken.insert(li);
                renamed.insert(ri, li);
                break;
            }
        }
    }

    let mut lines: Vec<Entry> = Vec::new();
    for (ri, r) in right_decls.iter().enumerate() {
        if let Some(&li) = renamed.get(&ri) {
            let l = &left_decls[li];
            let mut lv = l.value.clone();
            let mut rv = r.value.clone();
            for v in [&mut lv, &mut rv] {
                if let Value::Object(o) = v {
                    for column in ["name", "qualifiedName", "renamed_from"] {
                        o.remove(column);
                    }
                }
            }
            diff_declaration(&r.shown, &r.kind, &lv, &rv, &mut lines);
            lines.push(Entry {
                text: format!("~ {} {} → {} (renamed)", r.kind, l.shown, r.shown),
                json: serde_json::json!({
                    "op": "renamed", "scope": "declaration",
                    "kind": r.kind, "from": l.shown, "to": r.shown,
                }),
            });
        } else if let Some(&li) = left_by_key.get(&key(r)) {
            diff_declaration(
                &r.shown,
                &r.kind,
                &left_decls[li].value,
                &r.value,
                &mut lines,
            );
            // A declaration can change which kind it is only inside the model family.
            let l = &left_decls[li];
            if l.kind != r.kind {
                let (before, after) = (Value::from(l.kind.clone()), Value::from(r.kind.clone()));
                lines.push(modified(
                    format!("~ {}: ", r.shown),
                    serde_json::json!({ "scope": "declaration", "kind": r.kind, "name": r.shown }),
                    vec![Detail::new(
                        format!("kind: {} → {}", l.kind, r.kind),
                        "kind",
                        Some(&before),
                        Some(&after),
                    )],
                ));
            }
        } else {
            lines.push(presence(
                "added",
                format!("+ {} {}", r.kind, r.shown),
                serde_json::json!({ "scope": "declaration", "kind": r.kind, "name": r.shown }),
            ));
        }
    }
    for (li, l) in left_decls.iter().enumerate() {
        if !taken.contains(&li) && !right_by_key.contains_key(&key(l)) {
            lines.push(presence(
                "removed",
                format!("- {} {}", l.kind, l.shown),
                serde_json::json!({ "scope": "declaration", "kind": l.kind, "name": l.shown }),
            ));
        }
    }

    // The project and the attribute registry are part of what a schema says.
    let project = |ast: &m3l_core::M3lAst| -> Result<Value, String> {
        serde_json::to_value(&ast.project).map_err(|e| format!("JSON serialization error: {e}"))
    };
    let mut project_changes = Vec::new();
    diff_value("", &project(left)?, &project(right)?, &mut project_changes);
    for c in project_changes {
        lines.push(modified(
            "~ project: ".to_string(),
            serde_json::json!({ "scope": "project" }),
            vec![c],
        ));
    }
    let registry = |ast: &m3l_core::M3lAst| -> Result<Value, String> {
        serde_json::to_value(&ast.attribute_registry)
            .map_err(|e| format!("JSON serialization error: {e}"))
    };
    let mut registry_changes = Vec::new();
    diff_value(
        "",
        &registry(left)?,
        &registry(right)?,
        &mut registry_changes,
    );
    for c in registry_changes {
        lines.push(modified(
            "~ attribute registry".to_string(),
            serde_json::json!({ "scope": "attributeRegistry" }),
            vec![c],
        ));
    }

    if lines.is_empty() {
        return Ok(DiffReport {
            text: "No differences found.".into(),
            differs: false,
            entries: Vec::new(),
        });
    }
    lines.sort_by(|a, b| a.text.cmp(&b.text));
    lines.dedup_by(|a, b| a.text == b.text);
    let add_count = lines.iter().filter(|l| l.text.starts_with('+')).count();
    let rem_count = lines.iter().filter(|l| l.text.starts_with('-')).count();
    let mod_count = lines.iter().filter(|l| l.text.starts_with('~')).count();
    let mut text: Vec<&str> = lines.iter().map(|l| l.text.as_str()).collect();
    let summary = format!("\n{add_count} added, {rem_count} removed, {mod_count} modified");
    text.push(&summary);
    Ok(DiffReport {
        text: text.join("\n"),
        differs: true,
        entries: lines,
    })
}

/// Every declaration of the AST, its JSON stripped of where it was written.
fn declarations(ast: &m3l_core::M3lAst) -> Result<Vec<Decl>, String> {
    let mut out = Vec::new();
    let mut push = |kind: String,
                    name: &str,
                    namespace: &Option<String>,
                    renamed_from: Vec<String>,
                    value: Result<Value, serde_json::Error>|
     -> Result<(), String> {
        let mut value = value.map_err(|e| format!("JSON serialization error: {e}"))?;
        strip_positions(&mut value, true);
        out.push(Decl {
            kind,
            name: name.to_string(),
            namespace: namespace.clone(),
            renamed_from,
            shown: String::new(),
            value,
        });
        Ok(())
    };
    for m in ast
        .models
        .iter()
        .chain(ast.interfaces.iter())
        .chain(ast.views.iter())
        .chain(ast.flows.iter())
    {
        let kind = serde_json::to_value(m)
            .ok()
            .and_then(|v| v.get("type").and_then(Value::as_str).map(str::to_string))
            .unwrap_or_else(|| "model".to_string());
        push(
            kind,
            &m.name,
            &m.namespace,
            m.renamed_from.clone().unwrap_or_default(),
            serde_json::to_value(m),
        )?;
    }
    for (extension, members) in &ast.extensions {
        for m in members {
            push(
                extension.clone(),
                &m.name,
                &m.namespace,
                m.renamed_from.clone().unwrap_or_default(),
                serde_json::to_value(m),
            )?;
        }
    }
    for e in &ast.enums {
        push(
            "enum".to_string(),
            &e.name,
            &e.namespace,
            Vec::new(),
            serde_json::to_value(e),
        )?;
    }
    Ok(out)
}

/// Removes what says where a node was written: `loc` everywhere, and on a declaration its `line` and `source`
/// file — with the `source` of what merged into it (`origin`, `extendedBy`).
fn strip_positions(value: &mut Value, declaration: bool) {
    match value {
        Value::Object(map) => {
            map.remove("loc");
            if declaration {
                map.remove("line");
                map.remove("source");
            }
            // The text an entry of a section was written as (`raw`) is the form, its columns are the content.
            if let Some(Value::Object(sections)) = map.get_mut("sections") {
                for entries in sections.values_mut() {
                    if let Value::Array(items) = entries {
                        for item in items {
                            if let Value::Object(o) = item {
                                o.remove("raw");
                            }
                        }
                    }
                }
            }
            if let Some(Value::Object(origin)) = map.get_mut("origin") {
                origin.remove("source");
            }
            if let Some(Value::Array(blocks)) = map.get_mut("extendedBy") {
                for block in blocks {
                    if let Value::Object(b) = block {
                        b.remove("source");
                    }
                }
            }
            for (_, v) in map.iter_mut() {
                strip_positions(v, false);
            }
        }
        Value::Array(items) => {
            for item in items {
                strip_positions(item, false);
            }
        }
        _ => {}
    }
}

/// The changes inside one matched declaration, as `~ Name: …` lines — fields as `Name.field`.
fn diff_declaration(shown: &str, kind: &str, left: &Value, right: &Value, lines: &mut Vec<Entry>) {
    let changes = changes_between(left, right, &["fields"]);
    if !changes.is_empty() {
        lines.push(modified(
            format!("~ {shown}: "),
            serde_json::json!({ "scope": "declaration", "kind": kind, "name": shown }),
            changes,
        ));
    }
    diff_fields(
        shown,
        left.get("fields").unwrap_or(&Value::Null),
        right.get("fields").unwrap_or(&Value::Null),
        lines,
    );
}

/// A model's fields: added, removed, renamed (by `@renamed_from`), and the changes of those that stay.
fn diff_fields(shown: &str, left: &Value, right: &Value, lines: &mut Vec<Entry>) {
    let empty = Vec::new();
    let lf = left.as_array().unwrap_or(&empty);
    let rf = right.as_array().unwrap_or(&empty);
    let name_of = |f: &Value| {
        f.get("name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    };
    let left_names: HashSet<String> = lf.iter().map(name_of).collect();
    let right_names: HashSet<String> = rf.iter().map(name_of).collect();
    let left_by_name: HashMap<String, &Value> = lf.iter().map(|f| (name_of(f), f)).collect();

    let mut renamed_left: HashSet<String> = HashSet::new();
    for r in rf {
        let rname = name_of(r);
        if let Some(l) = left_by_name.get(&rname) {
            let changes = changes_between(l, r, &[]);
            if !changes.is_empty() {
                lines.push(modified(
                    format!("~ {shown}.{rname}: "),
                    serde_json::json!({ "scope": "field", "declaration": shown, "name": rname }),
                    changes,
                ));
            }
            continue;
        }
        // Not on the left: a rename if it lists a former name that was a field there and is gone now.
        let former = r
            .get("renamed_from")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .rev()
            .find(|f| {
                left_names.contains(*f) && !right_names.contains(*f) && !renamed_left.contains(*f)
            });
        match former {
            Some(f) => {
                renamed_left.insert(f.to_string());
                let l = left_by_name[f];
                let changes = changes_between(l, r, &["name", "renamed_from"]);
                let mut text = format!("~ {shown}.{f} → {rname} (renamed)");
                if !changes.is_empty() {
                    text.push_str(&format!(
                        ": {}",
                        changes
                            .iter()
                            .map(|d| d.text.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
                let mut json = serde_json::json!({
                    "op": "renamed", "scope": "field",
                    "declaration": shown, "from": f, "to": rname,
                });
                if !changes.is_empty() {
                    json["details"] = Value::Array(changes.into_iter().map(|d| d.json).collect());
                }
                lines.push(Entry { text, json });
            }
            None => lines.push(presence(
                "added",
                format!("+ {shown}.{rname}"),
                serde_json::json!({ "scope": "field", "declaration": shown, "name": rname }),
            )),
        }
    }
    for l in lf {
        let lname = name_of(l);
        if !right_names.contains(&lname) && !renamed_left.contains(&lname) {
            lines.push(presence(
                "removed",
                format!("- {shown}.{lname}"),
                serde_json::json!({ "scope": "field", "declaration": shown, "name": lname }),
            ));
        }
    }
}

/// The differences between two nodes of one object shape, minus the columns named in `ignore`.
fn changes_between(left: &Value, right: &Value, ignore: &[&str]) -> Vec<Detail> {
    let mut changes = Vec::new();
    let (Value::Object(lo), Value::Object(ro)) = (left, right) else {
        return changes;
    };
    let keys: BTreeSet<&String> = lo.keys().chain(ro.keys()).collect();
    for key in keys {
        if ignore.contains(&key.as_str()) {
            continue;
        }
        match (lo.get(key), ro.get(key)) {
            (Some(l), Some(r)) => diff_value(key, l, r, &mut changes),
            (Some(l), None) => changes.push(Detail::new(
                format!("{key}: {} → (none)", show(l)),
                key,
                Some(l),
                None,
            )),
            (None, Some(r)) => changes.push(Detail::new(
                format!("{key}: (none) → {}", show(r)),
                key,
                None,
                Some(r),
            )),
            (None, None) => {}
        }
    }
    changes
}

/// Appends the differences between two values found at `path`. Objects are compared column by column, lists of
/// named items by name, other lists of objects as sets, and lists of plain values as a whole.
fn diff_value(path: &str, left: &Value, right: &Value, out: &mut Vec<Detail>) {
    if left == right {
        return;
    }
    let at = |sub: &str| {
        if path.is_empty() {
            sub.to_string()
        } else {
            format!("{path}.{sub}")
        }
    };
    match (left, right) {
        (Value::Object(lo), Value::Object(ro)) => {
            let keys: BTreeSet<&String> = lo.keys().chain(ro.keys()).collect();
            for key in keys {
                match (lo.get(key), ro.get(key)) {
                    (Some(l), Some(r)) => diff_value(&at(key), l, r, out),
                    (Some(l), None) => out.push(Detail::new(
                        format!("{}: {} → (none)", at(key), show(l)),
                        &at(key),
                        Some(l),
                        None,
                    )),
                    (None, Some(r)) => out.push(Detail::new(
                        format!("{}: (none) → {}", at(key), show(r)),
                        &at(key),
                        None,
                        Some(r),
                    )),
                    (None, None) => {}
                }
            }
        }
        (Value::Array(la), Value::Array(ra)) if la.iter().chain(ra).all(Value::is_object) => {
            diff_object_lists(path, la, ra, out);
        }
        _ => out.push(Detail::new(
            format!("{path}: {} → {}", show(left), show(right)),
            path,
            Some(left),
            Some(right),
        )),
    }
}

/// Two lists of objects. Matched by `name` when every item has a distinct one on both sides; otherwise as sets
/// (an item that changed in place — same `name`, other content — is reported as one change).
fn diff_object_lists(path: &str, left: &[Value], right: &[Value], out: &mut Vec<Detail>) {
    let name = |v: &Value| v.get("name").and_then(Value::as_str).map(str::to_string);
    let distinct_names = |items: &[Value]| {
        let names: Vec<Option<String>> = items.iter().map(name).collect();
        names.iter().all(Option::is_some)
            && names.iter().flatten().collect::<HashSet<_>>().len() == names.len()
    };
    let attributes = left
        .iter()
        .chain(right)
        .any(|v| v.get("isStandard").is_some());
    if !attributes && distinct_names(left) && distinct_names(right) {
        let by_name = |items: &[Value]| -> BTreeMap<String, Value> {
            items
                .iter()
                .map(|v| (name(v).unwrap_or_default(), v.clone()))
                .collect()
        };
        let (lm, rm) = (by_name(left), by_name(right));
        for (n, l) in &lm {
            match rm.get(n) {
                Some(r) => diff_value(&format!("{path}.{n}"), l, r, out),
                None => out.push(Detail::new(format!("{path}: - {n}"), path, Some(l), None)),
            }
        }
        for (n, r) in rm.iter().filter(|(n, _)| !lm.contains_key(*n)) {
            out.push(Detail::new(format!("{path}: + {n}"), path, None, Some(r)));
        }
        return;
    }
    let mut removed: Vec<&Value> = left.iter().collect();
    let mut added: Vec<&Value> = Vec::new();
    for r in right {
        match removed.iter().position(|l| *l == r) {
            Some(i) => {
                removed.remove(i);
            }
            None => added.push(r),
        }
    }
    // An item whose name is on both sides changed in place.
    for a in added.clone() {
        let Some(n) = name(a) else { continue };
        if let Some(i) = removed
            .iter()
            .position(|l| name(l).as_deref() == Some(n.as_str()))
        {
            let l = removed.remove(i);
            added.retain(|x| !std::ptr::eq(*x, a));
            out.push(Detail::new(
                format!("{path}: {} → {}", show(l), show(a)),
                path,
                Some(l),
                Some(a),
            ));
        }
    }
    for l in removed {
        out.push(Detail::new(
            format!("{path}: - {}", show(l)),
            path,
            Some(l),
            None,
        ));
    }
    for a in added {
        out.push(Detail::new(
            format!("{path}: + {}", show(a)),
            path,
            None,
            Some(a),
        ));
    }
}

/// A value for a message: text as it is, an attribute as it is written, a whole number without its fraction,
/// anything else as compact JSON.
fn show(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => match n.as_f64() {
            Some(f) if f.fract() == 0.0 && f.abs() < 1e15 => format!("{}", f as i64),
            _ => n.to_string(),
        },
        Value::Array(items) => {
            let rendered: Vec<String> = items.iter().map(show).collect();
            format!("[{}]", rendered.join(", "))
        }
        Value::Object(o)
            if o.get("name").is_some_and(Value::is_string) && o.contains_key("isStandard") =>
        {
            let name = o["name"].as_str().unwrap_or_default();
            match o.get("args") {
                Some(Value::Array(args)) if !args.is_empty() => {
                    let rendered: Vec<String> = args.iter().map(show).collect();
                    format!("@{name}({})", rendered.join(", "))
                }
                _ => format!("@{name}"),
            }
        }
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ast(src: &str, file: &str) -> m3l_core::M3lAst {
        m3l_core::resolve(&[m3l_core::parse_string(src, file)], None)
    }

    fn diff(left: &str, right: &str) -> DiffReport {
        diff_asts(&ast(left, "left.m3l.md"), &ast(right, "right.m3l.md")).expect("diff")
    }

    fn model(fields: &str) -> String {
        format!("# Namespace: demo\n\n## T\n\n- id: identifier @pk\n{fields}\n")
    }

    /// A change the AST carries is a difference — the comparison must not be narrower than what the parser records.
    #[test]
    fn every_change_to_a_field_is_reported() {
        let cases = [
            (
                "- e: string(200) @unique",
                "- e: string(200)",
                "~ T.e: attributes: - @unique",
            ),
            (
                "- e: string(100)",
                "- e: string(200)",
                "~ T.e: params: [100] → [200]",
            ),
            (
                "- n: integer",
                "- n: integer = 0",
                "~ T.n: default_value: (none) → 0",
            ),
            (
                "- n: integer @min(1)",
                "- n: integer @min(5)",
                "~ T.n: attributes: @min(1) → @min(5)",
            ),
            (
                "- e: string",
                "- e: string \"hello\"",
                "~ T.e: description: (none) → hello",
            ),
            (
                "- e: string @index",
                "- e: string",
                "~ T.e: attributes: - @index",
            ),
            (
                "- e: string(100)",
                "- e: string(100)?",
                "~ T.e: nullable: false → true",
            ),
            ("- e: string", "- e: text", "~ T.e: type: string → text"),
            ("- e: string[]", "- e: string", "~ T.e: array: true → false"),
        ];
        for (left, right, expected) in cases {
            let report = diff(&model(left), &model(right));
            assert!(report.differs, "{left} → {right} reported no difference");
            assert!(
                report.text.contains(expected),
                "{left} → {right}:
{}",
                report.text
            );
        }
    }

    /// What is not a change: the order things are written in, and where they are written.
    #[test]
    fn order_and_position_are_not_changes() {
        let a = diff(
            &model("- e: string @unique @index\n- f: integer"),
            &model("- f: integer\n- e: string @index @unique"),
        );
        assert!(!a.differs, "reordered: {}", a.text);
        assert_eq!(a.text, "No differences found.");

        let moved = format!("\n\n> a note above\n\n{}", model("- e: string"));
        let b = diff(&model("- e: string"), &moved);
        assert!(!b.differs, "moved: {}", b.text);

        let c = diff_asts(
            &ast(&model("- e: string"), "a/one.m3l.md"),
            &ast(&model("- e: string"), "b/two.m3l.md"),
        )
        .expect("diff");
        assert!(!c.differs, "other file: {}", c.text);
    }

    #[test]
    fn indexes_enum_values_and_models_are_compared() {
        let indexes = "\n### Indexes\n- idx_e\n  - fields: [e]\n  - unique: true\n";
        let with =
            format!("# Namespace: demo\n\n## T\n\n- id: identifier @pk\n- e: string\n{indexes}");
        let without = model("- e: string");
        let removed = diff(&with, &without);
        assert!(
            removed.text.contains("~ T: sections.indexes: - idx_e"),
            "{}",
            removed.text
        );

        // The written form of an index does not matter, its content does.
        let directive = |args: &str| {
            format!(
                "# Namespace: demo

## T

- id: identifier @pk
- a: string
- b: string

### Indexes
- @unique({args})
"
            )
        };
        assert!(!diff(&directive("a,b"), &directive("a, b")).differs);
        let swapped = diff(&directive("a, b"), &directive("b, a"));
        assert!(swapped.differs, "{}", swapped.text);

        let one = "# Namespace: demo\n\n## S ::enum\n\n- a: \"A\"\n- b: \"B\"\n";
        let two = "# Namespace: demo\n\n## S ::enum\n\n- a: \"Changed\"\n- b: \"B\"\n- c: \"C\"\n";
        let report = diff(one, two);
        assert!(
            report
                .text
                .contains("~ S: values.a.description: A → Changed, values: + c"),
            "{}",
            report.text
        );

        let added = diff(
            &model("- e: string"),
            &format!("{}\n## U\n\n- id: identifier @pk\n", model("- e: string")),
        );
        assert!(added.text.contains("+ model U"), "{}", added.text);
    }

    #[test]
    fn a_view_definition_is_compared() {
        let view = |filter: &str| {
            format!(
                "# Namespace: demo\n\n## T\n\n- id: identifier @pk\n- active: boolean\n\n## V ::view\n\n### Source\n- from: T\n- where: \"{filter}\"\n"
            )
        };
        let report = diff(&view("active = true"), &view("active = false"));
        assert!(report.differs);
        assert!(report.text.contains("~ V:"), "{}", report.text);
        assert!(report.text.contains("active = false"), "{}", report.text);
    }

    /// A declaration that lists the name of one on the other side is that declaration under a new name.
    #[test]
    fn renamed_declarations_and_fields_are_paired() {
        let before = "# Namespace: demo\n\n## SalesOrder\n\n- id: identifier @pk\n- code: string(20)\n- amount: integer\n";
        let after = "# Namespace: demo\n\n## CustomerOrder @renamed_from(SalesOrder)\n\n- id: identifier @pk\n- order_code: string(20) @renamed_from(code)\n- total: integer @renamed_from(amount)\n";
        let report = diff(before, after);
        assert!(
            report
                .text
                .contains("~ model SalesOrder → CustomerOrder (renamed)"),
            "{}",
            report.text
        );
        assert!(
            report
                .text
                .contains("~ CustomerOrder.code → order_code (renamed)"),
            "{}",
            report.text
        );
        assert!(
            report
                .text
                .contains("~ CustomerOrder.amount → total (renamed)"),
            "{}",
            report.text
        );
        assert!(!report.text.contains("- model"), "{}", report.text);
        assert!(!report.text.contains("+ model"), "{}", report.text);
        assert!(
            report.text.ends_with("0 added, 0 removed, 3 modified"),
            "{}",
            report.text
        );

        // A rename that also changes the field says both.
        let changed = after.replace("order_code: string(20)", "order_code: string(40)");
        let report = diff(before, &changed);
        assert!(
            report
                .text
                .contains("~ CustomerOrder.code → order_code (renamed): params: [20] → [40]"),
            "{}",
            report.text
        );
    }

    /// A former name that names nothing on the other side pairs with nothing: the field is new.
    #[test]
    fn a_former_name_that_was_not_there_is_an_addition() {
        let report = diff(
            &model("- e: string"),
            &model("- e: string\n- g: string @renamed_from(never_existed)"),
        );
        assert!(report.text.contains("+ T.g"), "{}", report.text);
        assert!(!report.text.contains("renamed)"), "{}", report.text);
    }

    /// One short name in two namespaces is two declarations (the pairing key is the name it is shown under).
    #[test]
    fn a_short_name_in_two_namespaces_stays_two_declarations() {
        let files = |y: &str| {
            [
                m3l_core::parse_string(
                    "# Namespace: a

## T

- id: identifier @pk
- x: string
",
                    "a.m3l.md",
                ),
                m3l_core::parse_string(
                    &format!(
                        "# Namespace: b

## T

- id: identifier @pk
- y: {y}
"
                    ),
                    "b.m3l.md",
                ),
            ]
        };
        let (left, right) = (
            m3l_core::resolve(&files("string"), None),
            m3l_core::resolve(&files("text"), None),
        );
        let report = diff_asts(&left, &left).expect("diff");
        assert!(!report.differs, "{}", report.text);
        let report = diff_asts(&left, &right).expect("diff");
        assert!(
            report.text.contains("~ b.T.y: type: string → text"),
            "{}",
            report.text
        );
        assert!(!report.text.contains("a.T"), "{}", report.text);
    }
}
