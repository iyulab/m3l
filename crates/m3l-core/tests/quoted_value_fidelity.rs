//! Unwrapping a quoted value removes the one pair of delimiters that wraps it.
//! A quote character that belongs to the value -- SQL string literals are full
//! of them -- is not a delimiter and must survive.

use m3l_core::{parse_string, resolve, ValidateOptions};

fn model<'a>(ast: &'a m3l_core::M3lAst, name: &str) -> &'a m3l_core::ModelNode {
    ast.models
        .iter()
        .find(|m| m.name == name)
        .expect("the document declares this model")
}

fn field<'a>(
    ast: &'a m3l_core::M3lAst,
    model_name: &str,
    field_name: &str,
) -> &'a m3l_core::FieldNode {
    model(ast, model_name)
        .fields
        .iter()
        .find(|f| f.name == field_name)
        .expect("the model declares this field")
}

fn parse(input: &str) -> m3l_core::M3lAst {
    let parsed = parse_string(input, "quoted_value_fidelity.m3l.md");
    let ast = resolve(&[parsed], None);
    m3l_core::validate(&ast, &ValidateOptions { strict: false });
    ast
}

/// A computed expression that ends in a SQL string literal keeps its closing
/// quote -- without it the expression is not valid SQL any more.
#[test]
fn a_computed_expression_ending_in_a_string_literal_keeps_its_closing_quote() {
    let ast = parse(
        r#"
# Namespace: test

## Thing

- id: identifier @pk @generated
- label: string @computed("name || '-'")
"#,
    );

    let computed = field(&ast, "Thing", "label")
        .computed
        .as_ref()
        .expect("label is a computed field");
    assert_eq!(computed.expression, "name || '-'");
}

/// Same for the raw variant, whose expression is the one the generator emits
/// verbatim into the target dialect.
#[test]
fn a_raw_computed_expression_ending_in_a_string_literal_keeps_its_closing_quote() {
    let ast = parse(
        r#"
# Namespace: test

## Thing

- id: identifier @pk @generated
- json_val: string @computed_raw("metadata->>'category'", platform: "postgresql")
"#,
    );

    let computed = field(&ast, "Thing", "json_val")
        .computed
        .as_ref()
        .expect("json_val is a computed field");
    assert_eq!(computed.expression, "metadata->>'category'");
    assert_eq!(computed.platform.as_deref(), Some("postgresql"));
}

/// A metadata string carries whatever the document quoted, inner quotes and all.
#[test]
fn a_metadata_string_keeps_quote_characters_that_belong_to_it() {
    let ast = parse(
        r#"
# Namespace: test

## Thing

- id: identifier @pk @generated

### Metadata
- note: "ends with 'a quoted word'"
"#,
    );

    assert_eq!(
        model(&ast, "Thing").sections.metadata.get("note"),
        Some(&serde_json::json!("ends with 'a quoted word'"))
    );
}

/// And so does a description given through the extended field format.
#[test]
fn an_extended_format_description_keeps_quote_characters_that_belong_to_it() {
    let ast = parse(
        r#"
# Namespace: test

## Thing

- id: identifier @pk @generated
- label
  - type: string
  - description: "ends with 'a quoted word'"
"#,
    );

    assert_eq!(
        field(&ast, "Thing", "label").description.as_deref(),
        Some("ends with 'a quoted word'")
    );
}
