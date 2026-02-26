import type {
  Token,
  ParsedFile,
  ModelNode,
  EnumNode,
  FieldNode,
  FieldAttribute,
  CustomAttribute,
  EnumValue,
  FieldKind,
  ViewSourceDef,
  SourceLocation,
} from './types.js';
import { lex, parseTypeAndAttrs } from './lexer.js';

/**
 * Standard M3L attribute catalog.
 * These are the officially defined attributes in the M3L specification.
 */
export const STANDARD_ATTRIBUTES = new Set([
  // Field constraints
  'primary', 'unique', 'required', 'index', 'generated', 'immutable',
  // References / relations
  'reference', 'fk', 'relation', 'on_update', 'on_delete',
  // Search / display
  'searchable', 'description', 'visibility',
  // Validation
  'min', 'max', 'validate', 'not_null',
  // Derived fields
  'computed', 'computed_raw', 'lookup', 'rollup', 'from', 'persisted',
  // Model-level
  'public', 'private', 'materialized', 'meta', 'behavior', 'override', 'default_attribute',
]);

interface ParserState {
  file: string;
  namespace?: string;
  currentElement: ModelNode | EnumNode | null;
  currentSection: string | null;
  currentKind: FieldKind;
  lastField: FieldNode | null;
  models: ModelNode[];
  enums: EnumNode[];
  interfaces: ModelNode[];
  views: ModelNode[];
  attributeRegistry: import('./types.js').AttributeRegistryEntry[];
  currentAttrDef: { name: string; description?: string; fields: Map<string, unknown> } | null;
  sourceDirectivesDone: boolean;
}

/**
 * Parse M3L content string into a ParsedFile AST.
 */
export function parseString(content: string, file: string): ParsedFile {
  const tokens = lex(content, file);
  return parseTokens(tokens, file);
}

/**
 * Parse a token sequence into a ParsedFile AST.
 */
export function parseTokens(tokens: Token[], file: string): ParsedFile {
  const state: ParserState = {
    file,
    namespace: undefined,
    currentElement: null,
    currentSection: null,
    currentKind: 'stored',
    lastField: null,
    models: [],
    enums: [],
    interfaces: [],
    views: [],
    attributeRegistry: [],
    currentAttrDef: null,
    sourceDirectivesDone: false,
  };

  for (const token of tokens) {
    processToken(token, state);
  }

  // Finalize last element
  finalizeElement(state);

  return {
    source: file,
    namespace: state.namespace,
    models: state.models,
    enums: state.enums,
    interfaces: state.interfaces,
    views: state.views,
    attributeRegistry: state.attributeRegistry,
  };
}

function processToken(token: Token, state: ParserState): void {
  switch (token.type) {
    case 'namespace':
      handleNamespace(token, state);
      break;
    case 'model':
    case 'interface':
      handleModelStart(token, state);
      break;
    case 'enum':
      handleEnumStart(token, state);
      break;
    case 'view':
      handleViewStart(token, state);
      break;
    case 'attribute_def':
      handleAttributeDefStart(token, state);
      break;
    case 'section':
      handleSection(token, state);
      break;
    case 'field':
      handleField(token, state);
      break;
    case 'nested_item':
      handleNestedItem(token, state);
      break;
    case 'blockquote':
      handleBlockquote(token, state);
      break;
    case 'text':
      handleText(token, state);
      break;
    case 'horizontal_rule':
    case 'blank':
      // Ignored
      break;
  }
}

function handleNamespace(token: Token, state: ParserState): void {
  const data = token.data!;
  if (!state.currentElement) {
    state.namespace = data.name as string;
  }
}

function handleModelStart(token: Token, state: ParserState): void {
  finalizeElement(state);

  const data = token.data!;
  const modelAttrs = parseAttributes(data.attributes as { name: string; args?: string }[] | undefined);
  const model: ModelNode = {
    name: data.name as string,
    label: data.label as string | undefined,
    type: token.type as 'model' | 'interface',
    source: state.file,
    line: token.line,
    inherits: (data.inherits as string[]) || [],
    attributes: modelAttrs,
    fields: [],
    sections: {
      indexes: [],
      relations: [],
      behaviors: [],
      metadata: {},
    },
    loc: { file: state.file, line: token.line, col: 1 },
  };

  state.currentElement = model;
  state.currentSection = null;
  state.currentKind = 'stored';
  state.lastField = null;
  state.sourceDirectivesDone = false;
}

