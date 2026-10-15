//! `@renamed_from(...)` states a model's or a field's former names (specification §2.2.6, §2.3.5).
//!
//! The names arrive as their own key, oldest first, and the attribute is not repeated in the attribute
//! list — one fact, one place. A consumer that turns two versions of a model into storage changes reads
//! the key; it never has to find the attribute among the others.

use m3l_core::{parse_string, resolve, validate, ValidateOptions};

/// An inherited field is copied into every model that inherits it; a problem with its declaration is
/// reported once, where it is written — not once per model.
#[test]
fn an_inherited_declaration_is_reported_once() {
    let src = "## Stamped ::interface\n- made_at: timestamp @renamed_from(made_at)\n\n\
               ## A : Stamped\n- id: identifier @pk\n\n## B : Stamped\n- id: identifier @pk\n";
    let ast = resolve(&[parse_string(src, "inherit.m3l.md")], None);
    let result = validate(&ast, &ValidateOptions::default());
    let own_name: Vec<_> = result
        .errors
        .iter()
        .filter(|e| e.code == "M3L-E030")
        .collect();
    assert_eq!(own_name.len(), 1, "{:#?}", result.errors);
    assert_eq!(own_name[0].line, 2);
    assert!(
        own_name[0].message.contains("Stamped.made_at"),
        "{}",
        own_name[0].message
    );
}

const SOURCE: &str = "# Namespace: shop\n\n\
## Customer @renamed_from(Client) @description(\"buyer\")\n\
- id: identifier @pk\n\
- display_name: string(200) @renamed_from(name)\n\
- tier_code: string(20) @renamed_from(level, tier) @unique\n\
- email: string(200)\n";

#[test]
fn a_model_header_carries_its_former_names_in_their_own_key() {
    let ast = parse_string(SOURCE, "renamed_from.m3l.md");
    let customer = ast.models.iter().find(|m| m.name == "Customer").unwrap();
    assert_eq!(customer.renamed_from, Some(vec!["Client".to_string()]));
    assert!(customer.attributes.iter().all(|a| a.name != "renamed_from"));
    assert!(customer.attributes.iter().any(|a| a.name == "description"));
}

#[test]
fn a_field_carries_its_former_names_oldest_first() {
    let ast = parse_string(SOURCE, "renamed_from.m3l.md");
    let customer = ast.models.iter().find(|m| m.name == "Customer").unwrap();
    let field = |name: &str| customer.fields.iter().find(|f| f.name == name).unwrap();

    assert_eq!(
        field("display_name").renamed_from,
        Some(vec!["name".to_string()])
    );
    assert_eq!(
        field("tier_code").renamed_from,
        Some(vec!["level".to_string(), "tier".to_string()])
    );
    assert!(field("tier_code")
        .attributes
        .iter()
        .any(|a| a.name == "unique"));
    assert!(field("tier_code")
        .attributes
        .iter()
        .all(|a| a.name != "renamed_from"));
    assert_eq!(field("email").renamed_from, None);
}

#[test]
fn the_key_is_absent_when_nothing_is_declared_and_empty_when_no_name_is_given() {
    let ast = parse_string(
        "## Plain\n- id: identifier @pk\n- note: string(10) @renamed_from()\n",
        "renamed_from.m3l.md",
    );
    let plain = &ast.models[0];
    assert_eq!(plain.renamed_from, None);
    let json = serde_json::to_value(plain).unwrap();
    assert!(json.get("renamed_from").is_none());
    assert_eq!(
        plain
            .fields
            .iter()
            .find(|f| f.name == "note")
            .unwrap()
            .renamed_from,
        Some(vec![])
    );
}

#[test]
fn the_parsed_form_serializes_the_key_as_a_list() {
    let ast = parse_string(SOURCE, "renamed_from.m3l.md");
    let json = serde_json::to_value(&ast.models[0]).unwrap();
    assert_eq!(json["renamed_from"], serde_json::json!(["Client"]));
    let tier = json["fields"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["name"] == "tier_code")
        .unwrap();
    assert_eq!(tier["renamed_from"], serde_json::json!(["level", "tier"]));
}
