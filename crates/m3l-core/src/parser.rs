#![allow(clippy::collapsible_match)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::field_reassign_with_default)]

use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

use crate::catalogs::STANDARD_ATTRIBUTES;
use crate::lexer::{lex, parse_type_and_attrs};
use crate::types::*;

static RE_QUOTE_STR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"^"(.*)"$"#).unwrap());
static RE_CUSTOM_ATTR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([A-Za-z_][\w.]*)(?:\((.+)\))?$").unwrap());
static RE_AGG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\w+)(?:\((\w+)\))?$").unwrap());
static RE_WHERE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"^where:\s*"(.*)"$"#).unwrap());
static RE_PLATFORM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"platform\s*:\s*["']?([^"'\s]+)["']?"#).unwrap());

// --- Parser state ---

enum CurrentElement {
    Model(Box<ModelNode>),
    Enum(EnumNode),
    None,
}

struct AttrDef {
    name: String,
    description: Option<String>,
    fields: HashMap<String, String>,
}

struct ParserState {
    file: String,
    namespace: Option<String>,
    current_element: CurrentElement,
    current_section: Option<String>,
    current_kind: FieldKind,
    last_field_idx: Option<usize>, // index into current model's fields
    models: Vec<ModelNode>,
    enums: Vec<EnumNode>,
    interfaces: Vec<ModelNode>,
    views: Vec<ModelNode>,
    attribute_registry: Vec<AttributeRegistryEntry>,
    current_attr_def: Option<AttrDef>,
    source_directives_done: bool,
    imports: Vec<String>,
}

/// Parse M3L content string into a ParsedFile AST.
pub fn parse_string(content: &str, file: &str) -> ParsedFile {
    let tokens = lex(content, file);
    parse_tokens(&tokens, file)
}

/// Parse a token sequence into a ParsedFile AST.
pub fn parse_tokens(tokens: &[Token], file: &str) -> ParsedFile {
    let mut state = ParserState {
        file: file.to_string(),
        namespace: None,
        current_element: CurrentElement::None,
        current_section: None,
        current_kind: FieldKind::Stored,
        last_field_idx: None,
        models: Vec::new(),
        enums: Vec::new(),
        interfaces: Vec::new(),
        views: Vec::new(),
        attribute_registry: Vec::new(),
        current_attr_def: None,
        source_directives_done: false,
        imports: Vec::new(),
    };

    for token in tokens {
        process_token(token, &mut state);
    }

    finalize_element(&mut state);

    ParsedFile {
        source: file.to_string(),
        namespace: state.namespace,
        models: state.models,
        enums: state.enums,
        interfaces: state.interfaces,
        views: state.views,
        attribute_registry: state.attribute_registry,
        imports: state.imports,
    }
}

fn process_token(token: &Token, state: &mut ParserState) {
    match token.token_type {
        TokenType::Namespace => handle_namespace(token, state),
        TokenType::Model | TokenType::Interface => handle_model_start(token, state),
        TokenType::Enum => handle_enum_start(token, state),
        TokenType::View => handle_view_start(token, state),
        TokenType::AttributeDef => handle_attribute_def_start(token, state),
        TokenType::Section => handle_section(token, state),
        TokenType::Field => handle_field(token, state),
        TokenType::NestedItem => handle_nested_item(token, state),
        TokenType::Blockquote => handle_blockquote(token, state),
        TokenType::Text => handle_text(token, state),
        TokenType::HorizontalRule | TokenType::Blank => {}
    }
}

fn handle_namespace(token: &Token, state: &mut ParserState) {
    if matches!(state.current_element, CurrentElement::None) {
        state.namespace = token.data.name.clone();
    }
}

fn handle_model_start(token: &Token, state: &mut ParserState) {
    finalize_element(state);

    let model_attrs = parse_raw_attributes(&token.data.attributes);
    let model_type = if token.token_type == TokenType::Interface {
        ModelType::Interface
    } else {
        ModelType::Model
    };

    let model = ModelNode {
        name: token.data.name.clone().unwrap_or_default(),
        label: token.data.label.clone(),
        model_type,
        source: state.file.clone(),
        line: token.line,
        inherits: token.data.inherits.clone(),
        description: None,
        attributes: model_attrs,
        fields: Vec::new(),
        sections: Sections::default(),
        materialized: None,
        source_def: None,
        refresh: None,
        loc: SourceLocation {
            file: state.file.clone(),
            line: token.line,
            col: 1,
        },
    };

    state.current_element = CurrentElement::Model(Box::new(model));
    state.current_section = None;
    state.current_kind = FieldKind::Stored;
    state.last_field_idx = None;
    state.source_directives_done = false;
}

fn handle_enum_start(token: &Token, state: &mut ParserState) {
    finalize_element(state);

    let enum_node = EnumNode {
        name: token.data.name.clone().unwrap_or_default(),
        label: token.data.label.clone(),
        enum_type: ModelType::Enum,
        source: state.file.clone(),
        line: token.line,
        inherits: token.data.inherits.clone(),
        description: token.data.description.clone(),
        values: Vec::new(),
        loc: SourceLocation {
            file: state.file.clone(),
            line: token.line,
            col: 1,
        },
    };

    state.current_element = CurrentElement::Enum(enum_node);
    state.current_section = None;
    state.current_kind = FieldKind::Stored;
    state.last_field_idx = None;
}

fn handle_view_start(token: &Token, state: &mut ParserState) {
    finalize_element(state);

    let materialized = token.data.materialized.unwrap_or(false);
    let view = ModelNode {
        name: token.data.name.clone().unwrap_or_default(),
        label: token.data.label.clone(),
        model_type: ModelType::View,
        source: state.file.clone(),
        line: token.line,
        inherits: Vec::new(),
        description: None,
        attributes: Vec::new(),
        materialized: Some(materialized),
        fields: Vec::new(),
        sections: Sections::default(),
        source_def: None,
        refresh: None,
        loc: SourceLocation {
            file: state.file.clone(),
            line: token.line,
            col: 1,
        },
    };

    state.current_element = CurrentElement::Model(Box::new(view));
    state.current_section = None;
    state.current_kind = FieldKind::Stored;
    state.last_field_idx = None;
    state.source_directives_done = false;
}

fn handle_section(token: &Token, state: &mut ParserState) {
    let section_name = token.data.name.clone().unwrap_or_default();

    // Kind-context sections
    if token.data.kind_section {
        if matches!(state.current_element, CurrentElement::None) {
            return;
        }
        let lower = section_name.to_lowercase();
        if lower.starts_with("lookup") {
            state.current_kind = FieldKind::Lookup;
        } else if lower.starts_with("rollup") {
            state.current_kind = FieldKind::Rollup;
        } else if lower.starts_with("computed") {
            state.current_kind = FieldKind::Computed;
        }
        state.current_section = None;
        state.last_field_idx = None;
        return;
    }

    state.current_section = Some(section_name.clone());
    state.last_field_idx = None;

    // View Source SQL block
    if section_name == "Source" {
        if let CurrentElement::Model(ref mut model) = state.current_element {
            if model.model_type == ModelType::View {
                state.source_directives_done = false;
                if let Some(ref cb) = token.data.code_block {
                    let sd = model.source_def.get_or_insert(ViewSourceDef {
                        from: None,
                        joins: None,
                        where_clause: None,
                        order_by: None,
                        group_by: None,
                        raw_sql: None,
                        language_hint: None,
                    });
                    sd.raw_sql = Some(cb.content.clone());
                    sd.language_hint = cb.language.clone();
                }
            }
        }
    }
}

fn handle_field(token: &Token, state: &mut ParserState) {
    // Handle attribute definition fields
    if let Some(ref mut attr_def) = state.current_attr_def {
        let name = token.data.name.clone().unwrap_or_default();
        let raw = token.raw.trim().trim_start_matches("- ");
        if let Some(colon_idx) = raw.find(':') {
            let value = raw[colon_idx + 1..].trim().to_string();
            attr_def.fields.insert(name, value);
        }
        return;
    }

    match state.current_element {
        CurrentElement::None => (),
        CurrentElement::Enum(ref mut enum_node) => {
            let mut enum_val = EnumValue {
                name: token.data.name.clone().unwrap_or_default(),
                description: token.data.description.clone(),
                value_type: None,
                value: None,
            };

            if let Some(ref type_name) = token.data.type_name {
                if type_name != "enum" {
                    // Check if it's really a quoted description
                    if let Some(caps) = RE_QUOTE_STR.captures(type_name) {
                        enum_val.description = Some(caps[1].to_string());
                    } else {
                        enum_val.value_type = Some(type_name.clone());
                    }
                }
            }
            if let Some(ref dv) = token.data.default_value {
                enum_val.value = Some(serde_json::Value::String(dv.clone()));
            }
            // If no description from data but type looks like a quoted string
            if enum_val.description.is_none() {
                if let Some(ref tn) = token.data.type_name {
                    if let Some(caps) = RE_QUOTE_STR.captures(tn) {
                        enum_val.description = Some(caps[1].to_string());
                        enum_val.value_type = None;
                    }
                }
            }

            enum_node.values.push(enum_val);
        }
        CurrentElement::Model(ref mut model) => {
            // Directive-only lines
            if token.data.is_directive {
                handle_directive(&token.data, model, token, &state.file);
                return;
            }

            // Section-specific items
            if let Some(ref section) = state.current_section {
                let section = section.clone();
                handle_section_item(
                    &token.data,
                    model,
                    token,
                    &state.file,
                    &section,
                    &state.current_kind,
                    &mut state.source_directives_done,
                    &mut state.last_field_idx,
                );
                return;
            }

            // Regular field
            let field = build_field_node(&token.data, token, &state.file, &state.current_kind);
            model.fields.push(field);
            state.last_field_idx = Some(model.fields.len() - 1);
        }
    }
}

fn handle_directive(data: &TokenData, model: &mut ModelNode, token: &Token, file: &str) {
    if data.attributes.is_empty() {
        return;
    }

    let attr = &data.attributes[0];
    let loc = serde_json::json!({
        "file": file,
        "line": token.line,
        "col": 1
    });

    let raw_content = token.raw.trim().to_string();
    let args_val = if !attr.args.is_empty() {
        Some(attr_args_to_json(&attr.args))
    } else {
        None
    };

    if attr.name == "index" || attr.name == "unique" {
        let mut entry = serde_json::Map::new();
        entry.insert("type".into(), serde_json::json!("directive"));
        entry.insert("raw".into(), serde_json::json!(raw_content));
        if let Some(ref a) = args_val {
            entry.insert("args".into(), a.clone());
        }
        entry.insert("unique".into(), serde_json::json!(attr.name == "unique"));
        entry.insert("loc".into(), loc);
        model
            .sections
            .indexes
            .push(serde_json::Value::Object(entry));
    } else if attr.name == "relation" {
        let mut entry = serde_json::Map::new();
        entry.insert("type".into(), serde_json::json!("directive"));
        entry.insert("raw".into(), serde_json::json!(raw_content));
        if let Some(ref a) = args_val {
            entry.insert("args".into(), a.clone());
        }
        entry.insert("loc".into(), loc);
        model
            .sections
            .relations
            .push(serde_json::Value::Object(entry));
    } else {
        let mut section_name = attr.name.clone();
        if section_name == "behavior" {
            section_name = "behaviors".to_string();
        }

        let mut entry = serde_json::Map::new();
        entry.insert("raw".into(), serde_json::json!(raw_content));
        if let Some(ref a) = args_val {
            entry.insert("args".into(), a.clone());
        }

        if section_name == "behaviors" {
            model
                .sections
                .behaviors
                .push(serde_json::Value::Object(entry));
        } else {
            let section = model
                .sections
                .custom
                .entry(section_name)
                .or_insert_with(|| serde_json::json!([]));
            if let serde_json::Value::Array(ref mut arr) = section {
                arr.push(serde_json::Value::Object(entry));
            }
        }
    }
}

fn handle_section_item(
    data: &TokenData,
    model: &mut ModelNode,
    token: &Token,
    file: &str,
    section: &str,
    current_kind: &FieldKind,
    source_directives_done: &mut bool,
    last_field_idx: &mut Option<usize>,
) {
    let loc = serde_json::json!({
        "file": file,
        "line": token.line,
        "col": 1
    });

    // View Source section
    if section == "Source" && model.model_type == ModelType::View {
        let name = data.name.clone().unwrap_or_default();

        if is_source_directive(&name) && !*source_directives_done {
            let sd = model.source_def.get_or_insert(ViewSourceDef {
                from: None,
                joins: None,
                where_clause: None,
                order_by: None,
                group_by: None,
                raw_sql: None,
                language_hint: None,
            });
            set_source_directive(sd, data);
            return;
        }

        *source_directives_done = true;
        let field = build_field_node(data, token, file, current_kind);
        model.fields.push(field);
        *last_field_idx = Some(model.fields.len() - 1);
        return;
    }

    // Refresh section
    if section == "Refresh" && model.model_type == ModelType::View {
        let refresh = model.refresh.get_or_insert(RefreshDef {
            strategy: String::new(),
            interval: None,
        });
        let name = data.name.clone().unwrap_or_default();
        let type_name = data.type_name.clone();
        let desc = data.description.clone();
        if name == "strategy" {
            refresh.strategy = type_name.unwrap_or_default();
        } else if name == "interval" {
            refresh.interval = Some(desc.or(type_name).unwrap_or_default());
        }
        return;
    }

    // Indexes section
    if section == "Indexes" {
        let mut entry = serde_json::Map::new();
        entry.insert(
            "name".into(),
            serde_json::json!(data.name.clone().unwrap_or_default()),
        );
        if let Some(ref label) = data.label {
            entry.insert("label".into(), serde_json::json!(label));
        }
        entry.insert("loc".into(), loc);
        model
            .sections
            .indexes
            .push(serde_json::Value::Object(entry));
        *last_field_idx = Some(usize::MAX); // sentinel for index
        return;
    }

    // Relations section
    if section == "Relations" {
        let raw = token.raw.trim().trim_start_matches("- ").to_string();
        let mut entry = serde_json::Map::new();
        entry.insert("raw".into(), serde_json::json!(raw));
        entry.insert("loc".into(), loc);
        model
            .sections
            .relations
            .push(serde_json::Value::Object(entry));
        *last_field_idx = Some(usize::MAX); // sentinel
        return;
    }

    // Metadata section
    if section == "Metadata" {
        let name = data.name.clone().unwrap_or_default();
        let value = data
            .type_name
            .clone()
            .or_else(|| data.description.clone())
            .unwrap_or_default();
        model
            .sections
            .metadata
            .insert(name, parse_metadata_value(&value));
        return;
    }

    // Behaviors section
    if section == "Behaviors" {
        let mut entry = serde_json::Map::new();
        entry.insert(
            "name".into(),
            serde_json::json!(data.name.clone().unwrap_or_default()),
        );
        entry.insert("raw".into(), serde_json::json!(token.raw.trim()));
        entry.insert("loc".into(), loc);
        model
            .sections
            .behaviors
            .push(serde_json::Value::Object(entry));
        return;
    }

    // Generic section
    let mut entry = serde_json::Map::new();
    entry.insert(
        "name".into(),
        serde_json::json!(data.name.clone().unwrap_or_default()),
    );
    entry.insert("raw".into(), serde_json::json!(token.raw.trim()));
    let value_str = data.type_name.clone().or_else(|| data.description.clone());
    if let Some(v) = value_str {
        entry.insert("value".into(), serde_json::json!(v));
    }
    entry.insert("loc".into(), loc);

    let section_arr = model
        .sections
        .custom
        .entry(section.to_string())
        .or_insert_with(|| serde_json::json!([]));
    if let serde_json::Value::Array(ref mut arr) = section_arr {
        arr.push(serde_json::Value::Object(entry));
    }
    *last_field_idx = None;
}

fn handle_nested_item(token: &Token, state: &mut ParserState) {
    let data = &token.data;
    let key = data.key.as_deref();
    let value = data.value.as_deref();

    match state.current_element {
        CurrentElement::None => (),
        CurrentElement::Enum(ref mut enum_node) => {
            if let Some(k) = key {
                let mut val = EnumValue {
                    name: k.to_string(),
                    description: None,
                    value_type: None,
                    value: None,
                };
                if let Some(v) = value {
                    if let Some(caps) = RE_QUOTE_STR.captures(v) {
                        val.description = Some(caps[1].to_string());
                    } else {
                        val.value = Some(serde_json::Value::String(v.to_string()));
                    }
                }
                enum_node.values.push(val);
            }
        }
        CurrentElement::Model(ref mut model) => {
            // Nested items under index
            if state.current_section.as_deref() == Some("Indexes") && state.last_field_idx.is_some()
            {
                if let Some(k) = key {
                    if let Some(last) = model.sections.indexes.last_mut() {
                        if let serde_json::Value::Object(ref mut obj) = last {
                            obj.insert(k.to_string(), parse_nested_value(value.unwrap_or("")));
                        }
                    }
                }
                return;
            }

            // Nested items under relation
            if state.current_section.as_deref() == Some("Relations")
                && state.last_field_idx.is_some()
            {
                if let Some(k) = key {
                    if let Some(last) = model.sections.relations.last_mut() {
                        if let serde_json::Value::Object(ref mut obj) = last {
                            obj.insert(k.to_string(), parse_nested_value(value.unwrap_or("")));
                        }
                    }
                }
                return;
            }

            // Nested items under a field
            if let Some(field_idx) = state.last_field_idx {
                if field_idx < model.fields.len() {
                    // values: key for inline enum
                    if key == Some("values") && value.is_none() {
                        if model.fields[field_idx].enum_values.is_none() {
                            model.fields[field_idx].enum_values = Some(Vec::new());
                        }
                        return;
                    }

                    // If field has enum_values, add to it
                    if model.fields[field_idx].enum_values.is_some() {
                        if let Some(k) = key {
                            let mut ev = EnumValue {
                                name: k.to_string(),
                                description: None,
                                value_type: None,
                                value: None,
                            };
                            if let Some(v) = value {
                                if let Some(caps) = RE_QUOTE_STR.captures(v) {
                                    ev.description = Some(caps[1].to_string());
                                } else {
                                    ev.value = Some(serde_json::Value::String(v.to_string()));
                                }
                            }
                            model.fields[field_idx]
                                .enum_values
                                .as_mut()
                                .unwrap()
                                .push(ev);
                            return;
                        }
                    }

                    // Inline enum without values: key
                    if model.fields[field_idx].field_type.as_deref() == Some("enum") {
                        if let Some(k) = key {
                            if value.is_none_or(|v| !v.contains(':')) {
                                if model.fields[field_idx].enum_values.is_none() {
                                    model.fields[field_idx].enum_values = Some(Vec::new());
                                }
                                let mut ev = EnumValue {
                                    name: k.to_string(),
                                    description: None,
                                    value_type: None,
                                    value: None,
                                };
                                if let Some(v) = value {
                                    if let Some(caps) = RE_QUOTE_STR.captures(v) {
                                        ev.description = Some(caps[1].to_string());
                                    }
                                }
                                model.fields[field_idx]
                                    .enum_values
                                    .as_mut()
                                    .unwrap()
                                    .push(ev);
                                return;
                            }
                        }
                    }

                    // Sub-field for object type
                    if let (Some(k), Some(v)) = (key, value) {
                        if model.fields[field_idx].field_type.as_deref() == Some("object") {
                            let mut sub_data = TokenData::default();
                            sub_data.name = Some(k.to_string());
                            parse_type_and_attrs(v, &mut sub_data);
                            if sub_data.type_name.is_some() {
                                let sub_field = build_field_node(
                                    &sub_data,
                                    token,
                                    &state.file,
                                    &state.current_kind,
                                );
                                let is_object = sub_field.field_type.as_deref() == Some("object");
                                if model.fields[field_idx].fields.is_none() {
                                    model.fields[field_idx].fields = Some(Vec::new());
                                }
                                model.fields[field_idx]
                                    .fields
                                    .as_mut()
                                    .unwrap()
                                    .push(sub_field);
                                if is_object {
                                    let sub_idx =
                                        model.fields[field_idx].fields.as_ref().unwrap().len() - 1;
                                    // We can't easily track nested object lastField
                                    // without more complex state. For now, nested objects
                                    // at depth > 1 are handled via the existing logic.
                                    let _ = sub_idx;
                                }
                                return;
                            }
                        }
                    }

                    // Extended format field attributes
                    if let Some(k) = key {
                        apply_extended_attribute(
                            &mut model.fields[field_idx],
                            k,
                            value.unwrap_or(""),
                        );
                    }
                    return;
                }
            }

            // Source section nested items for views
            if state.current_section.as_deref() == Some("Source")
                && model.model_type == ModelType::View
            {
                if let Some(k) = key {
                    let mut sub_data = TokenData::default();
                    sub_data.name = Some(k.to_string());
                    sub_data.type_name = value.map(|v| v.to_string());
                    if let Some(ref mut sd) = model.source_def {
                        set_source_directive(sd, &sub_data);
                    }
                }
            }
        }
    }
}

fn handle_blockquote(token: &Token, state: &mut ParserState) {
    let text = token.data.name.clone().unwrap_or_default();

    // Attribute definition description
    if let Some(ref mut attr_def) = state.current_attr_def {
        attr_def.description = Some(text);
        return;
    }

    match state.current_element {
        CurrentElement::None => {}
        CurrentElement::Enum(ref mut en) => {
            if let Some(ref mut desc) = en.description {
                desc.push('\n');
                desc.push_str(&text);
            } else {
                en.description = Some(text);
            }
        }
        CurrentElement::Model(ref mut model) => {
            // Field-level blockquote
            if let Some(idx) = state.last_field_idx {
                if idx < model.fields.len() {
                    if let Some(ref mut desc) = model.fields[idx].description {
                        desc.push('\n');
                        desc.push_str(&text);
                    } else {
                        model.fields[idx].description = Some(text);
                    }
                    return;
                }
            }
            // Model-level blockquote
            if let Some(ref mut desc) = model.description {
                desc.push('\n');
                desc.push_str(&text);
            } else {
                model.description = Some(text);
            }
        }
    }
}

fn handle_text(token: &Token, state: &mut ParserState) {
    // Collect import directives
    if token.data.is_import {
        if let Some(ref path) = token.data.import_path {
            state.imports.push(path.clone());
        }
        return;
    }

    if let CurrentElement::Model(ref mut model) = state.current_element {
        if model.fields.is_empty() {
            let text = token.data.name.clone().unwrap_or_default();
            if !text.is_empty() && model.description.is_none() {
                model.description = Some(text);
            }
        }
    }
}

fn finalize_element(state: &mut ParserState) {
    finalize_attr_def(state);

    let element = std::mem::replace(&mut state.current_element, CurrentElement::None);
    match element {
        CurrentElement::Enum(en) => state.enums.push(en),
        CurrentElement::Model(model) => match model.model_type {
            ModelType::Interface => state.interfaces.push(*model),
            ModelType::View => state.views.push(*model),
            _ => state.models.push(*model),
        },
        CurrentElement::None => {}
    }

    state.current_section = None;
    state.current_kind = FieldKind::Stored;
    state.last_field_idx = None;
}

fn handle_attribute_def_start(token: &Token, state: &mut ParserState) {
    finalize_element(state);

    let name = token
        .data
        .name
        .clone()
        .unwrap_or_default()
        .trim_start_matches('@')
        .to_string();

    state.current_attr_def = Some(AttrDef {
        name,
        description: token.data.description.clone(),
        fields: HashMap::new(),
    });
}

fn finalize_attr_def(state: &mut ParserState) {
    let attr_def = match state.current_attr_def.take() {
        Some(d) => d,
        None => return,
    };

    let target_raw = attr_def.fields.get("target").cloned().unwrap_or_default();
    let target: Vec<String> = target_raw
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| s == "field" || s == "model")
        .collect();

    let range = attr_def.fields.get("range").and_then(|r| {
        let cleaned = r.trim_start_matches('[').trim_end_matches(']');
        // Support both "1, 10" and "1..10" formats
        let separator = if cleaned.contains("..") { ".." } else { "," };
        let nums: Vec<f64> = cleaned
            .split(separator)
            .map(|s| s.trim().parse::<f64>())
            .collect::<Result<Vec<_>, _>>()
            .ok()?;
        if nums.len() == 2 {
            Some((nums[0], nums[1]))
        } else {
            None
        }
    });

    let required = attr_def
        .fields
        .get("required")
        .map(|v| v == "true")
        .unwrap_or(false);

    let default_value = attr_def.fields.get("default").map(|v| {
        if v == "true" {
            AttrArgValue::Bool(true)
        } else if v == "false" {
            AttrArgValue::Bool(false)
        } else if let Ok(n) = v.parse::<f64>() {
            AttrArgValue::Number(n)
        } else {
            AttrArgValue::String(v.clone())
        }
    });

    let entry = AttributeRegistryEntry {
        name: attr_def.name,
        description: attr_def.description,
        target: if target.is_empty() {
            vec!["field".to_string()]
        } else {
            target
        },
        attr_type: attr_def
            .fields
            .get("type")
            .cloned()
            .unwrap_or_else(|| "boolean".to_string()),
        range,
        required,
        default_value,
    };

    state.attribute_registry.push(entry);
}

