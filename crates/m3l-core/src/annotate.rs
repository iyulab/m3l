//! Writes into the AST what each written name resolves to (§2.1.3) — the `qualifiedName` of every declaration, and
//! beside every name a declaration writes (a parent, a base, a key's target, a rollup target, a field type that names
//! a declaration, a view's sources) the `qualifiedName` of the declaration it names. A consumer reads the resolution
//! instead of re-deriving it: two declarations of one short name are told apart by these keys, and a re-resolution
//! that disagreed with the resolver would be a second, silently different answer.
//!
//! A name that resolves to none — or to more than one — is left without a `…QualifiedName`; the check that owns that
//! name reports it.

use crate::names::{pick, short_name, Aliases, Lookup, Referrer};
use crate::types::*;
use std::collections::HashMap;

/// `namespace.Name`, or the name alone in the default namespace.
pub(crate) fn qualified_name(namespace: Option<&str>, name: &str) -> String {
    match namespace {
        Some(ns) => format!("{ns}.{name}"),
        None => name.to_string(),
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Model,
    Interface,
    View,
    Flow,
    Enum,
}

struct Declared {
    kind: Kind,
    namespace: Option<String>,
    source: String,
    qualified: String,
}

struct Index {
    by_name: HashMap<String, Vec<Declared>>,
}

impl Index {
    /// The `qualifiedName` of the one declaration of `kinds` that `written`, read from `from`, names.
    fn find(&self, written: &str, from: Referrer<'_>, kinds: &[Kind]) -> Option<String> {
        let candidates: Vec<&Declared> = self
            .by_name
            .get(short_name(written))
            .map(|all| all.iter().filter(|d| kinds.contains(&d.kind)).collect())
            .unwrap_or_default();
        match pick(
            &candidates,
            written,
            from,
            |d| d.namespace.as_deref(),
            |d| d.source.as_str(),
        ) {
            Lookup::Found(d) => Some(d.qualified.clone()),
            _ => None,
        }
    }
}

const MODEL_LIKE: &[Kind] = &[Kind::Model, Kind::View, Kind::Flow];
const PARENTS: &[Kind] = &[Kind::Model, Kind::Interface];
const TYPES: &[Kind] = &[Kind::Enum, Kind::Model, Kind::Interface];

/// The model a key's `@reference(Model)` / `@fk(Model.field)` points at, as written — qualified or short — if the
/// field is such a key. `@reference` names only the model, so a dot in it is a namespace's; `@fk` names the field last.
pub(crate) fn referenced_model(field: &FieldNode) -> Option<&str> {
    let reference = field
        .attributes
        .iter()
        .find(|a| a.name == "reference" || a.name == "fk")?;
    match reference.args.as_deref() {
        Some([AttrArgValue::String(arg), ..]) if reference.name == "fk" => Some(
            arg.rsplit_once('.')
                .map_or(arg.as_str(), |(model, _)| model),
        ),
        Some([AttrArgValue::String(arg), ..]) => Some(arg.as_str()),
        _ => None,
    }
}

pub(crate) fn annotate(
    models: &mut [ModelNode],
    interfaces: &mut [ModelNode],
    views: &mut [ModelNode],
    flows: &mut [ModelNode],
    enums: &mut [EnumNode],
    extend_blocks: &mut [ModelNode],
    aliases: &Aliases,
) {
    let mut index = Index {
        by_name: HashMap::new(),
    };
    let groups: [(&[ModelNode], Kind); 4] = [
        (models, Kind::Model),
        (interfaces, Kind::Interface),
        (views, Kind::View),
        (flows, Kind::Flow),
    ];
    for (nodes, kind) in groups {
        for m in nodes {
            index
                .by_name
                .entry(m.name.clone())
                .or_default()
                .push(Declared {
                    kind,
                    namespace: m.namespace.clone(),
                    source: m.source.clone(),
                    qualified: qualified_name(m.namespace.as_deref(), &m.name),
                });
        }
    }
    for e in enums.iter() {
        index
            .by_name
            .entry(e.name.clone())
            .or_default()
            .push(Declared {
                kind: Kind::Enum,
                namespace: e.namespace.clone(),
                source: e.source.clone(),
                qualified: qualified_name(e.namespace.as_deref(), &e.name),
            });
    }

    for node in models
        .iter_mut()
        .chain(interfaces.iter_mut())
        .chain(views.iter_mut())
        .chain(flows.iter_mut())
        .chain(extend_blocks.iter_mut())
    {
        annotate_model(node, &index, aliases);
    }
    for e in enums.iter_mut() {
        let from = Referrer {
            ns: e.namespace.as_deref(),
            source: &e.source,
            aliases,
        };
        e.inherits_qualified_names = e
            .inherits
            .iter()
            .map(|p| index.find(p, from, &[Kind::Enum]))
            .collect();
        e.qualified_name = Some(qualified_name(e.namespace.as_deref(), &e.name));
    }
}

fn annotate_model(node: &mut ModelNode, index: &Index, aliases: &Aliases) {
    let namespace = node.namespace.clone();
    let source = node.source.clone();
    let from = Referrer {
        ns: namespace.as_deref(),
        source: &source,
        aliases,
    };
    node.qualified_name = Some(qualified_name(namespace.as_deref(), &node.name));
    node.inherits_qualified_names = node
        .inherits
        .iter()
        .map(|p| index.find(p, from, PARENTS))
        .collect();
    if let Some(base) = node.base.as_mut() {
        base.qualified_name = index.find(&base.model, from, &[Kind::Model]);
    }
    if let Some(sd) = node.source_def.as_mut() {
        sd.from_qualified_name = sd
            .from
            .as_deref()
            .and_then(|f| index.find(f, from, MODEL_LIKE));
        for join in sd.joins.iter_mut().flatten() {
            join.qualified_name = index.find(&join.model, from, MODEL_LIKE);
        }
    }
    annotate_fields(&mut node.fields, from, index);
}

fn annotate_fields(fields: &mut [FieldNode], from: Referrer<'_>, index: &Index) {
    for field in fields {
        // A field an `::extend` block added is read where that block was written (§3.4.7).
        let origin_source;
        let field_from = match &field.origin {
            Some(origin) => {
                origin_source = origin.source.clone();
                Referrer {
                    ns: origin.namespace.as_deref(),
                    source: &origin_source,
                    aliases: from.aliases,
                }
            }
            None => from,
        };
        field.reference_qualified_name =
            referenced_model(field).and_then(|m| index.find(m, field_from, MODEL_LIKE));
        field.type_qualified_name = field
            .field_type
            .as_deref()
            .and_then(|t| index.find(t, field_from, TYPES));
        if let Some(rollup) = field.rollup.as_mut() {
            rollup.target_qualified_name = index.find(&rollup.target, field_from, MODEL_LIKE);
        }
        if let Some(nested) = field.fields.as_mut() {
            annotate_fields(nested, field_from, index);
        }
    }
}
