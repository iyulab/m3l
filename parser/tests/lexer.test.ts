import { describe, it, expect } from 'vitest';
import { lex } from '../src/lexer.js';

describe('lexer', () => {
  it('should tokenize namespace (H1)', () => {
    const tokens = lex('# Library System', 'test.m3l.md');
    const nonBlank = tokens.filter(t => t.type !== 'blank');
    expect(nonBlank[0].type).toBe('namespace');
    expect(nonBlank[0].data?.name).toBe('Library System');
  });

  it('should tokenize namespace with Namespace: prefix', () => {
    const tokens = lex('# Namespace: domain.example', 'test.m3l.md');
    const nonBlank = tokens.filter(t => t.type !== 'blank');
    expect(nonBlank[0].type).toBe('namespace');
    expect(nonBlank[0].data?.name).toBe('domain.example');
    expect(nonBlank[0].data?.is_namespace).toBe(true);
  });

  it('should tokenize model with inheritance (H2)', () => {
    const tokens = lex('## Author : BaseModel', 'test.m3l.md');
    const nonBlank = tokens.filter(t => t.type !== 'blank');
    expect(nonBlank[0].type).toBe('model');
    expect(nonBlank[0].data?.name).toBe('Author');
    expect(nonBlank[0].data?.inherits).toEqual(['BaseModel']);
  });

  it('should tokenize model with multiple inheritance', () => {
    const tokens = lex('## User : BaseModel, Timestampable', 'test.m3l.md');
    const nonBlank = tokens.filter(t => t.type !== 'blank');
    expect(nonBlank[0].type).toBe('model');
    expect(nonBlank[0].data?.inherits).toEqual(['BaseModel', 'Timestampable']);
  });

  it('should tokenize model with label', () => {
    const tokens = lex('## Product(Products)', 'test.m3l.md');
    const nonBlank = tokens.filter(t => t.type !== 'blank');
    expect(nonBlank[0].type).toBe('model');
    expect(nonBlank[0].data?.name).toBe('Product');
    expect(nonBlank[0].data?.label).toBe('Products');
  });

  it('should tokenize enum definition', () => {
    const tokens = lex('## BookStatus ::enum', 'test.m3l.md');
    const nonBlank = tokens.filter(t => t.type !== 'blank');
    expect(nonBlank[0].type).toBe('enum');
    expect(nonBlank[0].data?.name).toBe('BookStatus');
  });

  it('should tokenize interface definition', () => {
    const tokens = lex('## Searchable ::interface', 'test.m3l.md');
    const nonBlank = tokens.filter(t => t.type !== 'blank');
    expect(nonBlank[0].type).toBe('interface');
    expect(nonBlank[0].data?.name).toBe('Searchable');
  });

  it('should tokenize view definition', () => {
    const tokens = lex('## OverdueLoans ::view', 'test.m3l.md');
    const nonBlank = tokens.filter(t => t.type !== 'blank');
    expect(nonBlank[0].type).toBe('view');
    expect(nonBlank[0].data?.name).toBe('OverdueLoans');
  });

  it('should tokenize view with materialized', () => {
    const tokens = lex('## PopularBooks ::view @materialized', 'test.m3l.md');
    const nonBlank = tokens.filter(t => t.type !== 'blank');
    expect(nonBlank[0].type).toBe('view');
    expect(nonBlank[0].data?.materialized).toBe(true);
  });

  it('should tokenize field with label, type, attributes', () => {
    const tokens = lex('- name(Author Name): string(100) @not_null @idx', 'test.m3l.md');
    const nonBlank = tokens.filter(t => t.type !== 'blank');
    expect(nonBlank[0].type).toBe('field');
    expect(nonBlank[0].data?.name).toBe('name');
    expect(nonBlank[0].data?.label).toBe('Author Name');
    expect(nonBlank[0].data?.type_name).toBe('string');
    expect(nonBlank[0].data?.type_params).toEqual(['100']);
    expect(nonBlank[0].data?.nullable).toBe(false);
  });

  it('should tokenize nullable field', () => {
    const tokens = lex('- bio(Biography): text?', 'test.m3l.md');
    const nonBlank = tokens.filter(t => t.type !== 'blank');
    expect(nonBlank[0].type).toBe('field');
    expect(nonBlank[0].data?.nullable).toBe(true);
    expect(nonBlank[0].data?.type_name).toBe('text');
  });

  it('should tokenize field with default value', () => {
    const tokens = lex('- status: string = "active"', 'test.m3l.md');
    const nonBlank = tokens.filter(t => t.type !== 'blank');
    expect(nonBlank[0].type).toBe('field');
    expect(nonBlank[0].data?.default_value).toBe('"active"');
  });

  it('should tokenize field with numeric default', () => {
    const tokens = lex('- quantity: integer = 0', 'test.m3l.md');
    const nonBlank = tokens.filter(t => t.type !== 'blank');
    expect(nonBlank[0].type).toBe('field');
    expect(nonBlank[0].data?.default_value).toBe('0');
  });

  it('should tokenize field with function default', () => {
    const tokens = lex('- created_at: timestamp = now()', 'test.m3l.md');
    const nonBlank = tokens.filter(t => t.type !== 'blank');
    expect(nonBlank[0].type).toBe('field');
    expect(nonBlank[0].data?.default_value).toBe('now()');
  });

  it('should tokenize blockquote', () => {
    const tokens = lex('> Stores author information.', 'test.m3l.md');
    const nonBlank = tokens.filter(t => t.type !== 'blank');
    expect(nonBlank[0].type).toBe('blockquote');
    expect(nonBlank[0].data?.text).toBe('Stores author information.');
  });

  it('should tokenize section header (H3)', () => {
    const tokens = lex('### Indexes', 'test.m3l.md');
    const nonBlank = tokens.filter(t => t.type !== 'blank');
    expect(nonBlank[0].type).toBe('section');
    expect(nonBlank[0].data?.name).toBe('Indexes');
  });

  it('should tokenize horizontal rule', () => {
    const tokens = lex('---', 'test.m3l.md');
    const nonBlank = tokens.filter(t => t.type !== 'blank');
    expect(nonBlank[0].type).toBe('horizontal_rule');
  });

  it('should tokenize nested item (indented list)', () => {
    const tokens = lex('  - type: string(100)', 'test.m3l.md');
    const nonBlank = tokens.filter(t => t.type !== 'blank');
    expect(nonBlank[0].type).toBe('nested_item');
    expect(nonBlank[0].indent).toBeGreaterThan(0);
    expect(nonBlank[0].data?.key).toBe('type');
    expect(nonBlank[0].data?.value).toBe('string(100)');
  });

  it('should tokenize field with framework attrs in backticks', () => {
    const tokens = lex('- password: string(100) `[JsonIgnore]`', 'test.m3l.md');
    const nonBlank = tokens.filter(t => t.type !== 'blank');
    expect(nonBlank[0].type).toBe('field');
    expect(nonBlank[0].data?.framework_attrs).toContain('[JsonIgnore]');
  });

  it('should tokenize kind section (# Lookup)', () => {
    const tokens = lex('# Lookup', 'test.m3l.md');
    const nonBlank = tokens.filter(t => t.type !== 'blank');
    expect(nonBlank[0].type).toBe('section');
    expect(nonBlank[0].data?.kind_section).toBe(true);
    expect(nonBlank[0].data?.name).toBe('Lookup');
  });

  it('should tokenize kind section (# Rollup)', () => {
    const tokens = lex('# Rollup', 'test.m3l.md');
    const nonBlank = tokens.filter(t => t.type !== 'blank');
    expect(nonBlank[0].type).toBe('section');
    expect(nonBlank[0].data?.kind_section).toBe(true);
  });

  it('should tokenize kind section (# Computed)', () => {
    const tokens = lex('# Computed', 'test.m3l.md');
    const nonBlank = tokens.filter(t => t.type !== 'blank');
    expect(nonBlank[0].type).toBe('section');
    expect(nonBlank[0].data?.kind_section).toBe(true);
  });

  it('should tokenize inline enum field', () => {
    const tokens = lex('- status(Status): enum = "available"', 'test.m3l.md');
    const nonBlank = tokens.filter(t => t.type !== 'blank');
    expect(nonBlank[0].type).toBe('field');
    expect(nonBlank[0].data?.type_name).toBe('enum');
    expect(nonBlank[0].data?.default_value).toBe('"available"');
  });

  it('should tokenize directive-only line', () => {
    const tokens = lex('- @index(email, username, name: "login_lookup")', 'test.m3l.md');
    const nonBlank = tokens.filter(t => t.type !== 'blank');
    expect(nonBlank[0].type).toBe('field');
    expect(nonBlank[0].data?.is_directive).toBe(true);
  });

  it('should tokenize field with description', () => {
    const tokens = lex('- name: string(200) @searchable "Product name"', 'test.m3l.md');
    const nonBlank = tokens.filter(t => t.type !== 'blank');
    expect(nonBlank[0].data?.description).toBe('Product name');
    expect(nonBlank[0].data?.name).toBe('name');
  });

  it('should tokenize full fixture file', () => {
    const content = [
      '# Library System',
      '',
      '## Author : BaseModel',
      '- name(Author Name): string(100) @not_null @idx',
      '- bio(Biography): text?',
      '',
      '> Stores author information.',
      '',
      '## BookStatus ::enum',
      '- available: "Available"',
      '- borrowed: "Borrowed"',
    ].join('\n');
    const tokens = lex(content, 'test.m3l.md');
    const types = tokens.filter(t => t.type !== 'blank').map(t => t.type);
    expect(types).toEqual([
      'namespace', 'model', 'field', 'field', 'blockquote', 'enum', 'field', 'field'
    ]);
  });
});
