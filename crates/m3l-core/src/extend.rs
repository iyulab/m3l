//! Merging `::extend` blocks into the models they target, and the checks on
//! models that name a base (`::aspect` / `::subtype`).

use std::collections::{HashMap, HashSet};

use crate::types::{Diagnostic, DiagnosticSeverity, ExtendedBy, FieldOrigin, ModelNode};

const NOT_ALLOWED_IN_EXTEND: [&str; 3] = ["pk", "primary", "override"];

/// Names the first non-field construct an extend block declares, for the `M3L-E015`
/// message — a model attribute (`## Target ::extend @attr`) or a section (`### Indexes`,
/// `### Relations`, `### Behaviors`, `### Metadata`, or a composite `@unique`/`@index`
/// directive, which lands in the same `sections.indexes` bucket as a declared one).
fn non_field_construct(block: &ModelNode) -> Option<&'static str> {
    if !block.attributes.is_empty() {
        return Some("a model attribute");
    }
    if !block.sections.indexes.is_empty()
        || !block.sections.relations.is_empty()
        || !block.sections.behaviors.is_empty()
        || !block.sections.metadata.is_empty()
        || !block.sections.custom.is_empty()
    {
        return Some("a section");
    }
    None
}

fn error(code: &str, file: &str, line: usize, message: String) -> Diagnostic {
    Diagnostic {
        code: code.to_string(),
        severity: DiagnosticSeverity::Error,
        file: file.to_string(),
        line,
        col: 1,
        message,
    }
}

/// Merges `blocks` into `models` in the order given. Returns nothing; failures go to `errors`.
pub(crate) fn merge_extend_blocks(
    models: &mut [ModelNode],
    blocks: Vec<ModelNode>,
    aliases: &crate::names::Aliases,
    errors: &mut Vec<Diagnostic>,
) {
    // Each model by short name, with what the lookup reads — copied, since the models are written below.
    let mut index: HashMap<String, Vec<(usize, Option<String>, String)>> = HashMap::new();
    for (i, m) in models.iter().enumerate() {
        index
            .entry(m.name.clone())
            .or_default()
            .push((i, m.namespace.clone(), m.source.clone()));
    }

    for block in blocks {
        // The block names its target the way a reference does (§2.1.3): qualified, or short and
        // read from the block's own namespace and file.
        let candidates = index
            .get(crate::names::short_name(&block.name))
            .map(Vec::as_slice)
            .unwrap_or_default();
        let from = crate::names::Referrer {
            ns: block.namespace.as_deref(),
            source: &block.source,
            aliases,
        };
        let found = crate::names::pick(
            candidates,
            &block.name,
            from,
            |c| c.1.as_deref(),
            |c| c.2.as_str(),
        );
        let target = match found {
            crate::names::Lookup::Found(&(i, _, _)) => Some(i),
            crate::names::Lookup::Ambiguous(namespaces) => {
                errors.push(error(
                    "M3L-E008",
                    &block.source,
                    block.line,
                    format!(
                        "Ambiguous extend target \"{}\" in namespaces {} — write the qualified name",
                        block.name,
                        namespaces.join(", ")
                    ),
                ));
                continue;
            }
            crate::names::Lookup::Missing => None,
        };
        let Some(target) = target else {
            errors.push(error(
                "M3L-E011",
                &block.source,
                block.line,
                format!(
                    "Extend target \"{}\" is not a model in this project",
                    block.name
                ),
            ));
            continue;
        };

        let mut block_ok = true;
        if !block.inherits.is_empty() {
            errors.push(error(
                "M3L-E014",
                &block.source,
                block.line,
                format!(
                    "Extend block \"{}\" cannot declare parents — inheritance belongs to the model itself",
                    block.name
                ),
            ));
            block_ok = false;
        }
        if let Some(construct) = non_field_construct(&block) {
            errors.push(error(
                "M3L-E015",
                &block.source,
                block.line,
                format!(
                    "Extend block \"{}\" declares {construct} — extend blocks may declare fields only",
                    block.name
                ),
            ));
            block_ok = false;
        }
        if !block_ok {
            continue;
        }

        let model = &mut models[target];
        let mut taken: HashSet<String> = model.fields.iter().map(|f| f.name.clone()).collect();
        let mut merged = 0usize;

        for mut field in block.fields {
            if let Some(bad) = field
                .attributes
                .iter()
                .find(|a| NOT_ALLOWED_IN_EXTEND.contains(&a.name.as_str()))
            {
                errors.push(error(
                    "M3L-E013",
                    &field.loc.file,
                    field.loc.line,
                    format!(
                        "@{} is not allowed on \"{}\" — an extend block cannot change the key or override a field of \"{}\"",
                        bad.name, field.name, model.name
                    ),
                ));
                continue;
            }
            if !taken.insert(field.name.clone()) {
                errors.push(error(
                    "M3L-E012",
                    &field.loc.file,
                    field.loc.line,
                    format!(
                        "Extend field \"{}\" collides with an existing field of \"{}\"",
                        field.name, model.name
                    ),
                ));
                continue;
            }
            field.origin = Some(FieldOrigin {
                prefix: block.prefix.clone(),
                namespace: block.namespace.clone(),
                source: block.source.clone(),
            });
            model.fields.push(field);
            merged += 1;
        }

        model.extended_by.push(ExtendedBy {
            prefix: block.prefix.clone(),
            namespace: block.namespace.clone(),
            source: block.source.clone(),
            fields: merged,
        });
    }
}

