use std::collections::{BTreeMap, HashMap, HashSet};

use crate::catalogs::{AST_VERSION, PARSER_VERSION};
use crate::types::*;

/// Resolve and merge multiple parsed file ASTs into a single M3lAst.
/// Handles: inheritance resolution, duplicate detection, attribute registry tagging.
/// Knobs for [`resolve_with`].
#[derive(Debug, Clone)]
pub struct ResolveOptions {
    /// Copy each parent's fields into the child model (the default, and what
    /// every semantic consumer wants).
    ///
    /// Turn it **off** for consumers that must reproduce the source document
    /// rather than its semantic closure — the formatter is the motivating case:
    /// inlined parent fields are indistinguishable from the model's own, so it
    /// re-emitted them *and* the `: Parent` header, duplicating the fields on
    /// every round-trip.
    pub inline_inherited: bool,
    /// Merge `::extend` blocks into the models they target. On by default.
    /// Off for consumers that reproduce the source document (the formatter):
    /// a merged field is indistinguishable from the model's own, so the block
    /// would be lost and its fields written into the base file.
    pub merge_extends: bool,
    /// The directory package import paths (`@import "common/base.m3l"`) are resolved from
    /// (§10.6.1), written the way the files' `source` paths are. `None` resolves them from the
    /// directory the `source` paths are relative to — what a caller handing in in-memory files
    /// named `a.m3l.md`, `common/b.m3l.md` means. Relative paths (`./`, `../`) never use it: they
    /// are resolved from the importing file's own directory.
    pub project_root: Option<String>,
    /// Whether the files handed in are the whole set the imports may name — a project, a directory, the files a
    /// tool loads together. Then an `@import` that names none of them is `M3L-E033`. Off for a single file read on its
    /// own: nothing follows its imports, so a file it imports being absent says nothing about the import.
    pub complete_file_set: bool,
}

impl Default for ResolveOptions {
    fn default() -> Self {
        Self {
            inline_inherited: true,
            merge_extends: true,
            project_root: None,
            complete_file_set: false,
        }
    }
}

pub fn resolve(files: &[ParsedFile], project: Option<ProjectInfo>) -> M3lAst {
    resolve_with(files, project, ResolveOptions::default())
}

pub fn resolve_with(
    files: &[ParsedFile],
    project: Option<ProjectInfo>,
    options: ResolveOptions,
) -> M3lAst {
    let mut errors: Vec<Diagnostic> = Vec::new();
    let mut warnings: Vec<Diagnostic> = Vec::new();

    // Collect all elements from all files
    let mut all_models: Vec<ModelNode> = Vec::new();
    let mut all_enums: Vec<EnumNode> = Vec::new();
    let mut all_interfaces: Vec<ModelNode> = Vec::new();
    let mut all_views: Vec<ModelNode> = Vec::new();
    let mut all_flows: Vec<ModelNode> = Vec::new();
    let mut all_extensions: BTreeMap<String, Vec<ModelNode>> = BTreeMap::new();
    let mut all_attr_registry: Vec<AttributeRegistryEntry> = Vec::new();
    let mut sources: Vec<String> = Vec::new();

    check_prefix_headers(files, &mut errors);
    let (aliases, imports) = file_imports(files, options.project_root.as_deref(), &mut errors);
    let aliases = Rc::new(aliases);
    check_extend_kind_args(files, &mut errors);
    check_unread_field_text(files, &mut warnings);
    check_unread_header_text(files, &mut warnings);

    for file in files {
        sources.push(file.source.clone());
        all_models.extend(file.models.iter().cloned());
        all_enums.extend(file.enums.iter().cloned());
        all_interfaces.extend(file.interfaces.iter().cloned());
        all_views.extend(file.views.iter().cloned());
        all_flows.extend(file.flows.iter().cloned());
        for (key, nodes) in &file.extensions {
            all_extensions
                .entry(key.clone())
                .or_default()
                .extend(nodes.iter().cloned());
        }
        all_attr_registry.extend(file.attribute_registry.iter().cloned());
    }

    // `::extend` blocks are named after their *target*, so they must not take part in
    // the duplicate-name pass — two blocks for one model are the normal case.
    let extend_blocks: Vec<ModelNode> = if options.merge_extends {
        all_extensions.remove("extend").unwrap_or_default()
    } else {
        Vec::new()
    };

    // Each declaration's namespace, from its file.
    let source_ns: HashMap<&str, Option<&str>> = files
        .iter()
        .map(|f| (f.source.as_str(), f.namespace.as_deref()))
        .collect();

    // Declarations by (namespace, name). One namespace is one scope (§2.1.3 Rule 1): a name declared twice in it is
    // `M3L-E005`, and the same name in two namespaces is two declarations — a reference that cannot tell them apart is
    // `M3L-E008` where it is written (Rule 4). `declared` is every short name declared as anything, so a parent name
    // that is some other kind of declaration is left to the check that owns that kind.
    let mut all_named: HashMap<(Option<String>, String), (&str, String, usize)> = HashMap::new();
    let mut declared: HashSet<String> = HashSet::new();
    let extension_nodes = all_extensions
        .iter()
        .filter(|(kind, _)| kind.as_str() != "extend")
        .flat_map(|(_, nodes)| nodes.iter());
    let declarations = all_models
        .iter()
        .map(|m| ("model", m.name.as_str(), m.source.as_str(), m.line))
        .chain(
            all_enums
                .iter()
                .map(|e| ("enum", e.name.as_str(), e.source.as_str(), e.line)),
        )
        .chain(
            all_interfaces
                .iter()
                .map(|i| ("interface", i.name.as_str(), i.source.as_str(), i.line)),
        )
        .chain(
            all_views
                .iter()
                .map(|v| ("view", v.name.as_str(), v.source.as_str(), v.line)),
        )
        .chain(
            all_flows
                .iter()
                .map(|f| ("flow", f.name.as_str(), f.source.as_str(), f.line)),
        )
        .chain(extension_nodes.map(|x| ("extension", x.name.as_str(), x.source.as_str(), x.line)));
    for (kind, name, source, line) in declarations {
        let ns = source_ns.get(source).copied().flatten().map(String::from);
        match all_named.entry((ns, name.to_string())) {
            std::collections::hash_map::Entry::Occupied(first) => {
                let (_, first_file, first_line) = first.get();
                errors.push(Diagnostic {
                    code: "M3L-E005".to_string(),
                    severity: DiagnosticSeverity::Error,
                    file: source.to_string(),
                    line,
                    col: 1,
                    message: format!(
                        "Duplicate {} name \"{}\" (first defined in {}:{})",
                        kind, name, first_file, first_line
                    ),
                });
            }
            std::collections::hash_map::Entry::Vacant(slot) => {
                slot.insert((kind, source.to_string(), line));
            }
        }
        declared.insert(name.to_string());
    }

    // Resolve inheritance — a parent is a model or an interface, found by the name the header writes.
    if options.inline_inherited {
        let namespace_of = |source: &str| -> Option<String> {
            source_ns.get(source).copied().flatten().map(String::from)
        };
        let mut parents = NameIndex::new(&aliases);
        for (idx, model) in all_models.iter().enumerate() {
            parents.add(
                &model.name,
                Declared {
                    idx,
                    namespace: namespace_of(&model.source),
                    source: model.source.clone(),
                    interface: false,
                },
            );
        }
        for (idx, iface) in all_interfaces.iter().enumerate() {
            parents.add(
                &iface.name,
                Declared {
                    idx,
                    namespace: namespace_of(&iface.source),
                    source: iface.source.clone(),
                    interface: true,
                },
            );
        }
        for i in 0..all_models.len() {
            resolve_inheritance(
                i,
                &mut all_models,
                &all_interfaces,
                &parents,
                &namespace_of,
                &declared,
                &mut errors,
            );
        }
    }

    // Enum inheritance rides the same switch as a model's: an inlined value is indistinguishable
    // from one the enum declared itself, so a consumer reproducing the source (the formatter) would
    // re-emit the inherited values *and* the `: Parent` header and duplicate them on every
    // round-trip — the failure this option exists to prevent, one type over.
    if options.inline_inherited {
        let namespace_of = |source: &str| -> Option<String> {
            source_ns.get(source).copied().flatten().map(String::from)
        };
        let mut enum_index = NameIndex::new(&aliases);
        for (idx, en) in all_enums.iter().enumerate() {
            enum_index.add(
                &en.name,
                Declared {
                    idx,
                    namespace: namespace_of(&en.source),
                    source: en.source.clone(),
                    interface: false,
                },
            );
        }
        for i in 0..all_enums.len() {
            resolve_enum_inheritance(i, &mut all_enums, &enum_index, &namespace_of, &mut errors);
        }
    }

    for en in all_enums.iter() {
        check_duplicate_enum_values(en, &mut errors);
    }

    crate::extend::merge_extend_blocks(&mut all_models, extend_blocks, &aliases, &mut errors);
    crate::extend::check_model_bases(&all_models, &aliases, &mut errors);

    // Check duplicate field names
    for model in all_models
        .iter()
        .chain(all_views.iter())
        .chain(all_flows.iter())
    {
        check_duplicate_fields(model, &mut errors);
    }
    for ext_nodes in all_extensions.values() {
        for ext in ext_nodes {
            check_duplicate_fields(ext, &mut errors);
        }
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
        for ext_nodes in all_extensions.values_mut() {
            for ext in ext_nodes.iter_mut() {
                tag_attrs(&mut ext.attributes);
                for f in ext.fields.iter_mut() {
                    tag_attrs(&mut f.attributes);
                }
            }
        }
        // Enum value attributes (`- print: "…" @help`) are a registrable
        // target too (`target: [value]`) — without
        // this loop they'd stay untagged even when correctly registered,
        // the same asymmetry the validator gap below fixes.
        for en in all_enums.iter_mut() {
            for v in en.values.iter_mut() {
                if let Some(attrs) = v.attributes.as_mut() {
                    tag_attrs(attrs);
                }
            }
        }
    }

    // Detect circular imports (E003)
    let file_imports: Vec<(&str, Vec<(&str, usize)>)> = files
        .iter()
        .map(|f| {
            let imports = f
                .imports
                .iter()
                .map(|i| (i.path.as_str(), i.line))
                .collect();
            (f.source.as_str(), imports)
        })
        .collect();
    let circular_errors =
        detect_circular_imports_internal(&file_imports, options.project_root.as_deref());
    errors.extend(circular_errors);
    if options.complete_file_set {
        errors.extend(detect_unmatched_imports(
            &file_imports,
            options.project_root.as_deref(),
        ));
    }

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
        flows: all_flows,
        extensions: all_extensions,
        attribute_registry: all_attr_registry,
        imports,
        errors,
        warnings,
    }
}

