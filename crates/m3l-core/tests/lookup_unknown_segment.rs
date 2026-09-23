//! `M3L-E022`: every segment of a Lookup path names a field of the model reached at that point.
//!
//! Specification 4.5.4 ("Path resolution"). `@lookup(order_id.nothing_id.name)` used to validate
//! clean: the walk stopped at the first segment it could not find and said nothing, so the mistake
//! surfaced only in whatever consumed the AST.

use m3l_core::{parse_string, validate, ValidateOptions};

fn unknown_segment_errors(input: &str) -> Vec<String> {
    let parsed = parse_string(input, "lookup_segments.m3l.md");
    let ast = m3l_core::resolve(&[parsed], None);
    validate(&ast, &ValidateOptions::default())
        .errors
        .iter()
        .filter(|e| e.code == "M3L-E022")
        .map(|e| e.message.clone())
        .collect()
}

const MODELS: &str = r#"
# Namespace: probe

## Named ::interface
- name: string(100)

## Customer : Named
- id: identifier @pk
- nickname: string(50) @lookup(referrer_id.name)
- referrer_id: identifier? @reference(Customer)

## Order
- id: identifier @pk
- customer_id: identifier @reference(Customer)
- archive_id: identifier @reference(Archived)
"#;

fn with_field(field: &str) -> String {
    format!("{MODELS}\n## Line\n- id: identifier @pk\n- order_id: identifier @reference(Order)\n{field}\n")
}

/// Kept first: the tests after it assert absences, which prove nothing unless the check can fire.
#[test]
fn an_unknown_later_hop_is_reported_on_the_model_it_was_looked_up_on() {
    let errors =
        unknown_segment_errors(&with_field("- x: string @lookup(order_id.nothing_id.name)"));
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("\"nothing_id\""), "{}", errors[0]);
    assert!(errors[0].contains("\"Order\""), "{}", errors[0]);
}

#[test]
fn an_unknown_first_hop_is_reported_on_the_declaring_model() {
    let errors = unknown_segment_errors(&with_field("- x: string @lookup(nothing_id.name)"));
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("\"nothing_id\"") && errors[0].contains("\"Line\""),
        "{}",
        errors[0]
    );
}

#[test]
fn an_unknown_last_segment_is_reported_on_the_final_model() {
    let errors = unknown_segment_errors(&with_field(
        "- x: string @lookup(order_id.customer_id.surname)",
    ));
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("\"surname\"") && errors[0].contains("\"Customer\""),
        "{}",
        errors[0]
    );
}

#[test]
fn a_resolvable_chain_is_silent() {
    assert!(
        unknown_segment_errors(&with_field("- x: string @lookup(order_id.customer_id.id)"))
            .is_empty()
    );
}

/// Fields reach a model through inheritance as well as by declaration.
#[test]
fn an_inherited_field_is_a_field() {
    assert!(unknown_segment_errors(&with_field(
        "- x: string @lookup(order_id.customer_id.name)"
    ))
    .is_empty());
}

/// A Lookup may read another Lookup field — it is a field of that model.
#[test]
fn a_lookup_field_is_a_field() {
    assert!(unknown_segment_errors(&with_field(
        "- x: string @lookup(order_id.customer_id.nickname)"
    ))
    .is_empty());
}

/// A reference to a model this document does not define cannot be followed; a partial document is
/// not told its paths are wrong.
#[test]
fn a_path_through_an_undefined_model_is_not_reported() {
    assert!(unknown_segment_errors(&with_field(
        "- x: string @lookup(order_id.archive_id.anything)"
    ))
    .is_empty());
}