// --- Helpers ---

fn build_field_node(
    data: &TokenData,
    token: &Token,
    file: &str,
    current_kind: &FieldKind,
) -> FieldNode {
    let attrs = parse_raw_attributes(&data.attributes);
    let mut kind = current_kind.clone();

    // Detect kind from attributes
    let lookup_attr = attrs.iter().find(|a| a.name == "lookup");
    let rollup_attr = attrs.iter().find(|a| a.name == "rollup");
    let computed_attr = attrs.iter().find(|a| a.name == "computed");
    let computed_raw_attr = attrs.iter().find(|a| a.name == "computed_raw");

    if lookup_attr.is_some() {
        kind = FieldKind::Lookup;
    } else if rollup_attr.is_some() {
        kind = FieldKind::Rollup;
    } else if computed_attr.is_some() || computed_raw_attr.is_some() {
        kind = FieldKind::Computed;
    }

    // Process default_value
    let (default_value, default_value_type) = process_default_value(data.default_value.as_deref());

    // Type params
    let params = if data.type_params.is_empty() {
        None
    } else {
        Some(data.type_params.clone())
    };

    let generic_params = if data.type_generic_params.is_empty() {
        None
    } else {
        Some(data.type_generic_params.clone())
    };

    let framework_attrs = parse_custom_attributes(&data.framework_attrs);

    let mut field = FieldNode {
        name: data.name.clone().unwrap_or_default(),
        label: data.label.clone(),
        field_type: data.type_name.clone(),
        params,
        generic_params,
        nullable: data.nullable,
        array: data.array,
        array_item_nullable: data.array_item_nullable,
        kind,
        default_value,
        default_value_type,
        description: data.description.clone(),
        attributes: attrs.clone(),
        framework_attrs,
        lookup: None,
        rollup: None,
        computed: None,
        enum_values: None,
        fields: None,
        loc: SourceLocation {
            file: file.to_string(),
            line: token.line,
            col: 1,
        },
    };

    // Parse lookup
    if let Some(la) = lookup_attr {
        if let Some(arg) = la.args.as_ref().and_then(|a| a.first()) {
            if let AttrArgValue::String(path) = arg {
                field.lookup = Some(LookupDef { path: path.clone() });
            }
        }
    }

    // Parse rollup — args may be split into multiple elements by the lexer,
    // so rejoin them before parsing (e.g. ["Order.customer_id", "count"] → "Order.customer_id, count")
    if let Some(ra) = rollup_attr {
        if let Some(args) = ra.args.as_ref() {
            let args_str: String = args
                .iter()
                .map(|a| match a {
                    AttrArgValue::String(s) => s.clone(),
                    AttrArgValue::Number(n) => n.to_string(),
                    AttrArgValue::Bool(b) => b.to_string(),
                })
                .collect::<Vec<_>>()
                .join(", ");
            field.rollup = Some(parse_rollup_args(&args_str));
        }
    }

    // Parse computed
    if let Some(ca) = computed_attr {
        if let Some(arg) = ca.args.as_ref().and_then(|a| a.first()) {
            if let AttrArgValue::String(expr) = arg {
                let cleaned = expr
                    .trim_start_matches(['"', '\'', '`'])
                    .trim_end_matches(['"', '\'', '`']);
                field.computed = Some(ComputedDef {
                    expression: cleaned.to_string(),
                    platform: None,
                });
            }
        }
    }

    // Parse computed_raw — args may be split by the lexer
    if let Some(cra) = computed_raw_attr {
        if let Some(args) = cra.args.as_ref() {
            if let Some(AttrArgValue::String(expr_raw)) = args.first() {
                let parts = split_computed_raw_args(expr_raw);
                let cleaned = parts
                    .0
                    .trim_start_matches(['"', '\'', '`'])
                    .trim_end_matches(['"', '\'', '`']);
                let mut platform = parts.1;
                // If platform not found in the first arg, check remaining args
                if platform.is_none() {
                    for arg in args.iter().skip(1) {
                        if let AttrArgValue::String(s) = arg {
                            if let Some(caps) = RE_PLATFORM.captures(s) {
                                platform = Some(caps[1].to_string());
                                break;
                            }
                        }
                    }
                }
                field.computed = Some(ComputedDef {
                    expression: cleaned.to_string(),
                    platform,
                });
            }
        }
    }

    // Code block computed
    if field.computed.is_none() {
        if let Some(ref cb) = data.code_block {
            if computed_attr.is_some() || computed_raw_attr.is_some() {
                let mut computed = ComputedDef {
                    expression: cb.content.clone(),
                    platform: None,
                };
                // Check for platform in attr args
                if let Some(cra) = computed_raw_attr {
                    if let Some(arg) = cra.args.as_ref().and_then(|a| a.first()) {
                        if let AttrArgValue::String(s) = arg {
                            if let Some(caps) = RE_PLATFORM.captures(s) {
                                computed.platform = Some(caps[1].to_string());
                            }
                        }
                    }
                }
                field.computed = Some(computed);
            }
        }
    }

    // Inline comment as field description
    if field.description.is_none() {
        if let Some(ref comment) = data.comment {
            field.description = Some(comment.clone());
        }
    }

    // Blockquote description overrides
    if let Some(ref bq) = data.blockquote_desc {
        field.description = Some(bq.clone());
    }

    field
}

