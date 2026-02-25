/** Source location for error reporting */
export interface SourceLocation {
  file: string;
  line: number;
  col: number;
}

// --- Token types ---

export type TokenType =
  | 'namespace'
  | 'model'
  | 'enum'
  | 'interface'
  | 'view'
  | 'attribute_def'
  | 'section'
  | 'field'
  | 'nested_item'
  | 'blockquote'
  | 'horizontal_rule'
  | 'blank'
  | 'text';

export interface Token {
  type: TokenType;
  raw: string;
  line: number;
  indent: number;
  data?: Record<string, unknown>;
}

// --- AST types ---

export type FieldKind = 'stored' | 'computed' | 'lookup' | 'rollup';

export interface FieldAttribute {
  name: string;
  args?: (string | number | boolean)[];
  cascade?: string;
  /** Whether this is a standard M3L attribute (from the official catalog) */
  isStandard?: boolean;
  /** Whether this attribute is registered in an Attribute Registry (::attribute definition) */
  isRegistered?: boolean;
}

/** Structured representation of a backtick-wrapped framework attribute like `[MaxLength(100)]` */
export interface CustomAttribute {
  /** Content inside brackets, e.g., "MaxLength(100)" for `[MaxLength(100)]` */
  content: string;
  /** Original text including brackets, e.g., "[MaxLength(100)]" */
  raw: string;
  /** Parsed structure — name and arguments extracted from the content */
  parsed?: { name: string; arguments: (string | number | boolean)[] };
}

export interface EnumValue {
  name: string;
  description?: string;
  type?: string;
  value?: unknown;
}

export interface FieldNode {
  name: string;
  label?: string;
  type?: string;
  params?: (string | number)[];
  generic_params?: string[];
  nullable: boolean;
  array: boolean;
  arrayItemNullable: boolean;
  kind: FieldKind;
  default_value?: string;
  description?: string;
  attributes: FieldAttribute[];
  framework_attrs?: CustomAttribute[];
  lookup?: { path: string };
  rollup?: {
    target: string;
    fk: string;
    aggregate: string;
    field?: string;
    where?: string;
  };
  computed?: { expression: string; platform?: string };
  enum_values?: EnumValue[];
  fields?: FieldNode[];
  loc: SourceLocation;
}

export interface ModelNode {
  name: string;
  label?: string;
  type: 'model' | 'enum' | 'interface' | 'view';
  source: string;
  line: number;
  inherits: string[];
  description?: string;
  attributes: FieldAttribute[];
  fields: FieldNode[];
  sections: {
    indexes: unknown[];
    relations: unknown[];
    behaviors: unknown[];
    metadata: Record<string, unknown>;
    [key: string]: unknown;
  };
  materialized?: boolean;
  source_def?: ViewSourceDef;
  refresh?: { strategy: string; interval?: string };
  loc: SourceLocation;
}

export interface ViewSourceDef {
  from: string;
  joins?: { model: string; on: string }[];
  where?: string;
  order_by?: string;
  group_by?: string[];
}

export interface EnumNode {
  name: string;
  label?: string;
  type: 'enum';
  source: string;
  line: number;
  description?: string;
  values: EnumValue[];
  loc: SourceLocation;
}

export interface ProjectInfo {
  name?: string;
  version?: string;
}

export interface Diagnostic {
  code: string;
  severity: 'error' | 'warning';
  file: string;
  line: number;
  col: number;
  message: string;
}

export interface ParsedFile {
  source: string;
  namespace?: string;
  models: ModelNode[];
  enums: EnumNode[];
  interfaces: ModelNode[];
  views: ModelNode[];
  attributeRegistry: AttributeRegistryEntry[];
}

export interface AttributeRegistryEntry {
  /** Attribute name (without @) */
  name: string;
  /** Description */
  description?: string;
  /** Valid targets: 'field', 'model' */
  target: ('field' | 'model')[];
  /** Value type: 'boolean', 'integer', 'string', etc. */
  type: string;
  /** Valid range for numeric types */
  range?: [number, number];
  /** Whether the attribute is required */
  required: boolean;
  /** Default value */
  defaultValue?: string | number | boolean;
}

export interface M3LAST {
  /** Parser package version (semver) */
  parserVersion: string;
  /** AST schema version — incremented on breaking AST structure changes */
  astVersion: string;
  project: ProjectInfo;
  sources: string[];
  models: ModelNode[];
  enums: EnumNode[];
  interfaces: ModelNode[];
  views: ModelNode[];
  /** Attribute registry entries parsed from ::attribute definitions */
  attributeRegistry: AttributeRegistryEntry[];
  errors: Diagnostic[];
  warnings: Diagnostic[];
}

export interface ValidateOptions {
  strict?: boolean;
}

export interface ValidateResult {
  errors: Diagnostic[];
  warnings: Diagnostic[];
}
