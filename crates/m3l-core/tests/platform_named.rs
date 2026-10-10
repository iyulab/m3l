//! A platform expression names its platform (specification §10.7.5): `@computed_raw`, a rollup's `where_raw:`
//! and a view's `- where_raw:` are platform syntax by declaration, and an implementation picks the one for its
//! platform. One that names none is never chosen — `M3L-W012`, a warning, so the declaration is not dropped
//! without a word. In a rollup the `platform:` follows the `where_raw:` it is for; written before it, it names
//! nothing and leaves that `where_raw:` without a platform.

use m3l_core::{parse_string, validate, Diagnostic, ValidateOptions};

fn w012(input: &str) -> Vec<Diagnostic> {
    let parsed = parse_string(input, "platform_named.m3l.md");
    let ast = m3l_core::resolve(&[parsed], None);
    validate(&ast, &ValidateOptions::default())
        .warnings
        .into_iter()
        .filter(|w| w.code == "M3L-W012")
        .collect()
}

const ORDER: &str = "## Customer\n- id: identifier @pk\n\n\
## Order\n- id: identifier @pk\n- customer_id: identifier @reference(Customer)\n- created_at: timestamp\n\n";

#[test]
fn a_computed_raw_with_no_platform_is_w012() {
    let found = w012(
        "## T\n- id: identifier @pk\n- created_at: timestamp\n\
                      - y: integer @computed_raw(\"YEAR(created_at)\")",
    );
    let w = found.first().expect("W012");
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        w.message.contains("\"T.y\" declares @computed_raw"),
        "{}",
        w.message
    );
    assert!(w.message.contains("platform:"), "{}", w.message);
}

#[test]
fn a_view_where_raw_with_no_platform_is_w012() {
    let found = w012(&format!(
        "{ORDER}## Recent ::view\n### Source\n- from: Order\n- where_raw: \"Order.created_at > GETDATE() - 30\"\n\n\
         - id: identifier @from(Order.id)"
    ));
    let w = found.first().expect("W012");
    assert!(
        w.message.contains("View \"Recent\" declares where_raw"),
        "{}",
        w.message
    );
}

#[test]
fn a_rollup_where_raw_with_no_platform_is_w012() {
    let found = w012(&format!(
        "{ORDER}## Summary\n- id: identifier @pk\n\
         - n: integer @rollup(Order.customer_id, count, where_raw: \"created_at > GETDATE() - 30\")"
    ));
    let w = found.first().expect("W012");
    assert!(
        w.message
            .contains("\"Summary.n\" declares @rollup where_raw"),
        "{}",
        w.message
    );
}

#[test]
fn a_rollup_platform_written_before_its_where_raw_is_w012_and_says_where_it_goes() {
    let found = w012(&format!(
        "{ORDER}## Summary\n- id: identifier @pk\n\
         - n: integer @rollup(Order.customer_id, count, platform: \"sqlserver\", where_raw: \"created_at > GETDATE() - 30\")"
    ));
    let w = found.first().expect("W012");
    assert!(
        w.message.contains("follows the where_raw: it is for"),
        "{}",
        w.message
    );
}

#[test]
fn platform_expressions_that_name_their_platform_are_silent() {
    let found = w012(&format!(
        "{ORDER}## T\n- id: identifier @pk\n- created_at: timestamp\n\
         - y: integer @computed_raw(\"YEAR(created_at)\", platform: \"sqlserver\")\n\
         - tier: string @computed_raw(platform: \"sqlserver\")\n  ```\n  IIF(y > 2000, 'new', 'old')\n  ```\n\n\
         ## Summary\n- id: identifier @pk\n\
         - n: integer @rollup(Order.customer_id, count, where_raw: \"a = 1\", platform: \"sqlserver\", where_raw: \"a = 2\", platform: \"postgresql\")\n\n\
         ## Recent ::view\n### Source\n- from: Order\n- where_raw: \"a = 1\" platform: sqlserver\n\n\
         - id: identifier @from(Order.id)"
    ));
    assert!(found.is_empty(), "{found:?}");
}
