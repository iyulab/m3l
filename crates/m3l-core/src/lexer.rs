use regex::Regex;
use std::sync::LazyLock;

use crate::catalogs::KIND_SECTIONS;
use crate::types::*;

// --- Regex patterns ---

static RE_H1: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^# (.+)$").unwrap());
static RE_H2: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^## (.+)$").unwrap());
static RE_H3: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^### (.+)$").unwrap());
static RE_HR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^-{3,}$").unwrap());
static RE_BLOCKQUOTE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\s*)> (.+)$").unwrap());
static RE_LIST_ITEM: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\s*)- (.+)$").unwrap());
static RE_BLANK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*$").unwrap());

// H2 sub-patterns
static RE_TYPE_INDICATOR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(@?[\w][\w.]*(?:\([^)]*\))?)\s*::(\w+)(.*)$").unwrap());
static RE_MODEL_DEF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^([\w][\w.]*(?:\([^)]*\))?)\s*(?::\s*(.+?))?(\s+@.+)?$").unwrap()
});

// Field line patterns
static RE_FIELD_NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([\w]+)(?:\(([^)]*)\))?\s*(?::\s*(.+))?$").unwrap());
static RE_TYPE_PART: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^([\w][\w.]*)(?:<([^>]+)>)?(?:\(([^)]*)\))?(\?)?(\[\])?(\?)?").unwrap()
});
static RE_FRAMEWORK_ATTR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"`\[([^\]]+)\]`").unwrap());
static RE_INLINE_COMMENT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+#\s+(.+)$").unwrap());

static RE_NAME_LABEL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([\w][\w.]*)\(([^)]*)\)$").unwrap());
static RE_NAMESPACE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^Namespace:\s*(.+)$").unwrap());
static RE_IMPORT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^@import\s+["'](.+?)["']\s*$"#).unwrap());
static RE_ENUM_VALUE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^([\w]+)(?:\(([^)]*)\))?\s+"((?:[^"\\]|\\.)*)"$"#).unwrap());
static RE_NESTED_KV: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([\w]+)\s*:\s*(.+)$").unwrap());
static RE_H2_INHERIT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^:\s*(.+?)(?:\s+@|\s*"|\s*$)"#).unwrap());
static RE_H2_DESC: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#""([^"]+)""#).unwrap());
static RE_MODEL_ATTR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"@([\w]+)(?:\(([^)]*)\))?").unwrap());

