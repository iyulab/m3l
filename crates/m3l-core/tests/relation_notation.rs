//! Relationship notation reaches the AST as a relationship, not as a field with
//! the source line for a name.
//!
//! Specification sections 3.2.2 and 3.2.4 define `>name`, `<name`, `<>name` and
//! the arrow spellings, with an optional cardinality after a colon. Until this
//! was structured, the parser kept the whole line -- direction, colon and
//! cardinality included -- and a consumer that wanted any part of it had to
//! parse that string again. A language that defines a syntax owes its consumers
//! the parsed form of it.

use m3l_core::{parse_string, validate, ValidateOptions};
use serde_json::Value;

fn relations(input: &str) -> Vec<Value> {
    let parsed = parse_string(input, "relation_notation.m3l.md");
    let model = parsed
        .models
        .iter()
        .find(|m| m.name == "Post")
        .expect("the document declares a model named Post");
    model.sections.relations.clone()
}

fn field_names(input: &str) -> Vec<String> {
    let parsed = parse_string(input, "relation_notation.m3l.md");
    parsed
        .models
        .iter()
        .find(|m| m.name == "Post")
        .expect("the document declares a model named Post")
        .fields
        .iter()
        .map(|f| f.name.clone())
        .collect()
}

fn get<'a>(rel: &'a Value, key: &str) -> Option<&'a str> {
    rel.get(key).and_then(|v| v.as_str())
}

const IN_SECTION: &str = r#"
# Namespace: probe

## Post
- id: identifier @primary

### Relations
- >author
  - target: Person
  - from: author_id
- <posts: one-to-many
- <>tags: many-to-many
- -> Person
- <- Comment.post_id
"#;

/// The short spellings carry a direction and a name, and the cardinality after
/// the colon is a separate fact from either of them.
#[test]
fn short_notation_yields_direction_name_and_cardinality() {
    let rels = relations(IN_SECTION);

    let author = &rels[0];
    assert_eq!(get(author, "direction"), Some("to"));
    assert_eq!(get(author, "name"), Some("author"));
    assert_eq!(author.get("cardinality"), None, "no colon, no cardinality");
    // A nested item still wins for the keys it names.
    assert_eq!(get(author, "target"), Some("Person"));
    assert_eq!(get(author, "from"), Some("author_id"));

    let posts = &rels[1];
    assert_eq!(get(posts, "direction"), Some("from"));
    assert_eq!(get(posts, "name"), Some("posts"));
    assert_eq!(get(posts, "cardinality"), Some("one-to-many"));

    let tags = &rels[2];
    assert_eq!(get(tags, "direction"), Some("many-to-many"));
    assert_eq!(get(tags, "name"), Some("tags"));
    assert_eq!(get(tags, "cardinality"), Some("many-to-many"));
}

/// `<>` must be read before `<`, and `->` before `>`, or the longer spelling
/// loses its first character to the shorter one.
#[test]
fn arrow_spellings_name_a_target_rather_than_a_relationship_name() {
    let rels = relations(IN_SECTION);

    let to = &rels[3];
    assert_eq!(get(to, "direction"), Some("to"));
    assert_eq!(get(to, "target"), Some("Person"));
    assert_eq!(to.get("name"), None, "an arrow spelling names a target");

    let from = &rels[4];
    assert_eq!(get(from, "direction"), Some("from"));
    assert_eq!(get(from, "target"), Some("Comment.post_id"));
}

/// The raw line is kept as well, so nothing that read it before has to change
/// at once.
#[test]
fn the_source_line_is_still_available() {
    let rels = relations(IN_SECTION);
    assert_eq!(get(&rels[1], "raw"), Some("<posts: one-to-many"));
}

const IN_FIELD_LIST: &str = r#"
# Namespace: probe

## Post
- id: identifier @primary
- title: string(120)
- <>tags: many-to-many
- >category: one-to-one
"#;

/// This is the shape that sent the notation to consumers as a field: the line
/// became a name, the type stayed empty, and the entry was classified as
/// stored. It is a relationship wherever it is written.
#[test]
fn notation_among_the_fields_is_not_a_field() {
    assert_eq!(
        field_names(IN_FIELD_LIST),
        vec!["id".to_string(), "title".to_string()],
        "only the two declared fields are fields"
    );

    let rels = relations(IN_FIELD_LIST);
    assert_eq!(rels.len(), 2);
    assert_eq!(get(&rels[0], "name"), Some("tags"));
    assert_eq!(get(&rels[0], "direction"), Some("many-to-many"));
    assert_eq!(get(&rels[1], "name"), Some("category"));
    assert_eq!(get(&rels[1], "cardinality"), Some("one-to-one"));
    // Where it was written is kept, so the placement can be reported.
    assert_eq!(get(&rels[0], "declaredIn"), Some("fields"));
}

/// Reading it correctly is not the same as accepting it silently: the section
/// is where the specification puts it, and the document is told so.
#[test]
fn notation_among_the_fields_is_reported() {
    let parsed = parse_string(IN_FIELD_LIST, "relation_notation.m3l.md");
    let ast = m3l_core::resolve(&[parsed], None);
    let result = validate(&ast, &ValidateOptions::default());

    let placement: Vec<_> = result
        .warnings
        .iter()
        .filter(|w| w.code == "M3L-W009")
        .collect();
    assert_eq!(placement.len(), 2, "one per misplaced line");
    assert!(
        result.errors.is_empty(),
        "the meaning is clear; it is not an error"
    );
    assert!(
        placement[0].message.contains("### Relations"),
        "the report names where it belongs: {}",
        placement[0].message
    );
}