fn process_default_value(raw: Option<&str>) -> (Option<String>, Option<DefaultValueType>) {
    match raw {
        None => (None, None),
        Some(v) => {
            if v.starts_with('`') && v.ends_with('`') && v.len() >= 2 {
                (
                    Some(v[1..v.len() - 1].to_string()),
                    Some(DefaultValueType::Expression),
                )
            } else if v.starts_with('"') && v.ends_with('"') && v.len() >= 2 {
                (
                    Some(v[1..v.len() - 1].to_string()),
                    Some(DefaultValueType::Literal),
                )
            } else {
                let dvt = if v.contains('(') {
                    DefaultValueType::Expression
                } else {
                    DefaultValueType::Literal
                };
                (Some(v.to_string()), Some(dvt))
            }
        }
    }
}

fn parse_raw_attributes(raw_attrs: &[RawAttribute]) -> Vec<FieldAttribute> {
    raw_attrs
        .iter()
        .map(|a| {
            let args = if a.args.is_empty() {
                None
            } else {
                Some(a.args.clone())
            };
            let is_standard = if STANDARD_ATTRIBUTES.contains(a.name.as_str()) {
                Some(true)
            } else {
                None
            };
            FieldAttribute {
                name: a.name.clone(),
                args,
                cascade: a.cascade.clone(),
                is_standard,
                is_registered: None,
            }
        })
        .collect()
}

