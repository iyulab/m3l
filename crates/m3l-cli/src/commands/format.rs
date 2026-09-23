use serde_json::Value;
use std::path::Path;

use crate::build_ast_with;

pub fn run_format(input_path: &Path) -> Result<String, String> {
    // The formatter reproduces the *source document*, not its semantic closure,
    // so inherited fields must stay in the parent. Inlining them here re-emitted
    // every parent field under the child while keeping the `: Parent` header —
    // the fields then doubled on each successive format.
    let ast = build_ast_with(
        input_path,
        m3l_core::ResolveOptions {
            inline_inherited: false,
            merge_extends: false,
        },
    )?;
    Ok(format_ast(&ast))
}

fn format_ast(ast: &m3l_core::M3lAst) -> String {
    let mut lines: Vec<String> = Vec::new();

    // Namespace
    if let Some(ref name) = ast.project.name {
        lines.push(format!("# Namespace: {name}"));
        lines.push(String::new());
    }

    // Owner prefix — a file-level fact; every declaration of the file carries the same
    // value, so any one of them (model, enum, interface, view, flow, or extend block)
    // is enough to recover it. A file that declares only interfaces or only views
    // dropped the header before every declaration kind was chained here.
    let prefix = ast
        .models
        .iter()
        .filter_map(|m| m.prefix.as_deref())
        .chain(
            ast.extensions
                .values()
                .flatten()
                .filter_map(|m| m.prefix.as_deref()),
        )
        .chain(ast.enums.iter().filter_map(|e| e.prefix.as_deref()))
        .chain(ast.interfaces.iter().filter_map(|m| m.prefix.as_deref()))
        .chain(ast.views.iter().filter_map(|m| m.prefix.as_deref()))
        .chain(ast.flows.iter().filter_map(|m| m.prefix.as_deref()))
        .next();
    if let Some(p) = prefix {
        // keep it directly under the namespace line, before the blank separator
        let at = if ast.project.name.is_some() { 1 } else { 0 };
        lines.insert(at, format!("# Prefix: {p}"));
        if at == 0 {
            lines.insert(1, String::new());
        }
    }

    // Attribute definitions first: they are what the attributes used below refer to.
    format_attribute_registry(&mut lines, &ast.attribute_registry);

    // Models
    for model in &ast.models {
        format_model(&mut lines, model);
        lines.push(String::new());
    }

    // Enums
    for e in &ast.enums {
        format_enum(&mut lines, e);
        lines.push(String::new());
    }

    // Interfaces
    for iface in &ast.interfaces {
        lines.push(format!(
            "## {} ::interface",
            labelled(&iface.name, &iface.label)
        ));
        format_model_body(&mut lines, iface);
        lines.push(String::new());
    }

    // Views
    for view in &ast.views {
        format_view(&mut lines, view);
        lines.push(String::new());
    }

    // Extend blocks — kept as blocks: the formatter resolves with `merge_extends: false`.
    if let Some(blocks) = ast.extensions.get("extend") {
        for block in blocks {
            lines.push(format!(
                "## {} ::extend",
                labelled(&block.name, &block.label)
            ));
            format_model_body(&mut lines, block);
            lines.push(String::new());
        }
    }

    // Remove trailing empty lines
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }

    lines.join("\n")
}

fn format_model(lines: &mut Vec<String>, model: &m3l_core::ModelNode) {
    let mut header = format!("## {}", labelled(&model.name, &model.label));
    if let Some(base) = &model.base {
        header.push_str(&format!(" ::{}({})", base.kind, base.model));
    }
    if !model.inherits.is_empty() {
        header.push_str(&format!(" : {}", model.inherits.join(", ")));
    }
    if !model.attributes.is_empty() {
        for attr in &model.attributes {
            header.push_str(&format!(" @{}", attr.name));
            if let Some(args_str) = format_attr_args(attr) {
                header.push_str(&args_str);
            }
        }
    }
    lines.push(header);
    format_model_body(lines, model);
}

/// `Name(Label)` when the declaration carries a label, `Name` otherwise.
///
/// The label is the declaration's display name, written only in the header
/// or field line — a format that dropped it would delete it from the file while
/// leaving one that still parses.
fn labelled(name: &str, label: &Option<String>) -> String {
    match label {
        Some(label) => format!("{name}({label})"),
        None => name.to_string(),
    }
}

/// Emit a description as blockquote lines, one per source line.
///
/// A description carrying newlines must never be written as a single line —
/// the embedded newline splits the output and every line after the first is
/// orphaned, so it disappears on the next parse.
fn push_description(lines: &mut Vec<String>, desc: &str, prefix: &str) {
    for desc_line in desc.split('\n') {
        lines.push(format!("{prefix}> {desc_line}"));
    }
}

fn format_model_body(lines: &mut Vec<String>, model: &m3l_core::ModelNode) {
    if let Some(ref desc) = model.description {
        push_description(lines, desc, "");
    }
    for field in &model.fields {
        format_field(lines, field, 0);
    }
    format_sections(lines, &model.sections);
}