function handleEnumStart(token: Token, state: ParserState): void {
  finalizeElement(state);

  const data = token.data!;
  const enumNode: EnumNode = {
    name: data.name as string,
    label: data.label as string | undefined,
    type: 'enum',
    source: state.file,
    line: token.line,
    inherits: (data.inherits as string[]) || [],
    description: data.description as string | undefined,
    values: [],
    loc: { file: state.file, line: token.line, col: 1 },
  };

  state.currentElement = enumNode;
  state.currentSection = null;
  state.currentKind = 'stored';
  state.lastField = null;
}

function handleViewStart(token: Token, state: ParserState): void {
  finalizeElement(state);

  const data = token.data!;
  const view: ModelNode = {
    name: data.name as string,
    label: data.label as string | undefined,
    type: 'view',
    source: state.file,
    line: token.line,
    inherits: [],
    attributes: [],
    materialized: (data.materialized as boolean) || false,
    fields: [],
    sections: {
      indexes: [],
      relations: [],
      behaviors: [],
      metadata: {},
    },
    loc: { file: state.file, line: token.line, col: 1 },
  };

  state.currentElement = view;
  state.currentSection = null;
  state.currentKind = 'stored';
  state.lastField = null;
  state.sourceDirectivesDone = false;
}

function handleSection(token: Token, state: ParserState): void {
  const data = token.data!;
  const sectionName = data.name as string;

  // Kind-context sections (# Lookup, # Rollup, # Computed)
  if (data.kind_section) {
    if (!state.currentElement) return;
    const lower = sectionName.toLowerCase();
    if (lower.startsWith('lookup')) {
      state.currentKind = 'lookup';
    } else if (lower.startsWith('rollup')) {
      state.currentKind = 'rollup';
    } else if (lower.startsWith('computed')) {
      state.currentKind = 'computed';
    }
    state.currentSection = null;
    state.lastField = null;
    return;
  }

  // ### sections
  state.currentSection = sectionName;
  state.lastField = null;

  // Reset source directives tracking for views
  if (sectionName === 'Source' && state.currentElement?.type === 'view') {
    state.sourceDirectivesDone = false;
  }
}

function handleField(token: Token, state: ParserState): void {
  // Handle attribute definition fields (- target: [field, model])
  if (state.currentAttrDef) {
    const data = token.data!;
    const name = data.name as string;
    const raw = token.raw.trim().replace(/^-\s*/, '');
    const colonIdx = raw.indexOf(':');
    if (colonIdx >= 0) {
      const value = raw.substring(colonIdx + 1).trim();
      state.currentAttrDef.fields.set(name, value);
    }
    return;
  }

  if (!state.currentElement) return;

  const data = token.data!;

  // Handle enum element
  if (isEnumNode(state.currentElement)) {
    const enumVal: EnumValue = {
      name: data.name as string,
      description: data.description as string | undefined,
    };
    if (data.type_name && data.type_name !== 'enum') {
      enumVal.type = data.type_name as string;
    }
    if (data.default_value !== undefined) {
      enumVal.value = data.default_value;
    }
    // If no explicit type but has a quoted string after colon, treat as description
    if (!enumVal.description && data.type_name) {
      const raw = data.type_name as string;
      const strMatch = raw.match(/^"(.*)"$/);
      if (strMatch) {
        enumVal.description = strMatch[1];
        enumVal.type = undefined;
      }
    }
    state.currentElement.values.push(enumVal);
    return;
  }

  const model = state.currentElement as ModelNode;

  // Handle directive-only lines (@index, @relation, etc.)
  if (data.is_directive) {
    handleDirective(data, model, token, state);
    return;
  }

  // Handle section-specific items
  if (state.currentSection) {
    handleSectionItem(data, model, token, state);
    return;
  }

  // Regular field
  const field = buildFieldNode(data, token, state);
  model.fields.push(field);
  state.lastField = field;
}

