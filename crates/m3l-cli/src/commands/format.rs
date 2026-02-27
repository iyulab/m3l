use std::path::Path;

use crate::build_ast;

pub fn run_format(input_path: &Path) -> Result<String, String> {
    let ast = build_ast(input_path)?;
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

fn format_model_body(lines: &mut Vec<String>, model: &m3l_core::ModelNode) {
    if let Some(ref desc) = model.description {
        lines.push(format!("> {desc}"));
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
    if field.nullable {
        line.push('?');
    }
    if field.array {
        line.push_str("[]");
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

    if let Some(ref desc) = field.description {
        line.push_str(&format!(" # {desc}"));
    }

    lines.push(line);

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
        lines.push(format!("> {desc}"));
    }
    for val in &e.values {
        let mut line = format!("- {}", val.name);
        if let Some(ref desc) = val.description {
            line.push_str(&format!(" \"{desc}\""));
        }
        lines.push(line);
    }
}

fn format_arg(arg: &m3l_core::AttrArgValue) -> String {
    match arg {
        m3l_core::AttrArgValue::String(s) => s.clone(),
        m3l_core::AttrArgValue::Number(n) => n.to_string(),
        m3l_core::AttrArgValue::Bool(b) => b.to_string(),
    }
}