/// Tokenize M3L markdown content into a sequence of tokens.
pub fn lex(content: &str, _file: &str) -> Vec<Token> {
    let lines: Vec<&str> = content.split('\n').collect();
    let mut tokens: Vec<Token> = Vec::new();
    let total = lines.len();
    let mut i = 0;

    while i < total {
        let raw_line = lines[i];
        // Strip trailing \r for CRLF
        let raw = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        let line_num = i + 1;

        // Fenced code block detection
        let trimmed_for_fence = raw.trim_start();
        if let Some(after_fence) = trimmed_for_fence.strip_prefix("```") {
            let lang_hint = {
                let hint = after_fence.trim();
                if hint.is_empty() {
                    None
                } else {
                    Some(hint.to_string())
                }
            };
            let fence_indent = raw.len() - raw.trim_start().len();
            let mut code_lines: Vec<&str> = Vec::new();
            i += 1;
            while i < total {
                let next_raw = lines[i].strip_suffix('\r').unwrap_or(lines[i]);
                if next_raw.trim_start().starts_with("```") {
                    break;
                }
                code_lines.push(next_raw);
                i += 1;
            }
            // i now points at closing fence line

            let code_content = code_lines
                .iter()
                .map(|l| {
                    if l.len() > fence_indent {
                        &l[fence_indent..]
                    } else {
                        l.trim_start()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            let code_content = code_content.trim().to_string();

            // Attach to previous field or section token
            for j in (0..tokens.len()).rev() {
                let tt = tokens[j].token_type;
                if tt == TokenType::Field || tt == TokenType::Section {
                    tokens[j].data.code_block = Some(CodeBlock {
                        language: lang_hint.clone(),
                        content: code_content.clone(),
                    });
                    break;
                }
                if tt != TokenType::Blank {
                    break;
                }
            }

            i += 1;
            continue;
        }

        // Blank line
        if RE_BLANK.is_match(raw) {
            tokens.push(Token {
                token_type: TokenType::Blank,
                raw: raw.to_string(),
                line: line_num,
                indent: 0,
                data: TokenData::default(),
            });
            i += 1;
            continue;
        }

        // Horizontal rule
        if RE_HR.is_match(raw.trim()) {
            tokens.push(Token {
                token_type: TokenType::HorizontalRule,
                raw: raw.to_string(),
                line: line_num,
                indent: 0,
                data: TokenData::default(),
            });
            i += 1;
            continue;
        }

        // H3 — Section header
        if let Some(caps) = RE_H3.captures(raw) {
            let h3_name = caps[1].trim().to_string();
            let data = TokenData {
                kind_section: KIND_SECTIONS.contains(h3_name.as_str()),
                name: Some(h3_name),
                ..Default::default()
            };
            tokens.push(Token {
                token_type: TokenType::Section,
                raw: raw.to_string(),
                line: line_num,
                indent: 0,
                data,
            });
            i += 1;
            continue;
        }

        // H2 — Model/Enum/Interface/View
        if let Some(caps) = RE_H2.captures(raw) {
            let h2_content = caps[1].trim();
            tokens.push(tokenize_h2(h2_content, raw, line_num));
            i += 1;
            continue;
        }

        // H1 — Namespace
        if let Some(caps) = RE_H1.captures(raw) {
            let h1_content = caps[1].trim();
            let data = parse_namespace(h1_content);
            tokens.push(Token {
                token_type: TokenType::Namespace,
                raw: raw.to_string(),
                line: line_num,
                indent: 0,
                data,
            });
            i += 1;
            continue;
        }

        // Blockquote
        if let Some(caps) = RE_BLOCKQUOTE.captures(raw) {
            let bq_indent = caps[1].len();
            let bq_text = caps[2].trim().to_string();

            if bq_indent >= 2 {
                // Indented blockquote — attach to previous field token
                for j in (0..tokens.len()).rev() {
                    let tt = tokens[j].token_type;
                    if tt == TokenType::Field {
                        let existing = tokens[j].data.blockquote_desc.take();
                        tokens[j].data.blockquote_desc = Some(match existing {
                            Some(prev) => format!("{}\n{}", prev, bq_text),
                            None => bq_text.clone(),
                        });
                        break;
                    }
                    if tt != TokenType::Blank && tt != TokenType::Blockquote {
                        break;
                    }
                }
                i += 1;
                continue;
            }

            // Model-level blockquote
            let data = TokenData {
                name: Some(bq_text),
                ..Default::default()
            };
            tokens.push(Token {
                token_type: TokenType::Blockquote,
                raw: raw.to_string(),
                line: line_num,
                indent: 0,
                data,
            });
            i += 1;
            continue;
        }

        // List item
        if let Some(caps) = RE_LIST_ITEM.captures(raw) {
            let indent = caps[1].len();
            let item_content = &caps[2];

            if indent >= 2 {
                tokens.push(Token {
                    token_type: TokenType::NestedItem,
                    raw: raw.to_string(),
                    line: line_num,
                    indent,
                    data: parse_nested_item(item_content),
                });
            } else {
                tokens.push(Token {
                    token_type: TokenType::Field,
                    raw: raw.to_string(),
                    line: line_num,
                    indent: 0,
                    data: parse_field_line(item_content),
                });
            }
            i += 1;
            continue;
        }

        // @import directive
        let trimmed = raw.trim();
        if let Some(caps) = RE_IMPORT.captures(trimmed) {
            let data = TokenData {
                is_import: true,
                import_path: Some(caps[1].to_string()),
                name: Some(trimmed.to_string()),
                ..Default::default()
            };
            tokens.push(Token {
                token_type: TokenType::Text,
                raw: raw.to_string(),
                line: line_num,
                indent: 0,
                data,
            });
            i += 1;
            continue;
        }

        // Plain text
        let data = TokenData {
            name: Some(trimmed.to_string()),
            ..Default::default()
        };
        tokens.push(Token {
            token_type: TokenType::Text,
            raw: raw.to_string(),
            line: line_num,
            indent: 0,
            data,
        });
        i += 1;
    }

    tokens
}

#[allow(clippy::field_reassign_with_default)]
fn tokenize_h2(content: &str, raw: &str, line: usize) -> Token {
    // Check for type indicator: ## Name ::enum, ::interface, ::view, ::attribute
    if let Some(caps) = RE_TYPE_INDICATOR.captures(content) {
        let namepart = &caps[1];
        let type_indicator = &caps[2];
        let rest = caps.get(3).map(|m| m.as_str().trim()).unwrap_or("");

        let (name, label) = parse_name_label(namepart);
        let mut data = TokenData::default();
        data.name = Some(name);
        data.label = label;

        // Parse inheritance
        if let Some(inherit_caps) = RE_H2_INHERIT.captures(rest) {
            data.inherits = inherit_caps[1]
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }

        if type_indicator == "view" {
            data.materialized = Some(rest.contains("@materialized"));
        }

        // Extract description
        if let Some(desc_caps) = RE_H2_DESC.captures(rest) {
            data.description = Some(desc_caps[1].to_string());
        }

        let token_type = match type_indicator {
            "attribute" => TokenType::AttributeDef,
            "enum" => TokenType::Enum,
            "interface" => TokenType::Interface,
            "view" => TokenType::View,
            _ => TokenType::Model,
        };

        return Token {
            token_type,
            raw: raw.to_string(),
            line,
            indent: 0,
            data,
        };
    }

    // Regular model: ## Name : Parent1, Parent2
    if let Some(caps) = RE_MODEL_DEF.captures(content) {
        let namepart = &caps[1];
        let inherits_str = caps.get(2).map(|m| m.as_str().trim());
        let attrs_str = caps.get(3).map(|m| m.as_str().trim());

        let (name, label) = parse_name_label(namepart);
        let inherits = match inherits_str {
            Some(s) if !s.is_empty() => s
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            _ => Vec::new(),
        };

        let mut data = TokenData::default();
        data.name = Some(name);
        data.label = label;
        data.inherits = inherits;

        // Parse model-level attributes
        if let Some(attrs_s) = attrs_str {
            let mut attrs = Vec::new();
            for caps in RE_MODEL_ATTR.captures_iter(attrs_s) {
                let attr_name = caps[1].to_string();
                let args_str = caps.get(2).map(|m| m.as_str().to_string());
                let args = match args_str {
                    Some(s) if !s.is_empty() => parse_attr_args_string(&s),
                    _ => Vec::new(),
                };
                attrs.push(RawAttribute {
                    name: attr_name,
                    args,
                    cascade: None,
                });
            }
            data.attributes = attrs;
        }

        return Token {
            token_type: TokenType::Model,
            raw: raw.to_string(),
            line,
            indent: 0,
            data,
        };
    }

    // Fallback
    let data = TokenData {
        name: Some(content.to_string()),
        ..Default::default()
    };
    Token {
        token_type: TokenType::Model,
        raw: raw.to_string(),
        line,
        indent: 0,
        data,
    }
}

fn parse_name_label(s: &str) -> (String, Option<String>) {
    if let Some(caps) = RE_NAME_LABEL.captures(s) {
        (caps[1].to_string(), Some(caps[2].to_string()))
    } else {
        (s.to_string(), None)
    }
}

fn parse_namespace(content: &str) -> TokenData {
    let mut data = TokenData::default();
    if let Some(caps) = RE_NAMESPACE.captures(content) {
        data.name = Some(caps[1].trim().to_string());
        data.is_directive = true; // re-using for is_namespace
    } else {
        data.name = Some(content.to_string());
    }
    data
}

fn parse_field_line(content: &str) -> TokenData {
    let mut data = TokenData::default();

    // Check for attribute-only line: @meta(...), @index(...), @relation(...)
    if content.starts_with('@') {
        data.is_directive = true;
        data.attributes = parse_attributes_balanced(content);
        return data;
    }

    let mut content = content.to_string();

    // Strip inline comment
    if let Some(caps) = RE_INLINE_COMMENT.captures(&content) {
        data.comment = Some(caps[1].to_string());
        content = RE_INLINE_COMMENT.replace(&content, "").to_string();
    }

    // Extract framework attributes (backtick-wrapped)
    let mut framework_attrs = Vec::new();
    for caps in RE_FRAMEWORK_ATTR.captures_iter(&content) {
        framework_attrs.push(format!("[{}]", &caps[1]));
    }
    if !framework_attrs.is_empty() {
        data.framework_attrs = framework_attrs;
        content = RE_FRAMEWORK_ATTR
            .replace_all(&content, "")
            .trim()
            .to_string();
    }

    // Enum value with description: NAME "desc" or NAME(label) "desc"
    if let Some(caps) = RE_ENUM_VALUE.captures(&content) {
        data.name = Some(caps[1].to_string());
        if let Some(m) = caps.get(2) {
            data.label = Some(m.as_str().to_string());
        }
        data.description = Some(caps[3].to_string());
        return data;
    }

    // Parse name(label): type_and_rest
    let field_match = RE_FIELD_NAME.captures(&content);
    match field_match {
        None => {
            data.name = Some(content.to_string());
            return data;
        }
        Some(caps) => {
            data.name = Some(caps[1].to_string());
            if let Some(m) = caps.get(2) {
                data.label = Some(m.as_str().to_string());
            }

            match caps.get(3) {
                None => return data,
                Some(m) => {
                    let rest = m.as_str().trim();
                    if rest.is_empty() {
                        return data;
                    }
                    parse_type_and_attrs(rest, &mut data);
                }
            }
        }
    }

    data
}

/// Parse type, default value, attributes, and description from a field's rest string.
pub fn parse_type_and_attrs(rest: &str, data: &mut TokenData) {
    let bytes = rest.as_bytes();
    let len = bytes.len();
    let mut pos = 0;

    let skip_ws = |pos: &mut usize| {
        while *pos < len && bytes[*pos] == b' ' {
            *pos += 1;
        }
    };

    // Check if entire rest is a quoted string
    if !rest.is_empty() && bytes[0] == b'"' {
        let close_idx = find_closing_quote(rest, 0);
        if close_idx >= 0 && close_idx as usize == len - 1 {
            data.description = Some(rest[1..close_idx as usize].to_string());
            return;
        }
    }

    // Parse type: word<generics>?(params)?[]??
    if let Some(caps) = RE_TYPE_PART.captures(rest) {
        data.type_name = Some(caps[1].to_string());

        // Generic params <K,V>
        if let Some(m) = caps.get(2) {
            data.type_generic_params = m
                .as_str()
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
        }

        // Type params (n,m)
        if let Some(m) = caps.get(3) {
            data.type_params = m
                .as_str()
                .split(',')
                .map(|s| {
                    let s = s.trim();
                    if let Ok(n) = s.parse::<f64>() {
                        ParamValue::Number(n)
                    } else {
                        ParamValue::String(s.to_string())
                    }
                })
                .collect();
        }

        // Array
        data.array = caps.get(5).is_some_and(|m| m.as_str() == "[]");
        let q4 = caps.get(4).is_some_and(|m| m.as_str() == "?");
        let q6 = caps.get(6).is_some_and(|m| m.as_str() == "?");

        if data.array {
            data.nullable = q6;
            data.array_item_nullable = q4;
        } else {
            data.nullable = q4 || q6;
            data.array_item_nullable = false;
        }

        pos = caps[0].len();
        skip_ws(&mut pos);
    }

    // Parse default value: = "quoted" or = `backtick` or = unquoted
    if pos < len && bytes[pos] == b'=' {
        pos += 1;
        skip_ws(&mut pos);
        if pos < len && bytes[pos] == b'"' {
            let close_idx = find_closing_quote(rest, pos);
            if close_idx >= 0 {
                data.default_value = Some(rest[pos..=close_idx as usize].to_string());
                pos = close_idx as usize + 1;
                skip_ws(&mut pos);
            }
        } else if pos < len && bytes[pos] == b'`' {
            let close_idx = find_closing_backtick(rest, pos);
            if close_idx >= 0 {
                data.default_value = Some(rest[pos..=close_idx as usize].to_string());
                pos = close_idx as usize + 1;
                skip_ws(&mut pos);
            }
        } else {
            let start = pos;
            while pos < len
                && bytes[pos] != b' '
                && bytes[pos] != b'@'
                && bytes[pos] != b'"'
                && bytes[pos] != b'`'
            {
                if bytes[pos] == b'(' {
                    let close_p = find_balanced_paren(rest, pos);
                    pos = if close_p >= 0 {
                        close_p as usize + 1
                    } else {
                        pos + 1
                    };
                } else {
                    pos += 1;
                }
            }
            data.default_value = Some(rest[start..pos].to_string());
            skip_ws(&mut pos);
        }
    }

    // Parse attributes: @name or @name(args), with cascade symbols
    let mut attrs: Vec<RawAttribute> = Vec::new();
    while pos < len && (bytes[pos] == b'@' || bytes[pos] == b'!' || bytes[pos] == b'?') {
        // Cascade symbols
        if bytes[pos] == b'!' || bytes[pos] == b'?' {
            let mut symbol = String::from(bytes[pos] as char);
            pos += 1;
            if symbol == "!" && pos < len && bytes[pos] == b'!' {
                symbol = "!!".to_string();
                pos += 1;
            }
            if let Some(last) = attrs.last_mut() {
                last.cascade = Some(symbol);
            }
            skip_ws(&mut pos);
            continue;
        }

        pos += 1; // skip @
        let name_start = pos;
        while pos < len && is_word_char(bytes[pos]) {
            pos += 1;
        }
        let attr_name = rest[name_start..pos].to_string();
        let mut args = Vec::new();
        if pos < len && bytes[pos] == b'(' {
            let close_p = find_balanced_paren(rest, pos);
            if close_p >= 0 {
                let args_str = &rest[pos + 1..close_p as usize];
                args = parse_attr_args_string(args_str);
                pos = close_p as usize + 1;
            }
        }
        attrs.push(RawAttribute {
            name: attr_name,
            args,
            cascade: None,
        });
        skip_ws(&mut pos);
    }
    if !attrs.is_empty() {
        data.attributes = attrs;
    }

    // Trailing description
    skip_ws(&mut pos);
    if pos < len && bytes[pos] == b'"' {
        let close_idx = find_closing_quote(rest, pos);
        if close_idx >= 0 {
            data.description = Some(rest[pos + 1..close_idx as usize].to_string());
        }
    }
}

fn find_balanced_paren(s: &str, open_pos: usize) -> i32 {
    let bytes = s.as_bytes();
    let mut depth = 0i32;
    let mut i = open_pos;
    while i < bytes.len() {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return i as i32;
                }
            }
            b'"' => {
                let close_q = find_closing_quote(s, i);
                if close_q >= 0 {
                    i = close_q as usize;
                }
            }
            b'\'' => {
                if let Some(close_q) = s[i + 1..].find('\'') {
                    i = i + 1 + close_q;
                }
            }
            b'`' => {
                let close_q = find_closing_backtick(s, i);
                if close_q >= 0 {
                    i = close_q as usize;
                }
            }
            _ => {}
        }
        i += 1;
    }
    -1
}

