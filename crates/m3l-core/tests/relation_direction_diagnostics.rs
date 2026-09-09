//! `M3L-E010` belongs to outgoing relations only, and the direction it keys on
//! is the parsed one.
//!
//! Only a "to" relationship puts the foreign key on the model that declares it,
//! so only there can the key be checked against that model's own fields. The
//! check used to decide this by looking for a `>` in the source line, which is
//! true of three things that are not an outgoing relation: the many-to-many
//! spelling `<>`, any description that happens to contain the character, and --
//! were one ever written -- a target name carrying it. Each reported a missing
//! `@reference` against a model that never owned the key.
//!
//! Specification 3.2.4 has the parser hand out `direction`, so the check reads
//! that instead. What the notation could not classify carries no `direction`
//! and is left alone: an entry whose direction is unknown cannot be said to own
//! a key either.

use m3l_core::{parse_string, validate, ValidateOptions};

fn fk_errors(input: &str) -> Vec<String> {
    let parsed = parse_string(input, "relation_direction.m3l.md");
    let ast = m3l_core::resolve(&[parsed], None);
    validate(&ast, &ValidateOptions::default())
        .errors
        .iter()
        .filter(|e| e.code == "M3L-E010")
        .map(|e| e.message.clone())
        .collect()
}

/// The one case the check is for: this model declares the key, and it is not
/// marked as a reference. Kept first, because every other test here asserts an
/// absence -- and an absence proves nothing if the check cannot fire at all.
#[test]
fn an_outgoing_relation_still_reports_a_key_without_reference() {
    let errors = fk_errors(
        r#"
# Namespace: probe

## Post
- id: identifier @primary
- author_id: identifier

### Relations
- >author
  - target: Person
  - from: author_id
"#,
    );
    assert_eq!(errors.len(), 1, "the check fires: {errors:?}");
    assert!(errors[0].contains("author_id"), "{}", errors[0]);
}

/// ...and stays silent once the key says what it references.
#[test]
fn an_outgoing_relation_with_a_reference_is_silent() {
    assert!(fk_errors(
        r#"
# Namespace: probe

## Person
- id: identifier @primary

## Post
- id: identifier @primary
- author_id: identifier @reference(Person)

### Relations
- >author
  - target: Person
  - from: author_id
"#,
    )
    .is_empty());
}

/// Many-to-many holds its keys in a join table, never in the declaring model.
/// `<>` contains a `>`, so the old check read it as outgoing and demanded a
/// `@reference` the model has no reason to carry.
#[test]
fn a_many_to_many_relation_is_not_checked_for_a_local_key() {
    assert!(
        fk_errors(
            r#"
# Namespace: probe

## Post
- id: identifier @primary
- tag_bag: identifier

### Relations
- <>tags: many-to-many
  - from: tag_bag
"#,
        )
        .is_empty(),
        "the key of a many-to-many is not this model's to declare"
    );
}

/// An incoming relation puts the key on the *other* model. A `>` inside a
/// description is not a direction, and used to be read as one.
#[test]
fn a_description_containing_an_angle_bracket_does_not_make_a_relation_outgoing() {
    assert!(
        fk_errors(
            r#"
# Namespace: probe

## Post
- id: identifier @primary
- tag_bag: identifier

### Relations
- <comments "count > 0"
  - from: tag_bag
"#,
        )
        .is_empty(),
        "direction comes from the leading characters, not from the prose"
    );
}

/// The fallback, stated as a contract rather than left to whatever the old
/// substring test happened to return: an entry written without any of the
/// leading spellings has no `direction`, and is not diagnosed.
#[test]
fn an_entry_without_the_notation_is_left_undiagnosed() {
    let input = r#"
# Namespace: probe

## Post
- id: identifier @primary
- author_id: identifier

### Relations
- author
  - from: author_id
"#;
    let parsed = parse_string(input, "relation_direction.m3l.md");
    let post = parsed
        .models
        .iter()
        .find(|m| m.name == "Post")
        .expect("the document declares a model named Post");
    let rel = post
        .sections
        .relations
        .first()
        .expect("the entry reaches the relations section");
    assert!(
        rel.get("direction").is_none(),
        "the premise of this test: no direction was parsed -- {rel:?}"
    );

    assert!(
        fk_errors(input).is_empty(),
        "an unclassifiable entry is not guessed at"
    );
}