/// `## name ::attribute` blocks. The registry is what makes a custom attribute known
/// (`isRegistered`); dropping it turns every use of that attribute into an unknown one.
fn format_attribute_registry(
    lines: &mut Vec<String>,
    registry: &[m3l_core::AttributeRegistryEntry],
) {
    for entry in registry {
        lines.push(format!("## {} ::attribute", entry.name));
        if let Some(ref desc) = entry.description {
            push_description(lines, desc, "");
        }
        lines.push(format!("- target: [{}]", entry.target.join(", ")));
        lines.push(format!("- type: {}", entry.attr_type));
        if let Some((lo, hi)) = entry.range {
            lines.push(format!("- range: [{lo}, {hi}]"));
        }
        if entry.required {
            lines.push("- required: true".to_string());
        }
        if let Some(ref default) = entry.default_value {
            lines.push(format!("- default: {}", format_arg(default, false)));
        }
        lines.push(String::new());
    }
}

/// A view: `@materialized` on the header, then `### Source` (directives or a SQL block) ahead of
/// the fields — the parser reads fields inside `### Source` as the view's columns — and
/// `### Refresh` after them.
fn format_view(lines: &mut Vec<String>, view: &m3l_core::ModelNode) {
    let mut header = format!("## {} ::view", labelled(&view.name, &view.label));
    if view.materialized == Some(true) {
        header.push_str(" @materialized");
    }
    lines.push(header);
    if let Some(ref desc) = view.description {
        push_description(lines, desc, "");
    }
    if let Some(ref source) = view.source_def {
        lines.push(String::new());
        lines.push("### Source".to_string());
        if let Some(ref sql) = source.raw_sql {
            lines.push(format!(
                "```{}",
                source.language_hint.as_deref().unwrap_or_default()
            ));
            lines.extend(sql.lines().map(str::to_string));
            lines.push("```".to_string());
            lines.push(String::new());
        }
        if let Some(ref from) = source.from {
            lines.push(format!("- from: {}", directive_value(from)));
        }
        for join in source.joins.iter().flatten() {
            lines.push(format!("- join: \"{} on {}\"", join.model, join.on));
        }
        if let Some(ref w) = source.where_clause {
            lines.push(format!("- where: \"{w}\""));
        }
        if let Some(ref g) = source.group_by {
            lines.push(format!("- group_by: \"[{}]\"", g.join(", ")));
        }
        if let Some(ref o) = source.order_by {
            lines.push(format!("- order_by: \"{o}\""));
        }
    }
    for field in &view.fields {
        format_field(lines, field, 0);
    }
    format_sections(lines, &view.sections);
    if let Some(ref refresh) = view.refresh {
        lines.push(String::new());
        lines.push("### Refresh".to_string());
        lines.push(format!("- strategy: {}", refresh.strategy));
        if let Some(ref interval) = refresh.interval {
            lines.push(format!("- interval: \"{interval}\""));
        }
    }
}

/// A `### Source` directive value: a plain name as is, anything else quoted.
fn directive_value(s: &str) -> String {
    if is_plain_word(s) {
        s.to_string()
    } else {
        format!("\"{s}\"")
    }
}

/// Re-emit a model's `###` sections from the AST.
///
/// The parser keeps each item's source line in `raw` wherever it has one, and folds `- key: value`
/// sub-items into the item's object; the formatter writes the line back and the remaining keys as
/// sub-items. Directive items (`- @index(a, b)`, `- @behavior(...)`) are written in the model body,
/// where they are read the same way wherever they sit. Sections come out in a fixed order —
/// indexes, relations, behaviors, the other named sections in source order, metadata — so a
/// second format produces the same text.
fn format_sections(lines: &mut Vec<String>, sections: &m3l_core::Sections) {
    let is_directive = |item: &Value| raw_of(item).is_some_and(|r| r.starts_with("- @"));

    // Directive items first, in the body.
    let directive_lists = [&sections.indexes, &sections.relations, &sections.behaviors];
    for item in directive_lists
        .into_iter()
        .flatten()
        .filter(|i| is_directive(i))
    {
        lines.push(raw_of(item).unwrap_or_default().to_string());
    }
    let mut custom: Vec<(&String, &Vec<Value>)> = sections
        .custom
        .iter()
        .filter_map(|(name, v)| v.as_array().map(|a| (name, a)))
        .collect();
    custom.sort_by_key(|(name, items)| {
        (
            items.first().and_then(first_line).unwrap_or(usize::MAX),
            *name,
        )
    });
    for (_, items) in &custom {
        for item in items.iter().filter(|i| is_directive(i)) {
            lines.push(raw_of(item).unwrap_or_default().to_string());
        }
    }

    let indexes: Vec<&Value> = sections
        .indexes
        .iter()
        .filter(|i| !is_directive(i))
        .collect();
    if !indexes.is_empty() {
        push_section_heading(lines, "Indexes");
        for item in indexes {
            lines.push(index_head(item));
            push_sub_items(
                lines,
                item,
                &["name", "label", "type", "attr", "unique", "args", "nulls"],
            );
        }
    }

    let relations: Vec<&Value> = sections
        .relations
        .iter()
        .filter(|i| !is_directive(i))
        .collect();
    if !relations.is_empty() {
        push_section_heading(lines, "Relations");
        for item in relations {
            // The notation is parsed back out of `raw`; only keys a sub-item set are written.
            lines.push(format!("- {}", raw_of(item).unwrap_or_default()));
            push_sub_items(
                lines,
                item,
                &[
                    "raw",
                    "direction",
                    "name",
                    "target",
                    "cardinality",
                    "from",
                    "type",
                    "args",
                ],
            );
        }
    }

    let behaviors: Vec<&Value> = sections
        .behaviors
        .iter()
        .filter(|i| !is_directive(i))
        .collect();
    if !behaviors.is_empty() {
        push_section_heading(lines, "Behaviors");
        for item in behaviors {
            lines.push(raw_of(item).unwrap_or_default().to_string());
            push_sub_items(lines, item, &["name", "raw", "args"]);
        }
    }

    for (name, items) in &custom {
        let items: Vec<&Value> = items.iter().filter(|i| !is_directive(i)).collect();
        if items.is_empty() {
            continue;
        }
        push_section_heading(lines, name);
        for item in items {
            lines.push(raw_of(item).unwrap_or_default().to_string());
            push_sub_items(lines, item, &["name", "raw", "value", "args"]);
        }
    }

    if !sections.metadata.is_empty() {
        push_section_heading(lines, "Metadata");
        let mut keys: Vec<&String> = sections.metadata.keys().collect();
        keys.sort();
        for key in keys {
            lines.push(format!(
                "- {key}: {}",
                metadata_value(&sections.metadata[key])
            ));
        }
    }
}

