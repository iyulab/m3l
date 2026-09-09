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
/// Start of a trailing attribute run on a nested item's right-hand side.
static RE_TRAILING_ATTRS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s*@\w+").unwrap());
static RE_CUSTOM_ATTR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([A-Za-z_][\w.]*)(?:\((.+)\))?$").unwrap());
static RE_AGG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\w+)(?:\((\w+)\))?$").unwrap());
// The `where:` value has already had a single pair of enclosing double quotes
// stripped by the generic attribute-argument tokenizer by the time it reaches
// here (`args_str` is rebuilt from already-tokenized `AttrArgValue::String`s,
// see the call site) — so the quotes in this pattern are optional, not
// required. Requiring them unconditionally silently dropped every `where:`
// clause a rollup ever declared.
static RE_WHERE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"^where:\s*"?(.*?)"?$"#).unwrap());
static RE_PLATFORM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"platform\s*:\s*["']?([^"'\s]+)["']?"#).unwrap());

// --- Parser state ---

enum CurrentElement {
    Model(Box<ModelNode>),
    Enum(Box<EnumNode>),
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
    flows: Vec<ModelNode>,
    extensions: HashMap<String, Vec<ModelNode>>,
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
        flows: Vec::new(),
        extensions: HashMap::new(),
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
        flows: state.flows,
        extensions: state.extensions,
        attribute_registry: state.attribute_registry,
        imports: state.imports,
    }
}

