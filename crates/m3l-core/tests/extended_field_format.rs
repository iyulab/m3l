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

fn names(f: &FieldNode) -> Vec<&str> {
    f.attributes.iter().map(|a| a.name.as_str()).collect()
}

#[test]
fn a_true_flag_item_is_the_bare_attribute() {
    let (inline, extended) = pair(
        "string @unique @index @searchable",
        "  - type: string\n  - unique: true\n  - index: true\n  - searchable: true\n",
    );
    assert_eq!(names(&inline), names(&extended));
    for (a, b) in inline.attributes.iter().zip(&extended.attributes) {
        assert_eq!(a.args, b.args, "@{}", a.name);
    }
}

#[test]
fn a_false_flag_item_is_no_attribute() {
    let f = field(
        "## M\n- id: identifier @pk\n- f\n  - type: string\n  - unique: false\n  - index: false\n",
        "f",
    );
    assert!(f.attributes.is_empty(), "{:?}", f.attributes);
}

#[test]
fn an_attribute_that_takes_a_value_keeps_a_boolean_as_its_value() {
    let f = field(
        "## M\n- id: identifier @pk\n- f\n  - type: boolean\n  - default: false\n  - unique: \"false\"\n",
        "f",
    );
    let default = f.attributes.iter().find(|a| a.name == "default").unwrap();
    assert_eq!(
        default.args,
        Some(vec![m3l_core::AttrArgValue::Bool(false)])
    );
    // Quoted, it is a string argument, not the flag's truth value.
    assert!(f.attributes.iter().any(|a| a.name == "unique"));
}

#[test]
fn a_custom_attribute_keeps_a_boolean_as_its_argument() {
    let f = field(
        "## M\n- id: identifier @pk\n- f\n  - type: string\n  - audited: false\n",
        "f",
    );
    let audited = f.attributes.iter().find(|a| a.name == "audited").unwrap();
    assert_eq!(
        audited.args,
        Some(vec![m3l_core::AttrArgValue::Bool(false)])
    );
}

/// The derived part of a field — its kind and definition — as JSON, so two spellings compare whole.
fn derived(f: &FieldNode) -> serde_json::Value {
    serde_json::json!({
        "kind": f.kind,
        "lookup": f.lookup,
        "rollup": f.rollup,
        "computed": f.computed,
        "persisted": f.attributes.iter().any(|a| a.name == "persisted"),
    })
}

#[test]
fn an_extended_computed_item_makes_a_computed_field() {
    let (inline, extended) = pair(
        "decimal(10,2) @computed(\"price * qty\")",
        "  - type: decimal(10,2)\n  - computed: \"price * qty\"\n",
    );
    assert_eq!(derived(&inline), derived(&extended));
    assert_eq!(extended.kind, m3l_core::FieldKind::Computed);
}

/// The specification's own extended computed field (§4.4.2): `computed: true` with the expression as `formula:`.
#[test]
fn a_formula_item_is_the_computed_expression() {
    let (inline, extended) = pair(
        "decimal(12,2) @computed(\"quantity * unit_price\") @persisted",
        "  - computed: true\n  - formula: \"quantity * unit_price\"\n  - persisted: true\n  - type: decimal(12,2)\n",
    );
    assert_eq!(derived(&inline), derived(&extended));
    assert!(extended.attributes.iter().all(|a| a.name != "formula"));
    assert_eq!(
        extended
            .attributes
            .iter()
            .filter(|a| a.name == "computed")
            .count(),
        1
    );
}

#[test]
fn a_formula_before_computed_true_reads_the_same() {
    let (inline, extended) = pair(
        "integer @computed(\"a + b\")",
        "  - type: integer\n  - formula: \"a + b\"\n  - computed: true\n",
    );
    assert_eq!(derived(&inline), derived(&extended));
}

#[test]
fn an_extended_lookup_item_makes_a_lookup_field() {
    let (inline, extended) = pair(
        "string @lookup(order_id.customer_id.name)",
        "  - type: string\n  - lookup: order_id.customer_id.name\n",
    );
    assert_eq!(derived(&inline), derived(&extended));
    assert_eq!(extended.kind, m3l_core::FieldKind::Lookup);
}

#[test]
fn an_extended_rollup_item_makes_a_rollup_field() {
    let (inline, extended) = pair(
        "integer @rollup(Order.customer_id, count, where: \"status = 'paid'\")",
        "  - type: integer\n  - rollup: Order.customer_id, count, where: \"status = 'paid'\"\n",
    );
    assert_eq!(derived(&inline), derived(&extended));
    assert_eq!(extended.kind, m3l_core::FieldKind::Rollup);
}

/// A former name on a derived field without storage is the error it is inline (§2.2.6) — the kind the check reads
/// is the extended field's own.
#[test]
fn a_rename_on_an_extended_computed_field_is_misplaced() {
    let src = "## M\n- id: identifier @pk\n- price: decimal\n- t\n  - type: decimal\n  - computed: \"price * 2\"\n  - renamed_from: old_t\n";
    let ast = resolve(&[parse_string(src, "extended.m3l.md")], None);
    let result = validate(&ast, &ValidateOptions::default());
    assert!(
        result.errors.iter().any(|e| e.code == "M3L-E031"),
        "{:#?}",
        result.errors
    );
}