function handleDirective(
  data: Record<string, unknown>,
  model: ModelNode,
  token: Token,
  state: ParserState
): void {
  const attrs = data.attributes as { name: string; args?: string }[];
  if (!attrs || attrs.length === 0) return;

  const attr = attrs[0];
  if (attr.name === 'index' || attr.name === 'unique') {
    model.sections.indexes.push({
      type: 'directive',
      raw: data.raw_content,
      args: attr.args,
      unique: attr.name === 'unique',
      loc: { file: state.file, line: token.line, col: 1 },
    });
  } else if (attr.name === 'relation') {
    model.sections.relations.push({
      type: 'directive',
      raw: data.raw_content,
      args: attr.args,
      loc: { file: state.file, line: token.line, col: 1 },
    });
  } else {
    // Generic directive — normalize singular form
    let sectionName = attr.name;
    if (sectionName === 'behavior') sectionName = 'behaviors';

    if (sectionName === 'behaviors') {
      model.sections.behaviors.push({
        raw: data.raw_content,
        args: attr.args,
      });
    } else {
      if (!model.sections[sectionName]) {
        model.sections[sectionName] = [];
      }
      (model.sections[sectionName] as unknown[]).push({
        raw: data.raw_content,
        args: attr.args,
      });
    }
  }
}

function handleSectionItem(
  data: Record<string, unknown>,
  model: ModelNode,
  token: Token,
  state: ParserState
): void {
  const section = state.currentSection!;
  const loc: SourceLocation = { file: state.file, line: token.line, col: 1 };

  // View Source section — handle directives and fields
  if (section === 'Source' && model.type === 'view') {
    const name = data.name as string;

    // Source directives: from, where, order_by, group_by, join
    if (isSourceDirective(name) && !state.sourceDirectivesDone) {
      if (!model.source_def) {
        model.source_def = { from: '' };
      }
      setSourceDirective(model.source_def, data);
      return;
    }

    // Once we hit a non-directive field, mark source directives as done
    state.sourceDirectivesDone = true;

    // View field
    const field = buildFieldNode(data, token, state);
    model.fields.push(field);
    state.lastField = field;
    return;
  }

  // Refresh section
  if (section === 'Refresh' && model.type === 'view') {
    if (!model.refresh) {
      model.refresh = { strategy: '' };
    }
    const name = data.name as string;
    const typeName = data.type_name as string | undefined;
    const desc = data.description as string | undefined;
    // Parse key: value pattern — these come as name: type_name in the lexer
    if (name === 'strategy') {
      model.refresh.strategy = typeName || '';
    } else if (name === 'interval') {
      model.refresh.interval = desc || typeName || '';
    }
    return;
  }

  // Indexes section
  if (section === 'Indexes') {
    model.sections.indexes.push({
      name: data.name,
      label: data.label,
      loc,
    });
    state.lastField = { name: data.name as string } as FieldNode;
    return;
  }

  // Relations section
  if (section === 'Relations') {
    model.sections.relations.push({
      raw: token.raw.trim().replace(/^- /, ''),
      loc,
    });
    return;
  }

  // Metadata section
  if (section === 'Metadata') {
    const name = data.name as string;
    const value = data.type_name as string ?? data.description;
    model.sections.metadata[name] = parseMetadataValue(value);
    return;
  }

  // Behaviors section
  if (section === 'Behaviors') {
    model.sections.behaviors.push({
      name: data.name,
      raw: token.raw.trim(),
      loc,
    });
    return;
  }

  // Generic section — store as section items, NOT as fields
  if (!model.sections[section]) {
    model.sections[section] = [];
  }
  (model.sections[section] as unknown[]).push({
    name: data.name,
    raw: token.raw.trim(),
    value: data.type_name || data.description || data.raw_value,
    loc,
  });
  state.lastField = null;
}

