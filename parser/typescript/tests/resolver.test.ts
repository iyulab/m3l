import { describe, it, expect } from 'vitest';
import { lex } from '../src/lexer.js';
import { parseTokens } from '../src/parser.js';
import { resolve } from '../src/resolver.js';
import type { ParsedFile } from '../src/types.js';

function parseSource(content: string, file: string): ParsedFile {
  const tokens = lex(content, file);
  return parseTokens(tokens, file);
}

describe('resolver', () => {
  it('should merge multiple file ASTs', () => {
    const a = parseSource('## BaseModel\n- id: identifier @pk', 'a.m3l.md');
    const b = parseSource('## User : BaseModel\n- name: string(100)', 'b.m3l.md');
    const merged = resolve([a, b]);
    expect(merged.models).toHaveLength(2);
    expect(merged.sources).toEqual(['a.m3l.md', 'b.m3l.md']);
  });

  it('should resolve inheritance (copy parent fields)', () => {
    const a = parseSource(
      '## BaseModel\n- id: identifier @pk\n- created_at: timestamp = now()',
      'a.m3l.md'
    );
    const b = parseSource('## User : BaseModel\n- name: string(100)', 'b.m3l.md');
    const merged = resolve([a, b]);
    const user = merged.models.find(m => m.name === 'User')!;
    expect(user.fields.length).toBe(3); // id, created_at, name
    expect(user.fields[0].name).toBe('id');
    expect(user.fields[1].name).toBe('created_at');
    expect(user.fields[2].name).toBe('name');
  });

  it('should resolve multi-level inheritance', () => {
    const src = parseSource([
      '## Timestampable',
      '- created_at: timestamp = now()',
      '## BaseModel : Timestampable',
      '- id: identifier @pk',
      '## User : BaseModel',
      '- name: string(100)',
    ].join('\n'), 'test.m3l.md');
    const merged = resolve([src]);
    const user = merged.models.find(m => m.name === 'User')!;
    expect(user.fields.length).toBe(3); // created_at, id, name
    expect(user.fields[0].name).toBe('created_at');
    expect(user.fields[1].name).toBe('id');
    expect(user.fields[2].name).toBe('name');
  });

  it('should detect duplicate model names as error (E005)', () => {
    const a = parseSource('## User\n- name: string', 'a.m3l.md');
    const b = parseSource('## User\n- email: string', 'b.m3l.md');
    const merged = resolve([a, b]);
    expect(merged.errors.some(e => e.code === 'M3L-E005')).toBe(true);
  });

  it('should categorize enums, interfaces, views into separate arrays', () => {
    const src = parseSource([
      '## Status ::enum',
      '- active: "Active"',
      '## Searchable ::interface',
      '- search_text: text',
      '## Dashboard ::view',
      '### Source',
      '- from: User',
    ].join('\n'), 'test.m3l.md');
    const merged = resolve([src]);
    expect(merged.enums).toHaveLength(1);
    expect(merged.interfaces).toHaveLength(1);
    expect(merged.views).toHaveLength(1);
    expect(merged.models).toHaveLength(0);
  });

  it('should report unresolved inheritance references (E007)', () => {
    const src = parseSource('## User : NonExistent\n- name: string', 'test.m3l.md');
    const merged = resolve([src]);
    expect(merged.errors.some(e => e.code === 'M3L-E007')).toBe(true);
  });

  it('should use namespace as project name', () => {
    const src = parseSource('# Namespace: myproject\n## Model\n- field: string', 'test.m3l.md');
    const merged = resolve([src]);
    expect(merged.project.name).toBe('myproject');
  });

  it('should use provided project info', () => {
    const src = parseSource('## Model\n- field: string', 'test.m3l.md');
    const merged = resolve([src], { name: 'myapp', version: '1.0' });
    expect(merged.project.name).toBe('myapp');
    expect(merged.project.version).toBe('1.0');
  });

  it('should tag isRegistered on attributes matching the registry', () => {
    const src = parseSource([
      '## @pii ::attribute',
      '> Personal info marker',
      '- target: [field]',
      '- type: boolean',
      '',
      '## User',
      '- name: string(100) @searchable',
      '- ssn: string(11) @pii @unique',
    ].join('\n'), 'test.m3l.md');
    const merged = resolve([src]);

    expect(merged.attributeRegistry).toHaveLength(1);
    expect(merged.attributeRegistry[0].name).toBe('pii');

    const user = merged.models.find(m => m.name === 'User')!;
    // @pii should be isRegistered=true
    const piiAttr = user.fields[1].attributes.find(a => a.name === 'pii');
    expect(piiAttr?.isRegistered).toBe(true);

    // @unique is standard but not in registry — isRegistered should be undefined
    const uniqueAttr = user.fields[1].attributes.find(a => a.name === 'unique');
    expect(uniqueAttr?.isRegistered).toBeUndefined();

    // @searchable is standard, not in registry — isRegistered should be undefined
    const searchAttr = user.fields[0].attributes.find(a => a.name === 'searchable');
    expect(searchAttr?.isRegistered).toBeUndefined();
  });

  it('should tag isRegistered across multiple files with registry', () => {
    const registry = parseSource([
      '## @audit_level ::attribute',
      '> Audit compliance level',
      '- target: [field, model]',
      '- type: integer',
    ].join('\n'), 'registry.m3l.md');
    const model = parseSource([
      '## Order @audit_level(3)',
      '- amount: decimal(10,2) @audit_level(5)',
    ].join('\n'), 'model.m3l.md');
    const merged = resolve([registry, model]);

    // Model-level attribute should be tagged
    const order = merged.models.find(m => m.name === 'Order')!;
    const modelAttr = order.attributes.find(a => a.name === 'audit_level');
    expect(modelAttr?.isRegistered).toBe(true);

    // Field-level attribute should be tagged
    const fieldAttr = order.fields[0].attributes.find(a => a.name === 'audit_level');
    expect(fieldAttr?.isRegistered).toBe(true);
  });
});