fn find_closing_quote(s: &str, open_pos: usize) -> i32 {
    let bytes = s.as_bytes();
    let mut i = open_pos + 1;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if bytes[i] == b'"' {
            return i as i32;
        }
        i += 1;
    }
    -1
}

fn find_closing_backtick(s: &str, open_pos: usize) -> i32 {
    let bytes = s.as_bytes();
    let mut i = open_pos + 1;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if bytes[i] == b'`' {
            return i as i32;
        }
        i += 1;
    }
    -1
}

fn parse_attributes_balanced(content: &str) -> Vec<RawAttribute> {
    let bytes = content.as_bytes();
    let len = bytes.len();
    let mut attrs = Vec::new();
    let mut pos = 0;
    while pos < len {
        match content[pos..].find('@') {
            None => break,
            Some(offset) => pos += offset + 1,
        }
        let name_start = pos;
        while pos < len && is_word_char(bytes[pos]) {
            pos += 1;
        }
        let name = content[name_start..pos].to_string();
        if name.is_empty() {
            continue;
        }
        let mut args = Vec::new();
        if pos < len && bytes[pos] == b'(' {
            let close_p = find_balanced_paren(content, pos);
            if close_p >= 0 {
                let args_str = &content[pos + 1..close_p as usize];
                args = parse_attr_args_string(args_str);
                pos = close_p as usize + 1;
            }
        }
        attrs.push(RawAttribute {
            name,
            args,
            cascade: None,
        });
    }
    attrs
}

