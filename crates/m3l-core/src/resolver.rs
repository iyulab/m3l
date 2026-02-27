use std::collections::{HashMap, HashSet};

use crate::catalogs::{AST_VERSION, PARSER_VERSION};
use crate::types::*;

/// Resolve and merge multiple parsed file ASTs into a single M3lAst.
/// Handles: inheritance resolution, duplicate detection, attribute registry tagging.
pub fn resolve(files: &[ParsedFile], project: Option<ProjectInfo>) -> M3lAst {
    let mut errors: Vec<Diagnostic> = Vec::new();
    let warnings: Vec<Diagnostic> = Vec::new();

    // Collect all elements from all files
    let mut all_models: Vec<ModelNode> = Vec::new();
    let mut all_enums: Vec<EnumNode> = Vec::new();
    let mut all_interfaces: Vec<ModelNode> = Vec::new();
    let mut all_views: Vec<ModelNode> = Vec::new();
    let mut all_attr_registry: Vec<AttributeRegistryEntry> = Vec::new();
    let mut sources: Vec<String> = Vec::new();

    for file in files {
        sources.push(file.source.clone());
        all_models.extend(file.models.iter().cloned());
        all_enums.extend(file.enums.iter().cloned());
        all_interfaces.extend(file.interfaces.iter().cloned());
        all_views.extend(file.views.iter().cloned());
        all_attr_registry.extend(file.attribute_registry.iter().cloned());
    }

    // Build source → namespace map for E008 ambiguity detection
    let source_ns: HashMap<&str, Option<&str>> = files
        .iter()
        .map(|f| (f.source.as_str(), f.namespace.as_deref()))
        .collect();

    // name → Vec<(namespace, file, line)> for E008 cross-namespace ambiguity
    let mut name_ns_map: HashMap<String, Vec<(Option<String>, String, usize)>> = HashMap::new();

    // Build name maps and check duplicates
    let mut model_map: HashMap<String, usize> = HashMap::new(); // name → index in all_models
    let mut interface_map: HashMap<String, usize> = HashMap::new();
    let mut all_named: HashMap<String, (String, String, usize)> = HashMap::new(); // name → (type, file, line)

    for (i, model) in all_models.iter().enumerate() {
        check_duplicate(
            &model.name,
            "model",
            &model.source,
            model.line,
            &all_named,
            &mut errors,
        );
        model_map.insert(model.name.clone(), i);
        all_named.insert(
            model.name.clone(),
            ("model".into(), model.source.clone(), model.line),
        );
        let ns = source_ns
            .get(model.source.as_str())
            .copied()
            .flatten()
            .map(String::from);
        name_ns_map.entry(model.name.clone()).or_default().push((
            ns,
            model.source.clone(),
            model.line,
        ));
    }

    for en in all_enums.iter() {
        check_duplicate(
            &en.name,
            "enum",
            &en.source,
            en.line,
            &all_named,
            &mut errors,
        );
        all_named.insert(en.name.clone(), ("enum".into(), en.source.clone(), en.line));
        let ns = source_ns
            .get(en.source.as_str())
            .copied()
            .flatten()
            .map(String::from);
        name_ns_map
            .entry(en.name.clone())
            .or_default()
            .push((ns, en.source.clone(), en.line));
    }

    for (i, iface) in all_interfaces.iter().enumerate() {
        check_duplicate(
            &iface.name,
            "interface",
            &iface.source,
            iface.line,
            &all_named,
            &mut errors,
        );
        interface_map.insert(iface.name.clone(), i);
        all_named.insert(
            iface.name.clone(),
            ("interface".into(), iface.source.clone(), iface.line),
        );
        let ns = source_ns
            .get(iface.source.as_str())
            .copied()
            .flatten()
            .map(String::from);
        name_ns_map.entry(iface.name.clone()).or_default().push((
            ns,
            iface.source.clone(),
            iface.line,
        ));
    }

    for view in &all_views {
        all_named.insert(
            view.name.clone(),
            ("view".into(), view.source.clone(), view.line),
        );
        let ns = source_ns
            .get(view.source.as_str())
            .copied()
            .flatten()
            .map(String::from);
        name_ns_map.entry(view.name.clone()).or_default().push((
            ns,
            view.source.clone(),
            view.line,
        ));
    }

    // E008: Ambiguous model reference — same name in multiple namespaces
    for (name, entries) in &name_ns_map {
        if entries.len() < 2 {
            continue;
        }
        // Collect distinct namespaces (only if at least 2 different namespaces)
        let distinct_ns: HashSet<Option<&str>> =
            entries.iter().map(|(ns, _, _)| ns.as_deref()).collect();
        if distinct_ns.len() >= 2 {
            let ns_list: Vec<String> = distinct_ns
                .iter()
                .map(|ns| ns.unwrap_or("(none)").to_string())
                .collect();
            let ns_display = ns_list.join(", ");
            // Report on the second occurrence
            let (_, file, line) = &entries[1];
            errors.push(Diagnostic {
                code: "M3L-E008".to_string(),
                severity: DiagnosticSeverity::Error,
                file: file.clone(),
                line: *line,
                col: 1,
                message: format!(
                    "Ambiguous model reference \"{}\" in namespaces {}",
                    name, ns_display
                ),
            });
        }
    }

    // Resolve inheritance
    for i in 0..all_models.len() {
        resolve_inheritance(
            i,
            &mut all_models,
            &model_map,
            &all_interfaces,
            &interface_map,
            &all_named,
            &mut errors,
        );
    }

    // Check duplicate field names
    for model in all_models.iter().chain(all_views.iter()) {
        check_duplicate_fields(model, &mut errors);
    }

    // Tag isRegistered on attributes matching the registry
    if !all_attr_registry.is_empty() {
        let registered_names: HashSet<String> =
            all_attr_registry.iter().map(|r| r.name.clone()).collect();

        let tag_attrs = |attrs: &mut [FieldAttribute]| {
            for a in attrs.iter_mut() {
                if registered_names.contains(&a.name) {
                    a.is_registered = Some(true);
                }
            }
        };

        for m in all_models.iter_mut() {
            tag_attrs(&mut m.attributes);
            for f in m.fields.iter_mut() {
                tag_attrs(&mut f.attributes);
            }
        }
        for v in all_views.iter_mut() {
            tag_attrs(&mut v.attributes);
            for f in v.fields.iter_mut() {
                tag_attrs(&mut f.attributes);
            }
        }
        for iface in all_interfaces.iter_mut() {
            tag_attrs(&mut iface.attributes);
            for f in iface.fields.iter_mut() {
                tag_attrs(&mut f.attributes);
            }
        }
    }

    // Detect circular imports (E003)
    let file_imports: Vec<(&str, &[String])> = files
        .iter()
        .map(|f| (f.source.as_str(), f.imports.as_slice()))
        .collect();
    let circular_errors = detect_circular_imports_internal(&file_imports);
    errors.extend(circular_errors);

    // Project info
    let mut project_info = project.unwrap_or(ProjectInfo {
        name: None,
        version: None,
    });
    if project_info.name.is_none() {
        project_info.name = files.iter().find_map(|f| f.namespace.clone());
    }

    M3lAst {
        parser_version: PARSER_VERSION.to_string(),
        ast_version: AST_VERSION.to_string(),
        project: project_info,
        sources,
        models: all_models,
        enums: all_enums,
        interfaces: all_interfaces,
        views: all_views,
        attribute_registry: all_attr_registry,
        errors,
        warnings,
    }
}

