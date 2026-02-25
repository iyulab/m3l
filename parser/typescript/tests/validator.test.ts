import { describe, it, expect } from 'vitest';
import { lex } from '../src/lexer.js';
import { parseTokens } from '../src/parser.js';
import { resolve } from '../src/resolver.js';
import { validate } from '../src/validator.js';

function buildAST(content: string, file = 'test.m3l.md') {
  const tokens = lex(content, file);
  const parsed = parseTokens(tokens, file);
  return resolve([parsed]);
}

describe('validator', () => {
  it('E001: @rollup FK missing @reference', () => {
    const ast = buildAST([
      '## Author',
      '- name: string(100)',
      '### Rollup',
      '- book_count: integer @rollup(Book.author_id, count)',
      '## Book',
      '- title: string(200)',
      '- author_id: identifier',
    ].join('\n'));
    const result = validate(ast);
    expect(result.errors.some(e => e.code === 'M3L-E001')).toBe(true);
  });

  it('E001: should pass when FK has @fk', () => {
    const ast = buildAST([
      '## Author',
      '- name: string(100)',
      '### Rollup',
      '- book_count: integer @rollup(Book.author_id, count)',
      '## Book',
      '- title: string(200)',
      '- author_id: identifier @fk(Author.id)',
    ].join('\n'));
    const result = validate(ast);
    expect(result.errors.filter(e => e.code === 'M3L-E001')).toHaveLength(0);
  });

  it('E002: @lookup path FK missing @reference', () => {
    const ast = buildAST([
      '## Order',
      '- customer_id: identifier',
      '### Lookup',
      '- customer_name: string @lookup(customer_id.name)',
    ].join('\n'));
    const result = validate(ast);
    expect(result.errors.some(e => e.code === 'M3L-E002')).toBe(true);
  });

  it('E002: should pass when FK has @fk', () => {
    const ast = buildAST([
      '## Customer',
      '- id: identifier @pk',
      '- name: string(100)',
      '## Order',
      '- customer_id: identifier @fk(Customer.id)',
      '### Lookup',
      '- customer_name: string @lookup(customer_id.name)',
    ].join('\n'));
    const result = validate(ast);
    expect(result.errors.filter(e => e.code === 'M3L-E002')).toHaveLength(0);
  });

  it('E004: View references non-existent model', () => {
    const ast = buildAST([
      '## SomeView ::view',
      '### Source',
      '- from: NonExistentModel',
    ].join('\n'));
    const result = validate(ast);
    expect(result.errors.some(e => e.code === 'M3L-E004')).toBe(true);
  });

  it('M3L-E006: duplicate field names within a model', () => {
    const ast = buildAST([
      '## User',
      '- name: string(100)',
      '- name: string(200)',
    ].join('\n'));
    const result = validate(ast);
    expect(result.errors.some(e => e.code === 'M3L-E006')).toBe(true);
  });

  it('should pass validation on clean input', () => {
    const ast = buildAST([
      '## User',
      '- id: identifier @pk',
      '- name: string(100)',
      '- email: string(320) @unique',
    ].join('\n'));
    const result = validate(ast);
    expect(result.errors).toHaveLength(0);
  });

  it('W001: field line length >80 chars (strict mode)', () => {
    const ast = buildAST([
      '## Model',
      '- very_long_field_name_here: string(200) @not_null @unique @searchable @index "A very long description"',
    ].join('\n'));
    const result = validate(ast, { strict: true });
    expect(result.warnings.some(w => w.code === 'M3L-W001')).toBe(true);
  });

  it('should not report warnings without strict mode', () => {
    const ast = buildAST([
      '## Model',
      '- very_long_field_name: string(200) @not_null @unique @searchable "Long desc"',
    ].join('\n'));
    const result = validate(ast, { strict: false });
    expect(result.warnings.filter(w => w.code === 'M3L-W001')).toHaveLength(0);
  });

  it('W004: lookup chain >3 hops (strict mode)', () => {
    const ast = buildAST([
      '## Model',
      '### Lookup',
      '- deep_lookup: string @lookup(a.b.c.d)',
    ].join('\n'));
    const result = validate(ast, { strict: true });
    expect(result.warnings.some(w => w.code === 'M3L-W004')).toBe(true);
  });
});
