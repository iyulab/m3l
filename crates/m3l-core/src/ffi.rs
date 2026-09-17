//! FFI-oriented JSON API for cross-language bindings.
//!
//! All functions take string inputs and return JSON strings,
//! minimizing the FFI surface area.

use crate::types::*;
use crate::{parse_string, resolve, validate};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Options types (deserialized from JSON input)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ParseOptions {
    #[serde(default)]
    pub filename: String,
}

/// One file of a multi-file resolve unit, as the JSON input carries it.
#[derive(Debug, Clone, Deserialize)]
struct FileInput {
    content: String,
    filename: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ValidateJsonOptions {
    #[serde(default)]
    pub strict: bool,
    #[serde(default)]
    pub filename: String,
}

// ---------------------------------------------------------------------------
// Result types (serialized to JSON output)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct FfiResult<T: Serialize> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Public FFI functions
// ---------------------------------------------------------------------------

/// Parse a single M3L file from string content and return the AST as JSON.
///
/// Input: M3L markdown text
/// Output: JSON string containing the resolved AST (single-file)
pub fn parse_to_json(content: &str, filename: &str) -> String {
    let result = std::panic::catch_unwind(|| {
        let parsed = parse_string(content, filename);
        resolve(&[parsed], None)
    });

    match result {
        Ok(ast) => {
            let ffi_result = FfiResult {
                success: true,
                data: Some(ast),
                error: None,
            };
            serde_json::to_string(&ffi_result).unwrap_or_else(|e| {
                serde_json::to_string(&FfiResult::<()> {
                    success: false,
                    data: None,
                    error: Some(format!("JSON serialization error: {e}")),
                })
                .unwrap()
            })
        }
        Err(_) => serde_json::to_string(&FfiResult::<()> {
            success: false,
            data: None,
            error: Some("Internal parser panic".to_string()),
        })
        .unwrap(),
    }
}

/// Parse multiple M3L files from string content and return the merged AST as JSON.
///
/// Input: JSON array of `{ "content": "...", "filename": "..." }` objects
/// Output: JSON string containing the resolved AST
pub fn parse_multi_to_json(files_json: &str) -> String {
    let files: Vec<FileInput> = match serde_json::from_str(files_json) {
        Ok(f) => f,
        Err(e) => {
            return serde_json::to_string(&FfiResult::<()> {
                success: false,
                data: None,
                error: Some(format!("Invalid input JSON: {e}")),
            })
            .unwrap();
        }
    };

    let result = std::panic::catch_unwind(|| {
        let parsed_files: Vec<ParsedFile> = files
            .iter()
            .map(|f| parse_string(&f.content, &f.filename))
            .collect();
        resolve(&parsed_files, None)
    });

    match result {
        Ok(ast) => {
            let ffi_result = FfiResult {
                success: true,
                data: Some(ast),
                error: None,
            };
            serde_json::to_string(&ffi_result).unwrap_or_else(|e| {
                serde_json::to_string(&FfiResult::<()> {
                    success: false,
                    data: None,
                    error: Some(format!("JSON serialization error: {e}")),
                })
                .unwrap()
            })
        }
        Err(_) => serde_json::to_string(&FfiResult::<()> {
            success: false,
            data: None,
            error: Some("Internal parser panic".to_string()),
        })
        .unwrap(),
    }
}

/// Validate M3L content and return diagnostics as JSON.
///
/// Input: M3L markdown text + options JSON
/// Output: JSON string containing validation results
pub fn validate_to_json(content: &str, options_json: &str) -> String {
    let opts: ValidateJsonOptions = match serde_json::from_str(options_json) {
        Ok(o) => o,
        Err(e) => {
            return serde_json::to_string(&FfiResult::<()> {
                success: false,
                data: None,
                error: Some(format!("Invalid options JSON: {e}")),
            })
            .unwrap();
        }
    };

    let filename = if opts.filename.is_empty() {
        "input.m3l.md"
    } else {
        &opts.filename
    };

    let result = std::panic::catch_unwind(|| {
        let parsed = parse_string(content, filename);
        let ast = resolve(&[parsed], None);
        let validate_opts = ValidateOptions {
            strict: opts.strict,
        };
        validate(&ast, &validate_opts)
    });

    match result {
        Ok(validate_result) => {
            let ffi_result = FfiResult {
                success: true,
                data: Some(validate_result),
                error: None,
            };
            serde_json::to_string(&ffi_result).unwrap_or_else(|e| {
                serde_json::to_string(&FfiResult::<()> {
                    success: false,
                    data: None,
                    error: Some(format!("JSON serialization error: {e}")),
                })
                .unwrap()
            })
        }
        Err(_) => serde_json::to_string(&FfiResult::<()> {
            success: false,
            data: None,
            error: Some("Internal parser panic".to_string()),
        })
        .unwrap(),
    }
}

/// Validate a multi-file M3L model and return diagnostics as JSON.
///
/// Input: JSON array of `{ "content": "...", "filename": "..." }` objects + options JSON
/// Output: JSON string containing validation results
///
/// This is the validation counterpart of [`parse_multi_to_json`], and it exists for the same
/// reason: cross-file constructs — an inherited model, an interface reference, a custom
/// `::attribute` registered in one file and used in another — are only resolvable when the
/// whole file set is one resolve unit. Validating each file on its own reports what is missing
/// from that file rather than what is missing from the model, so a consumer that holds several
/// files has no correct way to assemble the single-file results.
///
/// Each diagnostic carries its own `file`, so callers do not have to attribute them by position.
pub fn validate_multi_to_json(files_json: &str, options_json: &str) -> String {
    let files: Vec<FileInput> = match serde_json::from_str(files_json) {
        Ok(f) => f,
        Err(e) => {
            return serde_json::to_string(&FfiResult::<()> {
                success: false,
                data: None,
                error: Some(format!("Invalid input JSON: {e}")),
            })
            .unwrap();
        }
    };

    let opts: ValidateJsonOptions = match serde_json::from_str(options_json) {
        Ok(o) => o,
        Err(e) => {
            return serde_json::to_string(&FfiResult::<()> {
                success: false,
                data: None,
                error: Some(format!("Invalid options JSON: {e}")),
            })
            .unwrap();
        }
    };

    let result = std::panic::catch_unwind(|| {
        let parsed_files: Vec<ParsedFile> = files
            .iter()
            .map(|f| parse_string(&f.content, &f.filename))
            .collect();
        let ast = resolve(&parsed_files, None);
        let validate_opts = ValidateOptions {
            strict: opts.strict,
        };
        validate(&ast, &validate_opts)
    });

    match result {
        Ok(validate_result) => {
            let ffi_result = FfiResult {
                success: true,
                data: Some(validate_result),
                error: None,
            };
            serde_json::to_string(&ffi_result).unwrap_or_else(|e| {
                serde_json::to_string(&FfiResult::<()> {
                    success: false,
                    data: None,
                    error: Some(format!("JSON serialization error: {e}")),
                })
                .unwrap()
            })
        }
        Err(_) => serde_json::to_string(&FfiResult::<()> {
            success: false,
            data: None,
            error: Some("Internal parser panic".to_string()),
        })
        .unwrap(),
    }
}
