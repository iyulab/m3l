// ---------------------------------------------------------------------------
// M3L TypeScript AST Type Definitions
// Generated from crates/m3l-core/src/types.rs
// ---------------------------------------------------------------------------

// --- Result wrapper ---

export interface M3lResult<T> {
  success: boolean;
  data?: T;
  error?: string;
}

// --- Source location ---

export interface SourceLocation {
  file: string;
  line: number;
  col: number;
}

// --- Primitive union types ---

/** Attribute argument value (untagged union: string | number | boolean) */
export type AttrArgValue = string | number | boolean;

/** Field type parameter (untagged union: string | number) */
export type ParamValue = string | number;

// --- Enums ---

/** Field kind — serialized as lowercase string */
export type FieldKind = "stored" | "computed" | "lookup" | "rollup";

/** Model type — serialized as lowercase string */
export type ModelType = "model" | "enum" | "interface" | "view";

/** Diagnostic severity — serialized as lowercase string */
export type DiagnosticSeverity = "error" | "warning";

/** Default value type — serialized as lowercase string */
export type DefaultValueType = "literal" | "expression";

// --- Attribute types ---

export interface FieldAttribute {
  name: string;
  args?: AttrArgValue[];
  cascade?: string;
  isStandard?: boolean;
  isRegistered?: boolean;
}

export interface CustomAttributeParsed {
  name: string;
  arguments: AttrArgValue[];
}

export interface CustomAttribute {
  content: string;
  raw: string;
  parsed?: CustomAttributeParsed;
}

// --- Enum types ---

export interface EnumValue {
  name: string;
  description?: string;
  type?: string;
  value?: unknown;
}

// --- Field definition types ---

export interface LookupDef {
  path: string;
}

export interface RollupDef {
  target: string;
  fk: string;
  aggregate: string;
  field?: string;
  where?: string;
}

export interface ComputedDef {
  expression: string;
  platform?: string;
}

// --- Field node ---

export interface FieldNode {
  name: string;
  label?: string;
  type?: string;
  params?: ParamValue[];
  generic_params?: string[];
  nullable: boolean;
  array: boolean;
  arrayItemNullable: boolean;
  kind: FieldKind;
  default_value?: string;
  default_value_type?: DefaultValueType;
  description?: string;
  attributes: FieldAttribute[];
  framework_attrs?: CustomAttribute[];
  lookup?: LookupDef;
  rollup?: RollupDef;
  computed?: ComputedDef;
  enum_values?: EnumValue[];
  fields?: FieldNode[];
  loc: SourceLocation;
}

// --- View types ---

export interface JoinDef {
  model: string;
  on: string;
}

export interface ViewSourceDef {
  from?: string;
  joins?: JoinDef[];
  where?: string;
  order_by?: string;
  group_by?: string[];
  raw_sql?: string;
  language_hint?: string;
}

export interface RefreshDef {
  strategy: string;
  interval?: string;
}

// --- Sections ---

export interface Sections {
  indexes: unknown[];
  relations: unknown[];
  behaviors: unknown[];
  metadata: Record<string, unknown>;
  /** Additional custom sections (via serde flatten) */
  [key: string]: unknown;
}

// --- Model node ---

export interface ModelNode {
  name: string;
  label?: string;
  type: ModelType;
  source: string;
  line: number;
  inherits: string[];
  description?: string;
  attributes: FieldAttribute[];
  fields: FieldNode[];
  sections: Sections;
  materialized?: boolean;
  source_def?: ViewSourceDef;
  refresh?: RefreshDef;
  loc: SourceLocation;
}

// --- Enum node ---

export interface EnumNode {
  name: string;
  label?: string;
  type: ModelType;
  source: string;
  line: number;
  inherits: string[];
  description?: string;
  values: EnumValue[];
  loc: SourceLocation;
}

// --- Project info ---

export interface ProjectInfo {
  name?: string;
  version?: string;
}

// --- Diagnostics ---

export interface Diagnostic {
  code: string;
  severity: DiagnosticSeverity;
  file: string;
  line: number;
  col: number;
  message: string;
}

// --- Attribute registry ---

export interface AttributeRegistryEntry {
  name: string;
  description?: string;
  target: string[];
  type: string;
  range?: [number, number];
  required: boolean;
  defaultValue?: AttrArgValue;
}

// --- Top-level AST ---

export interface M3lAst {
  parserVersion: string;
  astVersion: string;
  project: ProjectInfo;
  sources: string[];
  models: ModelNode[];
  enums: EnumNode[];
  interfaces: ModelNode[];
  views: ModelNode[];
  attributeRegistry: AttributeRegistryEntry[];
  errors: Diagnostic[];
  warnings: Diagnostic[];
}

// --- Validate result ---

export interface ValidateResult {
  errors: Diagnostic[];
  warnings: Diagnostic[];
}

// --- Lint types ---

/** Lint severity — serialized as lowercase string */
export type LintSeverity = "error" | "warning" | "info";

/** Lint rule level — serialized as lowercase string */
export type RuleLevel = "off" | "warn" | "error";

export interface LintDiagnostic {
  rule: string;
  severity: LintSeverity;
  file: string;
  line: number;
  col: number;
  message: string;
}

export interface LintConfig {
  rules?: Record<string, RuleLevel>;
}

export interface LintResult {
  diagnostics: LintDiagnostic[];
  file_count: number;
}

// --- Input types ---

export interface FileInput {
  content: string;
  filename: string;
}

export interface ValidateOptions {
  strict?: boolean;
  filename?: string;
}

// ---------------------------------------------------------------------------
// Function declarations
// ---------------------------------------------------------------------------

/**
 * Parse a single M3L file and return the AST as JSON.
 *
 * The returned JSON string deserializes to `M3lResult<M3lAst>`.
 *
 * @param content - M3L markdown text
 * @param filename - Source filename for error reporting
 * @returns JSON string with `{ success: boolean, data?: M3lAst, error?: string }`
 */
export function parse(content: string, filename: string): string;

/**
 * Parse multiple M3L files and return the merged AST as JSON.
 *
 * The returned JSON string deserializes to `M3lResult<M3lAst>`.
 *
 * @param filesJson - JSON array of `FileInput` objects
 * @returns JSON string with `{ success: boolean, data?: M3lAst, error?: string }`
 */
export function parseMulti(filesJson: string): string;

/**
 * Validate M3L content and return diagnostics as JSON.
 *
 * The returned JSON string deserializes to `M3lResult<ValidateResult>`.
 *
 * @param content - M3L markdown text
 * @param optionsJson - JSON options (`ValidateOptions`)
 * @returns JSON string with `{ success: boolean, data?: ValidateResult, error?: string }`
 */
export function validate(content: string, optionsJson: string): string;

/**
 * Lint M3L content and return diagnostics as JSON.
 *
 * The returned JSON string deserializes to `M3lResult<LintResult>`.
 *
 * @param content - M3L markdown text
 * @param configJson - JSON config (`LintConfig`)
 * @returns JSON string with `{ success: boolean, data?: LintResult, error?: string }`
 */
export function lint(content: string, configJson: string): string;