/// `M3L-E021`: a `::extend` header that carried a parenthesised argument.
///
/// `::extend` takes its target from the declaration name, so the argument has nowhere to go and
/// is discarded. Reported because the silence is actively misleading rather than merely lossy: an
/// author who writes `## Other ::extend(Base)` intending to extend `Base` gets `M3L-E011` naming
/// *`Other`* — a target they never wrote — and nothing connects that message to the parentheses.
/// `M3L-W010`: text on a field line that the field syntax does not read (§2.5.8). A warning,
/// not an error — the rest of the line is still a well-formed field — but never silence: the
/// commonest instance is a default written after an attribute, which is otherwise lost whole.
fn check_unread_field_text(files: &[ParsedFile], warnings: &mut Vec<Diagnostic>) {
    for file in files {
        for entry in &file.unread_field_text {
            warnings.push(Diagnostic {
                code: "M3L-W010".to_string(),
                severity: DiagnosticSeverity::Warning,
                file: entry.file.clone(),
                line: entry.line,
                col: 1,
                message: format!(
                    "Field \"{}\" in {}: \"{}\" is not read. {}",
                    entry.field,
                    entry.element,
                    entry.text,
                    entry.hint.unwrap_or(
                        "A field line reads `name: type = default @attributes \"description\"` in that \
                         order — move a default next to the type, and put the description last."
                    )
                ),
            });
        }
    }
}

/// `M3L-W010` for a header: a second description string, or a quote that never closes. Before the
/// header split took them off the line they became the element's name (or its parent's); now they
/// are named instead of read. Also attributes on a header whose kind takes none, which were dropped.
fn check_unread_header_text(files: &[ParsedFile], warnings: &mut Vec<Diagnostic>) {
    for file in files {
        for entry in &file.unread_header_text {
            // Quoted text and attributes go unread for different reasons; the warning says which.
            let message = match &entry.part {
                UnreadHeaderPart::Quoted => format!(
                    "Header of \"{}\": {} is not read. A header takes one description string \
                     (`## Name \"description\" @attributes`) — join the text into one, or move the \
                     rest into a blockquote below the header.",
                    entry.element, entry.text
                ),
                UnreadHeaderPart::Attributes { kind } => format!(
                    "Header of \"{}\": {} is not read. A `::{kind}` header takes no attributes — \
                     only a model header (an `::aspect` or `::subtype` one too) and a `::view` header \
                     do — so nothing applies it to \"{}\".",
                    entry.element, entry.text, entry.element
                ),
                UnreadHeaderPart::Words => format!(
                    "Header of \"{}\": `{}` is not read. A header is `## Name(Label) : Parents \
                     @attributes \"description\"` (or `## Name ::kind …`), and these words are none \
                     of its parts.",
                    entry.element, entry.text
                ),
            };
            warnings.push(Diagnostic {
                code: "M3L-W010".to_string(),
                severity: DiagnosticSeverity::Warning,
                file: entry.file.clone(),
                line: entry.line,
                col: 1,
                message,
            });
        }
    }
}

fn check_extend_kind_args(files: &[ParsedFile], errors: &mut Vec<Diagnostic>) {
    for file in files {
        for entry in &file.extend_kind_args {
            errors.push(Diagnostic {
                code: "M3L-E021".to_string(),
                severity: DiagnosticSeverity::Error,
                file: entry.file.clone(),
                line: entry.line,
                col: 1,
                message: format!(
                    "\"::extend({})\" takes no argument. An extend block names its target with \
                     the declaration name, so this block extends \"{}\" — rename the heading if \
                     that is not the target, and remove the parentheses either way.",
                    entry.arg, entry.declared_name
                ),
            });
        }
    }
}

