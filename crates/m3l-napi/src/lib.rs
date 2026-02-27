//! M3L parser Node.js native addon via napi-rs.
//!
//! Provides high-performance native functions for Node.js.
//! All functions take string inputs and return JSON strings.

#[macro_use]
extern crate napi_derive;

use m3l_core::{parse_multi_to_json, parse_to_json, validate_to_json};

/// Parse a single M3L file and return the AST as JSON.
///
/// @param content - M3L markdown text
/// @param filename - Source filename for error reporting
/// @returns JSON string with `{ success: boolean, data?: AST, error?: string }`
#[napi]
pub fn parse(content: String, filename: String) -> String {
    parse_to_json(&content, &filename)
}

/// Parse multiple M3L files and return the merged AST as JSON.
///
/// @param files_json - JSON array of `{ content: string, filename: string }` objects
/// @returns JSON string with `{ success: boolean, data?: AST, error?: string }`
#[napi(js_name = "parseMulti")]
pub fn parse_multi(files_json: String) -> String {
    parse_multi_to_json(&files_json)
}

/// Validate M3L content and return diagnostics as JSON.
///
/// @param content - M3L markdown text
/// @param options_json - JSON options `{ strict?: boolean, filename?: string }`
/// @returns JSON string with `{ success: boolean, data?: ValidateResult, error?: string }`
#[napi]
pub fn validate(content: String, options_json: String) -> String {
    validate_to_json(&content, &options_json)
}
