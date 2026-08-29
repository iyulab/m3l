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
        lines.push(format!("## {} ::interface", iface.name));
        format_model_body(&mut lines, iface);
        lines.push(String::new());
    }

    // Views
    for view in &ast.views {
        lines.push(format!("## {} ::view", view.name));
        format_model_body(&mut lines, view);
        lines.push(String::new());
    }

    // Remove trailing empty lines
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }

    lines.join("\n")
}

fn format_model(lines: &mut Vec<String>, model: &m3l_core::ModelNode) {
    let mut header = format!("## {}", model.name);
    if !model.inherits.is_empty() {
        header.push_str(&format!(" : {}", model.inherits.join(", ")));
    }
    if !model.attributes.is_empty() {
        for attr in &model.attributes {
            header.push_str(&format!(" @{}", attr.name));
            if let Some(ref args) = attr.args {
                let arg_strs: Vec<String> = args.iter().map(format_arg).collect();
                header.push_str(&format!("({})", arg_strs.join(", ")));
            }
        }
    }
    lines.push(header);
    format_model_body(lines, model);
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
}

fn format_field(lines: &mut Vec<String>, field: &m3l_core::FieldNode, indent: usize) {
    let prefix = "  ".repeat(indent);
    let mut line = format!("{prefix}- {}", field.name);

    if let Some(ref ft) = field.field_type {
        line.push_str(&format!(": {ft}"));
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
        line.push_str(&format!(" = {dv}"));
    }

    for attr in &field.attributes {
        line.push_str(&format!(" @{}", attr.name));
        if let Some(ref args) = attr.args {
            let arg_strs: Vec<String> = args.iter().map(format_arg).collect();
            line.push_str(&format!("({})", arg_strs.join(", ")));
        }
    }

    lines.push(line);

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
    lines.push(format!("## {} ::enum", e.name));
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
            if let Some(ref args) = attr.args {
                let arg_strs: Vec<String> = args.iter().map(format_arg).collect();
                rhs.push_str(&format!("({})", arg_strs.join(", ")));
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

fn format_arg(arg: &m3l_core::AttrArgValue) -> String {
    match arg {
        m3l_core::AttrArgValue::String(s) => s.clone(),
        m3l_core::AttrArgValue::Number(n) => n.to_string(),
        m3l_core::AttrArgValue::Bool(b) => b.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    /// or the other's position (ISSUE-m3l-20260829-format-roundtrip-fidelity-gaps).
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
}
