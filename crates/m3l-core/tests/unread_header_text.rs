//! `M3L-W010` on an element header: a second description string, or a quote that never closes.
//!
//! Specification 4.2.1. A header takes one description string. Before the header split took
//! quoted text off the definition, `## Product "A" "B"` named the model `Product  "B"` — the
//! leftover quote became part of the name — and `## Product "unclosed` named it
//! `Product "unclosed`.

use m3l_core::{parse_string, validate, ValidateOptions};

fn run(header: &str) -> (m3l_core::ParsedFile, Vec<(usize, String)>) {
    let input = format!("# Namespace: probe\n\n{header}\n- id: identifier @pk\n");
    let parsed = parse_string(&input, "unread.m3l.md");
    let ast = m3l_core::resolve(std::slice::from_ref(&parsed), None);
    let w010 = validate(&ast, &ValidateOptions::default())
        .warnings
        .iter()
        .filter(|w| w.code == "M3L-W010")
        .map(|w| (w.line, w.message.clone()))
        .collect();
    (parsed, w010)
}

#[test]
fn a_second_description_is_reported_and_kept_out_of_the_name() {
    let (parsed, found) = run(r#"## Product "A sellable item" "on sale" @audited"#);

    assert_eq!(parsed.models[0].name, "Product");
    assert_eq!(
        parsed.models[0].description.as_deref(),
        Some("A sellable item")
    );
    assert_eq!(parsed.models[0].attributes[0].name, "audited");
    let (line, message) = found.first().expect("M3L-W010 expected");
    assert_eq!(found.len(), 1);
    assert_eq!(*line, 3);
    assert!(message.contains(r#"Header of "Product""#), "{message}");
    assert!(message.contains(r#""on sale""#), "{message}");
}

#[test]
fn an_unclosed_quote_is_reported_and_kept_out_of_the_name() {
    let (parsed, found) = run(r#"## Product : Base "never closes"#);

    assert_eq!(parsed.models[0].name, "Product");
    assert_eq!(parsed.models[0].inherits, ["Base"]);
    assert!(parsed.models[0].description.is_none());
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].1.contains(r#""never closes"#), "{}", found[0].1);
}

#[test]
fn every_kind_of_header_is_checked() {
    for header in [
        r#"## Stamped ::interface "a" "b""#,
        r#"## Status ::enum "a" "b""#,
        r#"## OnSale ::view "a" "b""#,
    ] {
        assert_eq!(run(header).1.len(), 1, "{header}");
    }
}

/// What a header does read stays silent: one description, quoted arguments inside parentheses, a
/// comment (whatever it says), a label.
#[test]
fn the_forms_a_header_reads_are_not_reported() {
    for header in [
        r#"## Product "A sellable item" @audited"#,
        r#"## Product(상품) : Base @table("products") "A sellable item""#,
        r#"## Product # a "quoted" note"#,
        r#"## Product"#,
    ] {
        assert!(run(header).1.is_empty(), "{header}: {:?}", run(header).1);
    }
}

/// Specification 10.3.2: only a model and a `::view` header take attributes. On any other kind's
/// header they were dropped with no warning — `## Status ::enum @deprecated` parsed as if the
/// attribute had never been written.
#[test]
fn attributes_on_a_kind_whose_header_takes_none_are_reported() {
    for (header, kind) in [
        ("## Status ::enum @deprecated", "enum"),
        ("## Named ::interface @abstract", "interface"),
        ("## Settle ::flow @async", "flow"),
        ("## Profile ::aspect(Asset) @audited", "aspect"),
    ] {
        let (_, found) = run(header);
        assert_eq!(found.len(), 1, "{header}: {found:?}");
        let message = &found[0].1;
        assert!(
            message.contains(&format!("A `::{kind}` header takes no attributes")),
            "{message}"
        );
        assert!(message.contains('@'), "{message}");
    }
}

/// The attribute text is named as written, arguments included, and an inheritance list before it
/// is still read.
#[test]
fn unread_header_attributes_are_named_as_written() {
    let (parsed, found) = run(r#"## Status ::enum : Base @deprecated("use Phase") @internal"#);

    assert_eq!(parsed.enums[0].inherits, ["Base"]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].1.contains(r#"@deprecated("use Phase") @internal"#),
        "{}",
        found[0].1
    );
}

/// The kinds that do take header attributes stay silent.
#[test]
fn model_and_view_header_attributes_are_not_reported() {
    for header in [
        "## Product @audited",
        "## OnSale ::view @materialized @internal",
    ] {
        assert!(run(header).1.is_empty(), "{header}: {:?}", run(header).1);
    }
}
