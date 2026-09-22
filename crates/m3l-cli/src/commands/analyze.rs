use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::build_ast;

pub fn run_analyze(input_path: &Path, format: &str) -> Result<String, String> {
    analyze_ast(&build_ast(input_path)?, format)
}

/// The graph itself, split from file loading the way `format_ast` is, so what the command emits can be asserted without a fixture file on disk.
fn analyze_ast(ast: &m3l_core::M3lAst, format: &str) -> Result<String, String> {
    // Collect all defined model/enum/interface/view names
    let mut defined_names: HashSet<String> = HashSet::new();
    for m in ast
        .models
        .iter()
        .chain(ast.interfaces.iter())
        .chain(ast.views.iter())
    {
        defined_names.insert(m.name.clone());
    }
    for e in &ast.enums {
        defined_names.insert(e.name.clone());
    }

    // edges: (source_model, target_model, relation_type)
    let mut edges: Vec<(String, String, String)> = Vec::new();

    for m in ast
        .models
        .iter()
        .chain(ast.interfaces.iter())
        .chain(ast.views.iter())
    {
        // Inheritance edges
        for parent in &m.inherits {
            if defined_names.contains(parent.as_str()) {
                edges.push((m.name.clone(), parent.clone(), "inherits".into()));
            }
        }

        // Field type references and attribute references
        collect_field_edges(&m.name, &m.fields, &defined_names, &mut edges);
    }

    // Enums inherit too (§3.1.6), and an enum is already a node here — only the edge was
    // missing. While inheritance was recorded and never acted on, a graph without it was merely
    // incomplete; now that a parent supplies the child's values, a graph that omits the edge
    // shows the child as a standalone enum with fewer members than it has.
    for e in &ast.enums {
        for parent in &e.inherits {
            if defined_names.contains(parent.as_str()) {
                edges.push((e.name.clone(), parent.clone(), "inherits".into()));
            }
        }
    }

    // Deduplicate edges
    edges.sort();
    edges.dedup();

    match format {
        "dot" => Ok(render_dot(&defined_names, &edges)),
        _ => Ok(render_mermaid(&defined_names, &edges)),
    }
}

fn collect_field_edges(
    model_name: &str,
    fields: &[m3l_core::FieldNode],
    defined_names: &HashSet<String>,
    edges: &mut Vec<(String, String, String)>,
) {
    for field in fields {
        // Field type → model/enum reference
        if let Some(ref ft) = field.field_type {
            let type_name = resolve_type_name(ft);
            if defined_names.contains(&type_name) && type_name != model_name {
                edges.push((model_name.to_string(), type_name, "type_ref".into()));
            }
        }

        // @reference / @fk attributes → target from first arg
        for attr in &field.attributes {
            if attr.name == "reference" || attr.name == "fk" {
                if let Some(ref args) = attr.args {
                    if let Some(m3l_core::AttrArgValue::String(target)) = args.first() {
                        let target_model = target.split('.').next().unwrap_or(target);
                        if defined_names.contains(target_model) && target_model != model_name {
                            edges.push((
                                model_name.to_string(),
                                target_model.to_string(),
                                attr.name.clone(),
                            ));
                        }
                    }
                }
            }
        }

        // Recurse into nested fields
        if let Some(ref sub_fields) = field.fields {
            collect_field_edges(model_name, sub_fields, defined_names, edges);
        }
    }
}

/// Extract the simple type name from a type string (e.g. "Auth.User" → "User", "User" → "User")
fn resolve_type_name(type_str: &str) -> String {
    if let Some(dot_pos) = type_str.rfind('.') {
        type_str[dot_pos + 1..].to_string()
    } else {
        type_str.to_string()
    }
}

fn render_mermaid(defined_names: &HashSet<String>, edges: &[(String, String, String)]) -> String {
    let mut lines = vec!["graph LR".to_string()];

    // Nodes — group by whether they have edges
    let mut referenced: HashSet<&str> = HashSet::new();
    for (src, tgt, _) in edges {
        referenced.insert(src);
        referenced.insert(tgt);
    }

    // Add isolated nodes
    for name in defined_names {
        if !referenced.contains(name.as_str()) {
            lines.push(format!("    {name}"));
        }
    }

    // Edges with labels
    let edge_labels: HashMap<&str, &str> = HashMap::from([
        ("inherits", "inherits"),
        ("type_ref", "has"),
        ("reference", "ref"),
        ("fk", "fk"),
    ]);

    for (src, tgt, rel) in edges {
        let rel_str = rel.as_str();
        let label = edge_labels.get(rel_str).unwrap_or(&rel_str);
        lines.push(format!("    {src} -->|{label}| {tgt}"));
    }

    // Summary comment
    let node_count = defined_names.len();
    let edge_count = edges.len();
    lines.push(format!("%% {node_count} nodes, {edge_count} edges"));

    lines.join("\n")
}

fn render_dot(defined_names: &HashSet<String>, edges: &[(String, String, String)]) -> String {
    let mut lines = vec![
        "digraph M3L {".to_string(),
        "    rankdir=LR;".to_string(),
        "    node [shape=box, style=filled, fillcolor=lightyellow];".to_string(),
    ];

    // Nodes
    for name in defined_names {
        lines.push(format!("    \"{name}\";"));
    }

    // Edges
    let edge_styles: HashMap<&str, &str> = HashMap::from([
        ("inherits", "style=dashed, color=blue"),
        ("type_ref", "color=black"),
        ("reference", "color=red"),
        ("fk", "color=green"),
    ]);

    for (src, tgt, rel) in edges {
        let style = edge_styles.get(rel.as_str()).unwrap_or(&"color=gray");
        lines.push(format!(
            "    \"{src}\" -> \"{tgt}\" [label=\"{rel}\", {style}];",
        ));
    }

    lines.push("}".to_string());
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph(src: &str) -> String {
        let ast = m3l_core::resolve(&[m3l_core::parse_string(src, "t.m3l.md")], None);
        analyze_ast(&ast, "mermaid").expect("analyze")
    }

    /// An enum's parent is drawn, the same as a model's.
    ///
    /// While inheritance was recorded and never acted on, leaving it out made the graph merely
    /// incomplete. Now that a parent supplies the child's values, a child drawn with no edge reads
    /// as a standalone enum holding fewer members than it has.
    #[test]
    fn an_enums_parent_is_an_inherits_edge() {
        let out = graph(
            r#"## BasicStatus ::enum
- active: "Active"

## UserStatus ::enum : BasicStatus
- banned: "Banned"
"#,
        );

        assert!(
            out.contains("UserStatus -->|inherits| BasicStatus"),
            "graph was:
{out}"
        );
    }

    /// The negative control: an enum with no parent gets no edge, so the assertion above is
    /// reporting the parent list rather than an edge drawn for every enum.
    #[test]
    fn an_enum_without_a_parent_gets_no_inherits_edge() {
        let out = graph(
            r#"## Standalone ::enum
- only: "Only"
"#,
        );

        assert!(
            !out.contains("|inherits|"),
            "graph was:
{out}"
        );
    }
}