fn parse_rollup_args(args_str: &str) -> RollupDef {
    let parts = split_rollup_args(args_str);

    let target_fk = parts.first().map(|s| s.as_str()).unwrap_or("");
    let (target, fk) = match target_fk.find('.') {
        Some(idx) => (
            target_fk[..idx].to_string(),
            target_fk[idx + 1..].to_string(),
        ),
        None => (target_fk.to_string(), String::new()),
    };

    let (aggregate, field) = if parts.len() > 1 {
        let agg_part = parts[1].trim();
        if let Some(caps) = RE_AGG.captures(agg_part) {
            (
                caps[1].to_string(),
                caps.get(2).map(|m| m.as_str().to_string()),
            )
        } else {
            (agg_part.to_string(), None)
        }
    } else {
        (String::new(), None)
    };

    let mut where_clause = None;
    for p in parts.iter().skip(2) {
        let part = p.trim();
        if let Some(caps) = RE_WHERE.captures(part) {
            where_clause = Some(caps[1].to_string());
        }
    }

    RollupDef {
        target,
        fk,
        aggregate,
        field,
        where_clause,
    }
}

fn split_rollup_args(args_str: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth = 0;
    let mut in_quote = false;
    let mut quote_char = ' ';

    for ch in args_str.chars() {
        if in_quote {
            current.push(ch);
            if ch == quote_char {
                in_quote = false;
            }
            continue;
        }
        match ch {
            '"' | '\'' => {
                in_quote = true;
                quote_char = ch;
                current.push(ch);
            }
            '(' => {
                depth += 1;
                current.push(ch);
            }
            ')' => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 => {
                let trimmed = current.trim().to_string();
                if !trimmed.is_empty() {
                    parts.push(trimmed);
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        parts.push(trimmed);
    }
    parts
}

fn split_computed_raw_args(raw: &str) -> (String, Option<String>) {
    let bytes = raw.as_bytes();
    if bytes.is_empty() {
        return (raw.to_string(), None);
    }

    let first = bytes[0];
    if first == b'"' || first == b'\'' || first == b'`' {
        let mut i = 1;
        while i < bytes.len() {
            if bytes[i] == b'\\' {
                i += 2;
                continue;
            }
            if bytes[i] == first {
                break;
            }
            i += 1;
        }
        let expression = raw[..=i].to_string();
        let remainder = raw[i + 1..].trim();
        let platform = RE_PLATFORM.captures(remainder).map(|c| c[1].to_string());
        (expression, platform)
    } else {
        (raw.to_string(), None)
    }
}

fn is_source_directive(name: &str) -> bool {
    matches!(name, "from" | "where" | "order_by" | "group_by" | "join")
}

fn set_source_directive(def: &mut ViewSourceDef, data: &TokenData) {
    let name = data.name.as_deref().unwrap_or("");
    let value = data
        .description
        .clone()
        .or_else(|| data.type_name.clone())
        .unwrap_or_default();

    match name {
        "from" => def.from = Some(value),
        "where" => def.where_clause = Some(value),
        "order_by" => def.order_by = Some(value),
        "group_by" => def.group_by = Some(parse_array_value(&value)),
        "join" => {
            let joins = def.joins.get_or_insert_with(Vec::new);
            joins.push(parse_join_value(&value));
        }
        _ => {}
    }
}

fn parse_join_value(value: &str) -> JoinDef {
    let parts: Vec<&str> = value.splitn(2, " on ").collect();
    JoinDef {
        model: parts
            .first()
            .map(|s| s.trim().to_string())
            .unwrap_or_default(),
        on: parts
            .get(1)
            .map(|s| s.trim().to_string())
            .unwrap_or_default(),
    }
}

fn parse_array_value(value: &str) -> Vec<String> {
    let cleaned = value.trim_start_matches('[').trim_end_matches(']');
    cleaned
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn parse_metadata_value(value: &str) -> serde_json::Value {
    let was_quoted = (value.starts_with('"') && value.ends_with('"'))
        || (value.starts_with('\'') && value.ends_with('\''));
    let unquoted = value
        .trim_start_matches(['"', '\''])
        .trim_end_matches(['"', '\'']);

    if was_quoted {
        return serde_json::Value::String(unquoted.to_string());
    }
    if let Ok(n) = unquoted.parse::<f64>() {
        if unquoted.contains('.') {
            return serde_json::json!(n);
        }
        if let Ok(i) = unquoted.parse::<i64>() {
            return serde_json::json!(i);
        }
        return serde_json::json!(n);
    }
    if unquoted == "true" {
        return serde_json::Value::Bool(true);
    }
    if unquoted == "false" {
        return serde_json::Value::Bool(false);
    }
    serde_json::Value::String(unquoted.to_string())
}

fn parse_nested_value(value: &str) -> serde_json::Value {
    let s = value.trim();
    // Array
    if s.starts_with('[') && s.ends_with(']') {
        let items = parse_array_value(s);
        return serde_json::json!(items);
    }
    if s == "true" {
        return serde_json::Value::Bool(true);
    }
    if s == "false" {
        return serde_json::Value::Bool(false);
    }
    if let Ok(n) = s.parse::<f64>() {
        if s.contains('.') {
            return serde_json::json!(n);
        }
        if let Ok(i) = s.parse::<i64>() {
            return serde_json::json!(i);
        }
        return serde_json::json!(n);
    }
    let unquoted = s
        .trim_start_matches(['"', '\''])
        .trim_end_matches(['"', '\'']);
    serde_json::Value::String(unquoted.to_string())
}

fn apply_extended_attribute(field: &mut FieldNode, key: &str, value: &str) {
    match key {
        "type" => {
            let mut t = value.to_string();
            if t.ends_with('?') {
                field.nullable = true;
                t = t[..t.len() - 1].to_string();
            }
            if t.ends_with("[]") {
                field.array = true;
                t = t[..t.len() - 2].to_string();
            }
            field.field_type = Some(t);
        }
        "description" => {
            let parsed = value
                .trim_start_matches(['"', '\''])
                .trim_end_matches(['"', '\'']);
            field.description = Some(parsed.to_string());
        }
        "reference" => {
            field.attributes.push(FieldAttribute {
                name: "reference".to_string(),
                args: Some(vec![AttrArgValue::String(value.to_string())]),
                cascade: None,
                is_standard: Some(true),
                is_registered: None,
            });
        }
        "on_delete" => {
            field.attributes.push(FieldAttribute {
                name: "on_delete".to_string(),
                args: Some(vec![AttrArgValue::String(value.to_string())]),
                cascade: None,
                is_standard: Some(true),
                is_registered: None,
            });
        }
        _ => {
            let parsed_val = parse_arg_value(value);
            field.attributes.push(FieldAttribute {
                name: key.to_string(),
                args: Some(vec![parsed_val]),
                cascade: None,
                is_standard: if STANDARD_ATTRIBUTES.contains(key) {
                    Some(true)
                } else {
                    None
                },
                is_registered: None,
            });
        }
    }
}

fn parse_custom_attributes(raw_attrs: &[String]) -> Option<Vec<CustomAttribute>> {
    if raw_attrs.is_empty() {
        return None;
    }
    let result: Vec<CustomAttribute> = raw_attrs
        .iter()
        .map(|raw| {
            let content = raw
                .trim_start_matches('[')
                .trim_end_matches(']')
                .to_string();

            let parsed = RE_CUSTOM_ATTR.captures(&content).map(|caps| {
                let name = caps[1].to_string();
                let args: Vec<AttrArgValue> = match caps.get(2) {
                    Some(args_m) => split_balanced(args_m.as_str())
                        .into_iter()
                        .map(|s| parse_arg_value(s.trim()))
                        .collect(),
                    None => Vec::new(),
                };
                CustomAttributeParsed {
                    name,
                    arguments: args,
                }
            });

            CustomAttribute {
                content,
                raw: raw.clone(),
                parsed,
            }
        })
        .collect();
    Some(result)
}

fn split_balanced(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    for (i, ch) in s.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(s[start..i].to_string());
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(s[start..].to_string());
    parts
}

fn parse_arg_value(s: &str) -> AttrArgValue {
    if s == "true" {
        return AttrArgValue::Bool(true);
    }
    if s == "false" {
        return AttrArgValue::Bool(false);
    }
    if let Ok(n) = s.parse::<f64>() {
        if !s.is_empty() {
            return AttrArgValue::Number(n);
        }
    }
    let unquoted =
        if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
            &s[1..s.len() - 1]
        } else {
            s
        };
    AttrArgValue::String(unquoted.to_string())
}

fn attr_args_to_json(args: &[AttrArgValue]) -> serde_json::Value {
    if args.len() == 1 {
        match &args[0] {
            AttrArgValue::String(s) => serde_json::json!(s),
            AttrArgValue::Number(n) => serde_json::json!(n),
            AttrArgValue::Bool(b) => serde_json::json!(b),
        }
    } else {
        serde_json::json!(args
            .iter()
            .map(|a| match a {
                AttrArgValue::String(s) => serde_json::json!(s),
                AttrArgValue::Number(n) => serde_json::json!(n),
                AttrArgValue::Bool(b) => serde_json::json!(b),
            })
            .collect::<Vec<_>>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty() {
        let result = parse_string("", "test.m3l.md");
        assert!(result.models.is_empty());
        assert!(result.enums.is_empty());
    }

    #[test]
    fn parse_simple_model() {
        let input = "## User\n- id: identifier @pk\n- name: string @required";
        let result = parse_string(input, "test.m3l.md");
        assert_eq!(result.models.len(), 1);
        assert_eq!(result.models[0].name, "User");
        assert_eq!(result.models[0].fields.len(), 2);
        assert_eq!(result.models[0].fields[0].name, "id");
        assert_eq!(
            result.models[0].fields[0].field_type.as_deref(),
            Some("identifier")
        );
        assert_eq!(result.models[0].fields[1].name, "name");
    }

    #[test]
    fn parse_model_with_inheritance() {
        let input = "## Customer : BaseModel\n- email: string";
        let result = parse_string(input, "test.m3l.md");
        assert_eq!(result.models[0].inherits, vec!["BaseModel"]);
    }

    #[test]
    fn parse_enum() {
        let input = "## Status ::enum\n- Active \"Active status\"\n- Inactive";
        let result = parse_string(input, "test.m3l.md");
        assert_eq!(result.enums.len(), 1);
        assert_eq!(result.enums[0].name, "Status");
        assert_eq!(result.enums[0].values.len(), 2);
        assert_eq!(result.enums[0].values[0].name, "Active");
        assert_eq!(
            result.enums[0].values[0].description.as_deref(),
            Some("Active status")
        );
    }

    #[test]
    fn parse_view() {
        let input = "## SalesSummary ::view @materialized\n- total: decimal";
        let result = parse_string(input, "test.m3l.md");
        assert_eq!(result.views.len(), 1);
        assert_eq!(result.views[0].name, "SalesSummary");
        assert_eq!(result.views[0].materialized, Some(true));
    }

    #[test]
    fn parse_interface() {
        let input = "## Timestampable ::interface\n- created_at: timestamp";
        let result = parse_string(input, "test.m3l.md");
        assert_eq!(result.interfaces.len(), 1);
        assert_eq!(result.interfaces[0].name, "Timestampable");
    }

    #[test]
    fn parse_field_with_default() {
        let input = "## User\n- status: string = \"active\"";
        let result = parse_string(input, "test.m3l.md");
        let field = &result.models[0].fields[0];
        assert_eq!(field.default_value.as_deref(), Some("active"));
        assert_eq!(field.default_value_type, Some(DefaultValueType::Literal));
    }

    #[test]
    fn parse_field_expression_default() {
        let input = "## User\n- created_at: timestamp = now()";
        let result = parse_string(input, "test.m3l.md");
        let field = &result.models[0].fields[0];
        assert_eq!(field.default_value.as_deref(), Some("now()"));
        assert_eq!(field.default_value_type, Some(DefaultValueType::Expression));
    }

    #[test]
    fn parse_namespace() {
        let input = "# Namespace: sample.ecommerce\n## User\n- id: identifier";
        let result = parse_string(input, "test.m3l.md");
        assert_eq!(result.namespace.as_deref(), Some("sample.ecommerce"));
    }

    #[test]
    fn parse_non_namespace_h1_ignored() {
        // A non-namespace H1 should not change the namespace
        let input = "# My Data Model\n## User\n- name: string";
        let result = parse_string(input, "test.m3l.md");
        assert_eq!(result.namespace, None, "non-namespace H1 should not set namespace");
        assert_eq!(result.models.len(), 1);
        assert_eq!(result.models[0].name, "User");
    }

    #[test]
    fn parse_namespace_then_title_h1() {
        // Namespace should persist even if followed by a non-namespace H1
        let input = "# Namespace: sample.domain\n# Document Title\n## User\n- name: string";
        let result = parse_string(input, "test.m3l.md");
        assert_eq!(result.namespace.as_deref(), Some("sample.domain"));
        assert_eq!(result.models.len(), 1);
    }

    #[test]
    fn parse_section_indexes() {
        let input = "## User\n- id: identifier\n### Indexes\n- idx_email";
        let result = parse_string(input, "test.m3l.md");
        assert!(!result.models[0].sections.indexes.is_empty());
    }

    #[test]
    fn parse_attribute_def() {
        let input = "## custom_flag ::attribute\n> A custom flag\n- target: [field, model]\n- type: boolean\n- required: false\n- default: true";
        let result = parse_string(input, "test.m3l.md");
        assert_eq!(result.attribute_registry.len(), 1);
        let entry = &result.attribute_registry[0];
        assert_eq!(entry.name, "custom_flag");
        assert_eq!(entry.description.as_deref(), Some("A custom flag"));
        assert_eq!(entry.target, vec!["field", "model"]);
        assert_eq!(entry.attr_type, "boolean");
        assert!(!entry.required);
        assert_eq!(entry.default_value, Some(AttrArgValue::Bool(true)));
    }

    #[test]
    fn parse_lookup_field() {
        let input = "## Order\n- id: identifier\n### Lookup\n- customer_name: string @lookup(customer_id.Customer.name)";
        let result = parse_string(input, "test.m3l.md");
        let field = &result.models[0].fields[1];
        assert_eq!(field.kind, FieldKind::Lookup);
        assert!(field.lookup.is_some());
        assert_eq!(
            field.lookup.as_ref().unwrap().path,
            "customer_id.Customer.name"
        );
    }

    #[test]
    fn parse_computed_field() {
        let input = "## Order\n- total: decimal @computed(`price * qty`)";
        let result = parse_string(input, "test.m3l.md");
        let field = &result.models[0].fields[0];
        assert_eq!(field.kind, FieldKind::Computed);
        assert!(field.computed.is_some());
        assert_eq!(field.computed.as_ref().unwrap().expression, "price * qty");
    }

    #[test]
    fn parse_multiple_models() {
        let input = "## User\n- id: identifier\n\n## Product\n- id: identifier\n- name: string";
        let result = parse_string(input, "test.m3l.md");
        assert_eq!(result.models.len(), 2);
        assert_eq!(result.models[0].name, "User");
        assert_eq!(result.models[1].name, "Product");
    }

    #[test]
    fn parse_model_description() {
        let input = "## User\n> User account model\n- id: identifier";
        let result = parse_string(input, "test.m3l.md");
        assert_eq!(
            result.models[0].description.as_deref(),
            Some("User account model")
        );
    }

    #[test]
    fn parse_empty_model() {
        let input = "## EmptyModel";
        let result = parse_string(input, "test.m3l.md");
        assert_eq!(result.models.len(), 1);
        assert_eq!(result.models[0].name, "EmptyModel");
        assert!(result.models[0].fields.is_empty());
    }

    #[test]
    fn parse_multiple_inheritance() {
        let input = "## Admin : User, Auditable\n- level: integer";
        let result = parse_string(input, "test.m3l.md");
        assert_eq!(result.models[0].inherits, vec!["User", "Auditable"]);
    }

    #[test]
    fn parse_empty_enum() {
        let input = "## Status ::enum";
        let result = parse_string(input, "test.m3l.md");
        assert_eq!(result.enums.len(), 1);
        assert!(result.enums[0].values.is_empty());
    }

    #[test]
    fn parse_field_multiple_attrs() {
        let input = "## User\n- email: string @required @unique @searchable";
        let result = parse_string(input, "test.m3l.md");
        let attrs: Vec<&str> = result.models[0].fields[0]
            .attributes
            .iter()
            .map(|a| a.name.as_str())
            .collect();
        assert!(attrs.contains(&"required"));
        assert!(attrs.contains(&"unique"));
        assert!(attrs.contains(&"searchable"));
    }

    #[test]
    fn parse_nullable_array_item() {
        let input = "## User\n- tags: string?[]?";
        let result = parse_string(input, "test.m3l.md");
        let field = &result.models[0].fields[0];
        assert!(field.nullable);
        assert!(field.array);
    }

    #[test]
    fn parse_qualified_type_ref() {
        let input = "## Order\n- user: Auth.User";
        let result = parse_string(input, "test.m3l.md");
        assert_eq!(
            result.models[0].fields[0].field_type.as_deref(),
            Some("Auth.User")
        );
    }
}