/// `M3L-E019`: a `# Prefix:` header that is malformed, repeated, or declared after the
/// file's first declaration. Purely per-file — it reads the raw headers the parser
/// recorded, not the merged model set, so it runs independent of `merge_extends` (and
/// of `inline_inherited`): a file's headers are what they are regardless of how the
/// rest of the project resolves.
fn check_prefix_headers(files: &[ParsedFile], errors: &mut Vec<Diagnostic>) {
    for file in files {
        for (idx, header) in file.prefix_headers.iter().enumerate() {
            let message = if !crate::parser::is_valid_prefix_word(&header.value) {
                format!(
                    "\"{}\" is not a valid prefix — Prefix must match [a-z][a-z0-9]*",
                    header.value
                )
            } else if idx > 0 {
                "A file may declare only one \"# Prefix:\" header — a later one is ignored"
                    .to_string()
            } else if !header.before_first_declaration {
                "\"# Prefix:\" after the file's first declaration is ignored — move it \
                 above the first model, enum, interface, view, or extend block"
                    .to_string()
            } else {
                continue;
            };
            errors.push(Diagnostic {
                code: "M3L-E019".to_string(),
                severity: DiagnosticSeverity::Error,
                file: file.source.clone(),
                line: header.line,
                col: 1,
                message,
            });
        }
    }
}

/// One declaration a name can refer to: its index in its kind's list, its namespace and its file.
#[derive(Debug, Clone)]
struct Declared {
    idx: usize,
    namespace: Option<String>,
    source: String,
    /// For an inheritance parent: an interface rather than a model.
    interface: bool,
}

impl Declared {
    /// A key no other declaration of the index shares — the namespace and the index together.
    fn key(&self) -> (Option<String>, usize, bool) {
        (self.namespace.clone(), self.idx, self.interface)
    }
}

type Lookup<'a> = crate::names::Lookup<'a, Declared>;

/// Declarations of one kind, found by the name a reference writes — the rules are
/// [`crate::names::pick`]'s, shared with the validator.
#[derive(Debug, Default)]
struct NameIndex {
    by_name: HashMap<String, Vec<Declared>>,
    aliases: Rc<Aliases>,
}

impl NameIndex {
    fn new(aliases: &Rc<Aliases>) -> Self {
        Self {
            by_name: HashMap::new(),
            aliases: Rc::clone(aliases),
        }
    }

    fn add(&mut self, name: &str, declared: Declared) {
        self.by_name
            .entry(name.to_string())
            .or_default()
            .push(declared);
    }

    fn lookup(&self, written: &str, from_ns: Option<&str>, from_source: &str) -> Lookup<'_> {
        let candidates = self
            .by_name
            .get(short_name(written))
            .map(Vec::as_slice)
            .unwrap_or_default();
        let from = Referrer {
            ns: from_ns,
            source: from_source,
            aliases: &self.aliases,
        };
        crate::names::pick(
            candidates,
            written,
            from,
            |d| d.namespace.as_deref(),
            |d| d.source.as_str(),
        )
    }
}

/// An enum parent's index, or why it has none: the namespaces that make it ambiguous, or nothing
/// when no enum of that name exists.
type ParentLookup = Result<usize, Option<Vec<String>>>;

use crate::names::{short_name, Aliases, Referrer};
use std::rc::Rc;

/// Every file's imports as AST nodes, and the aliases they give (§10.6.2) — each import resolved to the loaded file it names — the same
/// resolution as the circular-import check and `M3L-E033`. An alias whose import names no loaded file names nothing
/// (that import is `M3L-E033`'s when the set is complete). One file giving one alias to two imports is `M3L-E034`.
fn file_imports(
    files: &[ParsedFile],
    project_root: Option<&str>,
    errors: &mut Vec<Diagnostic>,
) -> (Aliases, Vec<ImportNode>) {
    let loaded: HashMap<String, &str> = files
        .iter()
        .map(|f| (normalize_path(&f.source), f.source.as_str()))
        .collect();
    let mut aliases = Aliases::default();
    let mut nodes = Vec::new();
    for file in files {
        let mut seen: HashMap<&str, usize> = HashMap::new();
        for import in &file.imports {
            let target = import_target(&file.source, &import.path, project_root, &loaded);
            nodes.push(ImportNode {
                file: file.source.clone(),
                line: import.line,
                path: import.path.clone(),
                alias: import.alias.clone(),
                target: target.map(String::from),
            });
            let Some(alias) = import.alias.as_deref() else {
                continue;
            };
            if let Some(first) = seen.insert(alias, import.line) {
                errors.push(Diagnostic {
                    code: "M3L-E034".to_string(),
                    severity: DiagnosticSeverity::Error,
                    file: file.source.clone(),
                    line: import.line,
                    col: 1,
                    message: format!(
                        "@import \"{}\" as {}: the alias \"{}\" is already given to the import at line {} — one alias names one file",
                        import.path, alias, alias, first
                    ),
                });
                continue;
            }
            if let Some(target) = target {
                aliases.add(&file.source, alias, target);
            }
        }
    }
    (aliases, nodes)
}

