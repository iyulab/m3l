use m3l_core::types::*;
use pretty_assertions::assert_eq;

#[test]
fn field_node_json_keys() {
    let field = FieldNode {
        name: "email".into(),
        label: None,
        field_type: Some("string".into()),
        params: None,
        generic_params: None,
        nullable: false,
        array: false,
        array_item_nullable: false,
        kind: FieldKind::Stored,
        default_value: None,
        default_value_type: None,
        description: None,
        attributes: vec![],
        framework_attrs: None,
        lookup: None,
        rollup: None,
        computed: None,
        enum_values: None,
        fields: None,
        loc: SourceLocation {
            file: "test.m3l.md".into(),
            line: 1,
            col: 1,
        },
    };

    let json = serde_json::to_value(&field).unwrap();
    let obj = json.as_object().unwrap();

    // Must have these exact keys (camelCase / snake_case as TS outputs)
    assert!(obj.contains_key("name"));
    assert!(obj.contains_key("type")); // renamed from field_type
    assert!(obj.contains_key("nullable"));
    assert!(obj.contains_key("array"));
    assert!(obj.contains_key("arrayItemNullable")); // camelCase
    assert!(obj.contains_key("kind"));
    assert!(obj.contains_key("attributes"));
    assert!(obj.contains_key("loc"));

    // Optional fields should be absent when None
    assert!(!obj.contains_key("label"));
    assert!(!obj.contains_key("params"));
    assert!(!obj.contains_key("generic_params"));
    assert!(!obj.contains_key("default_value"));
    assert!(!obj.contains_key("default_value_type"));
    assert!(!obj.contains_key("description"));
    assert!(!obj.contains_key("framework_attrs"));
    assert!(!obj.contains_key("lookup"));
    assert!(!obj.contains_key("rollup"));
    assert!(!obj.contains_key("computed"));
    assert!(!obj.contains_key("enum_values"));
    assert!(!obj.contains_key("fields"));

    // Always-present fields have correct values
    assert_eq!(json["nullable"], false);
    assert_eq!(json["array"], false);
    assert_eq!(json["arrayItemNullable"], false);
    assert_eq!(json["kind"], "stored");
    assert_eq!(json["attributes"], serde_json::json!([]));
}

#[test]
fn field_attribute_optional_fields() {
    let attr = FieldAttribute {
        name: "pk".into(),
        args: None,
        cascade: None,
        is_standard: None,
        is_registered: None,
    };
    let json = serde_json::to_value(&attr).unwrap();
    let obj = json.as_object().unwrap();

    assert_eq!(obj.len(), 1); // only "name"
    assert_eq!(json["name"], "pk");
}

#[test]
fn field_attribute_with_standard() {
    let attr = FieldAttribute {
        name: "generated".into(),
        args: None,
        cascade: None,
        is_standard: Some(true),
        is_registered: None,
    };
    let json = serde_json::to_value(&attr).unwrap();

    assert_eq!(json["name"], "generated");
    assert_eq!(json["isStandard"], true); // camelCase key
    assert!(!json.as_object().unwrap().contains_key("isRegistered"));
}

#[test]
fn model_node_json_keys() {
    let model = ModelNode {
        name: "User".into(),
        label: None,
        model_type: ModelType::Model,
        source: "test.m3l.md".into(),
        line: 1,
        inherits: vec![],
        description: None,
        attributes: vec![],
        fields: vec![],
        sections: Sections::default(),
        materialized: None,
        source_def: None,
        refresh: None,
        loc: SourceLocation {
            file: "test.m3l.md".into(),
            line: 1,
            col: 1,
        },
    };

    let json = serde_json::to_value(&model).unwrap();
    let obj = json.as_object().unwrap();

    assert_eq!(json["type"], "model");
    assert!(obj.contains_key("inherits"));
    assert!(obj.contains_key("attributes"));
    assert!(obj.contains_key("fields"));
    assert!(obj.contains_key("sections"));
    assert!(!obj.contains_key("materialized"));
    assert!(!obj.contains_key("source_def"));
    assert!(!obj.contains_key("refresh"));
}

