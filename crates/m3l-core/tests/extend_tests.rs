use m3l_core::{parse_string, resolve, resolve_with, ModelType, ResolveOptions};

const BASE: &str = "## Timestampable ::interface\n- created_at: timestamp = now()\n\n## Asset : Timestampable\n- id: identifier @pk\n- name: string(100)\n";

fn codes(ast: &m3l_core::M3lAst) -> Vec<&str> {
    ast.errors.iter().map(|e| e.code.as_str()).collect()
}

#[test]
fn prefix_is_stamped_on_every_declaration() {
    let f = parse_string(
        "# Namespace: ex.insp\n# Prefix: insp\n\n## InspRecord\n- id: identifier @pk\n\n## InspGrade ::enum\n- a\n- b\n",
        "insp.m3l.md",
    );
    assert_eq!(f.prefix.as_deref(), Some("insp"));
    assert_eq!(f.models[0].prefix.as_deref(), Some("insp"));
    assert_eq!(f.enums[0].prefix.as_deref(), Some("insp"));
}

#[test]
fn prefix_after_first_model_is_ignored() {
    let f = parse_string("## A\n- id: identifier @pk\n\n# Prefix: late\n", "a.m3l.md");
    assert_eq!(f.prefix, None);
}

#[test]
fn aspect_is_a_model_with_a_base() {
    let f = parse_string(
        "## AssetProfile ::aspect(Asset) : Timestampable\n- level: integer?\n",
        "p.m3l.md",
    );
    assert!(f.extensions.is_empty());
    let m = &f.models[0];
    assert_eq!(m.model_type, ModelType::Model);
    let base = m.base.as_ref().expect("base");
    assert_eq!(
        (base.kind.as_str(), base.model.as_str()),
        ("aspect", "Asset")
    );
    assert_eq!(m.inherits, vec!["Timestampable".to_string()]);
}

#[test]
fn subtype_is_a_model_with_a_base() {
    let f = parse_string(
        "## Contractor ::subtype(Organization)\n- license_no: string(40)?\n",
        "c.m3l.md",
    );
    assert_eq!(f.models[0].base.as_ref().unwrap().kind, "subtype");
}

#[test]
fn extend_block_lands_in_the_extend_bucket_with_its_parents_kept() {
    let f = parse_string(
        "# Prefix: insp\n\n## Asset ::extend : Timestampable\n- insp_grade: string(20)?\n",
        "e.m3l.md",
    );
    let block = &f.extensions["extend"][0];
    assert_eq!(block.name, "Asset");
    assert_eq!(block.prefix.as_deref(), Some("insp"));
    assert_eq!(block.inherits, vec!["Timestampable".to_string()]);
    assert_eq!(block.fields.len(), 1);
}

#[test]
fn json_omits_the_new_members_when_absent() {
    let f = parse_string("## Plain\n- id: identifier @pk\n", "p.m3l.md");
    let ast = m3l_core::resolve(&[f], None);
    let json = serde_json::to_string(&ast).unwrap();
    for key in ["\"prefix\"", "\"base\"", "\"extended_by\"", "\"origin\""] {
        assert!(!json.contains(key), "{key} must be omitted");
    }
}

#[test]
fn extend_fields_are_appended_after_inherited_and_own_fields() {
    let base = parse_string(BASE, "base.m3l.md");
    let ext = parse_string(
        "# Namespace: ex.insp\n# Prefix: insp\n\n## Asset ::extend\n- insp_grade: string(20)?\n",
        "insp.m3l.md",
    );
    let ast = resolve(&[base, ext], None);
    assert!(ast.errors.is_empty(), "{:?}", ast.errors);
    let asset = ast.models.iter().find(|m| m.name == "Asset").unwrap();
    let names: Vec<&str> = asset.fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["created_at", "id", "name", "insp_grade"]);
    let origin = asset.fields[3].origin.as_ref().unwrap();
    assert_eq!(origin.prefix.as_deref(), Some("insp"));
    assert_eq!(origin.namespace.as_deref(), Some("ex.insp"));
    assert_eq!(origin.source, "insp.m3l.md");
    assert!(asset.fields[2].origin.is_none());
    assert_eq!(asset.extended_by.len(), 1);
    assert_eq!(asset.extended_by[0].fields, 1);
    assert!(!ast.extensions.contains_key("extend"));
}

#[test]
fn two_blocks_merge_in_file_order_without_a_duplicate_name_error() {
    let base = parse_string(BASE, "base.m3l.md");
    let a = parse_string(
        "# Prefix: insp\n\n## Asset ::extend\n- insp_grade: string(20)?\n",
        "a.m3l.md",
    );
    let b = parse_string(
        "# Prefix: acme\n\n## Asset ::extend\n- acme_code: string(10)?\n",
        "b.m3l.md",
    );
    let ast = resolve(&[base, a, b], None);
    assert!(ast.errors.is_empty(), "{:?}", ast.errors);
    let asset = ast.models.iter().find(|m| m.name == "Asset").unwrap();
    assert_eq!(asset.fields.last().unwrap().name, "acme_code");
    assert_eq!(
        asset
            .extended_by
            .iter()
            .map(|e| e.prefix.as_deref().unwrap())
            .collect::<Vec<_>>(),
        ["insp", "acme"]
    );
}

