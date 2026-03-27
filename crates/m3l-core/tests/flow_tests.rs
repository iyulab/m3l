use m3l_core::{parse_string, resolve, validate, ValidateOptions, ModelType};

fn full_pipeline(input: &str, source: &str) -> m3l_core::M3lAst {
    let parsed = parse_string(input, source);
    let ast = resolve(&[parsed], None);
    validate(&ast, &ValidateOptions { strict: false });
    ast
}

#[test]
fn flow_basic_parsing() {
    let input = r#"
# Namespace: test

## TestFlow ::flow
> 테스트 프로세스 흐름

### Transitions
- step_one
  - from: ModelA
  - to: ModelB
  - guard: status=completed
  - type: PRECEDES
  - modality: POSSIBILITY

- step_two
  - from: ModelB
  - to: ModelC
  - type: PRODUCES
  - modality: NECESSITY
"#;

    let ast = full_pipeline(input, "test.m3l");

    assert_eq!(ast.flows.len(), 1);
    assert_eq!(ast.flows[0].name, "TestFlow");
    assert_eq!(ast.flows[0].model_type, ModelType::Flow);
    assert!(ast.flows[0].description.as_deref() == Some("테스트 프로세스 흐름"));
}

#[test]
fn flow_not_in_models() {
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
"#;

    let ast = full_pipeline(input, "test.m3l");

    assert_eq!(ast.models.len(), 1, "flow가 models에 포함되면 안됨");
    assert_eq!(ast.models[0].name, "RegularModel");
    assert_eq!(ast.flows.len(), 1);
    assert_eq!(ast.flows[0].name, "TestFlow");
}

#[test]
fn flow_with_metadata() {
    let input = r#"
## OrderProcess ::flow
> 주문 처리 흐름

### Transitions
- order_created
  - from: Order
  - to: Payment
  - type: PRECEDES
  - modality: NECESSITY

### Metadata
- domain: ecommerce
- version: "1.0"
"#;

    let ast = full_pipeline(input, "test.m3l");

    assert_eq!(ast.flows.len(), 1);
    let flow = &ast.flows[0];
    assert_eq!(flow.name, "OrderProcess");
    assert_eq!(
        flow.sections.metadata.get("domain").and_then(|v| v.as_str()),
        Some("ecommerce")
    );
}

#[test]
fn flow_json_serialization() {
    let input = r#"
## SimpleFlow ::flow

### Transitions
- step
  - from: A
  - to: B
  - type: TRIGGERS
"#;

    let parsed = parse_string(input, "test.m3l");
    let ast = resolve(&[parsed], None);
    let json = serde_json::to_string(&ast).unwrap();
    let parsed_back: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert!(parsed_back["flows"].is_array());
    assert_eq!(parsed_back["flows"].as_array().unwrap().len(), 1);
    assert_eq!(parsed_back["flows"][0]["name"], "SimpleFlow");
    assert_eq!(parsed_back["flows"][0]["type"], "flow");
}

#[test]
fn flow_multiple_in_one_file() {
    let input = r#"
## OrderFlow ::flow
> 주문 흐름

### Transitions
- start
  - from: Order
  - to: Payment
  - type: PRECEDES

## ShippingFlow ::flow
> 배송 흐름

### Transitions
- ship
  - from: Payment
  - to: Shipment
  - type: TRIGGERS
"#;

    let ast = full_pipeline(input, "test.m3l");

    assert_eq!(ast.flows.len(), 2);
    assert_eq!(ast.flows[0].name, "OrderFlow");
    assert_eq!(ast.flows[1].name, "ShippingFlow");
}