/// Copies each parent's fields into the model (§2.3), parents found by the names its header
/// writes — qualified or short, through [`NameIndex`]. A parent's own parents are looked up from
/// the parent's namespace and file: a name resolves where it is written (Rule 2).
fn resolve_inheritance(
    model_idx: usize,
    all_models: &mut [ModelNode],
    all_interfaces: &[ModelNode],
    parents: &NameIndex,
    namespace_of: &dyn Fn(&str) -> Option<String>,
    declared: &HashSet<String>,
    errors: &mut Vec<Diagnostic>,
) {
    let inherits = all_models[model_idx].inherits.clone();
    if inherits.is_empty() {
        return;
    }

    let model_source = all_models[model_idx].source.clone();
    let model_line = all_models[model_idx].line;
    let model_name = all_models[model_idx].name.clone();

    struct Walk<'a> {
        model_source: &'a str,
        model_line: usize,
        model_name: &'a str,
        all_models: &'a [ModelNode],
        all_interfaces: &'a [ModelNode],
        parents: &'a NameIndex,
        namespace_of: &'a dyn Fn(&str) -> Option<String>,
        declared: &'a HashSet<String>,
        inherited_fields: Vec<FieldNode>,
        resolved: HashSet<(Option<String>, usize, bool)>,
        visiting: HashSet<(Option<String>, usize, bool)>,
        errors: &'a mut Vec<Diagnostic>,
    }

    impl Walk<'_> {
        fn collect(&mut self, written: &str, from_source: &str) {
            let from_ns = (self.namespace_of)(from_source);
            let parent = match self
                .parents
                .lookup(written, from_ns.as_deref(), from_source)
            {
                Lookup::Found(declared) => declared.clone(),
                Lookup::Ambiguous(namespaces) => {
                    self.errors.push(Diagnostic {
                        code: "M3L-E008".to_string(),
                        severity: DiagnosticSeverity::Error,
                        file: self.model_source.to_string(),
                        line: self.model_line,
                        col: 1,
                        message: format!(
                            "Ambiguous model reference \"{}\" in namespaces {} — write the qualified name of the parent of \"{}\"",
                            written,
                            namespaces.join(", "),
                            self.model_name
                        ),
                    });
                    return;
                }
                Lookup::Missing => {
                    // A name declared as something else (an enum, a view) is reported by the check that
                    // owns that kind of reference, not as an unresolved parent.
                    if !self.declared.contains(short_name(written)) || written.contains('.') {
                        self.errors.push(Diagnostic {
                            code: "M3L-E007".to_string(),
                            severity: DiagnosticSeverity::Error,
                            file: self.model_source.to_string(),
                            line: self.model_line,
                            col: 1,
                            message: format!(
                                "Unresolved inheritance reference \"{}\" in model \"{}\"",
                                written, self.model_name
                            ),
                        });
                    }
                    return;
                }
            };
            let key = parent.key();
            if self.resolved.contains(&key) || !self.visiting.insert(key.clone()) {
                return;
            }
            let node = if parent.interface {
                &self.all_interfaces[parent.idx]
            } else {
                &self.all_models[parent.idx]
            };
            // Grandparents first, looked up from where the parent wrote them.
            for grandparent in node.inherits.clone() {
                self.collect(&grandparent, &parent.source);
            }
            let node = if parent.interface {
                &self.all_interfaces[parent.idx]
            } else {
                &self.all_models[parent.idx]
            };
            for field in &node.fields {
                if !self.inherited_fields.iter().any(|f| f.name == field.name) {
                    self.inherited_fields.push(field.clone());
                }
            }
            self.visiting.remove(&key);
            self.resolved.insert(key);
        }
    }

    let mut walk = Walk {
        model_source: &model_source,
        model_line,
        model_name: &model_name,
        all_models,
        all_interfaces,
        parents,
        namespace_of,
        declared,
        inherited_fields: Vec::new(),
        resolved: HashSet::new(),
        visiting: HashSet::new(),
        errors,
    };
    for parent_name in &inherits {
        walk.collect(parent_name, &model_source);
    }
    let inherited_fields = walk.inherited_fields;

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

/// Resolves `## Child ::enum : Parent` by copying the parent's values into the child, the way
/// [`resolve_inheritance`] does for a model's fields.
///
/// §3.1.6 defines inheritance as the union of the parents' values and the enum's own, so a
/// consumer reads a complete list off `values` and never walks `inherits` to assemble it. Before
/// this existed the parser recorded `inherits` and left `values` holding the child's own block —
/// the declaration parsed, every target succeeded, and the enum was quietly missing members.
///
/// Ordering is part of the contract (an enum is often rendered as an ordered list of choices):
/// grandparents, then parents left to right, then the enum's own values.
fn resolve_enum_inheritance(
    enum_idx: usize,
    all_enums: &mut [EnumNode],
    enum_index: &NameIndex,
    namespace_of: &dyn Fn(&str) -> Option<String>,
    errors: &mut Vec<Diagnostic>,
) {
    // A parent named by its header, looked up from the file that wrote the name (Rule 2) —
    // its index, or the reason it has none.
    let find = |written: &str, from_source: &str| -> ParentLookup {
        match enum_index.lookup(written, namespace_of(from_source).as_deref(), from_source) {
            Lookup::Found(declared) => Ok(declared.idx),
            Lookup::Ambiguous(namespaces) => Err(Some(namespaces)),
            Lookup::Missing => Err(None),
        }
    };
    let inherits = all_enums[enum_idx].inherits.clone();
    if inherits.is_empty() {
        return;
    }

    let enum_name = all_enums[enum_idx].name.clone();
    let enum_source = all_enums[enum_idx].source.clone();
    let enum_line = all_enums[enum_idx].line;

    let mut inherited: Vec<EnumValue> = Vec::new();
    let mut resolved: HashSet<usize> = HashSet::new();
    let mut visiting: HashSet<usize> = HashSet::new();

    #[allow(clippy::too_many_arguments)]
    fn collect_values(
        name: &str,
        from_source: &str,
        enum_name: &str,
        enum_source: &str,
        enum_line: usize,
        all_enums: &[EnumNode],
        find: &dyn Fn(&str, &str) -> ParentLookup,
        inherited: &mut Vec<EnumValue>,
        resolved: &mut HashSet<usize>,
        visiting: &mut HashSet<usize>,
        errors: &mut Vec<Diagnostic>,
    ) {
        let idx = match find(name, from_source) {
            Ok(idx) => idx,
            Err(Some(namespaces)) => {
                errors.push(Diagnostic {
                    code: "M3L-E008".to_string(),
                    severity: DiagnosticSeverity::Error,
                    file: enum_source.to_string(),
                    line: enum_line,
                    col: 1,
                    message: format!(
                        "Ambiguous enum reference \"{}\" in namespaces {} — write the qualified name of the parent of \"{}\"",
                        name,
                        namespaces.join(", "),
                        enum_name
                    ),
                });
                return;
            }
            Err(None) => {
                errors.push(Diagnostic {
                    code: "M3L-E007".to_string(),
                    severity: DiagnosticSeverity::Error,
                    file: enum_source.to_string(),
                    line: enum_line,
                    col: 1,
                    message: format!(
                        "Unresolved inheritance reference \"{}\" in enum \"{}\"",
                        name, enum_name
                    ),
                });
                return;
            }
        };
        if resolved.contains(&idx) || !visiting.insert(idx) {
            return;
        }

        let parent_inherits = all_enums[idx].inherits.clone();
        let parent_source = all_enums[idx].source.clone();
        for grandparent in &parent_inherits {
            collect_values(
                grandparent,
                &parent_source,
                enum_name,
                enum_source,
                enum_line,
                all_enums,
                find,
                inherited,
                resolved,
                visiting,
                errors,
            );
        }

        for value in &all_enums[idx].values {
            match inherited.iter().find(|v| v.name == value.name) {
                // The same value reached by two paths is one value — the diamond case,
                // kept once in the position it was first reached.
                Some(existing) if *existing == *value => {}
                // Two parents disagree about what the name means. Nothing in the source
                // says which label wins, and choosing by parent order would make the
                // enum's meaning depend on the order the parents happen to be listed in.
                Some(_) => errors.push(Diagnostic {
                    code: "M3L-E020".to_string(),
                    severity: DiagnosticSeverity::Error,
                    file: enum_source.to_string(),
                    line: enum_line,
                    col: 1,
                    message: format!(
                        "Duplicate enum value \"{}\" in enum \"{}\": two parents declare \
                                 it differently",
                        value.name, enum_name
                    ),
                }),
                None => inherited.push(value.clone()),
            }
        }

        visiting.remove(&idx);
        resolved.insert(idx);
    }

    for parent_name in &inherits {
        collect_values(
            parent_name,
            &enum_source,
            &enum_name,
            &enum_source,
            enum_line,
            all_enums,
            &find,
            &mut inherited,
            &mut resolved,
            &mut visiting,
            errors,
        );
    }

    if !inherited.is_empty() {
        let own = std::mem::take(&mut all_enums[enum_idx].values);
        all_enums[enum_idx].values = inherited;
        all_enums[enum_idx].values.extend(own);
    }
}

