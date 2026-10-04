//! A view's references (specification §4.7): every relation its `### Source` reads is defined
//! (`M3L-E004` — `from` and `join` alike), and every `@from(Relation.field)` names one of those
//! relations and a field it has (`M3L-E025`).

use m3l_core::{parse_string, validate, Diagnostic, ValidateOptions};

fn errors(input: &str) -> Vec<Diagnostic> {
    let parsed = parse_string(input, "view_references.m3l.md");
    let ast = m3l_core::resolve(&[parsed], None);
    validate(&ast, &ValidateOptions::default()).errors
}

const MODELS: &str = "## Customer\n- id: identifier @pk\n- name: string(100)\n\n\
## Order\n- id: identifier @pk\n- customer_id: identifier @reference(Customer)\n\n";

fn view_errors(view: &str) -> Vec<Diagnostic> {
    errors(&format!("{MODELS}{view}"))
}

#[test]
fn a_view_whose_references_resolve_has_no_errors() {
    let found = view_errors(
        "## CustomerOrders ::view\n### Source\n- from: Customer\n\
         - join: Order on Order.customer_id = Customer.id\n\n\
         - customer_name: string @from(Customer.name)\n- order_id: identifier @from(Order.id)",
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_view_over_another_view_resolves_through_its_fields() {
    let found = view_errors(
        "## Named ::view\n### Source\n- from: Customer\n\n- label: string @from(Customer.name)\n\n\
         ## Labels ::view\n### Source\n- from: Named\n\n- label: string @from(Named.label)",
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_join_to_an_undefined_model_is_e004() {
    let found = view_errors(
        "## V ::view\n### Source\n- from: Customer\n- join: Nowhere on Nowhere.id = Customer.id\n\n\
         - name: string @from(Customer.name)",
    );
    let e = found.iter().find(|e| e.code == "M3L-E004").expect("E004");
    assert!(e.message.contains("\"Nowhere\""), "{}", e.message);
}

#[test]
fn from_naming_a_relation_the_view_does_not_read_is_e025() {
    let found = view_errors(
        "## V ::view\n### Source\n- from: Customer\n\n- id: identifier @from(Order.id)",
    );
    let e = found.iter().find(|e| e.code == "M3L-E025").expect("E025");
    assert!(e.message.contains("\"Order\""), "{}", e.message);
}

#[test]
fn from_naming_a_field_the_relation_lacks_is_e025() {
    let found = view_errors(
        "## V ::view\n### Source\n- from: Customer\n\n- nick: string @from(Customer.nickname)",
    );
    let e = found.iter().find(|e| e.code == "M3L-E025").expect("E025");
    assert!(e.message.contains("\"nickname\""), "{}", e.message);
}

#[test]
fn a_sql_block_source_is_not_checked_against_from() {
    let found = view_errors(
        "## V ::view\n### Source\n```sql\nSELECT c.name FROM Customer c\n```\n\n- name: string @from(Customer.name)",
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn from_on_an_undefined_relation_reports_the_relation_once() {
    // The undefined `from` is E004; the @from that names it is not reported a second time.
    let found =
        view_errors("## V ::view\n### Source\n- from: Ghost\n\n- name: string @from(Ghost.name)");
    assert_eq!(
        found.iter().filter(|e| e.code == "M3L-E004").count(),
        1,
        "{found:?}"
    );
    assert!(!found.iter().any(|e| e.code == "M3L-E025"), "{found:?}");
}
