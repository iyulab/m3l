//! The extended field format is another spelling of the basic one (specification §2.3.2): a field written as
//! nested `- key: value` items parses to the same field the one-line form does. A consumer reads one AST and
//! never needs to know which spelling the author chose.

use m3l_core::{parse_string, resolve, validate, FieldNode, ValidateOptions};

fn field(src: &str, name: &str) -> FieldNode {
    let ast = parse_string(src, "extended.m3l.md");
    ast.models
        .iter()
        .flat_map(|m| m.fields.iter())
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("no field {name}"))
        .clone()
}

/// Both spellings of one field, as a model of one key and that field.
fn pair(inline: &str, extended: &str) -> (FieldNode, FieldNode) {
    let one = format!("## M\n- id: identifier @pk\n- f: {inline}\n");
    let two = format!("## M\n- id: identifier @pk\n- f\n{extended}");
    (field(&one, "f"), field(&two, "f"))
}

fn assert_same_type(a: &FieldNode, b: &FieldNode) {
    assert_eq!(a.field_type, b.field_type, "type");
    assert_eq!(a.params, b.params, "params");
    assert_eq!(a.generic_params, b.generic_params, "generic params");
    assert_eq!(a.nullable, b.nullable, "nullable");
    assert_eq!(a.array, b.array, "array");
    assert_eq!(
        a.array_item_nullable, b.array_item_nullable,
        "array item nullable"
    );
}

#[test]
fn a_type_with_parameters_reads_the_same_in_both_spellings() {
    for t in [
        "string(200)",
        "decimal(10,2)?",
        "map<string,integer>",
        "string(20)[]?",
        "integer?[]",
        "identifier",
    ] {
        let (inline, extended) = pair(t, &format!("  - type: {t}\n"));
        assert_same_type(&inline, &extended);
    }
}

#[test]
fn a_parameterised_type_in_the_extended_format_is_not_an_undefined_type() {
    let src = "## M\n- id: identifier @pk\n- email\n  - type: string(100)\n  - unique: true\n";
    let ast = resolve(&[parse_string(src, "extended.m3l.md")], None);
    let result = validate(&ast, &ValidateOptions::default());
    assert!(
        result.errors.iter().all(|e| e.code != "M3L-E009"),
        "{:#?}",
        result.errors
    );
}

#[test]
fn former_names_in_the_extended_format_arrive_in_their_own_key() {
    let (inline, extended) = pair(
        "string(200) @renamed_from(level, tier)",
        "  - type: string(200)\n  - renamed_from: level, tier\n",
    );
    assert_eq!(
        extended.renamed_from,
        Some(vec!["level".to_string(), "tier".to_string()])
    );
    assert_eq!(inline.renamed_from, extended.renamed_from);
    assert!(extended.attributes.iter().all(|a| a.name != "renamed_from"));
}

#[test]
fn former_names_from_both_spellings_on_one_field_keep_their_order() {
    let f = field(
        "## M\n- id: identifier @pk\n- f: string @renamed_from(a)\n  - renamed_from: b\n",
        "f",
    );
    assert_eq!(f.renamed_from, Some(vec!["a".to_string(), "b".to_string()]));
}

#[test]
fn an_extended_rename_without_a_name_is_reported_like_the_inline_one() {
    let src = "## M\n- id: identifier @pk\n- f\n  - type: string\n  - renamed_from: \"\"\n";
    let ast = resolve(&[parse_string(src, "extended.m3l.md")], None);
    let result = validate(&ast, &ValidateOptions::default());
    assert!(
        result.errors.iter().any(|e| e.code == "M3L-E032"),
        "{:#?}",
        result.errors
    );
}

#[test]
fn an_extended_value_with_several_arguments_reads_as_the_inline_arguments() {
    let (inline, extended) = pair(
        "string @check(\"len > 0\", name: \"nonempty\")",
        "  - type: string\n  - check: \"len > 0\", name: \"nonempty\"\n",
    );
    let check = |f: &FieldNode| f.attributes.iter().find(|a| a.name == "check").cloned();
    assert_eq!(
        check(&inline).map(|a| a.args),
        check(&extended).map(|a| a.args)
    );
}
