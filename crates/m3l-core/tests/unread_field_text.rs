//! `M3L-W010`: text on a field line that the field syntax does not read.
//!
//! Specification 2.5.8. A field line reads `name: type = default @attributes "description"`
//! in that order. `- x: decimal(6,2) @not_null = 0 "X"` used to parse with the default and the
//! description both gone and validate clean — the parser stopped at the first token it did not
//! expect and dropped the rest of the line.

use m3l_core::{parse_string, validate, ValidateOptions};

fn w010(field: &str) -> Vec<(usize, String)> {
    let input = format!("# Namespace: probe\n\n## Item\n- id: identifier @pk\n{field}\n");
    let parsed = parse_string(&input, "unread.m3l.md");
    let ast = m3l_core::resolve(&[parsed], None);
    validate(&ast, &ValidateOptions::default())
        .warnings
        .iter()
        .filter(|w| w.code == "M3L-W010")
        .map(|w| (w.line, w.message.clone()))
        .collect()
}

#[test]
fn a_default_after_an_attribute_is_reported_with_its_line_and_text() {
    let found = w010(r#"- x: decimal(6,2) @not_null = 0 "X""#);

    let (line, message) = found.first().expect("M3L-W010 expected");
    assert_eq!(found.len(), 1);
    assert_eq!(*line, 5);
    assert!(message.contains(r#"Field "x" in Item"#), "{message}");
    assert!(message.contains(r#""= 0 "X"""#), "{message}");
}

#[test]
fn an_unclosed_description_is_reported() {
    assert_eq!(w010(r#"- x: string(10) "no end"#).len(), 1);
}

/// Every order the syntax does read stays silent — the warning names what is dropped, not
/// what is merely unusual.
#[test]
fn the_forms_the_syntax_reads_are_not_reported() {
    for field in [
        r#"- a: decimal(6,2) = 0 @not_null "A""#,
        r#"- b: string(20) = "x" @unique"#,
        r#"- c: timestamp = now()"#,
        r#"- d: integer = `1 + 1` "D""#,
        r#"- e: string(30)? "E" @index"#,
        r#"- f: identifier @reference(Item)! "F""#,
        r#"- g: text"#,
        r#"- h: "only a description""#,
    ] {
        assert!(w010(field).is_empty(), "{field} → {:?}", w010(field));
    }
}

/// The field itself still parses — the warning adds a diagnostic, it does not change the AST.
#[test]
fn the_rest_of_the_field_is_unchanged() {
    let parsed = parse_string(
        "# Namespace: probe\n\n## Item\n- x: decimal(6,2) @not_null = 0 \"X\"\n",
        "unread.m3l.md",
    );
    let field = &parsed.models[0].fields[0];
    assert_eq!(field.name, "x");
    assert_eq!(field.field_type.as_deref(), Some("decimal"));
    assert!(field.default_value.is_none());
}