fn parse_nested_item(content: &str) -> TokenData {
    let mut data = TokenData::default();

    if let Some(caps) = RE_NESTED_KV.captures(content) {
        data.key = Some(caps[1].to_string());
        data.value = Some(caps[2].trim().to_string());
    }

    // Also try to parse as field line for sub-fields
    let field_data = parse_field_line(content);
    if data.key.is_none() || data.name.is_none() {
        data.name = field_data.name;
    }
    // Carry over type info if present from parse_field_line on raw_content
    if field_data.type_name.is_some() && data.key.is_none() {
        data.type_name = field_data.type_name;
        data.type_params = field_data.type_params;
        data.type_generic_params = field_data.type_generic_params;
        data.nullable = field_data.nullable;
        data.array = field_data.array;
        data.array_item_nullable = field_data.array_item_nullable;
        data.default_value = field_data.default_value;
        data.attributes = field_data.attributes;
        data.description = field_data.description;
        data.framework_attrs = field_data.framework_attrs;
        data.label = field_data.label;
        data.comment = field_data.comment;
    }

    data
}

/// Parse a comma-separated attribute args string into AttrArgValue vec.
/// Handles quoted strings, backtick expressions, numbers, booleans, and plain strings.
pub fn parse_attr_args_string(s: &str) -> Vec<AttrArgValue> {
    let mut args = Vec::new();
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut pos = 0;

    while pos < len {
        // Skip whitespace and commas
        while pos < len && (bytes[pos] == b' ' || bytes[pos] == b',') {
            pos += 1;
        }
        if pos >= len {
            break;
        }

        if bytes[pos] == b'"' {
            let close = find_closing_quote(s, pos);
            if close >= 0 {
                args.push(AttrArgValue::String(s[pos + 1..close as usize].to_string()));
                pos = close as usize + 1;
            } else {
                pos += 1;
            }
        } else if bytes[pos] == b'`' {
            let close = find_closing_backtick(s, pos);
            if close >= 0 {
                // Include backticks as part of the value
                args.push(AttrArgValue::String(s[pos..=close as usize].to_string()));
                pos = close as usize + 1;
            } else {
                pos += 1;
            }
        } else if bytes[pos] == b'\'' {
            // Single-quoted string
            if let Some(close) = s[pos + 1..].find('\'') {
                args.push(AttrArgValue::String(
                    s[pos + 1..pos + 1 + close].to_string(),
                ));
                pos = pos + 2 + close;
            } else {
                pos += 1;
            }
        } else {
            // Unquoted: read until comma or end, handle balanced parens
            let start = pos;
            while pos < len && bytes[pos] != b',' {
                if bytes[pos] == b'(' {
                    let close_p = find_balanced_paren(s, pos);
                    pos = if close_p >= 0 {
                        close_p as usize + 1
                    } else {
                        pos + 1
                    };
                } else {
                    pos += 1;
                }
            }
            let token = s[start..pos].trim();
            if !token.is_empty() {
                // Check for key: value patterns (e.g., "platform: postgresql")
                if let Some(colon_pos) = token.find(':') {
                    let key = token[..colon_pos].trim();
                    let val = token[colon_pos + 1..].trim().trim_matches('"');
                    args.push(AttrArgValue::String(format!("{}: {}", key, val)));
                } else if token == "true" {
                    args.push(AttrArgValue::Bool(true));
                } else if token == "false" {
                    args.push(AttrArgValue::Bool(false));
                } else if let Ok(n) = token.parse::<f64>() {
                    args.push(AttrArgValue::Number(n));
                } else {
                    args.push(AttrArgValue::String(token.to_string()));
                }
            }
        }
    }
    args
}

