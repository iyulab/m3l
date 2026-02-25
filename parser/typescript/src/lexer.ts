import type { Token, TokenType } from './types.js';

// --- Regex patterns ---

const RE_H1 = /^# (.+)$/;
const RE_H2 = /^## (.+)$/;
const RE_H3 = /^### (.+)$/;
const RE_HR = /^-{3,}$/;
const RE_BLOCKQUOTE = /^> (.+)$/;
const RE_LIST_ITEM = /^(\s*)- (.+)$/;
const RE_BLANK = /^\s*$/;

// H2 sub-patterns
const RE_TYPE_INDICATOR = /^([\w][\w.]*(?:\([^)]*\))?)\s*::(\w+)(.*)$/;
const RE_MODEL_DEF = /^([\w][\w.]*(?:\([^)]*\))?)\s*(?::\s*(.+?))?(\s+@.+)?$/;

// Field line patterns
const RE_FIELD_NAME = /^([\w]+)(?:\(([^)]*)\))?\s*(?::\s*(.+))?$/;
const RE_TYPE_PART = /^([\w]+)(?:<([^>]+)>)?(?:\(([^)]*)\))?(\?)?(\[\])?(\?)?/;
const RE_FRAMEWORK_ATTR = /`\[([^\]]+)\]`/g;
const RE_INLINE_COMMENT = /\s+#\s+(.+)$/;

// Known kind-context H1 headers
const KIND_SECTIONS = new Set(['Lookup', 'Rollup', 'Computed', 'Computed from Rollup']);

/**
 * Tokenize M3L markdown content into a sequence of tokens.
 */
export function lex(content: string, file: string): Token[] {
  const lines = content.split(/\r?\n/);
  const tokens: Token[] = [];

  for (let i = 0; i < lines.length; i++) {
    const raw = lines[i];
    const lineNum = i + 1;

    // Blank line
    if (RE_BLANK.test(raw)) {
      tokens.push({ type: 'blank', raw, line: lineNum, indent: 0 });
      continue;
    }

    // Horizontal rule
    if (RE_HR.test(raw.trim())) {
      tokens.push({ type: 'horizontal_rule', raw, line: lineNum, indent: 0 });
      continue;
    }

    // H3 — Section header
    const h3Match = raw.match(RE_H3);
    if (h3Match) {
      tokens.push({
        type: 'section',
        raw,
        line: lineNum,
        indent: 0,
        data: { name: h3Match[1].trim() },
      });
      continue;
    }

    // H2 — Model/Enum/Interface/View
    const h2Match = raw.match(RE_H2);
    if (h2Match) {
      const h2Content = h2Match[1].trim();
      tokens.push(tokenizeH2(h2Content, raw, lineNum));
      continue;
    }

    // H1 — Namespace or kind-section context
    const h1Match = raw.match(RE_H1);
    if (h1Match) {
      const h1Content = h1Match[1].trim();
      // Check if this is a kind section (# Lookup, # Rollup, # Computed)
      if (KIND_SECTIONS.has(h1Content)) {
        tokens.push({
          type: 'section',
          raw,
          line: lineNum,
          indent: 0,
          data: { name: h1Content, kind_section: true },
        });
      } else {
        tokens.push({
          type: 'namespace',
          raw,
          line: lineNum,
          indent: 0,
          data: parseNamespace(h1Content),
        });
      }
      continue;
    }

    // Blockquote
    const bqMatch = raw.match(RE_BLOCKQUOTE);
    if (bqMatch) {
      tokens.push({
        type: 'blockquote',
        raw,
        line: lineNum,
        indent: 0,
        data: { text: bqMatch[1].trim() },
      });
      continue;
    }

    // List item (field or nested item)
    const listMatch = raw.match(RE_LIST_ITEM);
    if (listMatch) {
      const indent = listMatch[1].length;
      const itemContent = listMatch[2];

      if (indent >= 2) {
        // Nested item (indented)
        tokens.push({
          type: 'nested_item',
          raw,
          line: lineNum,
          indent,
          data: parseNestedItem(itemContent),
        });
      } else {
        // Top-level list item — field
        tokens.push({
          type: 'field',
          raw,
          line: lineNum,
          indent: 0,
          data: parseFieldLine(itemContent),
        });
      }
      continue;
    }

    // @import directive (top-level)
    const importMatch = raw.trim().match(/^@import\s+["'](.+?)["']\s*$/);
    if (importMatch) {
      tokens.push({
        type: 'text',
        raw,
        line: lineNum,
        indent: 0,
        data: { text: raw.trim(), is_import: true, import_path: importMatch[1] },
      });
      continue;
    }

    // Plain text (model description, etc.)
    tokens.push({
      type: 'text',
      raw,
      line: lineNum,
      indent: 0,
      data: { text: raw.trim() },
    });
  }

  return tokens;
}

function tokenizeH2(content: string, raw: string, line: number): Token {
  // Check for type indicator: ## Name ::enum, ::interface, ::view
  const typeMatch = content.match(RE_TYPE_INDICATOR);
  if (typeMatch) {
    const namepart = typeMatch[1];
    const typeIndicator = typeMatch[2] as 'enum' | 'interface' | 'view';
    const rest = typeMatch[3]?.trim() || '';

    const { name, label } = parseNameLabel(namepart);
    const data: Record<string, unknown> = { name, label };

    if (typeIndicator === 'view') {
      data.materialized = rest.includes('@materialized');
    }

    // Extract description from rest: "description text"
    const descMatch = rest.match(/"([^"]+)"/);
    if (descMatch) {
      data.description = descMatch[1];
    }

    return { type: typeIndicator, raw, line, indent: 0, data };
  }

  // Regular model: ## Name : Parent1, Parent2
  const modelMatch = content.match(RE_MODEL_DEF);
  if (modelMatch) {
    const namepart = modelMatch[1];
    const inheritsStr = modelMatch[2]?.trim();
    const attrsStr = modelMatch[3]?.trim();

    const { name, label } = parseNameLabel(namepart);
    const inherits = inheritsStr
      ? inheritsStr.split(',').map(s => s.trim()).filter(Boolean)
      : [];

    const data: Record<string, unknown> = { name, label, inherits };

    // Parse model-level attributes
    if (attrsStr) {
      const attrs: { name: string; args?: string[] }[] = [];
      const attrRe = /@([\w]+)(?:\(([^)]*)\))?/g;
      let m;
      while ((m = attrRe.exec(attrsStr)) !== null) {
        attrs.push({ name: m[1], args: m[2] ? [m[2]] : undefined });
      }
      data.attributes = attrs;
    }

    return { type: 'model', raw, line, indent: 0, data };
  }

  // Fallback
  return {
    type: 'model',
    raw,
    line,
    indent: 0,
    data: { name: content, inherits: [] },
  };
}