fn push_section_heading(lines: &mut Vec<String>, name: &str) {
    lines.push(String::new());
    lines.push(format!("### {name}"));
}

fn raw_of(item: &Value) -> Option<&str> {
    item.get("raw").and_then(Value::as_str)
}

fn first_line(item: &Value) -> Option<usize> {
    item.get("loc")?.get("line")?.as_u64().map(|l| l as usize)
}

/// `- name(Label): @index(a, b)` for the labelled form, `- name` when the index is described only
/// by sub-items.
fn index_head(item: &Value) -> String {
    let name = item.get("name").and_then(Value::as_str).unwrap_or_default();
    let label = item
        .get("label")
        .and_then(Value::as_str)
        .map(str::to_string);
    let mut head = format!("- {}", labelled(name, &label));
    if let Some(attr) = item.get("attr").and_then(Value::as_str) {
        let mut args: Vec<String> = item
            .get("args")
            .and_then(Value::as_array)
            .map(|a| a.iter().map(bare_value).collect())
            .unwrap_or_default();
        if let Some(nulls) = item.get("nulls").and_then(Value::as_str) {
            args.push(format!("nulls: \"{nulls}\""));
        }
        head.push_str(&format!(": @{attr}({})", args.join(", ")));
    }
    head
}

/// Every key the head line does not account for, as a `  - key: value` sub-item.
fn push_sub_items(lines: &mut Vec<String>, item: &Value, head_keys: &[&str]) {
    let Some(obj) = item.as_object() else { return };
    for (key, value) in obj {
        if key == "loc" || head_keys.contains(&key.as_str()) {
            continue;
        }
        lines.push(format!("  - {key}: {}", nested_value(value)));
    }
}

/// A sub-item value in the shape the parser reads back: arrays bracketed, strings bare when they
/// would read back as the same string, quoted otherwise.
fn nested_value(value: &Value) -> String {
    match value {
        Value::Array(items) => format!(
            "[{}]",
            items.iter().map(bare_value).collect::<Vec<_>>().join(", ")
        ),
        other => metadata_value(other),
    }
}

/// A metadata value. A string is written bare only when it is a plain word the parser cannot
/// mistake for a number or a boolean; anything else is quoted, and the parser strips the quotes.
fn metadata_value(value: &Value) -> String {
    match value {
        Value::String(s) if is_plain_word(s) => s.clone(),
        Value::String(s) => format!("\"{s}\""),
        other => other.to_string(),
    }
}

fn is_plain_word(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
        && s != "true"
        && s != "false"
        && s.parse::<f64>().is_err()
}