#[inline]
fn is_word_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lex_empty() {
        let tokens = lex("", "test.m3l.md");
        assert_eq!(tokens.len(), 1); // one blank line
        assert_eq!(tokens[0].token_type, TokenType::Blank);
    }

    #[test]
    fn lex_namespace() {
        let tokens = lex("# Namespace: sample.ecommerce", "test.m3l.md");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token_type, TokenType::Namespace);
        assert_eq!(tokens[0].data.name.as_deref(), Some("sample.ecommerce"));
        assert!(tokens[0].data.is_directive); // is_namespace
    }

    #[test]
    fn lex_model() {
        let tokens = lex("## User : BaseModel", "test.m3l.md");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token_type, TokenType::Model);
        assert_eq!(tokens[0].data.name.as_deref(), Some("User"));
        assert_eq!(tokens[0].data.inherits, vec!["BaseModel"]);
    }

    #[test]
    fn lex_enum_type_indicator() {
        let tokens = lex("## Status ::enum", "test.m3l.md");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token_type, TokenType::Enum);
        assert_eq!(tokens[0].data.name.as_deref(), Some("Status"));
    }

    #[test]
    fn lex_field() {
        let tokens = lex("- email: string @required", "test.m3l.md");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token_type, TokenType::Field);
        assert_eq!(tokens[0].data.name.as_deref(), Some("email"));
        assert_eq!(tokens[0].data.type_name.as_deref(), Some("string"));
        assert_eq!(tokens[0].data.attributes.len(), 1);
        assert_eq!(tokens[0].data.attributes[0].name, "required");
    }

    #[test]
    fn lex_section() {
        let tokens = lex("### Indexes", "test.m3l.md");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token_type, TokenType::Section);
        assert_eq!(tokens[0].data.name.as_deref(), Some("Indexes"));
        assert!(!tokens[0].data.kind_section);
    }

    #[test]
    fn lex_kind_section() {
        let tokens = lex("### Lookup", "test.m3l.md");
        assert_eq!(tokens[0].token_type, TokenType::Section);
        assert!(tokens[0].data.kind_section);
    }

    #[test]
    fn lex_nested_item() {
        let tokens = lex("  - key: value", "test.m3l.md");
        assert_eq!(tokens[0].token_type, TokenType::NestedItem);
        assert_eq!(tokens[0].data.key.as_deref(), Some("key"));
        assert_eq!(tokens[0].data.value.as_deref(), Some("value"));
    }

    #[test]
    fn lex_horizontal_rule() {
        let tokens = lex("---", "test.m3l.md");
        assert_eq!(tokens[0].token_type, TokenType::HorizontalRule);
    }

    #[test]
    fn lex_blockquote() {
        let tokens = lex("> Model description", "test.m3l.md");
        assert_eq!(tokens[0].token_type, TokenType::Blockquote);
    }

    #[test]
    fn lex_field_with_default() {
        let tokens = lex("- status: string = \"active\"", "test.m3l.md");
        assert_eq!(tokens[0].data.type_name.as_deref(), Some("string"));
        assert_eq!(tokens[0].data.default_value.as_deref(), Some("\"active\""));
    }

    #[test]
    fn lex_field_nullable_array() {
        let tokens = lex("- tags: string?[]?", "test.m3l.md");
        let d = &tokens[0].data;
        assert_eq!(d.type_name.as_deref(), Some("string"));
        assert!(d.array);
        assert!(d.nullable);
        assert!(d.array_item_nullable);
    }

    #[test]
    fn lex_code_block() {
        let input = "- computed_field: decimal @computed\n```sql\nSELECT 1\n```";
        let tokens = lex(input, "test.m3l.md");
        let field = &tokens[0];
        assert_eq!(field.token_type, TokenType::Field);
        assert!(field.data.code_block.is_some());
        let cb = field.data.code_block.as_ref().unwrap();
        assert_eq!(cb.language.as_deref(), Some("sql"));
        assert_eq!(cb.content, "SELECT 1");
    }

    #[test]
    fn lex_model_with_label() {
        let tokens = lex("## User(Users)", "test.m3l.md");
        assert_eq!(tokens[0].data.name.as_deref(), Some("User"));
        assert_eq!(tokens[0].data.label.as_deref(), Some("Users"));
    }

    #[test]
    fn lex_view_indicator() {
        let tokens = lex("## SalesSummary ::view @materialized", "test.m3l.md");
        assert_eq!(tokens[0].token_type, TokenType::View);
        assert_eq!(tokens[0].data.materialized, Some(true));
    }

    #[test]
    fn lex_attribute_def() {
        let tokens = lex("## custom_flag ::attribute", "test.m3l.md");
        assert_eq!(tokens[0].token_type, TokenType::AttributeDef);
        assert_eq!(tokens[0].data.name.as_deref(), Some("custom_flag"));
    }

    #[test]
    fn lex_import() {
        let tokens = lex("@import \"base.m3l.md\"", "test.m3l.md");
        assert_eq!(tokens[0].token_type, TokenType::Text);
        assert!(tokens[0].data.is_import);
        assert_eq!(tokens[0].data.import_path.as_deref(), Some("base.m3l.md"));
    }

    #[test]
    fn parse_type_and_attrs_basic() {
        let mut data = TokenData::default();
        parse_type_and_attrs("string @required @unique", &mut data);
        assert_eq!(data.type_name.as_deref(), Some("string"));
        assert!(!data.nullable);
        assert_eq!(data.attributes.len(), 2);
        assert_eq!(data.attributes[0].name, "required");
        assert_eq!(data.attributes[1].name, "unique");
    }

    #[test]
    fn parse_type_and_attrs_with_params() {
        let mut data = TokenData::default();
        parse_type_and_attrs("decimal(10,2) @min(0)", &mut data);
        assert_eq!(data.type_name.as_deref(), Some("decimal"));
        assert_eq!(data.type_params.len(), 2);
        assert_eq!(data.attributes.len(), 1);
        assert_eq!(data.attributes[0].name, "min");
    }

    #[test]
    fn parse_type_and_attrs_generics() {
        let mut data = TokenData::default();
        parse_type_and_attrs("map<string,integer>", &mut data);
        assert_eq!(data.type_name.as_deref(), Some("map"));
        assert_eq!(data.type_generic_params, vec!["string", "integer"]);
    }

    #[test]
    fn parse_type_and_attrs_cascade() {
        let mut data = TokenData::default();
        parse_type_and_attrs("identifier @reference(Order) !", &mut data);
        assert_eq!(data.attributes.len(), 1);
        assert_eq!(data.attributes[0].cascade.as_deref(), Some("!"));
    }

    #[test]
    fn parse_type_and_attrs_backtick_default() {
        let mut data = TokenData::default();
        parse_type_and_attrs("decimal @computed(`price * qty`)", &mut data);
        assert_eq!(data.type_name.as_deref(), Some("decimal"));
        assert_eq!(data.attributes.len(), 1);
        assert_eq!(data.attributes[0].name, "computed");
    }

    #[test]
    fn parse_attr_args_mixed() {
        let args = parse_attr_args_string("\"hello\", 42, true");
        assert_eq!(args.len(), 3);
        assert_eq!(args[0], AttrArgValue::String("hello".into()));
        assert_eq!(args[1], AttrArgValue::Number(42.0));
        assert_eq!(args[2], AttrArgValue::Bool(true));
    }
}
