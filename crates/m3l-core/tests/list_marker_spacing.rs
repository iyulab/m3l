//! A list marker is `-` followed by one or more spaces or tabs — Markdown allows `-   field`. The lexer used to
//! require exactly one space: more spaces made the field name start with blanks and swallow the whole line (no type,
//! no diagnostic), and a tab made the line vanish from the model without a word.

use m3l_core::{parse_string, resolve};

fn fields_of(src: &str) -> Vec<(String, String)> {
    let ast = resolve(&[parse_string(src, "spacing.m3l.md")], None);
    let model = ast
        .models
        .iter()
        .find(|m| m.name == "Customer")
        .expect("model Customer");
    model
        .fields
        .iter()
        .map(|f| (f.name.clone(), f.field_type.clone().unwrap_or_default()))
        .collect()
}

#[test]
fn extra_spaces_after_the_marker_do_not_change_the_field() {
    let fields = fields_of(
        "# Namespace: demo\n\n## Customer\n-   id: identifier @pk\n-  name: string(100)\n",
    );
    assert_eq!(
        fields,
        vec![
            ("id".to_string(), "identifier".to_string()),
            ("name".to_string(), "string".to_string())
        ]
    );
}

#[test]
fn a_tab_after_the_marker_is_a_field_too() {
    let fields = fields_of("# Namespace: demo\n\n## Customer\n-\tid: identifier @pk\n");
    assert_eq!(fields, vec![("id".to_string(), "identifier".to_string())]);
}

#[test]
fn one_space_is_unchanged() {
    let fields = fields_of("# Namespace: demo\n\n## Customer\n- id: identifier @pk\n");
    assert_eq!(fields, vec![("id".to_string(), "identifier".to_string())]);
}
