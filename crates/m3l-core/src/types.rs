use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Source location
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SourceLocation {
    pub file: String,
    pub line: usize,
    pub col: usize,
}

// ---------------------------------------------------------------------------
// Token types (internal, not serialized to JSON output)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    Namespace,
    Model,
    Enum,
    Interface,
    View,
    AttributeDef,
    Section,
    Field,
    NestedItem,
    Blockquote,
    HorizontalRule,
    Blank,
    Text,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub raw: String,
    pub line: usize,
    pub indent: usize,
    pub data: TokenData,
}

/// Typed token data — replaces the TS `Record<string, unknown>`.
#[derive(Debug, Clone, Default)]
pub struct TokenData {
    // Common
    pub name: Option<String>,
    pub label: Option<String>,
    pub description: Option<String>,
    pub comment: Option<String>,

    // Model / Enum / View / Interface
    pub inherits: Vec<String>,
    pub attributes: Vec<RawAttribute>,
    pub materialized: Option<bool>,

    // Field / Nested item
    pub type_name: Option<String>,
    pub type_params: Vec<ParamValue>,
    pub type_generic_params: Vec<String>,
    pub nullable: bool,
    pub array: bool,
    pub array_item_nullable: bool,
    pub default_value: Option<String>,
    pub is_directive: bool,
    pub is_import: bool,
    pub import_path: Option<String>,
    pub framework_attrs: Vec<String>,
    pub blockquote_desc: Option<String>,
    pub enum_value_description: Option<String>,

    // Section
    pub kind_section: bool,

    // Code block
    pub code_block: Option<CodeBlock>,

    // Nested item key/value
    pub key: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CodeBlock {
    pub language: Option<String>,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct RawAttribute {
    pub name: String,
    pub args: Vec<AttrArgValue>,
    pub cascade: Option<String>,
}

// ---------------------------------------------------------------------------
// AST types (serialized to JSON — field names must match TS output exactly)
// ---------------------------------------------------------------------------

/// Union type for attribute/param arguments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AttrArgValue {
    String(String),
    Number(f64),
    Bool(bool),
}

/// Union type for field type params (string | number).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ParamValue {
    String(String),
    Number(f64),
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldKind {
    #[default]
    Stored,
    Computed,
    Lookup,
    Rollup,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldAttribute {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<AttrArgValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cascade: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "isStandard")]
    pub is_standard: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "isRegistered")]
    pub is_registered: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomAttributeParsed {
    pub name: String,
    pub arguments: Vec<AttrArgValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomAttribute {
    pub content: String,
    pub raw: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parsed: Option<CustomAttributeParsed>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnumValue {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub value_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LookupDef {
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RollupDef {
    pub target: String,
    pub fk: String,
    pub aggregate: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "where")]
    pub where_clause: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComputedDef {
    pub expression: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DefaultValueType {
    Literal,
    Expression,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldNode {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub field_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Vec<ParamValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generic_params: Option<Vec<String>>,
    pub nullable: bool,
    pub array: bool,
    #[serde(rename = "arrayItemNullable")]
    pub array_item_nullable: bool,
    pub kind: FieldKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value_type: Option<DefaultValueType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub attributes: Vec<FieldAttribute>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub framework_attrs: Option<Vec<CustomAttribute>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lookup: Option<LookupDef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rollup: Option<RollupDef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub computed: Option<ComputedDef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<EnumValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<FieldNode>>,
    pub loc: SourceLocation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelType {
    Model,
    Enum,
    Interface,
    View,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JoinDef {
    pub model: String,
    pub on: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewSourceDef {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub joins: Option<Vec<JoinDef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "where")]
    pub where_clause: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_by: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_sql: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RefreshDef {
    pub strategy: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<String>,
}

/// Sections block — always has indexes, relations, behaviors, metadata,
/// plus arbitrary custom sections.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Sections {
    pub indexes: Vec<serde_json::Value>,
    pub relations: Vec<serde_json::Value>,
    pub behaviors: Vec<serde_json::Value>,
    pub metadata: HashMap<String, serde_json::Value>,
    #[serde(flatten)]
    pub custom: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelNode {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(rename = "type")]
    pub model_type: ModelType,
    pub source: String,
    pub line: usize,
    pub inherits: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub attributes: Vec<FieldAttribute>,
    pub fields: Vec<FieldNode>,
    pub sections: Sections,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub materialized: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_def: Option<ViewSourceDef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh: Option<RefreshDef>,
    pub loc: SourceLocation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnumNode {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(rename = "type")]
    pub enum_type: ModelType, // always ModelType::Enum
    pub source: String,
    pub line: usize,
    pub inherits: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub values: Vec<EnumValue>,
    pub loc: SourceLocation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub severity: DiagnosticSeverity,
    pub file: String,
    pub line: usize,
    pub col: usize,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttributeRegistryEntry {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub target: Vec<String>,
    #[serde(rename = "type")]
    pub attr_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<(f64, f64)>,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "defaultValue")]
    pub default_value: Option<AttrArgValue>,
}

/// Intermediate result from parsing a single file (not directly serialized as final output).
#[derive(Debug, Clone)]
pub struct ParsedFile {
    pub source: String,
    pub namespace: Option<String>,
    pub models: Vec<ModelNode>,
    pub enums: Vec<EnumNode>,
    pub interfaces: Vec<ModelNode>,
    pub views: Vec<ModelNode>,
    pub attribute_registry: Vec<AttributeRegistryEntry>,
    /// Import paths found in this file (for circular import detection).
    pub imports: Vec<String>,
}

/// Final AST — top-level JSON output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct M3lAst {
    #[serde(rename = "parserVersion")]
    pub parser_version: String,
    #[serde(rename = "astVersion")]
    pub ast_version: String,
    pub project: ProjectInfo,
    pub sources: Vec<String>,
    pub models: Vec<ModelNode>,
    pub enums: Vec<EnumNode>,
    pub interfaces: Vec<ModelNode>,
    pub views: Vec<ModelNode>,
    #[serde(rename = "attributeRegistry")]
    pub attribute_registry: Vec<AttributeRegistryEntry>,
    pub errors: Vec<Diagnostic>,
    pub warnings: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Default)]
pub struct ValidateOptions {
    pub strict: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidateResult {
    pub errors: Vec<Diagnostic>,
    pub warnings: Vec<Diagnostic>,
}