/// Every value name in an enum is unique once §3.1.6 has been applied.
///
/// Runs whether or not inheritance was inlined, because a block that repeats a name on its own was
/// never reported either — the second declaration simply sat in the list, and which one a consumer
/// honoured was its own business.
fn check_duplicate_enum_values(en: &EnumNode, errors: &mut Vec<Diagnostic>) {
    let mut seen: HashSet<String> = HashSet::new();
    for value in &en.values {
        if !seen.insert(value.name.clone()) {
            errors.push(Diagnostic {
                code: "M3L-E020".to_string(),
                severity: DiagnosticSeverity::Error,
                file: en.source.clone(),
                line: en.line,
                col: 1,
                message: format!(
                    "Duplicate enum value \"{}\" in enum \"{}\"",
                    value.name, en.name
                ),
            });
        }
    }
}

fn check_duplicate_fields(model: &ModelNode, errors: &mut Vec<Diagnostic>) {
    let mut seen: HashMap<String, usize> = HashMap::new(); // name → line
    for field in &model.fields {
        if let Some(&existing_line) = seen.get(&field.name) {
            let model_type = match &model.model_type {
                ModelType::Model => "model",
                ModelType::View => "view",
                ModelType::Interface => "interface",
                ModelType::Enum => "enum",
                ModelType::Flow => "flow",
                ModelType::Extension(s) => s.as_str(),
            };
            errors.push(Diagnostic {
                // §10.5.1: E006 is a duplicate field; E005 is a duplicate name in the namespace (models, views, …).
                code: "M3L-E006".to_string(),
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
/// for any cycles detected using DFS. Import paths are resolved as [`resolve`] resolves them
/// with no project root — see [`ResolveOptions::project_root`]. The pairs carry no line numbers,
/// so a cycle is reported at line 1 of the file that closes it; [`resolve`] reports it at the
/// `@import` line.
pub fn detect_circular_imports(file_imports: &[(String, Vec<String>)]) -> Vec<Diagnostic> {
    let refs: Vec<(&str, Vec<(&str, usize)>)> = file_imports
        .iter()
        .map(|(src, imports)| {
            (
                src.as_str(),
                imports.iter().map(|i| (i.as_str(), 1)).collect(),
            )
        })
        .collect();
    detect_circular_imports_internal(&refs, None)
}

/// `file_imports`: each loaded file's source path with its imports as (path as written, line).
fn detect_circular_imports_internal(
    file_imports: &[(&str, Vec<(&str, usize)>)],
    project_root: Option<&str>,
) -> Vec<Diagnostic> {
    let mut errors = Vec::new();

    // The graph's nodes are the loaded files, so each import is first turned into the file it
    // names. Comparing the import text with the `source` text instead only matched when the files
    // were named relative to the directory the imports are written from — a cycle between files
    // read from any other directory, or imported as the spec writes it (`./b.m3l`), went unseen.
    let loaded: HashMap<String, &str> = file_imports
        .iter()
        .map(|&(src, _)| (normalize_path(src), src))
        .collect();
    let adj: HashMap<&str, Vec<(&str, usize)>> = file_imports
        .iter()
        .map(|(src, imports)| {
            let targets = imports
                .iter()
                .filter_map(|&(import, line)| {
                    import_target(src, import, project_root, &loaded).map(|t| (t, line))
                })
                .collect();
            (*src, targets)
        })
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

/// `M3L-E033` — an `@import` that names no file of the set, at the import's line. The same resolution the cycle
/// check uses (relative to the importing file, or to the project root for a package path; `.md` may be left off),
/// so an import is unmatched exactly when the cycle check could not follow it.
fn detect_unmatched_imports(
    file_imports: &[(&str, Vec<(&str, usize)>)],
    project_root: Option<&str>,
) -> Vec<Diagnostic> {
    let loaded: HashMap<String, &str> = file_imports
        .iter()
        .map(|&(src, _)| (normalize_path(src), src))
        .collect();
    let mut errors = Vec::new();
    for &(src, ref imports) in file_imports {
        for &(import, line) in imports {
            if import_target(src, import, project_root, &loaded).is_none() {
                errors.push(Diagnostic {
                    code: "M3L-E033".to_string(),
                    severity: DiagnosticSeverity::Error,
                    file: src.to_string(),
                    line,
                    col: 1,
                    message: format!(
                        "@import \"{import}\" names no file of this set — check the path, or add the file to what is read together"
                    ),
                });
            }
        }
    }
    errors
}

fn dfs_detect_cycle<'a>(
    node: &'a str,
    adj: &HashMap<&'a str, Vec<(&'a str, usize)>>,
    visited: &mut HashSet<&'a str>,
    rec_stack: &mut Vec<&'a str>,
    errors: &mut Vec<Diagnostic>,
) {
    visited.insert(node);
    rec_stack.push(node);

    if let Some(imports) = adj.get(node) {
        for &(import_str, import_line) in imports {
            if !visited.contains(import_str) {
                dfs_detect_cycle(import_str, adj, visited, rec_stack, errors);
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
                    line: import_line,
                    col: 1,
                    message: format!("Circular import detected: {}", chain_str),
                });
            }
        }
    }

    rec_stack.pop();
}

/// The loaded file an `@import` path names (§10.6.1), or `None` when it names none of them. A
/// relative path (`./`, `../`) is resolved from the importing file's directory, any other from the
/// project root. The spec writes imports without the `.md` of a `.m3l.md` file
/// (`@import "common/base.m3l"`), so both spellings are tried.
fn import_target<'a>(
    importer: &str,
    import: &str,
    project_root: Option<&str>,
    loaded: &HashMap<String, &'a str>,
) -> Option<&'a str> {
    let import = import.replace('\\', "/");
    let base = if import.starts_with("./") || import.starts_with("../") {
        parent_dir(importer)
    } else {
        project_root.unwrap_or("")
    };
    let path = if base.is_empty() {
        normalize_path(&import)
    } else {
        normalize_path(&format!("{base}/{import}"))
    };
    loaded
        .get(&path)
        .or_else(|| loaded.get(&format!("{path}.md")))
        .copied()
}

fn parent_dir(path: &str) -> &str {
    path.rfind(['/', '\\']).map_or("", |i| &path[..i])
}

