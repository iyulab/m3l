use m3l_core::{parse_multi_to_json, parse_to_json, validate_multi_to_json, validate_to_json};
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

#[test]
fn ffi_parser_version_matches_cargo_version() {
    let content = "## Item\n- id : int\n";
    let result = parse_to_json(content, "test.m3l.md");
    let v = assert_success(&result);

    let parser_version = v["data"]["parserVersion"].as_str().unwrap();
    assert_eq!(
        parser_version,
        env!("CARGO_PKG_VERSION"),
        "parserVersion in AST should match Cargo.toml version"
    );
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

// ---------------------------------------------------------------------------
// validate_multi_to_json
// ---------------------------------------------------------------------------

/// A custom attribute registered in one file and used in another: the registry entry and the
/// usage only meet when the files are one resolve unit.
const REGISTRY_FILE: &str = "## help ::attribute
- type: string
- target: [value, field]
";
const USAGE_FILE: &str = "## Status ::enum
- active: \"Active\" @help(5)
";

fn files_json() -> String {
    serde_json::json!([
        {"content": REGISTRY_FILE, "filename": "registry.m3l.md"},
        {"content": USAGE_FILE, "filename": "usage.m3l.md"}
    ])
    .to_string()
}

#[test]
fn ffi_validate_multi_sees_a_registry_declared_in_another_file() {
    let result = validate_multi_to_json(&files_json(), r#"{"strict": false}"#);
    let v = assert_success(&result);

    let warnings = v["data"]["warnings"].as_array().unwrap();
    assert!(
        warnings.iter().any(|w| w["code"] == "M3L-W005"),
        "a registered string attribute given an integer should warn: {warnings:?}"
    );
}

#[test]
fn ffi_validate_multi_attributes_each_diagnostic_to_its_own_file() {
    let result = validate_multi_to_json(&files_json(), "{}");
    let v = assert_success(&result);

    let warning = v["data"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["code"] == "M3L-W005")
        .expect("M3L-W005");

    assert_eq!(
        warning["file"], "usage.m3l.md",
        "the diagnostic belongs to the file that used the attribute, not the one that declared it"
    );
}

#[test]
fn ffi_validate_single_file_cannot_see_the_other_file_registry() {
    // The reason the multi-file entry point has to exist: validating the usage on its own finds
    // nothing, because from that file alone the attribute was never registered.
    let result = validate_to_json(USAGE_FILE, r#"{"filename": "usage.m3l.md"}"#);
    let v = assert_success(&result);

    assert!(
        !v["data"]["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["code"] == "M3L-W005"),
        "per-file validation reports what that file alone can know — this is the gap"
    );
}

#[test]
fn ffi_validate_multi_invalid_input_json() {
    let result = validate_multi_to_json("not valid json", "{}");
    let v = assert_failure(&result);
    assert!(v["error"].as_str().unwrap().contains("Invalid input JSON"));
}

#[test]
fn ffi_validate_multi_invalid_options_json() {
    let result = validate_multi_to_json(&files_json(), "not valid json");
    let v = assert_failure(&result);
    assert!(v["error"]
        .as_str()
        .unwrap()
        .contains("Invalid options JSON"));
}

#[test]
fn ffi_validate_multi_empty_array() {
    let result = validate_multi_to_json("[]", "{}");
    let v = assert_success(&result);
    assert_eq!(v["data"]["errors"].as_array().unwrap().len(), 0);
    assert_eq!(v["data"]["warnings"].as_array().unwrap().len(), 0);
}

#[test]
fn ffi_validate_multi_resolves_a_cross_file_type_reference() {
    // The other half of "one resolve unit": a type defined in another file must not be reported
    // as undefined (M3L-E009).
    let files = serde_json::json!([
        {"content": "## Category
- title : string
", "filename": "category.m3l.md"},
        {"content": "## Product
- category : Category
", "filename": "product.m3l.md"}
    ]);
    let result = validate_multi_to_json(&files.to_string(), "{}");
    let v = assert_success(&result);

    assert!(
        !v["data"]["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["code"] == "M3L-E009"),
        "a type defined in a sibling file is defined"
    );
}
