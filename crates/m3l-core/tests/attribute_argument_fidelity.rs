//! An attribute argument reaches the AST spelled the way the document spelled
//! it. The lexer decides where one argument ends, not what the characters
//! inside it mean -- so a colon there stays part of the value, and reading a
//! `key: value` shape out of it is left to whichever attribute defines one.

use m3l_core::{parse_string, AttrArgValue};

fn reference_args(input: &str) -> Vec<AttrArgValue> {
    let parsed = parse_string(input, "attribute_argument_fidelity.m3l.md");
    let model = parsed
        .models
        .iter()
        .find(|m| m.name == "Thing")
        .expect("the document declares a model named Thing");
    let field = model
        .fields
        .iter()
        .find(|f| f.name == "other_id")
        .expect("Thing declares a field named other_id");
    field
        .attributes
        .iter()
        .find(|a| a.name == "reference")
        .and_then(|a| a.args.clone())
        .expect("other_id carries a reference attribute with arguments")
}

/// Specification section 5.2 writes its own example without quotes. A parser
/// that rewrites the scheme separator hands every consumer a target string the
/// document never contained.
#[test]
fn an_unquoted_uri_reference_target_survives_parsing() {
    let args = reference_args(
        r#"
# Namespace: test

## Thing

- id: identifier @pk @generated
- other_id: identifier @reference(external://taxonomy.Category)
"#,
    );

    assert_eq!(args.len(), 1);
    assert_eq!(
        args[0],
        AttrArgValue::String("external://taxonomy.Category".into())
    );
}

/// Quoting it was the workaround while the unquoted spelling was being
/// rewritten. Both spellings name the same target, so both must arrive as it.
#[test]
fn a_quoted_uri_reference_target_arrives_the_same_way() {
    let args = reference_args(
        r#"
# Namespace: test

## Thing

- id: identifier @pk @generated
- other_id: identifier @reference("external://taxonomy.Category")
"#,
    );

    assert_eq!(args.len(), 1);
    assert_eq!(
        args[0],
        AttrArgValue::String("external://taxonomy.Category".into())
    );
}
