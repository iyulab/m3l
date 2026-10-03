//! `rowversion` — the row version the database engine maintains (specification §10.4.1).
//!
//! A model declares at most one, and the engine — not the application — writes its value, so the
//! field takes no modifier or attribute that would give anyone else a say in it: `M3L-E023` for a
//! second row version on the same model, `M3L-E024` for a nullable, array, defaulted, key, unique
//! or reference row version.

use m3l_core::{parse_string, validate, Diagnostic, ValidateOptions};

fn errors(input: &str) -> Vec<Diagnostic> {
    let parsed = parse_string(input, "row_version.m3l.md");
    let ast = m3l_core::resolve(&[parsed], None);
    validate(&ast, &ValidateOptions::default()).errors
}

fn codes(input: &str) -> Vec<String> {
    errors(input).into_iter().map(|e| e.code).collect()
}

#[test]
fn a_row_version_is_a_catalog_type() {
    let found =
        codes("## Order\n- id: identifier @pk\n- row_version: rowversion\n- total: decimal(10,2)");
    assert!(found.is_empty(), "rowversion is a catalog type: {found:?}");
}

#[test]
fn a_row_version_reaches_a_model_through_an_interface() {
    let found = codes(
        "## Versioned ::interface\n- row_version: rowversion\n\n## Order : Versioned\n- id: identifier @pk",
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_second_row_version_on_one_model_is_an_error() {
    let found = errors(
        "## Order\n- id: identifier @pk\n- row_version: rowversion\n- other_version: rowversion",
    );
    let e023: Vec<_> = found.iter().filter(|e| e.code == "M3L-E023").collect();
    assert_eq!(e023.len(), 1, "{found:?}");
    assert!(
        e023[0].message.contains("row_version"),
        "{}",
        e023[0].message
    );
    assert!(
        e023[0].message.contains("other_version"),
        "{}",
        e023[0].message
    );
}

#[test]
fn an_inherited_row_version_counts_against_the_model() {
    let found = codes(
        "## Versioned ::interface\n- row_version: rowversion\n\n## Order : Versioned\n- id: identifier @pk\n- revision: rowversion",
    );
    assert!(found.contains(&"M3L-E023".to_string()), "{found:?}");
}

#[test]
fn row_versions_on_different_models_are_independent() {
    let found = codes(
        "## Order\n- id: identifier @pk\n- row_version: rowversion\n\n## Customer\n- id: identifier @pk\n- row_version: rowversion",
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn modifiers_and_attributes_the_engine_would_contradict_are_errors() {
    let cases = [
        ("- row_version: rowversion?", "nullable"),
        ("- row_version: rowversion[]", "array"),
        ("- row_version: rowversion = 0", "default"),
        ("- row_version: rowversion @pk", "@pk"),
        ("- row_version: rowversion @primary", "@primary"),
        ("- row_version: rowversion @unique", "@unique"),
        ("- row_version: rowversion @reference(Order)", "@reference"),
    ];
    for (line, what) in cases {
        let found = errors(&format!("## Order\n- id: identifier @pk\n{line}"));
        let e024: Vec<_> = found.iter().filter(|e| e.code == "M3L-E024").collect();
        assert_eq!(e024.len(), 1, "{line}: {found:?}");
        assert!(
            e024[0].message.contains(what),
            "{line}: {}",
            e024[0].message
        );
    }
}

#[test]
fn the_diagnostic_points_at_the_field() {
    let found = errors("## Order\n- id: identifier @pk\n- row_version: rowversion?");
    let e = found.iter().find(|e| e.code == "M3L-E024").expect("E024");
    assert_eq!(e.line, 3);
}
