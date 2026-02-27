use m3l_core::{parse_multi_to_json, parse_to_json, validate_to_json};
use serde_json::Value;

fn assert_success(json: &str) -> Value {
    let v: Value = serde_json::from_str(json).expect("valid JSON");
    assert_eq!(v["success"], true, "expected success=true, got: {json}");
    v
}

fn assert_failure(json: &str) -> Value {
    let v: Value = serde_json::from_str(json).expect("valid JSON");
    assert_eq!(v["success"], false, "expected success=false, got: {json}");
    v
}

// ---------------------------------------------------------------------------
// parse_to_json
// ---------------------------------------------------------------------------

#[test]
fn ffi_parse_single_model() {
    let content = "## Product\n- name : string\n- price : decimal\n";
    let result = parse_to_json(content, "test.m3l.md");
    let v = assert_success(&result);

    let models = v["data"]["models"].as_array().unwrap();
    assert_eq!(models.len(), 1);
    assert_eq!(models[0]["name"], "Product");

    let fields = models[0]["fields"].as_array().unwrap();
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0]["name"], "name");
    assert_eq!(fields[1]["name"], "price");
}

#[test]
fn ffi_parse_empty_input() {
    let result = parse_to_json("", "empty.m3l.md");
    let v = assert_success(&result);
    assert_eq!(v["data"]["models"].as_array().unwrap().len(), 0);
}

#[test]
fn ffi_parse_with_enum() {
    let content = "## Status ::enum\n- Active\n- Inactive\n- Deleted\n";
    let result = parse_to_json(content, "test.m3l.md");
    let v = assert_success(&result);

    let enums = v["data"]["enums"].as_array().unwrap();
    assert_eq!(enums.len(), 1);
    assert_eq!(enums[0]["name"], "Status");
    assert_eq!(enums[0]["values"].as_array().unwrap().len(), 3);
}

#[test]
fn ffi_parse_json_structure() {
    let content = "## Item\n- id : int\n";
    let result = parse_to_json(content, "test.m3l.md");
    let v = assert_success(&result);

    // Check top-level AST keys exist
    assert!(v["data"]["parserVersion"].is_string());
    assert!(v["data"]["astVersion"].is_string());
    assert!(v["data"]["sources"].is_array());
    assert!(v["data"]["models"].is_array());
    assert!(v["data"]["enums"].is_array());
    assert!(v["data"]["errors"].is_array());
    assert!(v["data"]["warnings"].is_array());
}

// ---------------------------------------------------------------------------
// parse_multi_to_json
// ---------------------------------------------------------------------------

#[test]
fn ffi_parse_multi_files() {
    let files = serde_json::json!([
        {"content": "## Product\n- name : string\n", "filename": "product.m3l.md"},
        {"content": "## Category\n- title : string\n", "filename": "category.m3l.md"}
    ]);
    let result = parse_multi_to_json(&files.to_string());
    let v = assert_success(&result);

    let models = v["data"]["models"].as_array().unwrap();
    assert_eq!(models.len(), 2);

    let sources = v["data"]["sources"].as_array().unwrap();
    assert_eq!(sources.len(), 2);
}

#[test]
fn ffi_parse_multi_invalid_json() {
    let result = parse_multi_to_json("not valid json");
    let v = assert_failure(&result);
    assert!(v["error"].as_str().unwrap().contains("Invalid input JSON"));
}

#[test]
fn ffi_parse_multi_empty_array() {
    let result = parse_multi_to_json("[]");
    let v = assert_success(&result);
    assert_eq!(v["data"]["models"].as_array().unwrap().len(), 0);
}

// ---------------------------------------------------------------------------
// validate_to_json
// ---------------------------------------------------------------------------

#[test]
fn ffi_validate_clean() {
    let content = "## Product\n- name : string\n- price : decimal\n";
    let options = r#"{"strict": false, "filename": "test.m3l.md"}"#;
    let result = validate_to_json(content, options);
    let v = assert_success(&result);

    assert_eq!(v["data"]["errors"].as_array().unwrap().len(), 0);
    assert_eq!(v["data"]["warnings"].as_array().unwrap().len(), 0);
}

#[test]
fn ffi_validate_with_errors() {
    // M3L-E009: undefined type reference
    let content = "## Product\n- category : UnknownType\n";
    let options = r#"{"strict": false, "filename": "test.m3l.md"}"#;
    let result = validate_to_json(content, options);
    let v = assert_success(&result);

    let errors = v["data"]["errors"].as_array().unwrap();
    assert!(!errors.is_empty(), "should have M3L-E009 error");
    assert!(errors.iter().any(|e| e["code"] == "M3L-E009"));
}

#[test]
fn ffi_validate_invalid_options() {
    let content = "## Product\n- name : string\n";
    let result = validate_to_json(content, "not valid json");
    let v = assert_failure(&result);
    assert!(v["error"]
        .as_str()
        .unwrap()
        .contains("Invalid options JSON"));
}

#[test]
fn ffi_validate_default_options() {
    let content = "## Product\n- name : string\n";
    let options = "{}";
    let result = validate_to_json(content, options);
    let v = assert_success(&result);
    assert_eq!(v["data"]["errors"].as_array().unwrap().len(), 0);
}

#[test]
fn ffi_validate_default_filename() {
    let content = "## Item\n- name : string\n";
    let options = r#"{"strict": false}"#;
    let result = validate_to_json(content, options);
    let v = assert_success(&result);
    // Should work with default filename
    assert!(v["data"].is_object());
}
