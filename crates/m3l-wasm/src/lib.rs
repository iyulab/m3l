//! M3L parser WASM bindings.
//!
//! Provides JavaScript-callable functions via wasm-bindgen.
//! All functions take string inputs and return JSON strings.

use m3l_core::{parse_multi_to_json, parse_to_json, validate_to_json};
use wasm_bindgen::prelude::*;

/// Parse a single M3L file and return the AST as JSON.
///
/// @param content - M3L markdown text
/// @param filename - Source filename for error reporting
/// @returns JSON string with `{ success: boolean, data?: AST, error?: string }`
#[wasm_bindgen(js_name = "parse")]
pub fn wasm_parse(content: &str, filename: &str) -> String {
    parse_to_json(content, filename)
}

/// Parse multiple M3L files and return the merged AST as JSON.
///
/// @param files_json - JSON array of `{ content: string, filename: string }` objects
/// @returns JSON string with `{ success: boolean, data?: AST, error?: string }`
#[wasm_bindgen(js_name = "parseMulti")]
pub fn wasm_parse_multi(files_json: &str) -> String {
    parse_multi_to_json(files_json)
}

/// Validate M3L content and return diagnostics as JSON.
///
/// @param content - M3L markdown text
/// @param options_json - JSON options `{ strict?: boolean, filename?: string }`
/// @returns JSON string with `{ success: boolean, data?: ValidateResult, error?: string }`
#[wasm_bindgen(js_name = "validate")]
pub fn wasm_validate(content: &str, options_json: &str) -> String {
    validate_to_json(content, options_json)
}