function parseNameLabel(s: string): { name: string; label?: string } {
  const m = s.match(/^([\w][\w.]*)\(([^)]*)\)$/);
  if (m) {
    return { name: m[1], label: m[2] };
  }
  return { name: s };
}

function parseNamespace(content: string): Record<string, unknown> {
  // # Namespace: domain.example
  const nsMatch = content.match(/^Namespace:\s*(.+)$/);
  if (nsMatch) {
    return { name: nsMatch[1].trim(), is_namespace: true };
  }
  // # Document Title
  return { name: content, is_namespace: false };
}

function parseFieldLine(content: string): Record<string, unknown> {
  const data: Record<string, unknown> = {};

  // Check for attribute-only line: @meta(...), @index(...), @relation(...)
  if (content.startsWith('@')) {
    data.is_directive = true;
    data.raw_content = content;
    data.attributes = parseAttributesBalanced(content);
    return data;
  }

  // Strip inline comment
  const commentMatch = content.match(RE_INLINE_COMMENT);
  if (commentMatch) {
    data.comment = commentMatch[1];
    content = content.replace(RE_INLINE_COMMENT, '');
  }

  // Extract framework attributes (backtick-wrapped)
  const frameworkAttrs: string[] = [];
  let fwMatch;
  const fwRe = /`\[([^\]]+)\]`/g;
  while ((fwMatch = fwRe.exec(content)) !== null) {
    frameworkAttrs.push(`[${fwMatch[1]}]`);
  }
  if (frameworkAttrs.length > 0) {
    data.framework_attrs = frameworkAttrs;
    content = content.replace(/`\[[^\]]+\]`/g, '').trim();
  }

  // Special case: NAME "desc" or NAME(label) "desc" (no colon — enum value with description)
  const enumValueMatch = content.match(/^([\w]+)(?:\(([^)]*)\))?\s+"((?:[^"\\]|\\.)*)"$/);
  if (enumValueMatch) {
    data.name = enumValueMatch[1];
    if (enumValueMatch[2]) data.label = enumValueMatch[2];
    data.description = enumValueMatch[3];
    return data;
  }

  // Parse name(label): type_and_rest
  const fieldMatch = content.match(RE_FIELD_NAME);
  if (!fieldMatch) {
    data.name = content;
    return data;
  }

  data.name = fieldMatch[1];
  if (fieldMatch[2]) {
    data.label = fieldMatch[2];
  }

  const rest = fieldMatch[3]?.trim();
  if (!rest) {
    // Name only — could be extended format header or enum value
    return data;
  }

  // Preserve full rest for context-dependent parsing (e.g., source directives)
  data.raw_value = rest;

  // Parse: type(params)?[]? = default @attrs "description"
  parseTypeAndAttrs(rest, data);

  return data;
}