function handleNestedItem(token: Token, state: ParserState): void {
  if (!state.currentElement) return;

  const data = token.data!;
  const key = data.key as string | undefined;
  const value = data.value as string | undefined;

  // Nested items under an enum — enum values
  if (isEnumNode(state.currentElement)) {
    if (key) {
      const val: EnumValue = { name: key };
      const strMatch = value?.match(/^"(.*)"$/);
      if (strMatch) {
        val.description = strMatch[1];
      } else if (value) {
        val.value = value;
      }
      state.currentElement.values.push(val);
    }
    return;
  }

  const model = state.currentElement as ModelNode;

  // Nested items under index in Indexes section
  if (state.currentSection === 'Indexes' && state.lastField) {
    const lastIndex = model.sections.indexes[model.sections.indexes.length - 1];
    if (lastIndex && typeof lastIndex === 'object') {
      if (key) {
        (lastIndex as Record<string, unknown>)[key] = parseNestedValue(value || '');
      }
    }
    return;
  }

  // Nested items under a field
  if (state.lastField) {
    const field = state.lastField;

    // values: key for inline enum
    if (key === 'values' && !value) {
      // Next nested items will be enum values — mark field
      if (!field.enum_values) {
        field.enum_values = [];
      }
      return;
    }

    // If field has enum_values (after values: key), add to it
    if (field.enum_values && key) {
      const strMatch = value?.match(/^"(.*)"$/);
      field.enum_values.push({
        name: key,
        description: strMatch ? strMatch[1] : undefined,
        value: strMatch ? undefined : value,
      });
      return;
    }

    // Inline enum without values: key (legacy)
    if (field.type === 'enum' && key && !value?.includes(':')) {
      if (!field.enum_values) {
        field.enum_values = [];
      }
      const strMatch = value?.match(/^"(.*)"$/);
      field.enum_values.push({
        name: key,
        description: strMatch ? strMatch[1] : undefined,
      });
      return;
    }

    // Sub-field for object/nested type
    if (key && value) {
      // Walk up to find the nearest object-type ancestor for this indent level
      const parentField = field;
      if (parentField.type === 'object') {
        if (!parentField.fields) parentField.fields = [];
        // Re-parse value as type and attributes
        const subData: Record<string, unknown> = { name: key };
        parseTypeAndAttrs(value, subData);
        // Only treat as sub-field if a type was extracted
        if (subData.type_name) {
          const subField = buildFieldNode(subData, token, state);
          parentField.fields.push(subField);
          // If the sub-field is also an object, set it as lastField for deeper nesting
          if (subField.type === 'object') {
            state.lastField = subField;
          }
          // Otherwise keep lastField pointing to the parent object for siblings
          return;
        }
      }
    }

    // Extended format field attributes
    if (key) {
      applyExtendedAttribute(field, key, value || '');
    }
    return;
  }

  // Source section nested items for views
  if (state.currentSection === 'Source' && model.type === 'view') {
    if (key && model.source_def) {
      setSourceDirective(model.source_def, { name: key, type_name: value });
    }
    return;
  }
}

function handleBlockquote(token: Token, state: ParserState): void {
  // Handle attribute definition description
  if (state.currentAttrDef) {
    const text = token.data!.text as string;
    state.currentAttrDef.description = text;
    return;
  }
  if (!state.currentElement) return;
  const text = token.data!.text as string;
  if (state.currentElement.description) {
    state.currentElement.description += '\n' + text;
  } else {
    state.currentElement.description = text;
  }
}

function handleText(token: Token, state: ParserState): void {
  // Plain text before fields — model description
  if (state.currentElement && !isEnumNode(state.currentElement)) {
    const model = state.currentElement as ModelNode;
    if (model.fields.length === 0) {
      const text = (token.data!.text as string) || '';
      if (text && !model.description) {
        model.description = text;
      }
    }
  }
}

function finalizeElement(state: ParserState): void {
  // Finalize pending attribute definition
  finalizeAttrDef(state);

  if (!state.currentElement) return;

  if (isEnumNode(state.currentElement)) {
    state.enums.push(state.currentElement);
  } else {
    const model = state.currentElement as ModelNode;
    switch (model.type) {
      case 'interface':
        state.interfaces.push(model);
        break;
      case 'view':
        state.views.push(model);
        break;
      default:
        state.models.push(model);
        break;
    }
  }

  state.currentElement = null;
  state.currentSection = null;
  state.currentKind = 'stored';
  state.lastField = null;
}

function handleAttributeDefStart(token: Token, state: ParserState): void {
  finalizeElement(state);
  const data = token.data || {};
  const name = (data.name as string || '').replace(/^@/, '');

  state.currentAttrDef = {
    name,
    description: data.description as string | undefined,
    fields: new Map(),
  };
  state.currentElement = null;
}