#[test]
fn m3l_ast_json_keys() {
    let ast = M3lAst {
        parser_version: "0.4.0".into(),
        ast_version: "1.0".into(),
        project: ProjectInfo {
            name: Some("test".into()),
            version: None,
        },
        sources: vec![],
        models: vec![],
        enums: vec![],
        interfaces: vec![],
        views: vec![],
        attribute_registry: vec![],
        errors: vec![],
        warnings: vec![],
    };

    let json = serde_json::to_value(&ast).unwrap();
    let obj = json.as_object().unwrap();

    // camelCase keys
    assert!(obj.contains_key("parserVersion"));
    assert!(obj.contains_key("astVersion"));
    assert!(obj.contains_key("attributeRegistry"));

    // Not snake_case
    assert!(!obj.contains_key("parser_version"));
    assert!(!obj.contains_key("ast_version"));
    assert!(!obj.contains_key("attribute_registry"));
}

#[test]
fn enum_node_json() {
    let en = EnumNode {
        name: "Status".into(),
        label: None,
        enum_type: ModelType::Enum,
        source: "test.m3l.md".into(),
        line: 5,
        inherits: vec![],
        description: None,
        values: vec![EnumValue {
            name: "Active".into(),
            description: Some("Active status".into()),
            value_type: None,
            value: None,
        }],
        loc: SourceLocation {
            file: "test.m3l.md".into(),
            line: 5,
            col: 1,
        },
    };

    let json = serde_json::to_value(&en).unwrap();
    assert_eq!(json["type"], "enum");
    assert_eq!(json["values"][0]["name"], "Active");
    assert_eq!(json["values"][0]["description"], "Active status");
}

#[test]
fn attribute_registry_entry_json() {
    let entry = AttributeRegistryEntry {
        name: "custom_flag".into(),
        description: Some("A custom flag".into()),
        target: vec!["field".into(), "model".into()],
        attr_type: "boolean".into(),
        range: None,
        required: false,
        default_value: Some(AttrArgValue::Bool(false)),
    };

    let json = serde_json::to_value(&entry).unwrap();
    assert_eq!(json["defaultValue"], false); // camelCase
    assert_eq!(json["type"], "boolean");
    assert!(!json.as_object().unwrap().contains_key("range"));
}

#[test]
fn diagnostic_json() {
    let diag = Diagnostic {
        code: "M3L-E001".into(),
        severity: DiagnosticSeverity::Error,
        file: "test.m3l.md".into(),
        line: 10,
        col: 1,
        message: "Test error".into(),
    };

    let json = serde_json::to_value(&diag).unwrap();
    assert_eq!(json["severity"], "error");
    assert_eq!(json["code"], "M3L-E001");
}

#[test]
fn field_kind_serialization() {
    assert_eq!(serde_json::to_value(FieldKind::Stored).unwrap(), "stored");
    assert_eq!(
        serde_json::to_value(FieldKind::Computed).unwrap(),
        "computed"
    );
    assert_eq!(serde_json::to_value(FieldKind::Lookup).unwrap(), "lookup");
    assert_eq!(serde_json::to_value(FieldKind::Rollup).unwrap(), "rollup");
}

#[test]
fn sections_default_with_custom() {
    let mut sections = Sections::default();
    sections
        .custom
        .insert("CustomSection".into(), serde_json::json!(["item1"]));

    let json = serde_json::to_value(&sections).unwrap();
    let obj = json.as_object().unwrap();

    assert!(obj.contains_key("indexes"));
    assert!(obj.contains_key("relations"));
    assert!(obj.contains_key("behaviors"));
    assert!(obj.contains_key("metadata"));
    assert!(obj.contains_key("CustomSection")); // flattened
    assert_eq!(json["CustomSection"], serde_json::json!(["item1"]));
}

#[test]
fn catalogs_content() {
    use m3l_core::catalogs::{KIND_SECTIONS, STANDARD_ATTRIBUTES, TYPE_CATALOG};

    // Type catalog
    assert!(TYPE_CATALOG.contains("string"));
    assert!(TYPE_CATALOG.contains("datetime")); // deprecated but still accepted
    assert!(!TYPE_CATALOG.contains("unknown_type"));
    assert_eq!(TYPE_CATALOG.len(), 22);

    // Standard attributes
    assert!(STANDARD_ATTRIBUTES.contains("primary"));
    assert!(STANDARD_ATTRIBUTES.contains("override"));
    assert!(!STANDARD_ATTRIBUTES.contains("custom_attr"));
    assert_eq!(STANDARD_ATTRIBUTES.len(), 31);

    // Kind sections
    assert!(KIND_SECTIONS.contains("Lookup"));
    assert!(KIND_SECTIONS.contains("Computed from Rollup"));
    assert_eq!(KIND_SECTIONS.len(), 4);
}
