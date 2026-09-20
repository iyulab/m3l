use m3l_core::{parse_string, ModelType};

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