function finalizeAttrDef(state: ParserState): void {
  if (!state.currentAttrDef) return;
  const def = state.currentAttrDef;
  const fields = def.fields;

  const targetRaw = fields.get('target') as string | undefined;
  const target: ('field' | 'model')[] = [];
  if (targetRaw) {
    const cleaned = targetRaw.replace(/^\[|\]$/g, '').split(',').map(s => s.trim());
    for (const t of cleaned) {
      if (t === 'field' || t === 'model') target.push(t);
    }
  }

  const rangeRaw = fields.get('range') as string | undefined;
  let range: [number, number] | undefined;
  if (rangeRaw) {
    const nums = rangeRaw.replace(/^\[|\]$/g, '').split(',').map(s => Number(s.trim()));
    if (nums.length === 2 && !isNaN(nums[0]) && !isNaN(nums[1])) {
      range = [nums[0], nums[1]];
    }
  }

  const requiredRaw = fields.get('required');
  const required = requiredRaw === 'true' || requiredRaw === true;

  const defaultRaw = fields.get('default');
  let defaultValue: string | number | boolean | undefined;
  if (defaultRaw !== undefined) {
    if (defaultRaw === 'true') defaultValue = true;
    else if (defaultRaw === 'false') defaultValue = false;
    else if (typeof defaultRaw === 'string' && !isNaN(Number(defaultRaw))) defaultValue = Number(defaultRaw);
    else if (typeof defaultRaw === 'string') defaultValue = defaultRaw;
    else if (typeof defaultRaw === 'number' || typeof defaultRaw === 'boolean') defaultValue = defaultRaw;
  }

  const entry: import('./types.js').AttributeRegistryEntry = {
    name: def.name,
    target: target.length > 0 ? target : ['field'],
    type: (fields.get('type') as string) || 'boolean',
    required,
  };
  if (def.description) entry.description = def.description;
  if (range) entry.range = range;
  if (defaultValue !== undefined) entry.defaultValue = defaultValue;

  state.attributeRegistry.push(entry);
  state.currentAttrDef = null;
}

// --- Helpers ---