fn process_token(token: &Token, state: &mut ParserState) {
    match &token.token_type {
        TokenType::Namespace => handle_namespace(token, state),
        TokenType::Model | TokenType::Interface => handle_model_start(token, state),
        TokenType::Enum => handle_enum_start(token, state),
        TokenType::View => handle_view_start(token, state),
        TokenType::Flow => handle_flow_start(token, state),
        TokenType::Extension(ext_type) => handle_extension_start(token, ext_type, state),
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
        namespace: state.namespace.clone(),
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
        namespace: state.namespace.clone(),
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

    state.current_element = CurrentElement::Enum(Box::new(enum_node));
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
        namespace: state.namespace.clone(),
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

fn handle_flow_start(token: &Token, state: &mut ParserState) {
    finalize_element(state);

    let flow = ModelNode {
        name: token.data.name.clone().unwrap_or_default(),
        label: token.data.label.clone(),
        model_type: ModelType::Flow,
        source: state.file.clone(),
        namespace: state.namespace.clone(),
        line: token.line,
        inherits: Vec::new(),
        description: None,
        attributes: parse_raw_attributes(&token.data.attributes),
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

    state.current_element = CurrentElement::Model(Box::new(flow));
    state.current_section = None;
    state.current_kind = FieldKind::Stored;
    state.last_field_idx = None;
    state.source_directives_done = false;
}

fn handle_extension_start(token: &Token, ext_type: &str, state: &mut ParserState) {
    finalize_element(state);

    let node = ModelNode {
        name: token.data.name.clone().unwrap_or_default(),
        label: token.data.label.clone(),
        model_type: ModelType::Extension(ext_type.to_string()),
        source: state.file.clone(),
        namespace: state.namespace.clone(),
        line: token.line,
        inherits: Vec::new(),
        description: None,
        attributes: parse_raw_attributes(&token.data.attributes),
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

    state.current_element = CurrentElement::Model(Box::new(node));
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
                attributes: parse_raw_attributes_opt(&token.data.attributes),
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

            // 🔴 관계 표기는 필드가 아니다. 종전에는 여기까지 흘러와 원문 줄이 «필드 이름»이
            // 되고 타입이 비었다 — 소비자가 그것을 관계로도 필드로도 다룰 수 없었다.
            //
            // ⚠ 「타입을 선언한 줄은 필드로 남긴다」는 가드를 두려다 **실측으로 걷어냈다**:
            // `>`·`<` 로 시작하는 이름은 식별자가 아니라 렉서가 애초에 타입을 읽지 않는다.
            // 즉 그 가드는 한 번도 발화하지 않는 죽은 코드이면서 «있지도 않은 보호»를
            // 암시한다. 그런 줄(`- <weird: string(10)`)은 이 변경 «전»에도 타입 없는 가짜
            // 필드였고, 지금은 최소한 자기 선두 토큰대로 분류된다.
            {
                if let Some(n) = parse_relation_notation(&token.raw) {
                    let raw = token.raw.trim().trim_start_matches("- ").to_string();
                    let mut entry = serde_json::Map::new();
                    entry.insert("raw".into(), serde_json::json!(raw));
                    entry.insert("direction".into(), serde_json::json!(n.direction));
                    if let Some(name) = n.name {
                        entry.insert("name".into(), serde_json::json!(name));
                    }
                    if let Some(target) = n.target {
                        entry.insert("target".into(), serde_json::json!(target));
                    }
                    if let Some(c) = n.cardinality {
                        entry.insert("cardinality".into(), serde_json::json!(c));
                    }
                    // 어디에 쓰였는지 남긴다 — 정본 위치는 `### Relations` 이고(§3.2.3),
                    // 검증기가 이 표시를 보고 그 사실을 경고로 알린다. 구조화는 하되
                    // 잘못 놓인 것을 조용히 삼키지는 않는다.
                    entry.insert("declaredIn".into(), serde_json::json!("fields"));
                    entry.insert(
                        "loc".into(),
                        serde_json::json!({ "file": state.file, "line": token.line, "col": 1 }),
                    );
                    model
                        .sections
                        .relations
                        .push(serde_json::Value::Object(entry));
                    state.last_field_idx = Some(usize::MAX);
                    return;
                }
            }

            // Regular field
            let field = build_field_node(&token.data, token, &state.file, &state.current_kind);
            model.fields.push(field);
            state.last_field_idx = Some(model.fields.len() - 1);
        }
    }
}

/// 명세 §3.2.2·§3.2.4 의 관계 표기를 분해한 결과.
///
/// 종전에는 이 표기가 **원문 문자열로만** 남았다 — `### Relations` 항목은 `raw` 에 통째로,
/// 필드 목록에 쓰이면 아예 **필드 이름**이 됐다(`name = "<>tags: many-to-many"`, `type` 없음).
/// 그러면 소비자가 방향·타깃·카디널리티를 알려면 그 문자열을 자기가 다시 파싱해야 한다 —
/// 이 언어가 스스로 정의한 구문이므로 그것을 구조화해 내주는 것은 파서의 몫이다.
struct RelationNotation {
    /// `to`(`>`·`->`) · `from`(`<`·`<-`) · `many-to-many`(`<>`)
    direction: &'static str,
    /// 화살표 없는 짧은 형(`>author`)의 토큰 — 그 항목의 이름이다.
    name: Option<String>,
    /// 화살표 형(`-> Person`·`<- Comment.post_id`)의 토큰 — 대상이다.
    target: Option<String>,
    /// `: one-to-many` 처럼 뒤에 붙는 카디널리티.
    cardinality: Option<String>,
}

/// 한 줄에서 관계 표기를 읽는다. 표기가 아니면 `None` — 그때 호출부는 종전 경로를 그대로 간다.
///
/// ⚠ **좁게 판정한다.** 화살표/꺾쇠로 «시작»하는 줄만 표기로 본다. 넓히면 `- a < b` 같은
/// 평범한 텍스트가 관계로 오분류되고, 그 오분류는 필드 하나를 조용히 삼키는 형태라
/// 지금 고치는 결함보다 나쁘다.
fn parse_relation_notation(line: &str) -> Option<RelationNotation> {
    let mut s = line.trim();
    if let Some(rest) = s.strip_prefix("- ") {
        s = rest.trim();
    }
    // 뒤따르는 설명(`"..."`)은 표기의 일부가 아니다.
    if let Some(q) = s.find('"') {
        s = s[..q].trim();
    }

    // 긴 토큰을 «먼저» 본다 — `<>` 는 `<` 로도, `->` 는 `>` 로도 읽히기 때문이다.
    // (표는 순서가 곧 규칙이라, 조건 분기보다 이 형태가 그 사실을 드러낸다.)
    const PREFIXES: [(&str, &str, bool); 5] = [
        ("<>", "many-to-many", false),
        ("->", "to", true),
        ("<-", "from", true),
        (">", "to", false),
        ("<", "from", false),
    ];
    // `name: >Target …` — the notation sits after a key that names the entry.
    // The repository's own samples write relationships this way, and the shape
    // reached the AST as `raw` alone: nothing said which direction it went, so
    // a consumer had to read the line again. It is the same five prefixes in
    // the same table, one position further right, and 3.2.3 already gives an
    // entry both a name and a target -- it writes them on two lines where this
    // writes them on one.
    //
    // 🔴 The specification does not spell this form out: whether 3.2.4 should
    // document it, or the samples should be rewritten to 3.2.3 form, is a
    // language decision and not this function's. Recognising it changes no
    // meaning either way -- it only stops the direction from being lost.
    let mut named: Option<&str> = None;
    if !PREFIXES.iter().any(|(p, _, _)| s.starts_with(p)) {
        let (key, value) = s.split_once(':')?;
        let key = key.trim();
        let value = value.trim();
        if key.is_empty()
            || !key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
            || !PREFIXES.iter().any(|(p, _, _)| value.starts_with(p))
        {
            return None;
        }
        named = Some(key);
        s = value;
    }

    let (direction, rest, arrow) = PREFIXES
        .iter()
        .find_map(|(p, d, arrow)| s.strip_prefix(p).map(|r| (*d, r, *arrow)))?;

    let rest = rest.trim();
    let (token, cardinality) = match rest.split_once(':') {
        Some((t, c)) => (
            t.trim(),
            Some(c.trim().to_string()).filter(|c| !c.is_empty()),
        ),
        None => (rest, None),
    };
    // The token ends at the first space. `>Supplier via supplier_id (optional)`
    // names `Supplier`; without this the rest of the line rode along inside it.
    let token = token.split_whitespace().next().unwrap_or("");
    if token.is_empty() {
        return None;
    }

    // 콜론 뒤 값은 **원문 그대로** 싣는다. 명세 §3.2.4 는 카디널리티 어휘를 닫아 두지 않았고,
    // 이 리포는 이미 attribute 인자에 대해 같은 태도를 취한다(`attribute_argument_fidelity`:
    // *「인자가 어디서 끝나는지는 렉서가 정하고, 그 안의 글자가 무슨 뜻인지는 정하지 않는다」*).
    // 여기서 어휘를 검열하면 명세가 늘어날 때마다 파서가 먼저 거절하게 된다.
    //
    // 화살표 형은 명세가 대상(`-> Target`)이라 적고, 짧은 형은 그 항목의 이름(`>author`)이다.
    // 짧은 형의 토큰이 이름인지 대상인지는 명세가 두 곳에서 다르게 읽히므로 «추측하지 않는다» —
    // 짧은 형은 `name` 으로만 내고, 대상은 하위 항목 `target:` 이 그대로 채운다.
    // A key in front supplied the name, so the token after the notation is the
    // target whichever spelling was used -- `category: >Category` says both.
    let (name, target) = match (named, arrow) {
        (Some(n), _) => (Some(n.to_string()), Some(token.to_string())),
        (None, true) => (None, Some(token.to_string())),
        (None, false) => (Some(token.to_string()), None),
    };

    Some(RelationNotation {
        direction,
        name,
        target,
        cardinality,
    })
}

/// 분해 결과를 관계 항목 오브젝트에 얹는다.
fn apply_relation_notation(entry: &mut serde_json::Map<String, serde_json::Value>, raw: &str) {
    if let Some(n) = parse_relation_notation(raw) {
        entry.insert("direction".into(), serde_json::json!(n.direction));
        if let Some(name) = n.name {
            entry.insert("name".into(), serde_json::json!(name));
        }
        if let Some(target) = n.target {
            entry.insert("target".into(), serde_json::json!(target));
        }
        if let Some(c) = n.cardinality {
            entry.insert("cardinality".into(), serde_json::json!(c));
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
    //
    // 라벨드 형식 `idx_xxx: @index(col)` / `idx_yyy: @unique(c1, c2)` 도 첨부된 attribute의
    // name/args/unique 정보를 entry에 보존한다. 그래야 SQL/ORM 코드 생성기가 directive 형식
    // (`- @index(col)` / `- @unique(c1, c2)`)과 동등하게 처리 가능.
    // 0.5.5 이전: name/label/loc만 emit → consumer가 인덱스 컬럼을 알 수 없어 SQL emit 누락.
    if section == "Indexes" {
        let mut entry = serde_json::Map::new();
        entry.insert(
            "name".into(),
            serde_json::json!(data.name.clone().unwrap_or_default()),
        );
        if let Some(ref label) = data.label {
            entry.insert("label".into(), serde_json::json!(label));
        }

        // 첫 번째 @index/@unique attribute를 directive와 동등 구조로 emit
        if let Some(attr) = data
            .attributes
            .iter()
            .find(|a| a.name == "index" || a.name == "unique")
        {
            entry.insert("type".into(), serde_json::json!("indexed"));
            entry.insert("attr".into(), serde_json::json!(attr.name.clone()));
            entry.insert("unique".into(), serde_json::json!(attr.name == "unique"));
            if !attr.args.is_empty() {
                entry.insert("args".into(), attr_args_to_json(&attr.args));
            }
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
        entry.insert("raw".into(), serde_json::json!(raw.clone()));
        // 표기를 분해해 함께 싣는다 — 하위 항목(`- target: …`)은 이 뒤에 파싱되므로
        // 명시된 값이 있으면 그쪽이 이긴다(명세가 하위 항목을 정본으로 둔다).
        apply_relation_notation(&mut entry, &raw);
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
    *last_field_idx = Some(usize::MAX); // sentinel for custom section nested items
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
                    attributes: parse_raw_attributes_opt(&data.attributes),
                };
                if let Some(v) = value {
                    let v = strip_trailing_attrs(v, !data.attributes.is_empty());
                    if let Some(caps) = RE_QUOTE_STR.captures(v) {
                        val.description = Some(caps[1].to_string());
                    } else if !v.is_empty() {
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

            // Nested items under custom section entries
            if state.last_field_idx == Some(usize::MAX) {
                let section_name = state.current_section.as_deref().unwrap_or("");
                if section_name != "Indexes"
                    && section_name != "Relations"
                    && !section_name.is_empty()
                {
                    if let Some(k) = key {
                        if let Some(section_arr) = model.sections.custom.get_mut(section_name) {
                            if let serde_json::Value::Array(ref mut arr) = section_arr {
                                if let Some(last) = arr.last_mut() {
                                    if let serde_json::Value::Object(ref mut obj) = last {
                                        obj.insert(
                                            k.to_string(),
                                            parse_nested_value(value.unwrap_or("")),
                                        );
                                    }
                                }
                            }
                        }
                    }
                    return;
                }
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
                                attributes: parse_raw_attributes_opt(&data.attributes),
                            };
                            if let Some(v) = value {
                                let v = strip_trailing_attrs(v, !data.attributes.is_empty());
                                if let Some(caps) = RE_QUOTE_STR.captures(v) {
                                    ev.description = Some(caps[1].to_string());
                                } else if !v.is_empty() {
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
                                    attributes: parse_raw_attributes_opt(&data.attributes),
                                };
                                if let Some(v) = value {
                                    let v = strip_trailing_attrs(v, !data.attributes.is_empty());
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
        CurrentElement::Enum(en) => state.enums.push(*en),
        CurrentElement::Model(model) => match &model.model_type {
            ModelType::Interface => state.interfaces.push(*model),
            ModelType::View => state.views.push(*model),
            ModelType::Flow => state.flows.push(*model),
            ModelType::Extension(ext_type) => {
                state
                    .extensions
                    .entry(ext_type.clone())
                    .or_default()
                    .push(*model);
            }
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
    let (default_value, default_value_type, default_value_quoted, default_value_backtick) =
        process_default_value(data.default_value.as_deref());

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
        default_value_quoted,
        default_value_backtick,
        description: data.description.clone(),
        attributes: attrs.clone(),
        framework_attrs,
        lookup: None,
        rollup: None,
        computed: None,
        binding: match (&data.binding_entity, &data.binding_column) {
            (Some(entity), Some(column)) => Some(BindingDef {
                entity: entity.clone(),
                column: column.clone(),
                is_hard: data.binding_is_hard,
            }),
            _ => None,
        },
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
                let cleaned = unwrap_delimiters(expr, &['"', '\'', '`']);
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
                let cleaned = unwrap_delimiters(&parts.0, &['"', '\'', '`']);
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

/// Returns `(value, type, quoted, backtick)`. `quoted` is `Some(true)` only
/// for a `"..."`-wrapped `Literal` — the flag a formatter needs to tell
/// `= "active"` apart from the bareword `= active` once both are stored as
/// the same unwrapped `value`. `backtick` is `Some(true)` only for a
/// `` `...` ``-wrapped `Expression` — the same problem one level up: a bare
/// `now()` and a backtick-wrapped `` `price * qty` `` both parse to
/// `Expression`, but only the latter needs its delimiter restored on format
/// (without it, re-parsing the un-delimited text can silently truncate at the
/// first non-word character). Both flags are `None` where they don't apply.
fn process_default_value(
    raw: Option<&str>,
) -> (
    Option<String>,
    Option<DefaultValueType>,
    Option<bool>,
    Option<bool>,
) {
    match raw {
        None => (None, None, None, None),
        Some(v) => {
            if v.starts_with('`') && v.ends_with('`') && v.len() >= 2 {
                (
                    Some(v[1..v.len() - 1].to_string()),
                    Some(DefaultValueType::Expression),
                    None,
                    Some(true),
                )
            } else if v.starts_with('"') && v.ends_with('"') && v.len() >= 2 {
                (
                    Some(v[1..v.len() - 1].to_string()),
                    Some(DefaultValueType::Literal),
                    Some(true),
                    None,
                )
            } else {
                let dvt = if v.contains('(') {
                    DefaultValueType::Expression
                } else {
                    DefaultValueType::Literal
                };
                (Some(v.to_string()), Some(dvt), None, None)
            }
        }
    }
}

/// [`parse_raw_attributes`], collapsing the empty case to `None`.
///
/// Enum values keep attributes optional so a value carrying none serializes
/// exactly as it did before `EnumValue::attributes` existed.
/// Strip the trailing attribute run from a nested item's right-hand side.
///
/// `- legacy: "이관" @system` reaches the parser as key `legacy` and value
/// `"이관" @system` — the raw remainder of the line. The attributes have already
/// been lexed, so leaving them in the RHS made the quoted-label match fail and
/// stored `"이관" @system` verbatim as the enum value, losing the label.
///
/// Only strips when the lexer actually found attributes on this line, so an `@`
/// that is genuinely part of a value (an email default, say) is left alone.
fn strip_trailing_attrs(v: &str, had_attrs: bool) -> &str {
    if !had_attrs {
        return v;
    }
    match RE_TRAILING_ATTRS.find(v) {
        Some(m) => v[..m.start()].trim_end(),
        None => v,
    }
}

fn parse_raw_attributes_opt(raw_attrs: &[RawAttribute]) -> Option<Vec<FieldAttribute>> {
    if raw_attrs.is_empty() {
        None
    } else {
        Some(parse_raw_attributes(raw_attrs))
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
            let args_quoted = if a.args_quoted.iter().any(|q| *q) {
                Some(a.args_quoted.clone())
            } else {
                None
            };
            let is_standard = if STANDARD_ATTRIBUTES.contains(a.name.as_str()) {
                Some(true)
            } else {
                None
            };
            FieldAttribute {
                name: a.name.clone(),
                args,
                args_quoted,
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

/// Removes the one pair of delimiters wrapping a value, and nothing else.
///
/// A quote character can also belong to the value — SQL string literals end in
/// one routinely (`metadata->>'category'`) — and trimming by character set eats
/// it, because the set matches again as soon as the outer delimiter is gone.
/// Only a delimiter that is matched at both ends is one; anything else is text.
fn unwrap_delimiters<'a>(value: &'a str, delimiters: &[char]) -> &'a str {
    let mut chars = value.chars();
    match (chars.next(), chars.next_back()) {
        (Some(open), Some(close)) if open == close && delimiters.contains(&open) => {
            &value[open.len_utf8()..value.len() - close.len_utf8()]
        }
        _ => value,
    }
}

fn parse_metadata_value(value: &str) -> serde_json::Value {
    let unquoted = unwrap_delimiters(value, &['"', '\'']);
    let was_quoted = unquoted.len() != value.len();

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
    let unquoted = unwrap_delimiters(s, &['"', '\'']);
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
            let parsed = unwrap_delimiters(value, &['"', '\'']);
            field.description = Some(parsed.to_string());
        }
        "reference" => {
            field.attributes.push(FieldAttribute {
                name: "reference".to_string(),
                args: Some(vec![AttrArgValue::String(value.to_string())]),
                args_quoted: None,
                cascade: None,
                is_standard: Some(true),
                is_registered: None,
            });
        }
        "on_delete" => {
            field.attributes.push(FieldAttribute {
                name: "on_delete".to_string(),
                args: Some(vec![AttrArgValue::String(value.to_string())]),
                args_quoted: None,
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
                args_quoted: None,
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

/// directive(`- @index(...)` / `- @unique(...)` / `- @relation(...)` 등) attribute의
/// args를 JSON으로 직렬화. 인자 수와 무관하게 **항상 array**로 emit하여 consumer가
/// 분기 없이 처리 가능하게 한다.
///
/// 0.5.5 이전: 단일 인자는 raw scalar(string/number/bool), 다인자만 array.
/// 그 결과 모든 consumer가 ValueKind 분기를 강제로 작성해야 하는 부담이 있었음.
fn attr_args_to_json(args: &[AttrArgValue]) -> serde_json::Value {
    serde_json::json!(args
        .iter()
        .map(|a| match a {
            AttrArgValue::String(s) => serde_json::json!(s),
            AttrArgValue::Number(n) => serde_json::json!(n),
            AttrArgValue::Bool(b) => serde_json::json!(b),
        })
        .collect::<Vec<_>>())
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
    fn model_ledger_attribute_is_standard() {
        // Table-level append-only marker. The largest consumer (U-Solutions)
        // turns `@ledger` into REVOKE UPDATE,DELETE + a guard trigger.
        let result = parse_string("## StockMove @ledger\n- id: identifier", "t.m3l.md");
        let m = &result.models[0];
        assert_eq!(m.name, "StockMove");
        let ledger = m
            .attributes
            .iter()
            .find(|a| a.name == "ledger")
            .expect("ledger model attribute");
        assert_eq!(ledger.is_standard, Some(true));
    }

    #[test]
    fn model_attribute_tolerates_backticks() {
        // Authors often wrap attributes in backticks so they render as code in
        // Markdown (`## StockMove `@ledger``). This must parse identically to the
        // bare form — not silently fold the backtick text into the model name.
        let result = parse_string("## StockMove `@ledger`\n- id: identifier", "t.m3l.md");
        let m = &result.models[0];
        assert_eq!(m.name, "StockMove");
        let ledger = m
            .attributes
            .iter()
            .find(|a| a.name == "ledger")
            .expect("ledger attribute parsed from backtick form");
        assert_eq!(ledger.is_standard, Some(true));
    }

    #[test]
    fn rollup_where_clause_is_captured() {
        // Regression: §4.6.5's documented Conditional Rollup (`where: "..."`) parsed
        // without error but `RollupDef.where_clause` stayed None — the generic
        // attribute tokenizer already strips the arg's enclosing quotes before
        // `parse_rollup_args` ever sees it, so the old quote-requiring `RE_WHERE`
        // never matched.
        let input = "## Customer\n- id: identifier @pk\n\n### Rollup\n\
                     - active_orders: integer @rollup(Order.customer_id, count, where: \"status != 'cancelled'\")\n\n\
                     ## Order\n- id: identifier @pk\n- customer_id: identifier @reference(Customer)\n- status: string(20)";
        let result = parse_string(input, "t.m3l.md");
        let customer = result
            .models
            .iter()
            .find(|m| m.name == "Customer")
            .expect("Customer model");
        let field = customer
            .fields
            .iter()
            .find(|f| f.name == "active_orders")
            .expect("active_orders field");
        let rollup = field.rollup.as_ref().expect("parsed RollupDef");
        assert_eq!(rollup.target, "Order");
        assert_eq!(rollup.fk, "customer_id");
        assert_eq!(rollup.aggregate, "count");
        assert_eq!(
            rollup.where_clause.as_deref(),
            Some("status != 'cancelled'")
        );
    }

    #[test]
    fn rollup_where_clause_with_in_list_keeps_internal_commas_and_quotes() {
        let input = "## Order\n- id: identifier @pk\n\n### Rollup\n\
                     - item_count: integer @rollup(OrderItem.order_id, count, where: \"row_type IN ('product', 'print_order')\")\n\n\
                     ## OrderItem\n- id: identifier @pk\n- order_id: identifier @reference(Order)\n- row_type: string(20)";
        let result = parse_string(input, "t.m3l.md");
        let order = result
            .models
            .iter()
            .find(|m| m.name == "Order")
            .expect("Order model");
        let field = order
            .fields
            .iter()
            .find(|f| f.name == "item_count")
            .expect("item_count field");
        let rollup = field.rollup.as_ref().expect("parsed RollupDef");
        assert_eq!(
            rollup.where_clause.as_deref(),
            Some("row_type IN ('product', 'print_order')")
        );
    }

    #[test]
    fn rollup_without_where_still_parses_with_no_where_clause() {
        // The optional-quote regex change must not start inventing a where clause
        // for the plain, unfiltered form.
        let input = "## Foo\n- id: identifier @pk\n\n### Rollup\n\
                     - cnt: integer @rollup(Bar.foo_id, count)\n\n\
                     ## Bar\n- id: identifier @pk\n- foo_id: identifier @reference(Foo)";
        let result = parse_string(input, "t.m3l.md");
        let foo = &result.models[0];
        let field = &foo.fields[1];
        let rollup = field.rollup.as_ref().expect("parsed RollupDef");
        assert_eq!(rollup.where_clause, None);
    }

    #[test]
    fn field_check_attribute_is_standard() {
        let result = parse_string("## Move\n- qty: decimal @check(\"qty <> 0\")", "t.m3l.md");
        let f = &result.models[0].fields[0];
        let chk = f
            .attributes
            .iter()
            .find(|a| a.name == "check")
            .expect("check field attribute");
        assert_eq!(chk.is_standard, Some(true));
    }

    #[test]
    fn model_visibility_attribute_is_not_standard() {
        // `@visibility(level)` was removed from the standard catalog (spec §10.8.3,
        // §2.2.4) — `@public`/`@private` are the only visibility attributes M3L
        // defines. A model using `@visibility` still parses (custom/extension
        // attribute), it just no longer reports `is_standard: Some(true)`.
        let result = parse_string(
            "## Product @visibility(internal)\n- id: identifier",
            "t.m3l.md",
        );
        let m = &result.models[0];
        let visibility = m
            .attributes
            .iter()
            .find(|a| a.name == "visibility")
            .expect("visibility model attribute");
        assert_eq!(visibility.is_standard, None);
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
        assert_eq!(
            result.namespace, None,
            "non-namespace H1 should not set namespace"
        );
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

    /// 0.5.5 — 라벨드 형식 `idx_xxx: @index(col)`도 attribute name/args/unique 정보를
    /// directive 형식과 동등 구조로 보존해야 한다.
    /// 회귀: 0.5.4에서는 `name`만 보존되어 SQL 생성기가 인덱스 컬럼을 알 수 없었음.
    #[test]
    fn parse_section_indexes_labeled_preserves_attribute() {
        let input = "## Order\n- id: identifier\n- customer_id: identifier\n### Indexes\n- idx_customer: @index(customer_id)\n- idx_email: @unique(email, account_id)";
        let result = parse_string(input, "test.m3l.md");
        let indexes = &result.models[0].sections.indexes;
        assert_eq!(indexes.len(), 2);

        let first = &indexes[0];
        assert_eq!(
            first.get("name").and_then(|v| v.as_str()),
            Some("idx_customer")
        );
        assert_eq!(first.get("attr").and_then(|v| v.as_str()), Some("index"));
        assert_eq!(first.get("unique").and_then(|v| v.as_bool()), Some(false));
        let args = first.get("args").expect("args 필수");
        assert!(args.is_array(), "args는 항상 array — 0.5.5+");
        assert_eq!(args.as_array().unwrap()[0].as_str(), Some("customer_id"));

        let second = &indexes[1];
        assert_eq!(second.get("attr").and_then(|v| v.as_str()), Some("unique"));
        assert_eq!(second.get("unique").and_then(|v| v.as_bool()), Some(true));
        let args2 = second.get("args").expect("args 필수").as_array().unwrap();
        assert_eq!(args2.len(), 2);
        assert_eq!(args2[0].as_str(), Some("email"));
        assert_eq!(args2[1].as_str(), Some("account_id"));
    }

    /// 0.5.5 — directive args는 인자 수와 무관하게 항상 array.
    /// 회귀: 0.5.4에서는 단일 인자가 raw scalar로 emit되어 consumer가 ValueKind 분기 강제.
    #[test]
    fn parse_directive_args_always_array() {
        let input = "## Order\n- id: identifier\n- @index(customer_id)\n- @unique(part, season)";
        let result = parse_string(input, "test.m3l.md");
        let indexes = &result.models[0].sections.indexes;
        assert_eq!(indexes.len(), 2);

        let single = indexes[0].get("args").expect("args 필수");
        assert!(
            single.is_array(),
            "단일 인자 directive도 array (0.5.4 결함 fix)"
        );
        assert_eq!(single.as_array().unwrap().len(), 1);
        assert_eq!(single.as_array().unwrap()[0].as_str(), Some("customer_id"));

        let multi = indexes[1].get("args").expect("args 필수");
        assert!(multi.is_array());
        assert_eq!(multi.as_array().unwrap().len(), 2);
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

    #[test]
    fn parse_binding_field_end_to_end() {
        let input = "## Order\n- unit: string? # UserMasterItem.Key \"단위\"";
        let result = parse_string(input, "test.m3l.md");
        let field = &result.models[0].fields[0];
        assert_eq!(field.name, "unit");
        let binding = field.binding.as_ref().expect("binding should be set");
        assert_eq!(binding.entity, "UserMasterItem");
        assert_eq!(binding.column, "Key");
        assert!(!binding.is_hard);
        assert_eq!(field.description.as_deref(), Some("단위"));
    }

    #[test]
    fn parse_binding_field_hard() {
        let input = "## Order\n- status: string # Status.Id! \"상태\"";
        let result = parse_string(input, "test.m3l.md");
        let field = &result.models[0].fields[0];
        assert_eq!(field.name, "status");
        let binding = field.binding.as_ref().expect("binding should be set");
        assert_eq!(binding.entity, "Status");
        assert_eq!(binding.column, "Id");
        assert!(binding.is_hard);
        assert_eq!(field.description.as_deref(), Some("상태"));
    }

    // --- enum value attributes ---------------------------------------------
    // Attributes are a general M3L facility that `FieldNode` has always had.
    // Enum values used to drop them silently, so a model could not say anything
    // about an individual value beyond its name/label/stored value. The parser
    // stays meaning-agnostic here — it records `@whatever`; generators decide
    // what it means (same rule as `@display_labels`).

    #[test]
    fn enum_value_carries_attributes() {
        let input = "## PaymentMethod ::enum\n- cash: \"현금\"\n- legacy: \"이관 정리\" @system";
        let result = parse_string(input, "test.m3l.md");
        let values = &result.enums[0].values;
        assert_eq!(values[0].name, "cash");
        assert!(
            values[0].attributes.is_none(),
            "value without attributes must stay None (JSON stays backward-compatible)"
        );
        let attrs = values[1]
            .attributes
            .as_ref()
            .expect("legacy should carry @system");
        assert_eq!(attrs.len(), 1);
        assert_eq!(attrs[0].name, "system");
        // The attribute text must not bleed into the label.
        assert_eq!(values[1].description.as_deref(), Some("이관 정리"));
    }

    #[test]
    fn enum_value_attribute_args_are_preserved() {
        let input = "## Status ::enum\n- archived: \"보관\" @deprecated(\"use closed\")";
        let result = parse_string(input, "test.m3l.md");
        let attrs = result.enums[0].values[0]
            .attributes
            .as_ref()
            .expect("attributes");
        assert_eq!(attrs[0].name, "deprecated");
        assert!(attrs[0].args.is_some(), "args must survive");
    }

    #[test]
    fn inline_enum_value_carries_attributes() {
        // Inline enums under a field go through a different parser path
        // (handle_nested_item), which must behave identically.
        let input = "## Order\n- method: enum\n  - values:\n    - cash: \"현금\"\n    - legacy: \"이관\" @system";
        let result = parse_string(input, "test.m3l.md");
        let values = result.models[0].fields[0]
            .enum_values
            .as_ref()
            .expect("inline enum values");
        assert_eq!(values.len(), 2);
        assert!(values[0].attributes.is_none());
        let attrs = values[1]
            .attributes
            .as_ref()
            .expect("@system on inline value");
        assert_eq!(attrs[0].name, "system");
        assert_eq!(values[1].description.as_deref(), Some("이관"));
        // Regression: the attribute text used to defeat the quoted-label match,
        // so the whole RHS (`"이관" @system`) was stored as the value and the
        // label was lost.
        assert!(
            values[1].value.is_none(),
            "attribute text must not leak into the stored value"
        );
    }

    #[test]
    fn enum_value_without_attributes_keeps_raw_value() {
        // strip_trailing_attrs only engages when the lexer actually found
        // attributes on the line, so an unattributed stored value is untouched.
        let input = "## Code ::enum\n  - a: A_1\n  - b: B_2";
        let result = parse_string(input, "test.m3l.md");
        let values = &result.enums[0].values;
        assert_eq!(
            values[0].value.as_ref().and_then(|v| v.as_str()),
            Some("A_1")
        );
        assert!(values[0].attributes.is_none());
        assert_eq!(
            values[1].value.as_ref().and_then(|v| v.as_str()),
            Some("B_2")
        );
    }
}
