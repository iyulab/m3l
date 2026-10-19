//! `@import "…" as alias` (§10.6.2): a name qualified by an alias the referring file declares names the
//! declaration in the file that import names — an inheritance parent, a base model, an extend target.

use m3l_core::{parse_string, resolve, M3lAst};

const INVENTORY: &str = "# Namespace: domain.inventory\n\n## Stamped ::interface\n- created_at: timestamp = now()\n\n## Product\n- id: identifier @pk\n- name: string(100)\n";

fn codes(ast: &M3lAst) -> Vec<&str> {
    ast.errors.iter().map(|e| e.code.as_str()).collect()
}

fn resolve_two(order: &str) -> M3lAst {
    let inventory = parse_string(INVENTORY, "inventory/models.m3l.md");
    let order = parse_string(order, "order.m3l.md");
    resolve(&[inventory, order], None)
}

#[test]
fn an_import_records_its_alias() {
    let f = parse_string(
        "@import \"inventory/models.m3l\" as inventory\n@import \"common/base.m3l\"\n\n## A\n- id: identifier @pk\n",
        "a.m3l.md",
    );
    assert_eq!(f.imports.len(), 2);
    assert_eq!(f.imports[0].path, "inventory/models.m3l");
    assert_eq!(f.imports[0].alias.as_deref(), Some("inventory"));
    assert_eq!(f.imports[1].alias, None);
}

#[test]
fn an_aliased_parent_is_inherited() {
    let ast = resolve_two(
        "# Namespace: domain.sales\n@import \"inventory/models.m3l\" as inv\n\n## Order : inv.Stamped\n- id: identifier @pk\n",
    );
    assert_eq!(codes(&ast), Vec::<&str>::new());
    let order = ast.models.iter().find(|m| m.name == "Order").unwrap();
    assert!(
        order.fields.iter().any(|f| f.name == "created_at"),
        "{:?}",
        order.fields
    );
}

#[test]
fn an_aliased_base_and_extend_target_resolve() {
    let ast = resolve_two(
        "# Namespace: domain.sales\n@import \"inventory/models.m3l\" as inv\n\n## ProductSalesProfile ::aspect(inv.Product)\n- margin: decimal(5,2)?\n\n## inv.Product ::extend\n- sales_note: string(50)?\n",
    );
    assert_eq!(codes(&ast), Vec::<&str>::new());
    let product = ast.models.iter().find(|m| m.name == "Product").unwrap();
    assert!(product.fields.iter().any(|f| f.name == "sales_note"));
}

#[test]
fn an_alias_is_the_referring_files_own() {
    // `inv` means nothing in a file that does not declare it — read as a namespace there, it names none.
    let ast =
        resolve_two("# Namespace: domain.sales\n\n## Order : inv.Stamped\n- id: identifier @pk\n");
    assert!(codes(&ast).contains(&"M3L-E007"), "{:?}", ast.errors);
}

#[test]
fn the_ast_carries_each_import_and_the_file_it_names() {
    let ast = resolve_two(
        "@import \"inventory/models.m3l\" as inv\n@import \"missing.m3l\"\n\n## Order\n- id: identifier @pk\n",
    );
    assert_eq!(ast.imports.len(), 2);
    assert_eq!(ast.imports[0].file, "order.m3l.md");
    assert_eq!(ast.imports[0].alias.as_deref(), Some("inv"));
    assert_eq!(
        ast.imports[0].target.as_deref(),
        Some("inventory/models.m3l.md")
    );
    assert_eq!(ast.imports[1].target, None);
    // Without imports the key is left out — an AST of a set that imports nothing reads as before.
    let lone = resolve(&[parse_string(INVENTORY, "inventory/models.m3l.md")], None);
    assert!(!serde_json::to_string(&lone)
        .unwrap()
        .contains("\"imports\""));
}

#[test]
fn the_validator_follows_an_alias_in_a_view_and_a_reference() {
    let ast = resolve_two(
        "# Namespace: domain.sales\n@import \"inventory/models.m3l\" as inv\n\n## Order\n- id: identifier @pk\n- product_id: identifier @reference(inv.Product)\n- product_name: string @lookup(product_id.name)\n\n## OrderList ::view\n### Source\n- from: inv.Product\n\n- id: identifier @from(inv.Product.id)\n",
    );
    let result = m3l_core::validate(&ast, &m3l_core::ValidateOptions::default());
    assert!(result.errors.is_empty(), "{:?}", result.errors);
}

#[test]
fn one_alias_for_two_imports_is_e034() {
    let ast = resolve_two(
        "@import \"inventory/models.m3l\" as inv\n@import \"other.m3l\" as inv\n\n## Order\n- id: identifier @pk\n",
    );
    let e034: Vec<_> = ast.errors.iter().filter(|e| e.code == "M3L-E034").collect();
    assert_eq!(e034.len(), 1, "{:?}", ast.errors);
    assert_eq!(e034[0].line, 2);
}
