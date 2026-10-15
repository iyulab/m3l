//! §4.5.4: a lookup chain must not loop back to itself. A lookup whose path ends on another lookup
//! field depends on that field's definition; when following those dependencies arrives back at the
//! field it started from, the field is defined in terms of itself and no view can compute it. The
//! specification required an error here and nothing reported one.

use m3l_core::{parse_string, resolve, validate, ValidateOptions};

fn e027(input: &str) -> Vec<String> {
    let parsed = parse_string(input, "lookup_cycle.m3l.md");
    let ast = resolve(&[parsed], None);
    validate(&ast, &ValidateOptions { strict: false })
        .errors
        .into_iter()
        .filter(|d| d.code == "M3L-E027")
        .map(|d| d.message)
        .collect()
}

#[test]
fn a_lookup_that_ends_on_itself_through_a_self_reference_is_a_cycle() {
    let found = e027(
        r#"
# Namespace: test

## Category
- id: identifier @pk
- parent_id: identifier? @reference(Category)
- name: string(100)
- root_name: string? @lookup(parent_id.root_name)
"#,
    );

    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("Category.root_name"), "{}", found[0]);
}

#[test]
fn two_lookups_that_end_on_each_other_are_one_cycle() {
    let found = e027(
        r#"
# Namespace: test

## Order
- id: identifier @pk
- customer_id: identifier @reference(Customer)
- customer_label: string @lookup(customer_id.last_order_label)

## Customer
- id: identifier @pk
- last_order_id: identifier? @reference(Order)
- last_order_label: string? @lookup(last_order_id.customer_label)
"#,
    );

    assert_eq!(found.len(), 1, "one cycle, reported once: {found:?}");
    assert!(found[0].contains("Order.customer_label"), "{}", found[0]);
    assert!(
        found[0].contains("Customer.last_order_label"),
        "{}",
        found[0]
    );
}

/// Visiting the same model twice is not a cycle — a grandparent's name is a different row's stored field.
#[test]
fn a_path_that_revisits_a_model_but_ends_on_a_stored_field_is_not_a_cycle() {
    let found = e027(
        r#"
# Namespace: test

## Category
- id: identifier @pk
- parent_id: identifier? @reference(Category)
- name: string(100)
- grandparent_name: string? @lookup(parent_id.parent_id.name)
"#,
    );

    assert!(found.is_empty(), "{found:?}");
}

/// A lookup may end on another lookup field — the chain is followed, and only a loop is refused.
#[test]
fn a_lookup_ending_on_another_lookup_without_a_loop_is_allowed() {
    let found = e027(
        r#"
# Namespace: test

## Region
- id: identifier @pk
- name: string(100)

## Customer
- id: identifier @pk
- region_id: identifier @reference(Region)
- region_name: string @lookup(region_id.name)

## Order
- id: identifier @pk
- customer_id: identifier @reference(Customer)
- region_name: string @lookup(customer_id.region_name)
"#,
    );

    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_cycle_through_an_inherited_lookup_field_is_found() {
    let found = e027(
        r#"
# Namespace: test

## Linked ::interface
- next_id: identifier? @reference(Node)
- chain_label: string? @lookup(next_id.chain_label)

## Node : Linked
- id: identifier @pk
"#,
    );

    assert_eq!(found.len(), 1, "{found:?}");
}
