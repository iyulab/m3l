//! The keyword arguments of `@index` and `@unique` (specification §3.3) are not columns. The parser takes each out
//! of `args` — the column list every consumer reads — into a field of its own (`name`, `fulltext`, `nulls`), and
//! reports one the directive does not read as `M3L-W010` instead of leaving it as a column. Left in `args`, the
//! specification's own `@index(customer_id, order_date, name: "customer_orders")` reached a generator as a third
//! column named `name: "customer_orders"`.

use m3l_core::{parse_string, validate, ValidateOptions};
use serde_json::{json, Value};

const PRODUCT: &str = "# Namespace: probe\n\n## Product\n- id: identifier @pk\n- sku: string(20)\n- name: string(100)\n";

fn indexes(lines: &str) -> Vec<Value> {
    let parsed = parse_string(&format!("{PRODUCT}{lines}"), "index.m3l.md");
    parsed.models[0].sections.indexes.clone()
}

fn w010(lines: &str) -> Vec<String> {
    let parsed = parse_string(&format!("{PRODUCT}{lines}"), "index.m3l.md");
    let ast = m3l_core::resolve(&[parsed], None);
    validate(&ast, &ValidateOptions::default())
        .warnings
        .into_iter()
        .filter(|w| w.code == "M3L-W010")
        .map(|w| w.message)
        .collect()
}

#[test]
fn an_index_name_and_fulltext_are_not_columns() {
    let found = indexes("- @index(sku, name, name: \"product_search\", fulltext: true)\n");

    assert_eq!(found[0]["args"], json!(["sku", "name"]));
    assert_eq!(found[0]["name"], json!("product_search"));
    assert_eq!(found[0]["fulltext"], json!(true));
}

#[test]
fn a_unique_name_and_nulls_are_not_columns() {
    let found = indexes("- @unique(sku, name: \"uq_sku\", nulls: \"not_distinct\")\n");

    assert_eq!(found[0]["args"], json!(["sku"]));
    assert_eq!(found[0]["name"], json!("uq_sku"));
    assert_eq!(found[0]["nulls"], json!("not_distinct"));
    assert_eq!(found[0]["unique"], json!(true));
}

#[test]
fn a_keyword_the_directive_does_not_read_is_w010_not_a_column() {
    let lines = "- @index(sku, nme: \"x\")\n- @unique(sku, fulltext: true)\n- @index(sku, nulls: \"not_distinct\")\n";

    let found = indexes(lines);
    let warned = w010(lines);

    assert!(
        found.iter().all(|i| i["args"] == json!(["sku"])),
        "{found:?}"
    );
    assert_eq!(warned.len(), 3, "{warned:?}");
    assert!(
        warned[0].contains(r#"Field "@index" in Product: "nme: "x"" is not read"#),
        "{}",
        warned[0]
    );
    assert!(
        warned[0].contains("`@index` reads `name:` and `fulltext:`"),
        "{}",
        warned[0]
    );
}

#[test]
fn a_labelled_index_is_named_by_its_label_and_a_second_name_is_unread() {
    let lines = "\n### Indexes\n- by_sku: @index(sku, name: \"other\")\n";

    let found = indexes(lines);

    let warned = w010(lines);

    assert_eq!(found[0]["name"], json!("by_sku"));
    assert_eq!(found[0]["args"], json!(["sku"]));
    assert_eq!(warned.len(), 1);
    // The directive's own hint would say `@index` reads `name:` — true of a bare directive, and the opposite of what
    // happened here. The writer is told where the name came from instead.
    assert!(
        warned[0].contains("named by its item name"),
        "{}",
        warned[0]
    );
    assert!(!warned[0].contains("reads `name:`"), "{}", warned[0]);
}

#[test]
fn columns_alone_carry_no_keyword_fields_and_no_warning() {
    let lines = "- @index(sku, name)\n- @unique(sku)\n";

    let found = indexes(lines);

    assert_eq!(found[0]["args"], json!(["sku", "name"]));
    assert!(
        found
            .iter()
            .all(|i| i.get("name").is_none() && i.get("fulltext").is_none()),
        "{found:?}"
    );
    assert!(w010(lines).is_empty());
}
