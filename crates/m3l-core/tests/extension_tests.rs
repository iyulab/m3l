use m3l_core::{parse_string, resolve, validate, ModelType, ValidateOptions};

fn full_pipeline(input: &str, source: &str) -> m3l_core::M3lAst {
    let parsed = parse_string(input, source);
    let ast = resolve(&[parsed], None);
    validate(&ast, &ValidateOptions { strict: false });
    ast
}

#[test]
fn extension_basic_parsing() {
    let input = r#"
# Namespace: test

## TestDomain ::ontology
> 도메인 온톨로지 정의

### Relations
- teaches
  - from: Teacher
  - to: Student
  - type: TEACHES
"#;

    let ast = full_pipeline(input, "test.m3l");

    assert!(
        ast.extensions.contains_key("ontology"),
        "extensions should contain 'ontology' key"
    );
    let ontology_nodes = &ast.extensions["ontology"];
    assert_eq!(ontology_nodes.len(), 1);
    assert_eq!(ontology_nodes[0].name, "TestDomain");
    assert_eq!(
        ontology_nodes[0].model_type,
        ModelType::Extension("ontology".into())
    );
    assert_eq!(
        ontology_nodes[0].description.as_deref(),
        Some("도메인 온톨로지 정의")
    );
}

#[test]
fn extension_not_in_models_or_flows() {
    let input = r#"
## RegularModel
- id: identifier @primary

## TestFlow ::flow
> 흐름 정의

### Transitions
- step
  - from: RegularModel
  - to: OtherModel
  - type: TRIGGERS

## DomainOntology ::ontology
> 온톨로지

### Relations
- rel
  - from: A
  - to: B
"#;

    let ast = full_pipeline(input, "test.m3l");

    assert_eq!(ast.models.len(), 1, "only regular model in models");
    assert_eq!(ast.models[0].name, "RegularModel");
    assert_eq!(ast.flows.len(), 1, "flow in flows");
    assert_eq!(ast.flows[0].name, "TestFlow");
    assert!(
        ast.extensions.contains_key("ontology"),
        "ontology in extensions"
    );
    assert_eq!(ast.extensions["ontology"].len(), 1);
    assert_eq!(ast.extensions["ontology"][0].name, "DomainOntology");
}

#[test]
fn extension_multiple_types() {
    let input = r#"
## DomainRules ::ontology
> 온톨로지 규칙

### Relations
- rel
  - from: A
  - to: B

## AccessPolicy ::policy
> 접근 정책

### Rules
- admin_access
  - role: admin
  - action: allow
"#;

    let ast = full_pipeline(input, "test.m3l");

    assert!(
        ast.extensions.contains_key("ontology"),
        "should have ontology key"
    );
    assert!(
        ast.extensions.contains_key("policy"),
        "should have policy key"
    );
    assert_eq!(ast.extensions["ontology"].len(), 1);
    assert_eq!(ast.extensions["ontology"][0].name, "DomainRules");
    assert_eq!(ast.extensions["policy"].len(), 1);
    assert_eq!(ast.extensions["policy"][0].name, "AccessPolicy");
}

#[test]
fn extension_json_serialization() {
    let input = r#"
## TestOntology ::ontology

### Relations
- rel
  - from: A
  - to: B
"#;

    let parsed = parse_string(input, "test.m3l");
    let ast = resolve(&[parsed], None);
    let json = serde_json::to_string(&ast).unwrap();
    let parsed_back: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert!(
        parsed_back["extensions"].is_object(),
        "extensions should be an object"
    );
    assert!(
        parsed_back["extensions"]["ontology"].is_array(),
        "extensions.ontology should be array"
    );
    assert_eq!(
        parsed_back["extensions"]["ontology"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        parsed_back["extensions"]["ontology"][0]["name"],
        "TestOntology"
    );
    assert_eq!(parsed_back["extensions"]["ontology"][0]["type"], "ontology");

    // Round-trip: deserialize back to M3lAst
    let deserialized: m3l_core::M3lAst = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.extensions["ontology"].len(), 1);
    assert_eq!(
        deserialized.extensions["ontology"][0].model_type,
        ModelType::Extension("ontology".into())
    );
}

#[test]
fn extension_empty_extensions_not_serialized() {
    let input = r#"
## RegularModel
- id: identifier @primary
- name: string
"#;

    let parsed = parse_string(input, "test.m3l");
    let ast = resolve(&[parsed], None);
    let json = serde_json::to_string(&ast).unwrap();
    let parsed_back: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert!(
        parsed_back.get("extensions").is_none(),
        "empty extensions should not appear in JSON output"
    );
}

#[test]
fn extension_with_metadata() {
    let input = r#"
## DomainKnowledge ::ontology
> 도메인 지식 정의

### Relations
- teaches
  - from: Teacher
  - to: Student
  - type: TEACHES

### Metadata
- domain: education
- version: "1.0"
"#;

    let ast = full_pipeline(input, "test.m3l");

    assert_eq!(ast.extensions["ontology"].len(), 1);
    let node = &ast.extensions["ontology"][0];
    assert_eq!(node.name, "DomainKnowledge");
    assert_eq!(
        node.sections
            .metadata
            .get("domain")
            .and_then(|v| v.as_str()),
        Some("education")
    );
}

#[test]
fn existing_flow_tests_still_pass() {
    let input = r#"
## OrderFlow ::flow
> 주문 흐름

### Transitions
- order_created
  - from: Order
  - to: Payment
  - type: PRECEDES
"#;

    let ast = full_pipeline(input, "test.m3l");

    assert_eq!(
        ast.flows.len(),
        1,
        "::flow should go to flows, not extensions"
    );
    assert_eq!(ast.flows[0].name, "OrderFlow");
    assert_eq!(ast.flows[0].model_type, ModelType::Flow);
    assert!(
        !ast.extensions.contains_key("flow"),
        "::flow should NOT appear in extensions"
    );
}
