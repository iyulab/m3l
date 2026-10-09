//! Expressions in the three portable contexts (specification §10.7) — `@computed`, a rollup's `where:`
//! and a view's `where` — are held to the portable subset: a construct outside it is `M3L-W011`, a
//! warning that names the construct. `@computed_raw` and a view whose source is a SQL code block are
//! platform-specific by declaration and are not checked.

use m3l_core::{parse_string, validate, Diagnostic, ValidateOptions};

fn w011(input: &str) -> Vec<Diagnostic> {
    let parsed = parse_string(input, "portable_expressions.m3l.md");
    let ast = m3l_core::resolve(&[parsed], None);
    validate(&ast, &ValidateOptions::default())
        .warnings
        .into_iter()
        .filter(|w| w.code == "M3L-W011")
        .collect()
}

const MODELS: &str = "## Customer\n- id: identifier @pk\n- first_name: string(50)\n- last_name: string(50)\n\
- region: string(20)\n\n\
## Order\n- id: identifier @pk\n- customer_id: identifier @reference(Customer)\n- status: string(20)\n\
- total: decimal(12,2)\n- ordered_at: timestamp\n\n";

#[test]
fn portable_expressions_in_all_three_contexts_are_silent() {
    let found = w011(&format!(
        "{MODELS}## Card\n- id: identifier @pk\n- first_name: string(50)\n- last_name: string(50)\n\
         - full_name: string @computed(\"concat(first_name, ' ', last_name)\")\n\
         - tier: string @computed(\"CASE WHEN length(first_name) > 3 THEN 'Long' ELSE 'Short' END\")\n\n\
         ## Summary\n- id: identifier @pk\n- region: string(20)\n\
         - paid: integer @rollup(Order.customer_id, count, where: \"status = 'paid' AND ordered_at > date_add(now(), -30, 'day')\")\n\n\
         ## Recent ::view\n### Source\n- from: Order\n- where: \"Order.status <> 'cancelled' AND Order.total > 0\"\n\n\
         - status: string @from(Order.status)"
    ));
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_computed_expression_in_platform_syntax_is_w011_and_names_what_is_outside() {
    let found = w011(&format!(
        "{MODELS}## Card\n- id: identifier @pk\n- birth: date\n\
         - age: integer @computed(\"DATEDIFF(YEAR, birth, GETDATE())\")"
    ));
    let w = found.first().expect("W011");
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        w.message.contains("\"Card.age\" @computed"),
        "{}",
        w.message
    );
    assert!(
        w.message
            .contains("function DATEDIFF(), function GETDATE()"),
        "{}",
        w.message
    );
}

#[test]
fn plus_between_text_fields_is_named_as_concatenation() {
    let found = w011(&format!(
        "{MODELS}## Card\n- id: identifier @pk\n- first_name: string(50)\n- last_name: string(50)\n\
         - full_name: string @computed(\"first_name + last_name\")\n- sum: decimal @computed(\"1 + 2\")"
    ));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].message.contains("+ joining text"),
        "{}",
        found[0].message
    );
}

#[test]
fn a_rollup_where_reads_the_rolled_up_models_text_fields() {
    let found = w011(&format!(
        "{MODELS}## Summary\n- id: identifier @pk\n\
         - odd: integer @rollup(Order.customer_id, count, where: \"status + 'x' = 'paidx'\")"
    ));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].message.contains("\"Summary.odd\" @rollup where"),
        "{}",
        found[0].message
    );
}

#[test]
fn a_view_where_in_platform_syntax_is_w011() {
    let found = w011(&format!(
        "{MODELS}## Recent ::view\n### Source\n- from: Order\n- where: \"[status] <> 'cancelled'\"\n\n\
         - status: string @from(Order.status)"
    ));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].message.contains("View \"Recent\" where"),
        "{}",
        found[0].message
    );
    assert!(
        found[0].message.contains("bracketed identifier [status]"),
        "{}",
        found[0].message
    );
}

#[test]
fn declared_platform_expressions_are_not_checked() {
    let found = w011(&format!(
        "{MODELS}## Card\n- id: identifier @pk\n- created_at: timestamp\n\
         - year_created: integer @computed_raw(\"YEAR(created_at)\", platform: \"sqlserver\")\n\n\
         ## Totals ::view\n### Source\n```sql\nSELECT c.id, COUNT(o.id) AS n FROM Customer c JOIN Order o ON o.customer_id = c.id GROUP BY c.id\n```\n\n\
         - n: integer @computed(`COUNT(o.id)`)"
    ));
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_computed_field_an_interface_declares_is_reported_once() {
    let found = w011(
        "## Named ::interface\n- label: string @computed(\"ISNULL(code, 'none')\")\n- code: string(10)\n\n\
         ## A : Named\n- id: identifier @pk\n\n## B : Named\n- id: identifier @pk",
    );
    assert_eq!(found.len(), 1, "{found:?}");
}