/// What follows the colon is carried through as written. Section 3.2.4 does not
/// close the set of cardinalities, and this repository already takes that stance
/// for attribute arguments -- the lexer decides where a value ends, not what it
/// means. So a line that begins with one of these characters is read as a
/// relationship even when what follows the colon is nonsense; before this
/// change the same line was a field with the whole line for a name and no type,
/// which is not a better answer.
#[test]
fn what_follows_the_colon_is_carried_through_verbatim() {
    let input = r#"
# Namespace: probe

## Post
- id: identifier @primary
- <weird: string(10)
"#;
    assert_eq!(
        field_names(input),
        vec!["id".to_string()],
        "the line does not become a field"
    );

    let rels = relations(input);
    assert_eq!(rels.len(), 1);
    assert_eq!(get(&rels[0], "direction"), Some("from"));
    assert_eq!(get(&rels[0], "name"), Some("weird"));
    assert_eq!(
        get(&rels[0], "cardinality"),
        Some("string(10)"),
        "not policed -- carried as written"
    );
}

/// A line with no arrow is untouched. The check keys on the leading characters
/// alone; widening it would let ordinary prose be read as a relationship, and
/// swallowing a field is worse than the defect being fixed here.
#[test]
fn a_line_without_the_notation_is_still_a_field() {
    let input = r#"
# Namespace: probe

## Post
- id: identifier @primary
- title: string(120)
- summary: string(400)
"#;
    assert_eq!(
        field_names(input),
        vec!["id".to_string(), "title".to_string(), "summary".to_string()]
    );
    assert!(relations(input).is_empty());
}

/// `name: >Target via fk` — the notation after a key that names the entry. The
/// samples in this repository write relationships this way, and until now the
/// shape reached the AST as `raw` alone: the direction was there in the text
/// and nowhere in the parsed form, so the one consumer that needed it went
/// looking for a `>` in the line.
///
/// A key in front supplies the name, which makes the token after the arrow the
/// target no matter which spelling was used.
#[test]
fn a_key_in_front_supplies_the_name_and_the_notation_the_target() {
    let input = r#"
# Namespace: probe

## Post
- id: identifier @primary
- category_id: identifier

### Relations
- category: >Category via category_id
"#;
    assert_eq!(
        field_names(input),
        vec!["id".to_string(), "category_id".to_string()],
        "the entry belongs to the relations section, not the field list"
    );

    let rels = relations(input);
    assert_eq!(rels.len(), 1);
    assert_eq!(get(&rels[0], "direction"), Some("to"));
    assert_eq!(get(&rels[0], "name"), Some("category"));
    assert_eq!(
        get(&rels[0], "target"),
        Some("Category"),
        "the target ends at the first space -- `via category_id` is not part of it"
    );
    assert_eq!(
        get(&rels[0], "from"),
        Some("category_id"),
        "`via` names the FK field the same way 3.2.3's nested `- from:` does on its own line (3.2.4)"
    );
}

/// A nested `- from:` still wins over what `via` supplied -- the same override
/// rule the spec already states for `target` (3.2.3's `- target:` beats an
/// arrow spelling). `via` only ever fills a default.
#[test]
fn a_nested_from_overrides_the_via_clause() {
    let input = r#"
# Namespace: probe

## Post
- id: identifier @primary
- category_id: identifier
- owner_id: identifier

### Relations
- category: >Category via category_id
  - from: owner_id
"#;
    let rels = relations(input);
    assert_eq!(rels.len(), 1);
    assert_eq!(get(&rels[0], "from"), Some("owner_id"));
}

/// `via` with no key in front -- the short form without a leading key names
/// the *entry* (per the spec text above: a short spelling's token is its own
/// name, never a target), but `via` still reads the FK field regardless of
/// which of `name`/`target` the token became.
#[test]
fn via_without_a_leading_key_still_names_the_fk_field() {
    let input = r#"
# Namespace: probe

## Post
- id: identifier @primary
- category_id: identifier

### Relations
- >Category via category_id
"#;
    let rels = relations(input);
    assert_eq!(rels.len(), 1);
    assert_eq!(get(&rels[0], "name"), Some("Category"));
    assert_eq!(get(&rels[0], "target"), None);
    assert_eq!(get(&rels[0], "from"), Some("category_id"));
}

/// The key has to look like a name and the value has to start with the
/// notation. Widening either turns ordinary prose into a relationship, which
/// swallows a field silently -- worse than the defect being fixed.
#[test]
fn a_colon_line_that_is_not_the_notation_stays_a_field() {
    let input = r#"
# Namespace: probe

## Post
- id: identifier @primary
- summary: string(400)
- rank: int
"#;
    assert_eq!(
        field_names(input),
        vec!["id".to_string(), "summary".to_string(), "rank".to_string()]
    );
    assert!(relations(input).is_empty());
}

/// The key must look like a name. Without that, any line whose text after a
/// colon happens to begin with an angle bracket is read as a relationship --
/// and being read as a relationship removes the line from the field list, so
/// the failure is a field disappearing rather than a diagnostic appearing.
#[test]
fn a_colon_line_whose_key_is_not_a_name_is_not_a_relation() {
    let input = r#"
# Namespace: probe

## Post
- id: identifier @primary
- see also: > the appendix
"#;
    assert!(
        relations(input).is_empty(),
        "`see also` is prose, not the name of a relationship"
    );
    assert!(
        field_names(input).len() > 1,
        "the line stays where it was -- it is not swallowed: {:?}",
        field_names(input)
    );
}