fn bare_value(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn format_field(lines: &mut Vec<String>, field: &m3l_core::FieldNode, indent: usize) {
    let prefix = "  ".repeat(indent);
    let mut line = format!("{prefix}- {}", labelled(&field.name, &field.label));

    if let Some(ref ft) = field.field_type {
        line.push_str(&format!(": {ft}"));
        // `map<string, integer>` — the type arguments come before any `(params)`.
        if let Some(ref generic) = field.generic_params {
            line.push_str(&format!("<{}>", generic.join(", ")));
        }
        if let Some(ref params) = field.params {
            let param_strs: Vec<String> = params
                .iter()
                .map(|p| match p {
                    m3l_core::ParamValue::String(s) => s.clone(),
                    m3l_core::ParamValue::Number(n) => n.to_string(),
                })
                .collect();
            line.push_str(&format!("({})", param_strs.join(", ")));
        }
    }
    // `Type?[]?` — a leading `?` marks the array's items nullable, a trailing
    // `?` after `[]` marks the array itself nullable (lexer: array ⇒ nullable
    // reads the trailing marker, array_item_nullable reads the leading one).
    // For a non-array field, either marker means the scalar type is nullable.
    if field.array {
        if field.array_item_nullable {
            line.push('?');
        }
        line.push_str("[]");
        if field.nullable {
            line.push('?');
        }
    } else if field.nullable {
        line.push('?');
    }

    if let Some(ref dv) = field.default_value {
        if field.default_value_quoted == Some(true) {
            line.push_str(&format!(" = \"{dv}\""));
        } else if field.default_value_backtick == Some(true) {
            line.push_str(&format!(" = `{dv}`"));
        } else {
            line.push_str(&format!(" = {dv}"));
        }
    }

    for attr in &field.attributes {
        line.push_str(&format!(" @{}", attr.name));
        if let Some(args_str) = format_attr_args(attr) {
            line.push_str(&args_str);
        }
        // `@reference(Customer)?` — the referential-action symbol belongs to the attribute it
        // follows; without it the reference falls back to the default action.
        if let Some(ref symbol) = attr.cascade {
            line.push_str(symbol);
        }
    }

    // Framework attributes (`` `[MaxLength(70)]` ``) pass through untouched.
    for fa in field.framework_attrs.iter().flatten() {
        line.push_str(&format!(" `[{}]`", fa.content));
    }

    lines.push(line);

    // A multi-line `@computed` expression is written as a fenced block under the field, the one
    // place the parser takes an argument-less `@computed`'s expression from. It must come before
    // the description: the block attaches only to the line directly above it.
    if let Some(ref computed) = field.computed {
        let from_block = field
            .attributes
            .iter()
            .any(|a| (a.name == "computed" || a.name == "computed_raw") && a.args.is_none());
        if from_block {
            let indent = format!("{prefix}  ");
            lines.push(format!("{indent}```"));
            lines.extend(computed.expression.lines().map(|l| format!("{indent}{l}")));
            lines.push(format!("{indent}```"));
        }
    }

    // Descriptions go in blockquote continuation lines rather than a trailing
    // `# ...` comment: the comment form put a raw newline mid-line for
    // multi-line descriptions. The blockquote form is the source form and
    // handles one line and many identically.
    if let Some(ref desc) = field.description {
        push_description(lines, desc, &format!("{prefix}  "));
    }

    // Inline enum values — emitted under the recommended `values:` key so they
    // read as enum values rather than extended-format attributes (§3.1.7).
    // Without this the whole inline enum vanished on format.
    if let Some(ref enum_values) = field.enum_values {
        lines.push(format!("{prefix}  - values:"));
        for val in enum_values {
            lines.push(format_enum_value(val, indent + 2));
        }
    }

    // Nested fields
    if let Some(ref sub_fields) = field.fields {
        for sf in sub_fields {
            format_field(lines, sf, indent + 1);
        }
    }
}

fn format_enum(lines: &mut Vec<String>, e: &m3l_core::EnumNode) {
    let mut header = format!("## {} ::enum", labelled(&e.name, &e.label));
    // The parent list, for the same reason `format_model` emits a model's: it is the only place
    // the inherited values are written down. Dropping it used to cost nothing, because inheritance
    // was recorded and never acted on; now that it resolves, a format that dropped it would delete
    // members from the enum and leave a file that still parses.
    if !e.inherits.is_empty() {
        header.push_str(&format!(" : {}", e.inherits.join(", ")));
    }
    lines.push(header);
    if let Some(ref desc) = e.description {
        push_description(lines, desc, "");
    }
    for val in &e.values {
        lines.push(format_enum_value(val, 0));
    }
}

/// Render one enum value in the canonical shape
/// `- name: type = value "label" @attr(args)` (§3.1.8), omitting the parts the
/// value does not carry.
///
/// Every part must be emitted: a formatter that drops `type`/`value`/attributes
/// deletes them from the user's model on the first `m3l format`.
fn format_enum_value(val: &m3l_core::EnumValue, indent: usize) -> String {
    let prefix = "  ".repeat(indent);
    let mut rhs = String::new();

    match (&val.value_type, &val.value) {
        (Some(t), Some(v)) => rhs.push_str(&format!("{t} = {}", render_json_scalar(v))),
        (Some(t), None) => rhs.push_str(t),
        // A bare stored value with no declared type would re-parse as a *type*,
        // so it stays behind `=` to keep the two roles distinct.
        (None, Some(v)) => rhs.push_str(&format!("= {}", render_json_scalar(v))),
        (None, None) => {}
    }

    if let Some(ref desc) = val.description {
        if !rhs.is_empty() {
            rhs.push(' ');
        }
        rhs.push_str(&format!("\"{desc}\""));
    }

    if let Some(ref attrs) = val.attributes {
        for attr in attrs {
            if !rhs.is_empty() {
                rhs.push(' ');
            }
            rhs.push_str(&format!("@{}", attr.name));
            if let Some(args_str) = format_attr_args(attr) {
                rhs.push_str(&args_str);
            }
        }
    }

    // The colon form is required whenever anything follows the name: the
    // colon-less `- name "label"` shape only parses when the label is the
    // *entire* remainder, so `- name "label" @attr` would be swallowed into
    // the name.
    if rhs.is_empty() {
        format!("{prefix}- {}", val.name)
    } else {
        format!("{prefix}- {}: {rhs}", val.name)
    }
}

fn render_json_scalar(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn format_arg(arg: &m3l_core::AttrArgValue, quoted: bool) -> String {
    match arg {
        // A backtick-delimited argument's backticks are already part of `s`
        // (see `parse_attr_args_string_with_origin`), so it's never `quoted`
        // here — only a `"..."`/`'...'`-stripped string is.
        m3l_core::AttrArgValue::String(s) if quoted => format!("\"{s}\""),
        m3l_core::AttrArgValue::String(s) => s.clone(),
        m3l_core::AttrArgValue::Number(n) => n.to_string(),
        m3l_core::AttrArgValue::Bool(b) => b.to_string(),
    }
}

/// Render an attribute's `(args)` suffix, reproducing each arg's original
/// quoting from `args_quoted` (§HD-16 — `AttrArgValue` is untagged so it can't
/// carry that flag itself). `None` when the attribute has no args at all.
fn format_attr_args(attr: &m3l_core::FieldAttribute) -> Option<String> {
    let args = attr.args.as_ref()?;
    let arg_strs: Vec<String> = args
        .iter()
        .enumerate()
        .map(|(i, arg)| {
            let quoted = attr
                .args_quoted
                .as_ref()
                .and_then(|q| q.get(i))
                .copied()
                .unwrap_or(false);
            format_arg(arg, quoted)
        })
        .collect();
    Some(format!("({})", arg_strs.join(", ")))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Format a source string through the real `run_format` path (parse from a
    /// file, resolve with the formatter's `ResolveOptions`, format). A few
    /// pieces of formatter state — the file-owner prefix, unmerged `::extend`
    /// blocks — only exist once `run_format` reads from a path, so the
    /// in-memory `format_ast(&resolve(...))` shortcut the other tests use
    /// can't exercise them.
    ///
    /// The path must be unique per call: `cargo test` runs these in parallel
    /// threads, and a shared filename let two calls race on the same file —
    /// one test's `format_source` could read back another's fixture.
    fn format_source(src: &str) -> String {
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("m3l-cli-format-unit-test-{id}.m3l.md"));
        std::fs::write(&path, src).expect("write test fixture");
        let result = run_format(&path).expect("run_format");
        std::fs::remove_file(&path).ok();
        result
    }

    /// The formatter must reproduce the file-owner `# Prefix:` header, a
    /// based model's `::aspect(Base)` header, and an unmerged `::extend`
    /// block — all three were dropped before this fix (`extensions` was
    /// never printed, so `m3l format` deleted extend blocks from the file).
    #[test]
    fn format_keeps_prefix_extend_blocks_and_based_models() {
        let src = "# Namespace: ex.insp\n# Prefix: insp\n\n## InspProfile ::aspect(Asset) : Timestampable\n- level: integer?\n\n## Asset ::extend\n- insp_grade: string(20)?\n";
        let out = format_source(src);
        assert!(out.contains("# Prefix: insp"), "{out}");
        assert!(
            out.contains("## InspProfile ::aspect(Asset) : Timestampable"),
            "{out}"
        );
        assert!(
            out.contains("## Asset ::extend\n- insp_grade: string(20)?"),
            "{out}"
        );
        assert_eq!(format_source(&out), out, "formatting must be idempotent");
    }

    /// A file that declares only an interface still owns a `# Prefix:` header — the
    /// scan that recovers it for re-emission used to chain models, extend blocks and
    /// enums only, so an interface-only file lost the header on format.
    #[test]
    fn format_keeps_prefix_on_an_interface_only_file() {
        let src = "# Namespace: ex.insp\n# Prefix: insp\n\n## Timestampable ::interface\n- created_at: timestamp = now()\n";
        let out = format_source(src);
        assert!(out.contains("# Prefix: insp"), "{out}");
        assert_eq!(format_source(&out), out, "formatting must be idempotent");
    }

    /// Same gap, for a view-only file.
    #[test]
    fn format_keeps_prefix_on_a_view_only_file() {
        let src = "# Namespace: ex.insp\n# Prefix: insp\n\n## ActiveAssets ::view\n### Source\n- from: Asset\n- name_label: string @from(Asset.name)\n";
        let out = format_source(src);
        assert!(out.contains("# Prefix: insp"), "{out}");
        assert_eq!(format_source(&out), out, "formatting must be idempotent");
    }

    /// A prefixed file with no `# Namespace:` line exercises the insert-at-0 branch —
    /// the header must land as the very first line, not after a namespace that isn't
    /// there.
    #[test]
    fn format_keeps_prefix_with_no_namespace_line() {
        let src = "# Prefix: insp\n\n## InspRecord\n- id: identifier @pk\n";
        let out = format_source(src);
        assert!(out.starts_with("# Prefix: insp"), "{out}");
        assert_eq!(format_source(&out), out, "formatting must be idempotent");
    }

    /// Formatting must be information-preserving: parsing the formatted output
    /// has to yield the same enum values as parsing the original. Anything the
    /// formatter cannot re-emit is silently deleted from a user's model the
    /// first time they run `m3l format`.
    fn roundtrip_enum_values(src: &str) -> (Vec<m3l_core::EnumValue>, Vec<m3l_core::EnumValue>) {
        let ast = m3l_core::resolve(&[m3l_core::parse_string(src, "t.m3l.md")], None);
        let formatted = format_ast(&ast);
        let reparsed = m3l_core::resolve(&[m3l_core::parse_string(&formatted, "t.m3l.md")], None);
        (
            ast.enums
                .first()
                .map(|e| e.values.clone())
                .unwrap_or_default(),
            reparsed
                .enums
                .first()
                .map(|e| e.values.clone())
                .unwrap_or_default(),
        )
    }

    /// The formatter's own options, not the default ones — the round-trip helper above resolves
    /// with inheritance inlined, which is not what `m3l format` does.
    fn format_roundtrip_as_the_cli_does(src: &str) -> String {
        let opts = m3l_core::ResolveOptions {
            inline_inherited: false,
            merge_extends: false,
        };
        let ast = m3l_core::resolve_with(&[m3l_core::parse_string(src, "t.m3l.md")], None, opts);
        format_ast(&ast)
    }

    /// An enum's parent list survives a format.
    ///
    /// Until enum inheritance was resolved this was a cosmetic loss — the declaration did nothing,
    /// so dropping it changed no output. It is not cosmetic any more: the parent list is where the
    /// inherited values come from, so a formatter that drops it deletes members from every
    /// consumer of that enum, and the file still parses.
    #[test]
    fn format_preserves_an_enums_parent_list() {
        let formatted = format_roundtrip_as_the_cli_does(
            r#"## BasicStatus ::enum
- active: "Active"

## UserStatus ::enum : BasicStatus
- banned: "Banned""#,
        );

        assert!(
            formatted.contains("## UserStatus ::enum : BasicStatus"),
            "the parent list was dropped; formatted output was:
{formatted}"
        );

        // And the values still resolve after the round-trip.
        let reparsed = m3l_core::resolve(&[m3l_core::parse_string(&formatted, "t.m3l.md")], None);
        let user = reparsed
            .enums
            .iter()
            .find(|e| e.name == "UserStatus")
            .expect("UserStatus survived");
        let names: Vec<&str> = user.values.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(names, ["active", "banned"]);
    }

    /// Multiple parents keep their order, the same way a model's do.
    #[test]
    fn format_preserves_multiple_enum_parents_in_order() {
        let formatted = format_roundtrip_as_the_cli_does(
            r#"## A ::enum
- a: "A"

## B ::enum
- b: "B"

## C ::enum : A, B
- c: "C""#,
        );

        assert!(
            formatted.contains("## C ::enum : A, B"),
            "formatted output was:
{formatted}"
        );
    }

    #[test]
    fn format_preserves_enum_value_attributes() {
        let (before, after) = roundtrip_enum_values(
            "## PaymentMethod ::enum\n- cash: \"현금\"\n- legacy: \"이관 정리\" @system",
        );
        assert_eq!(before, after, "enum value attributes must survive format");
    }

    #[test]
    fn format_preserves_enum_value_type_and_stored_value() {
        let (before, after) = roundtrip_enum_values(
            "## OrderStatus ::enum\n- pending: integer = 100 \"대기\"\n- done: integer = 200 \"완료\" @system",
        );
        assert_eq!(
            before, after,
            "enum value type/stored value must survive format"
        );
    }

    #[test]
    fn format_preserves_enum_attribute_args() {
        let (before, after) = roundtrip_enum_values(
            "## Status ::enum\n- archived: \"보관\" @deprecated(\"use closed\")",
        );
        assert_eq!(before, after, "attribute args must survive format");
    }

    #[test]
    fn format_preserves_stored_value_without_type() {
        // Nested enum values can carry a stored value with no declared type.
        let (before, after) = roundtrip_enum_values("## Code ::enum\n  - a: A_1\n  - b: B_2");
        assert_eq!(before, after, "untyped stored value must survive format");
    }

    #[test]
    fn format_preserves_inline_enum_values() {
        let src = "## Order\n- method: enum = \"cash\"\n  - values:\n    - cash: \"현금\"\n    - legacy: \"이관\" @system";
        let ast = m3l_core::resolve(&[m3l_core::parse_string(src, "t.m3l.md")], None);
        let formatted = format_ast(&ast);
        let reparsed = m3l_core::resolve(&[m3l_core::parse_string(&formatted, "t.m3l.md")], None);
        assert_eq!(
            ast.models[0].fields[0].enum_values, reparsed.models[0].fields[0].enum_values,
            "inline enum values must survive format:\n{formatted}"
        );
    }

    /// `array` + `nullable`/`array_item_nullable`: parses `Type?[]?` (leading `?`
    /// = item-nullable, trailing `?` after `[]` = array-nullable). The formatter
    /// must reproduce both flags at their own position, not just one of them
    /// or the other's position.
    fn roundtrip_array_flags(field_decl: &str) -> (bool, bool) {
        let src = format!("## T\n- f: {field_decl}");
        let ast = m3l_core::resolve(&[m3l_core::parse_string(&src, "t.m3l.md")], None);
        let formatted = format_ast(&ast);
        let reparsed = m3l_core::resolve(&[m3l_core::parse_string(&formatted, "t.m3l.md")], None);
        let field = &reparsed.models[0].fields[0];
        (field.nullable, field.array_item_nullable)
    }

    #[test]
    fn format_preserves_item_nullable_without_array_nullable() {
        let (nullable, item_nullable) = roundtrip_array_flags("string?[]");
        assert!(!nullable, "array itself must stay non-nullable");
        assert!(item_nullable, "item-nullable marker must survive format");
    }

    #[test]
    fn format_preserves_array_nullable_without_item_nullable() {
        let (nullable, item_nullable) = roundtrip_array_flags("string[]?");
        assert!(nullable, "array-nullable marker must survive format");
        assert!(!item_nullable, "items must stay non-nullable");
    }

    #[test]
    fn format_preserves_both_nullable_markers_independently() {
        let (nullable, item_nullable) = roundtrip_array_flags("string?[]?");
        assert!(nullable, "array-nullable marker must survive format");
        assert!(item_nullable, "item-nullable marker must survive format");
    }

    /// A field's first attribute's `args`/`args_quoted`, round-tripped through
    /// one format pass — `AttrArgValue` is `#[serde(untagged)]`, so without
    /// `args_quoted` a quoted and bareword string arg parse to the identical
    /// value and the formatter can't tell them apart on the second pass.
    fn roundtrip_attr_args(
        field_decl: &str,
    ) -> (Option<Vec<m3l_core::AttrArgValue>>, Option<Vec<bool>>) {
        let src = format!("## T\n- f: {field_decl}");
        let ast = m3l_core::resolve(&[m3l_core::parse_string(&src, "t.m3l.md")], None);
        let formatted = format_ast(&ast);
        let reparsed = m3l_core::resolve(&[m3l_core::parse_string(&formatted, "t.m3l.md")], None);
        let attr = &reparsed.models[0].fields[0].attributes[0];
        (attr.args.clone(), attr.args_quoted.clone())
    }

    #[test]
    fn format_preserves_quoted_attribute_arg() {
        let (args, quoted) = roundtrip_attr_args(r#"string @reference("Category")"#);
        assert_eq!(
            args,
            Some(vec![m3l_core::AttrArgValue::String("Category".into())])
        );
        assert_eq!(quoted, Some(vec![true]), "quoted arg must stay quoted");
    }

    #[test]
    fn format_preserves_unquoted_attribute_arg() {
        let (args, quoted) = roundtrip_attr_args("string @reference(Category)");
        assert_eq!(
            args,
            Some(vec![m3l_core::AttrArgValue::String("Category".into())])
        );
        assert!(
            quoted.is_none_or(|q| !q[0]),
            "bareword arg must not gain quotes"
        );
    }

    /// A field's default-value shape, round-tripped through one format pass —
    /// covers a `Literal`'s quote origin and the backtick-`Expression` case.
    fn roundtrip_default_value(
        field_decl: &str,
    ) -> (
        Option<String>,
        Option<m3l_core::DefaultValueType>,
        Option<bool>,
        Option<bool>,
    ) {
        let src = format!("## T\n- f: {field_decl}");
        let ast = m3l_core::resolve(&[m3l_core::parse_string(&src, "t.m3l.md")], None);
        let formatted = format_ast(&ast);
        let reparsed = m3l_core::resolve(&[m3l_core::parse_string(&formatted, "t.m3l.md")], None);
        let field = &reparsed.models[0].fields[0];
        (
            field.default_value.clone(),
            field.default_value_type.clone(),
            field.default_value_quoted,
            field.default_value_backtick,
        )
    }

    #[test]
    fn format_preserves_quoted_default_literal() {
        let (value, ty, quoted, backtick) = roundtrip_default_value(r#"string = "active""#);
        assert_eq!(value.as_deref(), Some("active"));
        assert_eq!(ty, Some(m3l_core::DefaultValueType::Literal));
        assert_eq!(quoted, Some(true), "quoted default must stay quoted");
        assert_eq!(backtick, None);
    }

    #[test]
    fn format_preserves_bareword_default_literal() {
        let (value, ty, quoted, backtick) = roundtrip_default_value("string = active");
        assert_eq!(value.as_deref(), Some("active"));
        assert_eq!(ty, Some(m3l_core::DefaultValueType::Literal));
        assert_eq!(quoted, None, "bareword default must not gain quotes");
        assert_eq!(backtick, None);
    }

    #[test]
    fn format_preserves_backtick_default_expression() {
        // Bare `price * qty` re-parses past its first non-word character —
        // without the backtick restored, the second format pass silently
        // truncates the default to `price`.
        let (value, ty, quoted, backtick) = roundtrip_default_value("decimal = `price * qty`");
        assert_eq!(value.as_deref(), Some("price * qty"));
        assert_eq!(ty, Some(m3l_core::DefaultValueType::Expression));
        assert_eq!(quoted, None);
        assert_eq!(
            backtick,
            Some(true),
            "backtick expression must keep its delimiters"
        );
    }

    #[test]
    fn format_preserves_paren_default_expression() {
        // `now()` is Expression too (parenthesized call), but never had
        // backticks — must not gain them.
        let (value, ty, quoted, backtick) = roundtrip_default_value("timestamp = now()");
        assert_eq!(value.as_deref(), Some("now()"));
        assert_eq!(ty, Some(m3l_core::DefaultValueType::Expression));
        assert_eq!(quoted, None);
        assert_eq!(
            backtick, None,
            "paren-call expression must not gain backticks"
        );
    }
}

/// `m3l format` must not change what a file means: parse → format → parse has to give back the
/// same AST, source positions aside. Anything the formatter fails to write back is deleted from
/// the user's file by `m3l format` and leaves one that still parses, so nothing else catches it —
/// `format_idempotent` compares two formatted outputs and a first-pass loss is in both.
#[cfg(test)]
mod lossless {
    use super::format_ast;
    use serde_json::Value;

    fn options() -> m3l_core::ResolveOptions {
        m3l_core::ResolveOptions {
            inline_inherited: false,
            merge_extends: false,
        }
    }

    /// Source positions legitimately move when a file is reformatted.
    fn strip_positions(v: &mut Value) {
        match v {
            Value::Object(m) => {
                for k in ["loc", "line", "col"] {
                    m.remove(k);
                }
                m.values_mut().for_each(strip_positions);
            }
            Value::Array(a) => a.iter_mut().for_each(strip_positions),
            _ => {}
        }
    }

    fn diff(path: &str, a: &Value, b: &Value, out: &mut Vec<String>) {
        if a == b {
            return;
        }
        match (a, b) {
            (Value::Object(x), Value::Object(y)) => {
                let keys: std::collections::BTreeSet<&String> = x.keys().chain(y.keys()).collect();
                for k in keys {
                    let (l, r) = (
                        x.get(k).unwrap_or(&Value::Null),
                        y.get(k).unwrap_or(&Value::Null),
                    );
                    diff(&format!("{path}.{k}"), l, r, out);
                }
            }
            (Value::Array(x), Value::Array(y)) if x.len() == y.len() => {
                for (i, (l, r)) in x.iter().zip(y).enumerate() {
                    diff(&format!("{path}[{i}]"), l, r, out);
                }
            }
            _ => out.push(format!("{path}: {a} => {b}")),
        }
    }

    /// The AST paths that differ after one format round-trip.
    fn round_trip_losses(src: &str) -> Vec<String> {
        let before =
            m3l_core::resolve_with(&[m3l_core::parse_string(src, "t.m3l.md")], None, options());
        let formatted = format_ast(&before);
        let after = m3l_core::resolve_with(
            &[m3l_core::parse_string(&formatted, "t.m3l.md")],
            None,
            options(),
        );
        let mut a = serde_json::to_value(&before).unwrap();
        let mut b = serde_json::to_value(&after).unwrap();
        strip_positions(&mut a);
        strip_positions(&mut b);
        let mut out = Vec::new();
        diff("", &a, &b, &mut out);
        out
    }

    #[test]
    fn labels_survive_a_round_trip() {
        let losses = round_trip_losses(
            "# Namespace: t\n\n\
             ## Status(Order Status) ::enum\n- open: \"Open\"\n\n\
             ## Timestamped(Timestamps) ::interface\n- created_at: timestamp\n\n\
             ## Order(Sales Order) : Timestamped\n- id(Order ID): identifier @pk\n\
             - lines(Line Items): object[]\n  - sku(SKU): string\n\n\
             ## OpenOrders(Open Orders) ::view\n- id: identifier\n",
        );
        assert!(losses.is_empty(), "{losses:#?}");
    }

    #[test]
    fn field_line_constructs_survive_a_round_trip() {
        let losses = round_trip_losses(
            "# Namespace: t\n\n\
             ## Customer\n- id: identifier @pk\n\n\
             ## Order\n- id: identifier @pk\n\
             - customer_id: identifier? @reference(Customer)?\n\
             - owner_id: identifier @reference(Customer)!!\n\
             - tags: map<string, integer>\n\
             - secret: string(100) `[JsonIgnore]` `[MaxLength(100)]`\n",
        );
        assert!(losses.is_empty(), "{losses:#?}");
    }

    #[test]
    fn model_sections_survive_a_round_trip() {
        // Joined rather than written with `\` continuations, which would strip the
        // sub-items' indentation.
        let src = [
            "# Namespace: t",
            "",
            "## Order",
            "- id: identifier @pk",
            "- customer_id: identifier",
            "- code: string(10)?",
            "- @index(customer_id)",
            "- @behavior(before_create, stamp)",
            "",
            "### Indexes",
            "- idx_customer",
            "  - fields: [customer_id, code]",
            "  - where: \"code IS NOT NULL\"",
            "- uk_code(Unique code): @unique(code, nulls: \"not_distinct\")",
            "",
            "### Relations",
            "- customer: >Customer via customer_id",
            "  - on_delete: cascade",
            "",
            "### Behaviors",
            "- before_update: touch",
            "  - condition: always",
            "",
            "### Metadata",
            "- table_name: orders",
            "- cache_ttl: 300",
            "- audited: true",
            "- version_tag: \"2026\"",
            "",
            "### Validations",
            "- code_shape",
            "  - rule: \"LEN(code) = 10\"",
            "",
            "### Version",
            "- major: 2",
        ]
        .join("\n");
        let losses = round_trip_losses(&src);
        assert!(losses.is_empty(), "{losses:#?}");
    }

    #[test]
    fn views_computed_blocks_and_the_attribute_registry_survive_a_round_trip() {
        let src = [
            "# Namespace: t",
            "",
            "## tracked(Tracked) ::attribute",
            "> Audit marker",
            "- target: [field, model]",
            "- type: integer",
            "- range: [1, 5]",
            "- required: true",
            "- default: 3",
            "",
            "## Customer",
            "- id: identifier @pk",
            "- name: string(50) @tracked(2)",
            "- score: decimal @computed",
            "  ```",
            "  CASE",
            "    WHEN id IS NULL THEN 0",
            "    ELSE 1",
            "  END",
            "  ```",
            "  > The score.",
            "",
            "## Order",
            "- id: identifier @pk",
            "- customer_id: identifier @reference(Customer)",
            "",
            "## Recent(Recent orders) ::view @materialized",
            "> Orders of the last week.",
            "",
            "### Source",
            "- from: Order",
            "- join: \"Customer on Customer.id = Order.customer_id\"",
            "- where: \"id IS NOT NULL\"",
            "- group_by: \"[customer_id, id]\"",
            "- order_by: \"id desc\"",
            "- order_id: identifier @from(Order.id)",
            "",
            "### Refresh",
            "- strategy: incremental",
            "- interval: \"1 hour\"",
            "",
            "## Report ::view",
            "### Source",
            "```sql",
            "FROM Order o",
            "WHERE o.id IS NOT NULL",
            "```",
            "",
            "- order_id: identifier @from(Order.id)",
        ]
        .join("\n");
        let losses = round_trip_losses(&src);
        assert!(losses.is_empty(), "{losses:#?}");
    }

    #[test]
    fn shared_inputs_round_trip_without_loss() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut inputs: Vec<std::path::PathBuf> =
            std::fs::read_dir(root.join("spec/conformance/inputs"))
                .unwrap()
                .map(|e| e.unwrap().path())
                .filter(|p| p.extension().is_some_and(|x| x == "md"))
                .collect();
        inputs.push(root.join("samples/test/format/full.m3l.md"));

        let mut failures = Vec::new();
        for path in &inputs {
            let losses = round_trip_losses(&std::fs::read_to_string(path).unwrap());
            if !losses.is_empty() {
                failures.push(format!("{}:\n  {}", path.display(), losses.join("\n  ")));
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }
}