/// E016–E018 for every model that carries a `base`.
pub(crate) fn check_model_bases(
    models: &[ModelNode],
    aliases: &crate::names::Aliases,
    errors: &mut Vec<Diagnostic>,
) {
    let mut by_name: HashMap<&str, Vec<&ModelNode>> = HashMap::new();
    for m in models {
        by_name.entry(m.name.as_str()).or_default().push(m);
    }
    for model in models {
        let Some(base) = &model.base else { continue };
        if base.model.is_empty() {
            errors.push(error(
                "M3L-E016",
                &model.source,
                model.line,
                format!(
                    "\"{}\" ::{} needs a base model — write ::{}(Base)",
                    model.name, base.kind, base.kind
                ),
            ));
            continue;
        }
        // Read from the model that names its base (§2.1.3), qualified or short.
        let candidates = by_name
            .get(crate::names::short_name(&base.model))
            .map(Vec::as_slice)
            .unwrap_or_default();
        let from = crate::names::Referrer {
            ns: model.namespace.as_deref(),
            source: &model.source,
            aliases,
        };
        let found = match crate::names::pick(
            candidates,
            &base.model,
            from,
            |m| m.namespace.as_deref(),
            |m| m.source.as_str(),
        ) {
            crate::names::Lookup::Found(b) => Some(*b),
            crate::names::Lookup::Ambiguous(namespaces) => {
                errors.push(error(
                    "M3L-E008",
                    &model.source,
                    model.line,
                    format!(
                        "Ambiguous base model \"{}\" of \"{}\" in namespaces {} — write the qualified name",
                        base.model,
                        model.name,
                        namespaces.join(", ")
                    ),
                ));
                continue;
            }
            crate::names::Lookup::Missing => None,
        };
        match found {
            None => errors.push(error(
                "M3L-E017",
                &model.source,
                model.line,
                format!(
                    "Base model \"{}\" of \"{}\" is not a model in this project",
                    base.model, model.name
                ),
            )),
            Some(b) if b.base.as_ref().is_some_and(|bb| bb.kind == "aspect") => errors.push(error(
                "M3L-E018",
                &model.source,
                model.line,
                format!(
                    "\"{}\" cannot use \"{}\" as its base — an aspect cannot be a base",
                    model.name, base.model
                ),
            )),
            Some(_) => {}
        }
    }
}