export function parseTypeAndAttrs(rest: string, data: Record<string, unknown>): void {
  let pos = 0;
  const len = rest.length;
  const skipWS = () => { while (pos < len && rest[pos] === ' ') pos++; };

  // Check if the entire rest is a quoted string (e.g., available: "Available")
  if (rest[0] === '"') {
    const closeIdx = findClosingQuote(rest, 0);
    if (closeIdx >= 0 && closeIdx === len - 1) {
      data.description = rest.slice(1, closeIdx);
      return;
    }
  }

  // Parse type: word<generics>?(params)?[]??
  const typeMatch = rest.match(RE_TYPE_PART);
  if (typeMatch) {
    data.type_name = typeMatch[1];
    // Group 2: generic params from <K,V>
    if (typeMatch[2]) {
      data.type_generic_params = typeMatch[2].split(',').map(s => s.trim());
    }
    // Group 3: size/type params from (params)
    if (typeMatch[3]) {
      data.type_params = typeMatch[3].split(',').map(s => s.trim());
    }
    // Group 5: array
    data.array = typeMatch[5] === '[]';
    // Group 4: ? before [] = element nullable; Group 6: ? after [] = container nullable
    if (data.array) {
      data.nullable = typeMatch[6] === '?';
      data.arrayItemNullable = typeMatch[4] === '?';
    } else {
      data.nullable = typeMatch[4] === '?' || typeMatch[6] === '?';
      data.arrayItemNullable = false;
    }
    pos = typeMatch[0].length;
    skipWS();
  }

  // Parse default value: = "quoted" or = unquoted
  if (pos < len && rest[pos] === '=') {
    pos++; // skip =
    skipWS();
    if (pos < len && rest[pos] === '"') {
      const closeIdx = findClosingQuote(rest, pos);
      if (closeIdx >= 0) {
        data.default_value = rest.slice(pos, closeIdx + 1);
        pos = closeIdx + 1;
        skipWS();
      }
    } else {
      // Unquoted default: read until whitespace, @, or "
      const start = pos;
      while (pos < len && rest[pos] !== ' ' && rest[pos] !== '@' && rest[pos] !== '"') {
        if (rest[pos] === '(') {
          const closeP = findBalancedParen(rest, pos);
          pos = closeP >= 0 ? closeP + 1 : pos + 1;
        } else {
          pos++;
        }
      }
      data.default_value = rest.slice(start, pos);
      skipWS();
    }
  }

  // Parse attributes: @name or @name(balanced_args)
  const attrs: { name: string; args?: string }[] = [];
  while (pos < len && rest[pos] === '@') {
    pos++; // skip @
    const nameStart = pos;
    while (pos < len && /\w/.test(rest[pos])) pos++;
    const attrName = rest.slice(nameStart, pos);
    let args: string | undefined;
    if (pos < len && rest[pos] === '(') {
      const closeP = findBalancedParen(rest, pos);
      if (closeP >= 0) {
        args = rest.slice(pos + 1, closeP);
        pos = closeP + 1;
      }
    }
    attrs.push({ name: attrName, args });
    skipWS();
  }
  if (attrs.length > 0) {
    data.attributes = attrs;
  }

  // Trailing description "..."
  skipWS();
  if (pos < len && rest[pos] === '"') {
    const closeIdx = findClosingQuote(rest, pos);
    if (closeIdx >= 0) {
      data.description = rest.slice(pos + 1, closeIdx);
    }
  }
}

function findBalancedParen(str: string, openPos: number): number {
  let depth = 0;
  for (let i = openPos; i < str.length; i++) {
    if (str[i] === '(') {
      depth++;
    } else if (str[i] === ')') {
      depth--;
      if (depth === 0) return i;
    } else if (str[i] === '"') {
      const closeQ = findClosingQuote(str, i);
      if (closeQ >= 0) i = closeQ;
    } else if (str[i] === "'") {
      const closeQ = str.indexOf("'", i + 1);
      if (closeQ >= 0) i = closeQ;
    }
  }
  return -1;
}

function findClosingQuote(str: string, openPos: number): number {
  for (let i = openPos + 1; i < str.length; i++) {
    if (str[i] === '\\') { i++; continue; }
    if (str[i] === '"') return i;
  }
  return -1;
}

function parseAttributesBalanced(content: string): { name: string; args?: string }[] {
  const attrs: { name: string; args?: string }[] = [];
  let pos = 0;
  const len = content.length;
  while (pos < len) {
    const atIdx = content.indexOf('@', pos);
    if (atIdx < 0) break;
    pos = atIdx + 1;
    const nameStart = pos;
    while (pos < len && /\w/.test(content[pos])) pos++;
    const name = content.slice(nameStart, pos);
    if (!name) continue;
    let args: string | undefined;
    if (pos < len && content[pos] === '(') {
      const closeP = findBalancedParen(content, pos);
      if (closeP >= 0) {
        args = content.slice(pos + 1, closeP);
        pos = closeP + 1;
      }
    }
    attrs.push({ name, args });
  }
  return attrs;
}

function parseNestedItem(content: string): Record<string, unknown> {
  // key: value or just value
  const kvMatch = content.match(/^([\w]+)\s*:\s*(.+)$/);
  if (kvMatch) {
    return { key: kvMatch[1], value: kvMatch[2].trim(), raw_content: content };
  }

  // value-only nested item (rare)
  return { raw_content: content };
}
