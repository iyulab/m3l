//! C ABI bindings for the M3L parser.
//!
//! All functions use C-compatible string types:
//! - Input: `*const c_char` (null-terminated C string)
//! - Output: `*mut c_char` (caller must free with `m3l_free_string`)
//!
//! This crate builds as a cdylib for use via P/Invoke (C#), ctypes (Python), etc.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use m3l_core::{parse_multi_to_json, parse_to_json, validate_to_json};

/// Parse a single M3L file and return the AST as JSON.
///
/// # Safety
/// - `content` must be a valid null-terminated UTF-8 string.
/// - `filename` must be a valid null-terminated UTF-8 string.
/// - The returned pointer must be freed with `m3l_free_string`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn m3l_parse(content: *const c_char, filename: *const c_char) -> *mut c_char {
    let content = unsafe { CStr::from_ptr(content) };
    let filename = unsafe { CStr::from_ptr(filename) };

    let content_str = match content.to_str() {
        Ok(s) => s,
        Err(_) => return to_c_string(r#"{"success":false,"error":"Invalid UTF-8 in content"}"#),
    };
    let filename_str = match filename.to_str() {
        Ok(s) => s,
        Err(_) => return to_c_string(r#"{"success":false,"error":"Invalid UTF-8 in filename"}"#),
    };

    let result = parse_to_json(content_str, filename_str);
    to_c_string(&result)
}

/// Parse multiple M3L files and return the merged AST as JSON.
///
/// # Safety
/// - `files_json` must be a valid null-terminated UTF-8 JSON string.
/// - The returned pointer must be freed with `m3l_free_string`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn m3l_parse_multi(files_json: *const c_char) -> *mut c_char {
    let files_json = unsafe { CStr::from_ptr(files_json) };

    let json_str = match files_json.to_str() {
        Ok(s) => s,
        Err(_) => return to_c_string(r#"{"success":false,"error":"Invalid UTF-8 in files_json"}"#),
    };

    let result = parse_multi_to_json(json_str);
    to_c_string(&result)
}

/// Validate M3L content and return diagnostics as JSON.
///
/// # Safety
/// - `content` must be a valid null-terminated UTF-8 string.
/// - `options_json` must be a valid null-terminated UTF-8 JSON string.
/// - The returned pointer must be freed with `m3l_free_string`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn m3l_validate(
    content: *const c_char,
    options_json: *const c_char,
) -> *mut c_char {
    let content = unsafe { CStr::from_ptr(content) };
    let options_json = unsafe { CStr::from_ptr(options_json) };

    let content_str = match content.to_str() {
        Ok(s) => s,
        Err(_) => return to_c_string(r#"{"success":false,"error":"Invalid UTF-8 in content"}"#),
    };
    let options_str = match options_json.to_str() {
        Ok(s) => s,
        Err(_) => {
            return to_c_string(r#"{"success":false,"error":"Invalid UTF-8 in options_json"}"#)
        }
    };

    let result = validate_to_json(content_str, options_str);
    to_c_string(&result)
}

/// Free a string previously returned by m3l_parse, m3l_parse_multi, or m3l_validate.
///
/// # Safety
/// - `ptr` must be a pointer previously returned by one of the m3l_* functions,
///   or null (in which case this is a no-op).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn m3l_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(unsafe { CString::from_raw(ptr) });
    }
}

/// Helper: Convert a Rust string to a C-compatible heap-allocated string.
fn to_c_string(s: &str) -> *mut c_char {
    CString::new(s)
        .unwrap_or_else(|_| CString::new("").unwrap())
        .into_raw()
}