fn check_duplicate(
    name: &str,
    kind: &str,
    source: &str,
    line: usize,
    all_named: &HashMap<String, (String, String, usize)>,
    errors: &mut Vec<Diagnostic>,
) {
    if let Some((_, existing_file, existing_line)) = all_named.get(name) {
        errors.push(Diagnostic {
            code: "M3L-E005".to_string(),
            severity: DiagnosticSeverity::Error,
            file: source.to_string(),
            line,
            col: 1,
            message: format!(
                "Duplicate {} name \"{}\" (first defined in {}:{})",
                kind, name, existing_file, existing_line
            ),
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn resolve_inheritance(
    model_idx: usize,
    all_models: &mut [ModelNode],
    model_map: &HashMap<String, usize>,
    all_interfaces: &[ModelNode],
    interface_map: &HashMap<String, usize>,
    all_named: &HashMap<String, (String, String, usize)>,
    errors: &mut Vec<Diagnostic>,
) {
    let inherits = all_models[model_idx].inherits.clone();
    if inherits.is_empty() {
        return;
    }

    let mut inherited_fields: Vec<FieldNode> = Vec::new();
    let mut resolved: HashSet<String> = HashSet::new();
    let mut visiting: HashSet<String> = HashSet::new();

    let model_source = all_models[model_idx].source.clone();
    let model_line = all_models[model_idx].line;
    let model_name = all_models[model_idx].name.clone();

    fn collect_fields(
        name: &str,
        model_source: &str,
        model_line: usize,
        model_name: &str,
        all_models: &[ModelNode],
        model_map: &HashMap<String, usize>,
        all_interfaces: &[ModelNode],
        interface_map: &HashMap<String, usize>,
        all_named: &HashMap<String, (String, String, usize)>,
        inherited_fields: &mut Vec<FieldNode>,
        resolved: &mut HashSet<String>,
        visiting: &mut HashSet<String>,
        errors: &mut Vec<Diagnostic>,
    ) {
        if resolved.contains(name) || visiting.contains(name) {
            return;
        }
        visiting.insert(name.to_string());

        let parent = model_map
            .get(name)
            .map(|&idx| &all_models[idx])
            .or_else(|| interface_map.get(name).map(|&idx| &all_interfaces[idx]));

        match parent {
            None => {
                if !all_named.contains_key(name) {
                    errors.push(Diagnostic {
                        code: "M3L-E007".to_string(),
                        severity: DiagnosticSeverity::Error,
                        file: model_source.to_string(),
                        line: model_line,
                        col: 1,
                        message: format!(
                            "Unresolved inheritance reference \"{}\" in model \"{}\"",
                            name, model_name
                        ),
                    });
                }
                visiting.remove(name);
                return;
            }
            Some(parent_model) => {
                // Resolve grandparents first
                let parent_inherits = parent_model.inherits.clone();
                for grandparent in &parent_inherits {
                    collect_fields(
                        grandparent,
                        model_source,
                        model_line,
                        model_name,
                        all_models,
                        model_map,
                        all_interfaces,
                        interface_map,
                        all_named,
                        inherited_fields,
                        resolved,
                        visiting,
                        errors,
                    );
                }

                // Add parent's fields
                let parent_fields = parent_model.fields.clone();
                for field in &parent_fields {
                    if !inherited_fields.iter().any(|f| f.name == field.name) {
                        inherited_fields.push(field.clone());
                    }
                }
            }
        }

        visiting.remove(name);
        resolved.insert(name.to_string());
    }

    for parent_name in &inherits {
        collect_fields(
            parent_name,
            &model_source,
            model_line,
            &model_name,
            all_models,
            model_map,
            all_interfaces,
            interface_map,
            all_named,
            &mut inherited_fields,
            &mut resolved,
            &mut visiting,
            errors,
        );
    }

    // Handle @override
    let override_names: HashSet<String> = all_models[model_idx]
        .fields
        .iter()
        .filter(|f| f.attributes.iter().any(|a| a.name == "override"))
        .map(|f| f.name.clone())
        .collect();

    let filtered_inherited: Vec<FieldNode> = inherited_fields
        .into_iter()
        .filter(|f| !override_names.contains(&f.name))
        .collect();

    // Prepend inherited fields
    if !filtered_inherited.is_empty() {
        let own_fields = std::mem::take(&mut all_models[model_idx].fields);
        all_models[model_idx].fields = filtered_inherited;
        all_models[model_idx].fields.extend(own_fields);
    }
}

fn check_duplicate_fields(model: &ModelNode, errors: &mut Vec<Diagnostic>) {
    let mut seen: HashMap<String, usize> = HashMap::new(); // name → line
    for field in &model.fields {
        if let Some(&existing_line) = seen.get(&field.name) {
            let model_type = match model.model_type {
                ModelType::Model => "model",
                ModelType::View => "view",
                ModelType::Interface => "interface",
                ModelType::Enum => "enum",
            };
            errors.push(Diagnostic {
                code: "M3L-E005".to_string(),
                severity: DiagnosticSeverity::Error,
                file: field.loc.file.clone(),
                line: field.loc.line,
                col: 1,
                message: format!(
                    "Duplicate field name \"{}\" in {} \"{}\" (first at line {})",
                    field.name, model_type, model.name, existing_line
                ),
            });
        } else {
            seen.insert(field.name.clone(), field.loc.line);
        }
    }
}

/// Detect circular imports in a set of parsed files.
///
/// Takes a list of (source_path, import_paths) pairs and returns diagnostics
/// for any cycles detected using DFS.
pub fn detect_circular_imports(file_imports: &[(String, Vec<String>)]) -> Vec<Diagnostic> {
    let refs: Vec<(&str, &[String])> = file_imports
        .iter()
        .map(|(src, imports)| (src.as_str(), imports.as_slice()))
        .collect();
    detect_circular_imports_internal(&refs)
}

fn detect_circular_imports_internal(file_imports: &[(&str, &[String])]) -> Vec<Diagnostic> {
    let mut errors = Vec::new();

    // Build adjacency map: source → imports
    let adj: HashMap<&str, &[String]> = file_imports
        .iter()
        .map(|&(src, imports)| (src, imports))
        .collect();

    let mut visited: HashSet<&str> = HashSet::new();
    let mut rec_stack: Vec<&str> = Vec::new();

    for &(src, _) in file_imports {
        if !visited.contains(src) {
            dfs_detect_cycle(src, &adj, &mut visited, &mut rec_stack, &mut errors);
        }
    }

    errors
}

fn dfs_detect_cycle<'a>(
    node: &'a str,
    adj: &HashMap<&'a str, &'a [String]>,
    visited: &mut HashSet<&'a str>,
    rec_stack: &mut Vec<&'a str>,
    errors: &mut Vec<Diagnostic>,
) {
    visited.insert(node);
    rec_stack.push(node);

    if let Some(imports) = adj.get(node) {
        for import in *imports {
            let import_str = import.as_str();
            if !visited.contains(import_str) {
                if adj.contains_key(import_str) {
                    dfs_detect_cycle(import_str, adj, visited, rec_stack, errors);
                }
            } else if rec_stack.contains(&import_str) {
                // Found a cycle — build the chain
                let cycle_start = rec_stack.iter().position(|&n| n == import_str).unwrap();
                let chain: Vec<&str> = rec_stack[cycle_start..].to_vec();
                let chain_str = chain
                    .iter()
                    .chain(std::iter::once(&import_str))
                    .cloned()
                    .collect::<Vec<&str>>()
                    .join(" → ");

                errors.push(Diagnostic {
                    code: "M3L-E003".to_string(),
                    severity: DiagnosticSeverity::Error,
                    file: node.to_string(),
                    line: 1,
                    col: 1,
                    message: format!("Circular import detected: {}", chain_str),
                });
            }
        }
    }

    rec_stack.pop();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_string;

    #[test]
    fn resolve_single_file() {
        let parsed = parse_string(
            "## User\n- id: identifier @pk\n- name: string",
            "test.m3l.md",
        );
        let ast = resolve(&[parsed], None);
        assert_eq!(ast.models.len(), 1);
        assert_eq!(ast.models[0].name, "User");
        assert!(ast.errors.is_empty());
    }

    #[test]
    fn resolve_multiple_files() {
        let f1 = parse_string("## User\n- id: identifier", "a.m3l.md");
        let f2 = parse_string("## Product\n- id: identifier", "b.m3l.md");
        let ast = resolve(&[f1, f2], None);
        assert_eq!(ast.models.len(), 2);
        assert_eq!(ast.sources.len(), 2);
    }

    #[test]
    fn resolve_inheritance() {
        let input = "## Timestampable ::interface\n- created_at: timestamp\n- updated_at: timestamp\n\n## User : Timestampable\n- id: identifier";
        let parsed = parse_string(input, "test.m3l.md");
        let ast = resolve(&[parsed], None);

        assert_eq!(ast.models.len(), 1);
        assert_eq!(ast.models[0].name, "User");
        // Should have inherited fields + own field
        assert_eq!(ast.models[0].fields.len(), 3);
        assert_eq!(ast.models[0].fields[0].name, "created_at");
        assert_eq!(ast.models[0].fields[1].name, "updated_at");
        assert_eq!(ast.models[0].fields[2].name, "id");
    }

    #[test]
    fn resolve_duplicate_model() {
        let f1 = parse_string("## User\n- id: identifier", "a.m3l.md");
        let f2 = parse_string("## User\n- name: string", "b.m3l.md");
        let ast = resolve(&[f1, f2], None);
        assert!(ast.errors.iter().any(|e| e.code == "M3L-E005"));
    }

    #[test]
    fn resolve_unresolved_parent() {
        let parsed = parse_string("## User : NonExistent\n- id: identifier", "test.m3l.md");
        let ast = resolve(&[parsed], None);
        assert!(ast.errors.iter().any(|e| e.code == "M3L-E007"));
    }

    #[test]
    fn resolve_namespace_as_project() {
        let parsed = parse_string(
            "# Namespace: sample.ecommerce\n## User\n- id: identifier",
            "test.m3l.md",
        );
        let ast = resolve(&[parsed], None);
        assert_eq!(ast.project.name.as_deref(), Some("sample.ecommerce"));
    }

    #[test]
    fn resolve_attribute_registry_tagging() {
        let input = "## custom_flag ::attribute\n- target: [field]\n- type: boolean\n\n## User\n- id: identifier @custom_flag";
        let parsed = parse_string(input, "test.m3l.md");
        let ast = resolve(&[parsed], None);
        assert_eq!(ast.attribute_registry.len(), 1);
        // The id field should have @custom_flag tagged as isRegistered
        let id_field = &ast.models[0].fields[0];
        let cf_attr = id_field.attributes.iter().find(|a| a.name == "custom_flag");
        assert!(cf_attr.is_some());
        assert_eq!(cf_attr.unwrap().is_registered, Some(true));
    }

    #[test]
    fn detect_ambiguous_cross_namespace_e008() {
        let f1 = parse_string(
            "# Namespace: sales\n## Product\n- id: identifier",
            "sales.m3l.md",
        );
        let f2 = parse_string(
            "# Namespace: inventory\n## Product\n- id: identifier",
            "inventory.m3l.md",
        );
        let ast = resolve(&[f1, f2], None);
        assert!(
            ast.errors.iter().any(|e| e.code == "M3L-E008"),
            "Expected E008 for same name 'Product' in different namespaces"
        );
    }

    #[test]
    fn no_e008_same_namespace() {
        // Same name in same namespace → E005 (duplicate), not E008
        let f1 = parse_string(
            "# Namespace: sales\n## Product\n- id: identifier",
            "a.m3l.md",
        );
        let f2 = parse_string("# Namespace: sales\n## Product\n- name: string", "b.m3l.md");
        let ast = resolve(&[f1, f2], None);
        assert!(ast.errors.iter().any(|e| e.code == "M3L-E005"));
        assert!(
            !ast.errors.iter().any(|e| e.code == "M3L-E008"),
            "E008 should not fire for same namespace"
        );
    }

    #[test]
    fn no_e008_unique_names() {
        let f1 = parse_string(
            "# Namespace: sales\n## Order\n- id: identifier",
            "sales.m3l.md",
        );
        let f2 = parse_string(
            "# Namespace: inventory\n## Product\n- id: identifier",
            "inventory.m3l.md",
        );
        let ast = resolve(&[f1, f2], None);
        assert!(
            !ast.errors.iter().any(|e| e.code == "M3L-E008"),
            "No E008 when names are unique across namespaces"
        );
    }

    #[test]
    fn detect_no_circular_imports() {
        let file_imports = vec![
            ("a.m3l.md".to_string(), vec!["b.m3l.md".to_string()]),
            ("b.m3l.md".to_string(), vec!["c.m3l.md".to_string()]),
            ("c.m3l.md".to_string(), vec![]),
        ];
        let errors = detect_circular_imports(&file_imports);
        assert!(errors.is_empty(), "Should not detect any cycles");
    }

    #[test]
    fn detect_simple_circular_import() {
        let file_imports = vec![
            ("a.m3l.md".to_string(), vec!["b.m3l.md".to_string()]),
            ("b.m3l.md".to_string(), vec!["a.m3l.md".to_string()]),
        ];
        let errors = detect_circular_imports(&file_imports);
        assert!(!errors.is_empty(), "Should detect circular import");
        assert!(errors.iter().any(|e| e.code == "M3L-E003"));
        assert!(errors[0].message.contains("Circular import detected"));
    }

    #[test]
    fn detect_three_way_circular_import() {
        let file_imports = vec![
            ("a.m3l.md".to_string(), vec!["b.m3l.md".to_string()]),
            ("b.m3l.md".to_string(), vec!["c.m3l.md".to_string()]),
            ("c.m3l.md".to_string(), vec!["a.m3l.md".to_string()]),
        ];
        let errors = detect_circular_imports(&file_imports);
        assert!(!errors.is_empty(), "Should detect 3-way cycle");
        assert!(errors.iter().any(|e| e.code == "M3L-E003"));
    }

    #[test]
    fn detect_diamond_no_cycle() {
        // Diamond: a→b, a→c, b→d, c→d — no cycle
        let file_imports = vec![
            (
                "a.m3l.md".to_string(),
                vec!["b.m3l.md".to_string(), "c.m3l.md".to_string()],
            ),
            ("b.m3l.md".to_string(), vec!["d.m3l.md".to_string()]),
            ("c.m3l.md".to_string(), vec!["d.m3l.md".to_string()]),
            ("d.m3l.md".to_string(), vec![]),
        ];
        let errors = detect_circular_imports(&file_imports);
        assert!(
            errors.is_empty(),
            "Diamond dependency should not be a cycle"
        );
    }

    #[test]
    fn parse_import_directive() {
        let input = "@import \"base.m3l.md\"\n\n## User\n- id: identifier";
        let parsed = parse_string(input, "test.m3l.md");
        assert_eq!(parsed.imports.len(), 1);
        assert_eq!(parsed.imports[0], "base.m3l.md");
    }

    #[test]
    fn resolve_override_inheritance() {
        let input =
            "## Base ::interface\n- name: string\n\n## Child : Base\n- name: text @override";
        let parsed = parse_string(input, "test.m3l.md");
        let ast = resolve(&[parsed], None);
        assert_eq!(ast.models[0].fields.len(), 1);
        assert_eq!(ast.models[0].fields[0].field_type.as_deref(), Some("text"));
    }
}