#[test]
fn e011_target_not_found() {
    let ext = parse_string("## Ghost ::extend\n- x: string?\n", "e.m3l.md");
    assert!(codes(&resolve(&[ext], None)).contains(&"M3L-E011"));
}

#[test]
fn e012_collision_with_own_inherited_and_other_extension_fields() {
    for clash in ["name", "created_at"] {
        let base = parse_string(BASE, "base.m3l.md");
        let ext = parse_string(
            &format!("## Asset ::extend\n- {clash}: string?\n"),
            "e.m3l.md",
        );
        let ast = resolve(&[base, ext], None);
        assert_eq!(codes(&ast), ["M3L-E012"], "clash on {clash}");
    }
    let base = parse_string(BASE, "base.m3l.md");
    let a = parse_string("## Asset ::extend\n- x_code: string?\n", "a.m3l.md");
    let b = parse_string("## Asset ::extend\n- x_code: string?\n", "b.m3l.md");
    assert_eq!(codes(&resolve(&[base, a, b], None)), ["M3L-E012"]);
}

#[test]
fn e013_pk_and_override_are_not_allowed() {
    for attr in ["@pk", "@primary", "@override"] {
        let base = parse_string(BASE, "base.m3l.md");
        let ext = parse_string(
            &format!("## Asset ::extend\n- x_id: identifier {attr}\n"),
            "e.m3l.md",
        );
        assert!(
            codes(&resolve(&[base, ext], None)).contains(&"M3L-E013"),
            "{attr}"
        );
    }
}

#[test]
fn e014_parents_on_an_extend_block() {
    let base = parse_string(BASE, "base.m3l.md");
    let ext = parse_string(
        "## Asset ::extend : Timestampable\n- x: string?\n",
        "e.m3l.md",
    );
    let ast = resolve(&[base, ext], None);
    assert!(codes(&ast).contains(&"M3L-E014"));
    assert!(ast
        .models
        .iter()
        .find(|m| m.name == "Asset")
        .unwrap()
        .extended_by
        .is_empty());
}

#[test]
fn e015_anything_but_fields() {
    for body in [
        "- x: string?\n- @unique(name, x)\n",
        "- x: string?\n\n### Indexes\n- idx_x\n  - fields: [x]\n",
    ] {
        let base = parse_string(BASE, "base.m3l.md");
        let ext = parse_string(&format!("## Asset ::extend\n{body}"), "e.m3l.md");
        assert!(
            codes(&resolve(&[base, ext], None)).contains(&"M3L-E015"),
            "{body}"
        );
    }
}

#[test]
fn derived_fields_are_allowed_in_an_extend_block() {
    let base = parse_string(BASE, "base.m3l.md");
    let ext = parse_string(
        "## Asset ::extend\n- x_label: string @computed(\"name\")\n",
        "e.m3l.md",
    );
    assert!(resolve(&[base, ext], None).errors.is_empty());
}

#[test]
fn e016_e017_e018_base_checks() {
    let no_arg = parse_string("## P ::aspect\n- a: string?\n", "p.m3l.md");
    assert!(codes(&resolve(&[no_arg], None)).contains(&"M3L-E016"));

    let missing = parse_string("## P ::aspect(Ghost)\n- a: string?\n", "p.m3l.md");
    assert!(codes(&resolve(&[missing], None)).contains(&"M3L-E017"));

    let chain = parse_string(
        &format!("{BASE}\n## AssetP ::aspect(Asset)\n- a: string?\n\n## AssetPQ ::aspect(AssetP)\n- b: string?\n"),
        "c.m3l.md",
    );
    assert!(codes(&resolve(&[chain], None)).contains(&"M3L-E018"));

    let sub_of_aspect = parse_string(
        &format!(
            "{BASE}\n## AssetP ::aspect(Asset)\n- a: string?\n\n## X ::subtype(AssetP)\n- b: string?\n"
        ),
        "c.m3l.md",
    );
    assert!(codes(&resolve(&[sub_of_aspect], None)).contains(&"M3L-E018"));
}

#[test]
fn an_aspect_can_be_extended() {
    let src = format!(
        "{BASE}\n## AssetP ::aspect(Asset)\n- a: string?\n\n## AssetP ::extend\n- b: string?\n"
    );
    let ast = resolve(&[parse_string(&src, "c.m3l.md")], None);
    assert!(ast.errors.is_empty(), "{:?}", ast.errors);
}

#[test]
fn merge_extends_off_keeps_blocks_in_the_bucket() {
    let base = parse_string(BASE, "base.m3l.md");
    let ext = parse_string("## Asset ::extend\n- x: string?\n", "e.m3l.md");
    let ast = resolve_with(
        &[base, ext],
        None,
        ResolveOptions {
            inline_inherited: false,
            merge_extends: false,
        },
    );
    assert!(ast.errors.is_empty(), "{:?}", ast.errors);
    assert_eq!(ast.extensions["extend"].len(), 1);
    assert_eq!(
        ast.models
            .iter()
            .find(|m| m.name == "Asset")
            .unwrap()
            .fields
            .len(),
        2
    );
}