function buildFieldNode(
  data: Record<string, unknown>,
  token: Token,
  state: ParserState
): FieldNode {
  const attrs = parseAttributes(data.attributes as { name: string; args?: string }[] | undefined);
  let kind: FieldKind = state.currentKind;

  // Detect kind from attributes
  const lookupAttr = attrs.find(a => a.name === 'lookup');
  const rollupAttr = attrs.find(a => a.name === 'rollup');
  const computedAttr = attrs.find(a => a.name === 'computed');
  const computedRawAttr = attrs.find(a => a.name === 'computed_raw');
  const fromAttr = attrs.find(a => a.name === 'from');

  if (lookupAttr) kind = 'lookup';
  else if (rollupAttr) kind = 'rollup';
  else if (computedAttr) kind = 'computed';
  else if (computedRawAttr) kind = 'computed';

  const field: FieldNode = {
    name: data.name as string,
    label: data.label as string | undefined,
    type: data.type_name as string | undefined,
    params: parseTypeParams(data.type_params as string[] | undefined),
    generic_params: data.type_generic_params as string[] | undefined,
    nullable: (data.nullable as boolean) || false,
    array: (data.array as boolean) || false,
    arrayItemNullable: (data.arrayItemNullable as boolean) || false,
    kind,
    default_value: data.default_value as string | undefined,
    description: data.description as string | undefined,
    attributes: attrs,
    framework_attrs: parseCustomAttributes(data.framework_attrs as string[] | undefined),
    loc: { file: state.file, line: token.line, col: 1 },
  };

  // Parse lookup
  if (lookupAttr && lookupAttr.args?.[0]) {
    field.lookup = { path: lookupAttr.args[0] as string };
  }

  // Parse rollup: @rollup(Target.fk, aggregate) or @rollup(Target.fk, aggregate(field))
  if (rollupAttr && rollupAttr.args?.[0]) {
    field.rollup = parseRollupArgs(rollupAttr.args[0] as string);
  }

  // Parse computed: @computed("expression")
  if (computedAttr && computedAttr.args?.[0]) {
    const expr = (computedAttr.args[0] as string).replace(/^["']|["']$/g, '');
    field.computed = { expression: expr };
  }

  // Parse computed_raw: @computed_raw("expression", platform: "name")
  if (computedRawAttr && computedRawAttr.args?.[0]) {
    const rawArgs = computedRawAttr.args[0] as string;
    const parts = splitComputedRawArgs(rawArgs);
    const expr = parts.expression.replace(/^["']|["']$/g, '');
    field.computed = { expression: expr };
    if (parts.platform) {
      field.computed.platform = parts.platform;
    }
  }

  return field;
}

/**
 * Split @computed_raw args: "expr", platform: "name"
 * Returns the expression (first positional arg) and optional named params.
 */
function splitComputedRawArgs(raw: string): { expression: string; platform?: string } {
  // Find the first quoted string as the expression
  const quoteChar = raw[0];
  if (quoteChar === '"' || quoteChar === "'") {
    // Find matching closing quote (not escaped)
    let i = 1;
    while (i < raw.length) {
      if (raw[i] === '\\') { i += 2; continue; }
      if (raw[i] === quoteChar) break;
      i++;
    }
    const expression = raw.slice(0, i + 1); // includes quotes
    const remainder = raw.slice(i + 1).trim();
    // Parse named params from remainder: , platform: "sqlserver"
    let platform: string | undefined;
    const platformMatch = remainder.match(/platform\s*:\s*["']([^"']+)["']/);
    if (platformMatch) {
      platform = platformMatch[1];
    }
    return { expression, platform };
  }
  // No quotes — treat entire string as expression
  return { expression: raw };
}

function parseAttributes(
  rawAttrs: { name: string; args?: string; cascade?: string }[] | undefined
): FieldAttribute[] {
  if (!rawAttrs) return [];
  return rawAttrs.map(a => {
    const attr: FieldAttribute = { name: a.name };
    if (a.args) attr.args = [a.args];
    if (a.cascade) attr.cascade = a.cascade;
    if (STANDARD_ATTRIBUTES.has(a.name)) attr.isStandard = true;
    return attr;
  });
}

function parseTypeParams(params: string[] | undefined): (string | number)[] | undefined {
  if (!params) return undefined;
  return params.map(p => {
    const n = Number(p);
    return isNaN(n) ? p : n;
  });
}

function parseRollupArgs(argsStr: string): {
  target: string;
  fk: string;
  aggregate: string;
  field?: string;
  where?: string;
} {
  // Pattern: Target.fk, aggregate(field)?, where: "condition"
  const parts = splitRollupArgs(argsStr);

  const targetFk = parts[0] || '';
  const dotIdx = targetFk.indexOf('.');
  const target = dotIdx >= 0 ? targetFk.substring(0, dotIdx) : targetFk;
  const fk = dotIdx >= 0 ? targetFk.substring(dotIdx + 1) : '';

  let aggregate = '';
  let field: string | undefined;

  if (parts.length > 1) {
    const aggPart = parts[1].trim();
    const aggMatch = aggPart.match(/^(\w+)(?:\((\w+)\))?$/);
    if (aggMatch) {
      aggregate = aggMatch[1];
      field = aggMatch[2];
    } else {
      aggregate = aggPart;
    }
  }

  let where: string | undefined;
  for (let i = 2; i < parts.length; i++) {
    const part = parts[i].trim();
    const whereMatch = part.match(/^where:\s*"(.*)"$/);
    if (whereMatch) {
      where = whereMatch[1];
    }
  }

  return { target, fk, aggregate, field, where };
}

function splitRollupArgs(argsStr: string): string[] {
  const parts: string[] = [];
  let current = '';
  let depth = 0;
  let inQuote = false;
  let quoteChar = '';

  for (const ch of argsStr) {
    if (inQuote) {
      current += ch;
      if (ch === quoteChar) inQuote = false;
      continue;
    }
    if (ch === '"' || ch === "'") {
      inQuote = true;
      quoteChar = ch;
      current += ch;
      continue;
    }
    if (ch === '(') {
      depth++;
      current += ch;
      continue;
    }
    if (ch === ')') {
      depth--;
      current += ch;
      continue;
    }
    if (ch === ',' && depth === 0) {
      parts.push(current.trim());
      current = '';
      continue;
    }
    current += ch;
  }
  if (current.trim()) {
    parts.push(current.trim());
  }
  return parts;
}

function isSourceDirective(name: string): boolean {
  return ['from', 'where', 'order_by', 'group_by', 'join'].includes(name);
}

function setSourceDirective(
  def: ViewSourceDef,
  data: Record<string, unknown>
): void {
  const name = data.name as string;
  const typeName = data.type_name as string | undefined;
  const desc = data.description as string | undefined;
  const rawValue = data.raw_value as string | undefined;
  // desc: quoted values with quotes stripped by lexer
  // rawValue: full rest-of-line preserving spaces (e.g., "due_date asc")
  // typeName: first word parsed as type
  const value = desc || rawValue || typeName || '';

  switch (name) {
    case 'from':
      def.from = value;
      break;
    case 'where':
      def.where = value;
      break;
    case 'order_by':
      def.order_by = value;
      break;
    case 'group_by':
      def.group_by = parseArrayValue(value);
      break;
    case 'join':
      if (!def.joins) def.joins = [];
      def.joins.push(parseJoinValue(value));
      break;
  }
}

function parseJoinValue(value: string): { model: string; on: string } {
  // "Model on condition"
  const parts = value.split(/\s+on\s+/i);
  return {
    model: parts[0]?.trim() || '',
    on: parts[1]?.trim() || '',
  };
}

function parseArrayValue(value: string): string[] {
  // [a, b, c] or a, b, c
  const cleaned = value.replace(/^\[|\]$/g, '');
  return cleaned.split(',').map(s => s.trim()).filter(Boolean);
}

function parseMetadataValue(value: unknown): unknown {
  if (typeof value !== 'string') return value;
  const str = value as string;
  // If explicitly quoted, preserve as string (don't coerce "1.0" to number)
  const wasQuoted = (str.startsWith('"') && str.endsWith('"')) ||
                    (str.startsWith("'") && str.endsWith("'"));
  const unquoted = str.replace(/^["']|["']$/g, '');
  if (wasQuoted) return unquoted;
  // Try number
  const n = Number(unquoted);
  if (!isNaN(n) && unquoted !== '') return n;
  // Try boolean
  if (unquoted === 'true') return true;
  if (unquoted === 'false') return false;
  return unquoted;
}

function parseNestedValue(value: string): unknown {
  const str = value.trim();
  // Array: [a, b, c]
  if (str.startsWith('[') && str.endsWith(']')) {
    return parseArrayValue(str);
  }
  // Boolean
  if (str === 'true') return true;
  if (str === 'false') return false;
  // Number
  const n = Number(str);
  if (!isNaN(n) && str !== '') return n;
  // String (strip quotes)
  return str.replace(/^["']|["']$/g, '');
}

function applyExtendedAttribute(field: FieldNode, key: string, value: string): void {
  const parsed = parseNestedValue(value);
  switch (key) {
    case 'type':
      field.type = value.replace(/\?$/, '').replace(/\[\]$/, '');
      if (value.endsWith('?')) field.nullable = true;
      if (value.endsWith('[]')) field.array = true;
      break;
    case 'description':
      field.description = typeof parsed === 'string' ? parsed : String(parsed);
      break;
    case 'reference':
      field.attributes.push({ name: 'reference', args: [value] });
      break;
    case 'on_delete':
      field.attributes.push({ name: 'on_delete', args: [value] });
      break;
    default:
      field.attributes.push({ name: key, args: [parsed as string | number | boolean] });
      break;
  }
}

function parseCustomAttributes(rawAttrs: string[] | undefined): CustomAttribute[] | undefined {
  if (!rawAttrs || rawAttrs.length === 0) return undefined;
  return rawAttrs.map(raw => {
    // raw is "[MaxLength(100)]" — strip brackets to get content
    const content = raw.replace(/^\[|\]$/g, '');
    const attr: CustomAttribute = { content, raw };

    // Try to parse "Name(arg1, arg2)" or "Name" pattern
    const match = content.match(/^([A-Za-z_][\w.]*)(?:\((.+)\))?$/);
    if (match) {
      const name = match[1];
      const argsStr = match[2];
      const args: (string | number | boolean)[] = [];
      if (argsStr) {
        for (const part of splitBalanced(argsStr)) {
          const trimmed = part.trim();
          args.push(parseArgValue(trimmed));
        }
      }
      attr.parsed = { name, arguments: args };
    }

    return attr;
  });
}

/** Split a string by commas, respecting balanced parentheses */
function splitBalanced(s: string): string[] {
  const parts: string[] = [];
  let depth = 0;
  let start = 0;
  for (let i = 0; i < s.length; i++) {
    if (s[i] === '(') depth++;
    else if (s[i] === ')') depth--;
    else if (s[i] === ',' && depth === 0) {
      parts.push(s.substring(start, i));
      start = i + 1;
    }
  }
  parts.push(s.substring(start));
  return parts;
}

/** Parse a single argument value: numbers, booleans, or strings */
function parseArgValue(s: string): string | number | boolean {
  if (s === 'true') return true;
  if (s === 'false') return false;
  const n = Number(s);
  if (!isNaN(n) && s.length > 0) return n;
  // Strip surrounding quotes if present
  if ((s.startsWith('"') && s.endsWith('"')) || (s.startsWith("'") && s.endsWith("'"))) {
    return s.slice(1, -1);
  }
  return s;
}

function isEnumNode(el: ModelNode | EnumNode): el is EnumNode {
  return el.type === 'enum' && 'values' in el;
}
