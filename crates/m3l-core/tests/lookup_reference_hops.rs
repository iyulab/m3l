//! `M3L-E002` applies to every FK hop of a Lookup path, not only the first.
//!
//! Specification 4.5.4: "Each FK field in the Lookup path must have a
//! `@reference` attribute declared." A path of `a.b.name` has two FK fields --
//! `a` on the declaring model, and `b` on the model `a` references. The check
//! used to read only the first segment, so a chain whose second key was a plain
//! column validated clean and failed later, in whatever consumed the AST.

use m3l_core::{parse_string, validate, ValidateOptions};

fn lookup_errors(input: &str) -> Vec<String> {
    let parsed = parse_string(input, "lookup_hops.m3l.md");
    let ast = m3l_core::resolve(&[parsed], None);
    validate(&ast, &ValidateOptions::default())
        .errors
        .iter()
        .filter(|e| e.code == "M3L-E002")
        .map(|e| e.message.clone())
        .collect()
}

const MODELS: &str = r#"
# Namespace: probe

## Customer
- id: identifier @pk
- name: string(100)

## Order
- id: identifier @pk
- customer_id: identifier @reference(Customer)
- plain_customer_id: identifier
- fk_customer_id: identifier @fk(Customer.id)
"#;

fn with_line(field: &str) -> String {
    format!(
        "{MODELS}\n## Line\n- id: identifier @pk\n- order_id: identifier @reference(Order)\n- plain_order_id: identifier\n{field}\n"
    )
}

/// The first hop, as before -- kept first because the tests after it assert
/// absences, and an absence proves nothing unless the check can fire.
#[test]
fn a_first_hop_without_reference_is_reported() {
    let errors = lookup_errors(&with_line("- x: string @lookup(plain_order_id.name)"));
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("\"plain_order_id\""), "{}", errors[0]);
}

/// The case this file exists for: the second key is a plain column.
#[test]
fn a_later_hop_without_reference_is_reported_naming_the_model_it_is_on() {
    let errors = lookup_errors(&with_line(
        "- x: string @lookup(order_id.plain_customer_id.name)",
    ));
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("\"plain_customer_id\""), "{}", errors[0]);
    assert!(errors[0].contains("on \"Order\""), "{}", errors[0]);
}

#[test]
fn a_chain_whose_every_hop_is_a_reference_is_silent() {
    assert!(lookup_errors(&with_line("- x: string @lookup(order_id.customer_id.name)")).is_empty());
}

/// `@fk(Model.field)` counts as a reference, at any hop, and names the model to
/// step into before the dot.
#[test]
fn an_fk_attribute_counts_at_a_later_hop() {
    assert!(lookup_errors(&with_line(
        "- x: string @lookup(order_id.fk_customer_id.name)"
    ))
    .is_empty());
}

/// The walk steps into the model each hop references -- a key that exists only
/// on some other model is not found there. That is an unknown field
/// (`M3L-E022`, see `lookup_unknown_segment.rs`), not a missing reference.
#[test]
fn an_unknown_later_hop_is_not_reported_as_a_missing_reference() {
    assert!(lookup_errors(&with_line("- x: string @lookup(order_id.nothing_id.name)")).is_empty());
}

/// Three hops: the report names the hop that fails, not the first one.
#[test]
fn the_failing_hop_is_the_one_named_in_a_longer_chain() {
    let input = format!(
        "{MODELS}\n## Line\n- id: identifier @pk\n- order_id: identifier @reference(Order)\n\n## Note\n- id: identifier @pk\n- line_id: identifier @reference(Line)\n- x: string @lookup(line_id.order_id.plain_customer_id.name)\n"
    );
    let errors = lookup_errors(&input);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("\"plain_customer_id\""), "{}", errors[0]);
    assert!(errors[0].contains("on \"Order\""), "{}", errors[0]);
}
