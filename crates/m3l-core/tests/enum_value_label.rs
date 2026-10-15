//! An enum value may carry a display label in parentheses after its name — the
//! `name(Label)` form fields and models use. The lexer always read it; the AST had
//! no place for it, so the label was dropped without a word.

use m3l_core::{parse_string, resolve, EnumValue, ValidateOptions};

fn parse(input: &str) -> m3l_core::M3lAst {
    let parsed = parse_string(input, "enum_value_label.m3l.md");
    let ast = resolve(&[parsed], None);
    m3l_core::validate(&ast, &ValidateOptions { strict: false });
    ast
}

fn standalone<'a>(ast: &'a m3l_core::M3lAst, enum_name: &str, value: &str) -> &'a EnumValue {
    ast.enums
        .iter()
        .find(|e| e.name == enum_name)
        .expect("the document declares this enum")
        .values
        .iter()
        .find(|v| v.name == value)
        .expect("the enum declares this value")
}

#[test]
fn a_standalone_enum_value_keeps_its_label_and_its_description() {
    let ast = parse(
        r#"
# Namespace: test

## Status ::enum
- open(Opened): "The request is waiting for someone to pick it up"
- closed: "Done"
"#,
    );

    let open = standalone(&ast, "Status", "open");
    assert_eq!(open.label.as_deref(), Some("Opened"));
    assert_eq!(
        open.description.as_deref(),
        Some("The request is waiting for someone to pick it up")
    );
    assert_eq!(standalone(&ast, "Status", "closed").label, None);
}

#[test]
fn a_value_written_without_the_colon_keeps_its_label_too() {
    let ast = parse(
        r#"
# Namespace: test

## Status ::enum
- open(Opened) "The request is waiting"
"#,
    );

    let open = standalone(&ast, "Status", "open");
    assert_eq!(open.label.as_deref(), Some("Opened"));
    assert_eq!(open.description.as_deref(), Some("The request is waiting"));
}

#[test]
fn a_labeled_value_keeps_its_attributes() {
    let ast = parse(
        r#"
# Namespace: test

## PaymentMethod ::enum
- legacy(Legacy): "Carried over from the old system" @system
"#,
    );

    let legacy = standalone(&ast, "PaymentMethod", "legacy");
    assert_eq!(legacy.label.as_deref(), Some("Legacy"));
    assert_eq!(
        legacy
            .attributes
            .as_ref()
            .map(|a| a.iter().map(|x| x.name.as_str()).collect::<Vec<_>>()),
        Some(vec!["system"])
    );
}

#[test]
fn an_inline_enum_value_keeps_its_label() {
    let ast = parse(
        r#"
# Namespace: test

## Ticket
- id: identifier @pk
- state: enum = "open"
  - values:
    - open(Opened): "Waiting"
    - closed(Closed): "Done"
"#,
    );

    let values = ast
        .models
        .iter()
        .find(|m| m.name == "Ticket")
        .unwrap()
        .fields
        .iter()
        .find(|f| f.name == "state")
        .unwrap()
        .enum_values
        .as_ref()
        .expect("the field declares inline values");
    let labels: Vec<_> = values
        .iter()
        .map(|v| {
            (
                v.name.as_str(),
                v.label.as_deref(),
                v.description.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        labels,
        vec![
            ("open", Some("Opened"), Some("Waiting")),
            ("closed", Some("Closed"), Some("Done"))
        ]
    );
}

#[test]
fn a_value_without_a_label_serializes_exactly_as_before() {
    let ast = parse(
        r#"
# Namespace: test

## Status ::enum
- closed: "Done"
"#,
    );

    let json = serde_json::to_string(standalone(&ast, "Status", "closed")).unwrap();
    assert!(!json.contains("label"), "{json}");
}

/// Two parents that give the same value different labels disagree about it.
#[test]
fn parents_that_label_the_same_value_differently_collide() {
    let ast = parse(
        r#"
# Namespace: test

## A ::enum
- open(Opened): "Waiting"

## B ::enum
- open(Pending): "Waiting"

## C ::enum : A, B
- closed: "Done"
"#,
    );

    assert!(
        ast.errors.iter().any(|d| d.code == "M3L-E020"),
        "{:?}",
        ast.errors
    );
}

#[test]
fn an_inline_enum_value_without_the_values_key_keeps_its_label() {
    let ast = parse(
        r#"
# Namespace: test

## Ticket
- id: identifier @pk
- state: enum = "open"
  - open(Opened): "Waiting"
  - closed: "Done"
"#,
    );

    let values = ast.models[0]
        .fields
        .iter()
        .find(|f| f.name == "state")
        .unwrap()
        .enum_values
        .as_ref()
        .expect("the field declares inline values");
    assert_eq!(values[0].label.as_deref(), Some("Opened"));
    assert_eq!(values[1].label, None);
}