/// A path with `/` separators and its `.` and `..` segments folded away, lexically — the files
/// need not exist, and in-memory sources never do.
fn normalize_path(path: &str) -> String {
    let path = path.replace('\\', "/");
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." if parts.last().is_some_and(|p| *p != "..") => {
                parts.pop();
            }
            _ => parts.push(part),
        }
    }
    let joined = parts.join("/");
    if path.starts_with('/') {
        format!("/{joined}")
    } else {
        joined
    }
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

    /// A name is declared once, whatever is declared under it: a view named like a model (or
    /// like another view) and a flow named like either are the same mistake as two models.
    #[test]
    fn resolve_duplicate_view_and_flow_names() {
        let duplicates = |src: &str| {
            let ast = resolve(&[parse_string(src, "a.m3l.md")], None);
            ast.errors
                .iter()
                .filter(|e| e.code == "M3L-E005")
                .map(|e| e.message.clone())
                .collect::<Vec<_>>()
        };

        let view_over_model = duplicates(
            "## User
- id: identifier

## User ::view
- id: identifier",
        );
        assert_eq!(view_over_model.len(), 1, "{view_over_model:?}");
        assert!(view_over_model[0].contains("Duplicate view name \"User\""));

        let two_views = duplicates(
            "## Recent ::view
- id: identifier

## Recent ::view
- id: identifier",
        );
        assert_eq!(two_views.len(), 1, "{two_views:?}");

        let flow_over_model = duplicates(
            "## User
- id: identifier

## User ::flow
",
        );
        assert_eq!(flow_over_model.len(), 1, "{flow_over_model:?}");
        assert!(flow_over_model[0].contains("Duplicate flow name \"User\""));

        assert!(duplicates(
            "## User
- id: identifier

## Users ::view
- id: identifier"
        )
        .is_empty());
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
    fn resolve_attribute_registry_tagging_enum_value() {
        let input = "## help ::attribute\n- target: [value]\n- type: string\n\n## Status ::enum\n- active: \"Active\" @help(\"Currently in use\")";
        let parsed = parse_string(input, "test.m3l.md");
        let ast = resolve(&[parsed], None);
        assert_eq!(ast.attribute_registry.len(), 1);
        let value = &ast.enums[0].values[0];
        let help_attr = value
            .attributes
            .as_ref()
            .and_then(|attrs| attrs.iter().find(|a| a.name == "help"));
        assert!(help_attr.is_some());
        assert_eq!(help_attr.unwrap().is_registered, Some(true));
    }

    fn field_names(model: &ModelNode) -> Vec<&str> {
        model.fields.iter().map(|f| f.name.as_str()).collect()
    }

    #[test]
    fn a_qualified_parent_resolves_in_its_namespace() {
        let base = parse_string(
            "# Namespace: base\n## Stamped ::interface\n- created_at: timestamp",
            "base.m3l.md",
        );
        let app = parse_string(
            "# Namespace: app\n## Order : base.Stamped\n- id: identifier",
            "app.m3l.md",
        );
        let ast = resolve(&[base, app], None);
        assert!(ast.errors.is_empty(), "{:?}", ast.errors);
        let order = ast.models.iter().find(|m| m.name == "Order").unwrap();
        assert_eq!(field_names(order), vec!["created_at", "id"]);
        // The header keeps what was written.
        assert_eq!(order.inherits, vec!["base.Stamped".to_string()]);
    }

    #[test]
    fn a_qualified_enum_parent_resolves_in_its_namespace() {
        let base = parse_string(
            "# Namespace: base\n## Level ::enum\n- low: \"Low\"",
            "base.m3l.md",
        );
        let app = parse_string(
            "# Namespace: app\n## Priority ::enum : base.Level\n- high: \"High\"",
            "app.m3l.md",
        );
        let ast = resolve(&[base, app], None);
        assert!(ast.errors.is_empty(), "{:?}", ast.errors);
        let priority = ast.enums.iter().find(|e| e.name == "Priority").unwrap();
        let names: Vec<&str> = priority.values.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(names, vec!["low", "high"]);
    }

    #[test]
    fn a_qualified_parent_in_no_such_namespace_is_unresolved() {
        let base = parse_string(
            "# Namespace: base\n## Stamped ::interface\n- created_at: timestamp",
            "base.m3l.md",
        );
        let app = parse_string(
            "# Namespace: app\n## Order : other.Stamped\n- id: identifier",
            "app.m3l.md",
        );
        let ast = resolve(&[base, app], None);
        assert!(
            ast.errors
                .iter()
                .any(|e| e.code == "M3L-E007" && e.message.contains("other.Stamped")),
            "{:?}",
            ast.errors
        );
    }

    #[test]
    fn a_short_parent_name_prefers_the_referring_namespace() {
        let a = parse_string(
            "# Namespace: a\n## Stamp ::interface\n- from_a: string",
            "a.m3l.md",
        );
        let b = parse_string(
            "# Namespace: b\n## Stamp ::interface\n- from_b: string\n\n## Thing : Stamp\n- id: identifier",
            "b.m3l.md",
        );
        let ast = resolve(&[a, b], None);
        let thing = ast.models.iter().find(|m| m.name == "Thing").unwrap();
        assert_eq!(field_names(thing), vec!["from_b", "id"]);
    }

    #[test]
    fn a_short_parent_name_two_other_namespaces_declare_is_ambiguous() {
        let a = parse_string(
            "# Namespace: a\n## Stamp ::interface\n- from_a: string",
            "a.m3l.md",
        );
        let b = parse_string(
            "# Namespace: b\n## Stamp ::interface\n- from_b: string",
            "b.m3l.md",
        );
        let c = parse_string(
            "# Namespace: c\n## Thing : Stamp\n- id: identifier",
            "c.m3l.md",
        );
        let ast = resolve(&[a, b, c], None);
        let ambiguous: Vec<_> = ast
            .errors
            .iter()
            .filter(|e| e.code == "M3L-E008" && e.file == "c.m3l.md")
            .collect();
        assert_eq!(ambiguous.len(), 1, "{:?}", ast.errors);
        assert!(ambiguous[0].message.contains("a, b"));
        assert!(ambiguous[0].message.contains("qualified name"));
        let thing = ast.models.iter().find(|m| m.name == "Thing").unwrap();
        assert_eq!(field_names(thing), vec!["id"]);
    }

    #[test]
    fn the_same_name_in_two_namespaces_is_two_declarations() {
        // One namespace is one scope (§2.1.3 Rule 1): neither declaration is an error. A reference that cannot tell
        // them apart is M3L-E008 where it is written (the validator's check).
        let f1 = parse_string(
            "# Namespace: sales\n## Product\n- id: identifier",
            "sales.m3l.md",
        );
        let f2 = parse_string(
            "# Namespace: inventory\n## Product\n- id: identifier",
            "inventory.m3l.md",
        );
        let ast = resolve(&[f1, f2], None);
        assert!(ast.errors.is_empty(), "{:?}", ast.errors);
        assert_eq!(ast.models.len(), 2);
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

    fn complete_set() -> ResolveOptions {
        ResolveOptions {
            complete_file_set: true,
            ..ResolveOptions::default()
        }
    }

    #[test]
    fn an_import_naming_no_file_of_a_complete_set_is_e033_at_its_line() {
        let a = parse_string(
            "@import \"./missing.m3l\"\n\n## A\n- id: identifier",
            "app/a.m3l.md",
        );
        let b = parse_string("## B\n- id: identifier", "app/b.m3l.md");
        let ast = resolve_with(&[a, b], None, complete_set());
        let e033: Vec<_> = ast.errors.iter().filter(|e| e.code == "M3L-E033").collect();
        assert_eq!(e033.len(), 1, "{:?}", ast.errors);
        assert_eq!(e033[0].file, "app/a.m3l.md");
        assert_eq!(e033[0].line, 1);
        assert!(e033[0].message.contains("./missing.m3l"));
    }

    #[test]
    fn an_import_of_a_file_in_the_set_is_not_e033_with_or_without_md() {
        let a = parse_string(
            "@import \"./b.m3l\"\n@import \"./b.m3l.md\"\n\n## A\n- id: identifier",
            "app/a.m3l.md",
        );
        let b = parse_string("## B\n- id: identifier", "app/b.m3l.md");
        let ast = resolve_with(&[a, b], None, complete_set());
        assert!(
            !ast.errors.iter().any(|e| e.code == "M3L-E033"),
            "{:?}",
            ast.errors
        );
    }

    /// A file read on its own: nothing followed its imports, so their files being absent is no finding.
    #[test]
    fn a_single_file_read_on_its_own_does_not_judge_its_imports() {
        let a = parse_string("@import \"./b.m3l\"\n\n## A\n- id: identifier", "a.m3l.md");
        let ast = resolve(&[a], None);
        assert!(
            !ast.errors.iter().any(|e| e.code == "M3L-E033"),
            "{:?}",
            ast.errors
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

    fn cycle_codes(files: &[(&str, &str)], project_root: Option<&str>) -> Vec<String> {
        let parsed: Vec<_> = files.iter().map(|(n, s)| parse_string(s, n)).collect();
        let options = ResolveOptions {
            project_root: project_root.map(str::to_string),
            complete_file_set: false,
            ..ResolveOptions::default()
        };
        resolve_with(&parsed, None, options)
            .errors
            .into_iter()
            .filter(|e| e.code == "M3L-E003")
            .map(|e| e.message)
            .collect()
    }

    /// The imports are resolved to files before the graph is searched: a relative import is read
    /// from the importing file's directory, and the spec's `.m3l` spelling names a `.m3l.md` file.
    /// Comparing the import text with the `source` text found neither of these cycles.
    #[test]
    fn relative_imports_in_the_spec_spelling_form_a_cycle() {
        let messages = cycle_codes(
            &[
                (
                    "models/a.m3l.md",
                    "@import \"./shared/b.m3l\"\n## A\n- id: identifier",
                ),
                (
                    "models/shared/b.m3l.md",
                    "@import \"../a.m3l\"\n## B\n- id: identifier",
                ),
            ],
            None,
        );
        assert_eq!(
            messages,
            ["Circular import detected: models/a.m3l.md → models/shared/b.m3l.md → models/a.m3l.md"]
        );
    }

    /// A package path is read from the project root — which is not where the files are named
    /// from when they were read out of another directory.
    #[test]
    fn package_imports_resolve_from_the_project_root() {
        let files = [
            (
                "proj/a.m3l.md",
                "@import \"common/b.m3l\"\n## A\n- id: identifier",
            ),
            (
                "proj/common/b.m3l.md",
                "@import \"a.m3l.md\"\n## B\n- id: identifier",
            ),
        ];
        assert_eq!(cycle_codes(&files, Some("proj")).len(), 1);
        // Without the root those paths name files that were not loaded: no edge, no cycle.
        assert!(cycle_codes(&files, None).is_empty());
    }

    /// The cycle is reported where it closes: the `@import` line of the file whose import leads
    /// back into the chain — not line 1, which is a comment or a namespace header as often as not.
    #[test]
    fn a_cycle_is_reported_at_the_import_line_that_closes_it() {
        let parsed: Vec<_> = [
            (
                "a.m3l.md",
                "# Namespace: t\n\n@import \"./b.m3l\"\n## A\n- id: identifier",
            ),
            (
                "b.m3l.md",
                "# Namespace: t\n> about b\n\n@import \"./a.m3l\"\n## B\n- id: identifier",
            ),
        ]
        .iter()
        .map(|(n, s)| parse_string(s, n))
        .collect();
        let ast = resolve(&parsed, None);
        let at: Vec<(&str, usize)> = ast
            .errors
            .iter()
            .filter(|e| e.code == "M3L-E003")
            .map(|e| (e.file.as_str(), e.line))
            .collect();
        assert_eq!(at, [("b.m3l.md", 4)]);
    }

    #[test]
    fn an_import_of_a_file_that_was_not_loaded_is_not_an_edge() {
        let messages = cycle_codes(
            &[(
                "a.m3l.md",
                "@import \"./missing.m3l\"\n## A\n- id: identifier",
            )],
            None,
        );
        assert!(messages.is_empty());
    }

    #[test]
    fn paths_fold_dot_segments_lexically() {
        assert_eq!(normalize_path("./a/../b/./c.m3l.md"), "b/c.m3l.md");
        assert_eq!(normalize_path("../x/a.m3l"), "../x/a.m3l");
        assert_eq!(normalize_path("C:\\p\\q\\..\\r.m3l.md"), "C:/p/r.m3l.md");
        assert_eq!(normalize_path("/root/./a"), "/root/a");
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
        assert_eq!(parsed.imports[0].path, "base.m3l.md");
    }

    #[test]
    fn resolve_stamps_namespace_on_nodes() {
        // A multi-file project: each declared type must carry the namespace of
        // its source file so a consumer (e.g. a PG-DDL generator) can map each
        // model to its schema and qualify cross-schema references.
        let core = parse_string(
            "# Namespace: core\n## Item\n- id: identifier\n\n## Status ::enum\n- on: \"On\"",
            "core.m3l.md",
        );
        let stock = parse_string(
            "# Namespace: stock\n## StockMove\n- id: identifier",
            "stock.m3l.md",
        );
        let ast = resolve(&[core, stock], None);

        let item = ast.models.iter().find(|m| m.name == "Item").unwrap();
        let mv = ast.models.iter().find(|m| m.name == "StockMove").unwrap();
        assert_eq!(item.namespace.as_deref(), Some("core"));
        assert_eq!(mv.namespace.as_deref(), Some("stock"));

        let status = ast.enums.iter().find(|e| e.name == "Status").unwrap();
        assert_eq!(status.namespace.as_deref(), Some("core"));
    }

    #[test]
    fn resolve_namespace_none_when_absent() {
        let f = parse_string("## User\n- id: identifier", "no_ns.m3l.md");
        let ast = resolve(&[f], None);
        assert_eq!(ast.models[0].namespace, None);
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

#[cfg(test)]
mod enum_inheritance_tests {
    use super::*;
    use crate::parser::parse_string;

    /// Value names of an enum, in the order the resolver left them.
    fn values_of(ast: &M3lAst, name: &str) -> Vec<String> {
        ast.enums
            .iter()
            .find(|e| e.name == name)
            .unwrap_or_else(|| panic!("enum {name} not found"))
            .values
            .iter()
            .map(|v| v.name.clone())
            .collect()
    }

    fn codes(ast: &M3lAst) -> Vec<String> {
        ast.errors.iter().map(|e| e.code.clone()).collect()
    }

    /// The union, and its order. Asserted as a sequence rather than a set: §3.1.6 makes ordering
    /// part of the contract, and a set comparison would pass on a scrambled result.
    #[test]
    fn a_child_enum_carries_its_parents_values_first_then_its_own() {
        let parsed = parse_string(
            "## BasicStatus ::enum\n- active: \"Active\"\n- inactive: \"Inactive\"\n\n\
             ## UserStatus ::enum : BasicStatus\n- suspended: \"Suspended\"\n- banned: \"Banned\"",
            "t.m3l.md",
        );
        let ast = resolve(&[parsed], None);

        assert!(ast.errors.is_empty(), "unexpected: {:?}", ast.errors);
        assert_eq!(
            values_of(&ast, "UserStatus"),
            ["active", "inactive", "suspended", "banned"]
        );
        // The parent is untouched, and `inherits` still says what the source said.
        assert_eq!(values_of(&ast, "BasicStatus"), ["active", "inactive"]);
        assert_eq!(
            ast.enums
                .iter()
                .find(|e| e.name == "UserStatus")
                .unwrap()
                .inherits,
            ["BasicStatus"]
        );
    }

    #[test]
    fn a_grandparents_values_come_before_its_childs() {
        let parsed = parse_string(
            "## A ::enum\n- a: \"A\"\n\n## B ::enum : A\n- b: \"B\"\n\n## C ::enum : B\n- c: \"C\"",
            "t.m3l.md",
        );
        let ast = resolve(&[parsed], None);

        assert!(ast.errors.is_empty(), "unexpected: {:?}", ast.errors);
        assert_eq!(values_of(&ast, "C"), ["a", "b", "c"]);
    }

    #[test]
    fn multiple_parents_resolve_left_to_right() {
        let parsed = parse_string(
            "## Draft ::enum\n- draft: \"Draft\"\n\n## Published ::enum\n- published: \"Published\"\n\n\
             ## PostState ::enum : Draft, Published\n- archived: \"Archived\"",
            "t.m3l.md",
        );
        let ast = resolve(&[parsed], None);

        assert!(ast.errors.is_empty(), "unexpected: {:?}", ast.errors);
        assert_eq!(
            values_of(&ast, "PostState"),
            ["draft", "published", "archived"]
        );
    }

    /// The diamond: the same value reached by two paths is one value, not a collision.
    #[test]
    fn a_value_reached_by_two_paths_is_kept_once() {
        let parsed = parse_string(
            "## Base ::enum\n- shared: \"Shared\"\n\n\
             ## Left ::enum : Base\n- left: \"Left\"\n\n\
             ## Right ::enum : Base\n- right: \"Right\"\n\n\
             ## Both ::enum : Left, Right\n- own: \"Own\"",
            "t.m3l.md",
        );
        let ast = resolve(&[parsed], None);

        assert!(ast.errors.is_empty(), "unexpected: {:?}", ast.errors);
        assert_eq!(values_of(&ast, "Both"), ["shared", "left", "right", "own"]);
    }

    /// ...but only when they agree. Two parents that spell the same name differently are refused,
    /// because picking one would make the enum's meaning depend on the order of its parent list.
    #[test]
    fn two_parents_that_disagree_about_a_value_are_refused() {
        let parsed = parse_string(
            "## Left ::enum\n- shared: \"From the left\"\n\n\
             ## Right ::enum\n- shared: \"From the right\"\n\n\
             ## Both ::enum : Left, Right\n- own: \"Own\"",
            "t.m3l.md",
        );
        let ast = resolve(&[parsed], None);

        assert!(
            codes(&ast).contains(&"M3L-E020".to_string()),
            "{:?}",
            ast.errors
        );
    }

    #[test]
    fn a_child_redeclaring_an_inherited_value_is_refused() {
        let parsed = parse_string(
            "## Base ::enum\n- active: \"Active\"\n\n\
             ## Child ::enum : Base\n- active: \"Still active\"",
            "t.m3l.md",
        );
        let ast = resolve(&[parsed], None);

        assert!(
            codes(&ast).contains(&"M3L-E020".to_string()),
            "{:?}",
            ast.errors
        );
    }

    /// Previously unreported: a block that repeats a name on its own.
    #[test]
    fn a_name_declared_twice_in_one_block_is_refused() {
        let parsed = parse_string(
            "## Status ::enum\n- active: \"Active\"\n- active: \"Again\"",
            "t.m3l.md",
        );
        let ast = resolve(&[parsed], None);

        assert!(
            codes(&ast).contains(&"M3L-E020".to_string()),
            "{:?}",
            ast.errors
        );
    }

    #[test]
    fn an_unresolved_parent_is_reported_like_a_models() {
        let parsed = parse_string("## Child ::enum : Nowhere\n- own: \"Own\"", "t.m3l.md");
        let ast = resolve(&[parsed], None);

        assert!(
            codes(&ast).contains(&"M3L-E007".to_string()),
            "{:?}",
            ast.errors
        );
    }

    /// A parent list that loops does not hang the resolver.
    #[test]
    fn a_cycle_terminates() {
        let parsed = parse_string(
            "## A ::enum : B\n- a: \"A\"\n\n## B ::enum : A\n- b: \"B\"",
            "t.m3l.md",
        );
        let ast = resolve(&[parsed], None);

        // Whatever it reports, it returns.
        assert!(ast.enums.len() == 2);
    }

    /// The negative control for the switch: with inlining off, the values stay as declared.
    ///
    /// This is what the formatter reads. If flattening ignored the option, a round-trip would
    /// re-emit every inherited value *and* the `: Parent` header, growing the enum on each pass —
    /// the failure `inline_inherited` exists to prevent, one type over from where it was found.
    #[test]
    fn inlining_off_leaves_an_enums_own_values_alone() {
        let source = "## Base ::enum\n- active: \"Active\"\n\n\
                      ## Child ::enum : Base\n- own: \"Own\"";
        let parsed = parse_string(source, "t.m3l.md");
        let ast = resolve_with(
            &[parsed],
            None,
            ResolveOptions {
                inline_inherited: false,
                merge_extends: true,
                project_root: None,
                complete_file_set: false,
            },
        );

        assert_eq!(values_of(&ast, "Child"), ["own"]);
    }
}

#[cfg(test)]
mod extend_kind_arg_tests {
    use super::*;
    use crate::parser::parse_string;

    fn codes(src: &str) -> Vec<String> {
        let ast = resolve(&[parse_string(src, "t.m3l.md")], None);
        ast.errors.iter().map(|e| e.code.clone()).collect()
    }

    /// An argument that names the intended target is refused, because the block does not use it.
    #[test]
    fn an_extend_argument_is_refused() {
        let out =
            codes("## Base\n- id: identifier @pk\n\n## Other ::extend(Base)\n- extra: string");

        assert!(out.contains(&"M3L-E021".to_string()), "{out:?}");
    }

    /// Empty parentheses are refused too. They merge correctly today, which is exactly why the
    /// author never learns that the parentheses mean nothing — until they put a name in them.
    #[test]
    fn empty_extend_parentheses_are_refused() {
        let out = codes("## Base\n- id: identifier @pk\n\n## Base ::extend()\n- extra: string");

        assert!(out.contains(&"M3L-E021".to_string()), "{out:?}");
    }

    /// The negative control: the ordinary form draws no diagnostic, so the two assertions above
    /// report the parentheses rather than firing on every extend block.
    #[test]
    fn an_extend_without_parentheses_is_not_refused() {
        let out = codes("## Base\n- id: identifier @pk\n\n## Base ::extend\n- extra: string");

        assert!(!out.contains(&"M3L-E021".to_string()), "{out:?}");
        assert!(out.is_empty(), "{out:?}");
    }
}
