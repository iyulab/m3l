//! The AST carries what each written name resolves to (§2.1.3): every declaration's `qualifiedName`, and beside every
//! name a declaration writes the `qualifiedName` of the declaration it names — resolved where the name is written.

use m3l_core::{parse_string, resolve, M3lAst};

fn ast(files: &[(&str, &str)]) -> M3lAst {
    let parsed: Vec<_> = files
        .iter()
        .map(|(src, name)| parse_string(src, name))
        .collect();
    resolve(&parsed, None)
}

const CORE: &str = "# Namespace: core\n\n## Status ::enum\n- open\n- closed\n\n## Asset\n- id: identifier @pk\n- name: string(50)\n";
const MAINT: &str = "# Namespace: maint\n\n## Asset\n- id: identifier @pk\n\n## Tracked ::interface\n- asset_id: identifier @reference(Asset)\n";

fn model<'a>(ast: &'a M3lAst, qualified: &str) -> &'a m3l_core::ModelNode {
    ast.models
        .iter()
        .find(|m| m.qualified_name.as_deref() == Some(qualified))
        .unwrap_or_else(|| panic!("no {qualified}"))
}

#[test]
fn a_declaration_is_qualified_by_its_namespace_or_named_alone() {
    let a = ast(&[
        (CORE, "core.m3l.md"),
        ("## Plain\n- id: identifier @pk\n", "plain.m3l.md"),
    ]);
    assert_eq!(a.models.iter().filter(|m| m.name == "Asset").count(), 1);
    model(&a, "core.Asset");
    model(&a, "Plain");
    assert_eq!(a.enums[0].qualified_name.as_deref(), Some("core.Status"));
}

#[test]
fn an_inherited_field_keeps_the_resolution_made_where_it_was_written() {
    // `Tracked` (namespace maint) writes `@reference(Asset)` — maint's Asset. A model in core inherits it; read from
    // core the short name would be core's Asset, but the name was written in maint.
    let a = ast(&[
        (CORE, "core.m3l.md"),
        (MAINT, "maint.m3l.md"),
        ("# Namespace: core\n\n## WorkOrder : maint.Tracked\n- id: identifier @pk\n- status: Status\n", "wo.m3l.md"),
    ]);
    let wo = model(&a, "core.WorkOrder");
    assert_eq!(
        wo.inherits_qualified_names,
        vec![Some("maint.Tracked".to_string())]
    );
    let key = wo.fields.iter().find(|f| f.name == "asset_id").unwrap();
    assert_eq!(key.reference_qualified_name.as_deref(), Some("maint.Asset"));
    let status = wo.fields.iter().find(|f| f.name == "status").unwrap();
    assert_eq!(status.type_qualified_name.as_deref(), Some("core.Status"));
}

#[test]
fn a_name_that_fits_two_declarations_gets_no_resolution() {
    let a = ast(&[
        (CORE, "core.m3l.md"),
        (MAINT, "maint.m3l.md"),
        ("# Namespace: sales\n\n## Order\n- id: identifier @pk\n- asset_id: identifier @reference(Asset)\n- core_asset_id: identifier @reference(core.Asset)\n", "o.m3l.md"),
    ]);
    let order = model(&a, "sales.Order");
    let field = |n: &str| order.fields.iter().find(|f| f.name == n).unwrap();
    assert_eq!(field("asset_id").reference_qualified_name, None);
    assert_eq!(
        field("core_asset_id").reference_qualified_name.as_deref(),
        Some("core.Asset")
    );
    // A built-in type names no declaration.
    assert_eq!(field("id").type_qualified_name, None);
}

#[test]
fn a_field_an_extend_block_adds_is_resolved_where_the_block_was_written() {
    let a = ast(&[
        (CORE, "core.m3l.md"),
        (MAINT, "maint.m3l.md"),
        ("# Namespace: maint\n\n## core.Asset ::extend\n- twin_id: identifier? @reference(Asset)\n", "ext.m3l.md"),
    ]);
    let asset = model(&a, "core.Asset");
    let twin = asset.fields.iter().find(|f| f.name == "twin_id").unwrap();
    assert_eq!(
        twin.reference_qualified_name.as_deref(),
        Some("maint.Asset")
    );
}

#[test]
fn the_validator_follows_an_inherited_key_where_it_was_written() {
    // The lookup is written in maint, through maint's Asset — which has no `name`. Read from core (where the field is
    // copied to) the hop would land on core's Asset, which has one, and the error would be missed.
    let tracked = "# Namespace: maint\n\n## Asset\n- id: identifier @pk\n\n## Tracked ::interface\n- asset_id: identifier @reference(Asset)\n- asset_name: string @lookup(asset_id.name)\n";
    let a = ast(&[
        (CORE, "core.m3l.md"),
        (tracked, "maint.m3l.md"),
        (
            "# Namespace: core\n\n## WorkOrder : maint.Tracked\n- id: identifier @pk\n",
            "wo.m3l.md",
        ),
    ]);
    let result = m3l_core::validate(&a, &m3l_core::ValidateOptions::default());
    assert!(
        result.errors.iter().any(|e| e.code == "M3L-E022"),
        "{:?}",
        result.errors
    );
}
