import { describe, it, expect } from 'vitest';
import { lex } from '../src/lexer.js';
import { parseTokens } from '../src/parser.js';

function parse(content: string, file = 'test.m3l.md') {
  const tokens = lex(content, file);
  return parseTokens(tokens, file);
}

describe('parser', () => {
  it('should parse a simple model', () => {
    const result = parse([
      '# Test',
      '## User : BaseModel',
      '- name: string(100) @not_null',
      '- email: string(320)? @unique',
      '> User account',
    ].join('\n'));
    expect(result.models).toHaveLength(1);
    const user = result.models[0];
    expect(user.name).toBe('User');
    expect(user.inherits).toEqual(['BaseModel']);
    expect(user.fields).toHaveLength(2);
    expect(user.fields[0].name).toBe('name');
    expect(user.fields[0].nullable).toBe(false);
    expect(user.fields[0].type).toBe('string');
    expect(user.fields[0].params).toEqual([100]);
    expect(user.fields[1].name).toBe('email');
    expect(user.fields[1].nullable).toBe(true);
    expect(user.description).toBe('User account');
  });

  it('should parse an enum', () => {
    const result = parse([
      '## Status ::enum',
      '- active: "Active"',
      '- inactive: "Inactive"',
    ].join('\n'));
    expect(result.enums).toHaveLength(1);
    expect(result.enums[0].name).toBe('Status');
    expect(result.enums[0].values).toHaveLength(2);
    expect(result.enums[0].values[0].name).toBe('active');
    expect(result.enums[0].values[0].description).toBe('Active');
  });

  it('should parse inline enum on a field', () => {
    const result = parse([
      '## Order',
      '- status: enum = "pending"',
      '  - pending: "Pending"',
      '  - shipped: "Shipped"',
    ].join('\n'));
    expect(result.models).toHaveLength(1);
    const field = result.models[0].fields[0];
    expect(field.type).toBe('enum');
    expect(field.enum_values).toHaveLength(2);
    expect(field.enum_values![0].name).toBe('pending');
    expect(field.default_value).toBe('"pending"');
  });

  it('should parse sections (### Indexes, Metadata)', () => {
    const result = parse([
      '## Product',
      '- name: string(200)',
      '### Indexes',
      '- name_search(Name Search)',
      '  - fields: [name]',
      '  - fulltext: true',
      '### Metadata',
      '- domain: "retail"',
      '- version: 1.0',
    ].join('\n'));
    const product = result.models[0];
    expect(product.sections.indexes).toHaveLength(1);
    expect(product.sections.metadata).toHaveProperty('domain', 'retail');
    expect(product.sections.metadata).toHaveProperty('version', 1);
  });

  it('should parse view with source section', () => {
    const result = parse([
      '## OverdueLoans ::view',
      '> Currently overdue loans',
      '### Source',
      '- from: Loan',
      '- where: "status = \'ongoing\'"',
      '- order_by: due_date asc',
      '- book_title: string @lookup(book_id.title)',
    ].join('\n'));
    expect(result.views).toHaveLength(1);
    const view = result.views[0];
    expect(view.source_def?.from).toBe('Loan');
    expect(view.source_def?.where).toBe("status = 'ongoing'");
    expect(view.source_def?.order_by).toBe('due_date asc');
    expect(view.fields).toHaveLength(1);
    expect(view.fields[0].name).toBe('book_title');
    expect(view.fields[0].kind).toBe('lookup');
  });

  it('should parse lookup field in kind section', () => {
    const result = parse([
      '## Book',
      '- publisher_id: identifier @fk(Publisher.id)',
      '### Lookup',
      '- publisher_name: string @lookup(publisher_id.name)',
    ].join('\n'));
    const lookupField = result.models[0].fields.find(f => f.name === 'publisher_name');
    expect(lookupField).toBeDefined();
    expect(lookupField!.kind).toBe('lookup');
    expect(lookupField!.lookup?.path).toBe('publisher_id.name');
  });

  it('should parse rollup field', () => {
    const result = parse([
      '## Author',
      '- name: string(100)',
      '### Rollup',
      '- book_count: integer @rollup(BookAuthor.author_id, count)',
    ].join('\n'));
    const rollup = result.models[0].fields.find(f => f.name === 'book_count');
    expect(rollup).toBeDefined();
    expect(rollup!.kind).toBe('rollup');
    expect(rollup!.rollup?.target).toBe('BookAuthor');
    expect(rollup!.rollup?.fk).toBe('author_id');
    expect(rollup!.rollup?.aggregate).toBe('count');
  });

  it('should parse rollup with where clause', () => {
    const result = parse([
      '## Book',
      '- title: string(200)',
      '### Rollup',
      '- active_loans: integer @rollup(Loan.book_id, count, where: "status = \'ongoing\'")',
    ].join('\n'));
    const rollup = result.models[0].fields.find(f => f.name === 'active_loans');
    expect(rollup!.rollup?.where).toBe("status = 'ongoing'");
  });

  it('should parse rollup with aggregate function and field', () => {
    const result = parse([
      '## Member',
      '- name: string(100)',
      '### Rollup',
      '- total_fines: decimal(8,2) @rollup(Loan.member_id, sum(fine_amount))',
    ].join('\n'));
    const rollup = result.models[0].fields.find(f => f.name === 'total_fines');
    expect(rollup!.rollup?.aggregate).toBe('sum');
    expect(rollup!.rollup?.field).toBe('fine_amount');
  });

  it('should parse computed field', () => {
    const result = parse([
      '## Book',
      '- status: string',
      '- quantity: integer',
      '### Computed',
      '- is_available: boolean @computed("status = \'available\' AND quantity > 0")',
    ].join('\n'));
    const computed = result.models[0].fields.find(f => f.name === 'is_available');
    expect(computed).toBeDefined();
    expect(computed!.kind).toBe('computed');
    expect(computed!.computed?.expression).toBe("status = 'available' AND quantity > 0");
  });

  it('should handle H3 kind sections within a model', () => {
    const result = parse([
      '## Author : BaseModel',
      '- name: string(100)',
      '### Rollup',
      '- book_count: integer @rollup(BookAuthor.author_id, count)',
      '## Book',
      '- title: string(200)',
    ].join('\n'));
    expect(result.models).toHaveLength(2);
    expect(result.models[0].name).toBe('Author');
    expect(result.models[0].fields).toHaveLength(2);
    expect(result.models[1].name).toBe('Book');
  });

  it('should parse materialized view with refresh', () => {
    const result = parse([
      '## PopularBooks ::view @materialized',
      '### Source',
      '- from: Book',
      '### Refresh',
      '- strategy: scheduled',
      '- interval: "daily 03:00"',
      '- title: string @from(Book.title)',
    ].join('\n'));
    const view = result.views[0];
    expect(view.materialized).toBe(true);
    expect(view.refresh?.strategy).toBe('scheduled');
    expect(view.refresh?.interval).toBe('daily 03:00');
  });

  it('should parse multiple models separated by HR', () => {
    const result = parse([
      '## ModelA',
      '- field_a: string',
      '',
      '---',
      '',
      '## ModelB',
      '- field_b: integer',
    ].join('\n'));
    expect(result.models).toHaveLength(2);
    expect(result.models[0].name).toBe('ModelA');
    expect(result.models[1].name).toBe('ModelB');
  });

  it('should parse model blockquote description', () => {
    const result = parse([
      '## User',
      '> A registered user.',
      '> Supports OAuth.',
      '- name: string(100)',
    ].join('\n'));
    expect(result.models[0].description).toBe('A registered user.\nSupports OAuth.');
  });

  it('should parse framework attributes', () => {
    const result = parse([
      '## User',
      '- password: string(100) `[DataType(DataType.Password)]` `[JsonIgnore]`',
    ].join('\n'));
    const field = result.models[0].fields[0];
    expect(field.framework_attrs).toHaveLength(2);
    expect(field.framework_attrs![0].content).toBe('DataType(DataType.Password)');
    expect(field.framework_attrs![0].raw).toBe('[DataType(DataType.Password)]');
    expect(field.framework_attrs![1].content).toBe('JsonIgnore');
    expect(field.framework_attrs![1].raw).toBe('[JsonIgnore]');

    // Parsed structure
    expect(field.framework_attrs![0].parsed?.name).toBe('DataType');
    expect(field.framework_attrs![0].parsed?.arguments).toEqual(['DataType.Password']);
    expect(field.framework_attrs![1].parsed?.name).toBe('JsonIgnore');
    expect(field.framework_attrs![1].parsed?.arguments).toEqual([]);
  });

  it('should parse custom attribute with multiple args and types', () => {
    const result = parse([
      '## Config',
      '- port: integer `[Range(1, 65535)]` `[Description("Port number")]`',
    ].join('\n'));
    const field = result.models[0].fields[0];
    expect(field.framework_attrs![0].parsed?.name).toBe('Range');
    expect(field.framework_attrs![0].parsed?.arguments).toEqual([1, 65535]);
    expect(field.framework_attrs![1].parsed?.name).toBe('Description');
    expect(field.framework_attrs![1].parsed?.arguments).toEqual(['Port number']);
  });

  it('should parse field with default value and attributes', () => {
    const result = parse([
      '## Book',
      '- language: string(20) = "English"',
      '- quantity: integer = 0',
    ].join('\n'));
    expect(result.models[0].fields[0].default_value).toBe('"English"');
    expect(result.models[0].fields[1].default_value).toBe('0');
  });

  it('should parse Relations section', () => {
    const result = parse([
      '## Book',
      '- title: string',
      '### Relations',
      '- Book.id <-> Author.id {through: BookAuthor, as: authors}',
      '- Book.id <- Loan.book_id {type: o2m, as: loans}',
    ].join('\n'));
    expect(result.models[0].sections.relations).toHaveLength(2);
  });

  it('should parse namespace', () => {
    const result = parse('# Namespace: domain.example\n## Model\n- field: string');
    expect(result.namespace).toBe('domain.example');
  });

  it('should parse directive-only field lines', () => {
    const result = parse([
      '## User',
      '- name: string(100)',
      '- @index(email, username, name: "login_lookup")',
      '- @relation(posts, <- Post.author_id) "Posts by user"',
    ].join('\n'));
    expect(result.models[0].sections.indexes).toHaveLength(1);
    expect(result.models[0].sections.relations).toHaveLength(1);
  });

  it('should parse ::attribute type indicator as attribute registry entry', () => {
    const result = parse([
      '## @pii ::attribute',
      '> Personal information marker',
      '- target: [field]',
      '- type: boolean',
      '- default: false',
      '',
      '## @audit_level ::attribute',
      '> Audit compliance level',
      '- target: [field, model]',
      '- type: integer',
      '- range: [1, 5]',
      '- required: false',
      '- default: 1',
    ].join('\n'));

    expect(result.attributeRegistry).toHaveLength(2);

    const pii = result.attributeRegistry[0];
    expect(pii.name).toBe('pii');
    expect(pii.description).toBe('Personal information marker');
    expect(pii.target).toEqual(['field']);
    expect(pii.type).toBe('boolean');
    expect(pii.defaultValue).toBe(false);
    expect(pii.required).toBe(false);

    const audit = result.attributeRegistry[1];
    expect(audit.name).toBe('audit_level');
    expect(audit.description).toBe('Audit compliance level');
    expect(audit.target).toEqual(['field', 'model']);
    expect(audit.type).toBe('integer');
    expect(audit.range).toEqual([1, 5]);
    expect(audit.defaultValue).toBe(1);

    // Regular models should not be affected
    expect(result.models).toHaveLength(0);
    expect(result.enums).toHaveLength(0);
  });

  it('should parse @unique directive into indexes with unique flag', () => {
    const result = parse([
      '## User',
      '- name: string(100)',
      '- email: string(320)',
      '- @unique(email, name: "uq_email")',
    ].join('\n'));
    expect(result.models[0].sections.indexes).toHaveLength(1);
    const idx = result.models[0].sections.indexes[0] as {
      type: string; args?: string; unique?: boolean;
    };
    expect(idx.type).toBe('directive');
    expect(idx.unique).toBe(true);
    expect(idx.args).toContain('email');
  });

  it('should parse @index directive without unique flag', () => {
    const result = parse([
      '## User',
      '- name: string(100)',
      '- @index(name)',
    ].join('\n'));
    expect(result.models[0].sections.indexes).toHaveLength(1);
    const idx = result.models[0].sections.indexes[0] as {
      type: string; unique?: boolean;
    };
    expect(idx.type).toBe('directive');
    expect(idx.unique).toBe(false);
  });

  it('should tag standard attributes with isStandard', () => {
    const result = parse([
      '## User',
      '- id: identifier @primary @generated',
      '- name: string(100) @searchable',
      '- email: string @unique @my_custom_attr',
    ].join('\n'));
    const fields = result.models[0].fields;

    // @primary is standard
    const primary = fields[0].attributes.find(a => a.name === 'primary');
    expect(primary?.isStandard).toBe(true);

    // @generated is standard
    const generated = fields[0].attributes.find(a => a.name === 'generated');
    expect(generated?.isStandard).toBe(true);

    // @searchable is standard
    const searchable = fields[1].attributes.find(a => a.name === 'searchable');
    expect(searchable?.isStandard).toBe(true);

    // @unique is standard
    const unique = fields[2].attributes.find(a => a.name === 'unique');
    expect(unique?.isStandard).toBe(true);

    // @my_custom_attr is NOT standard — isStandard should be undefined
    const custom = fields[2].attributes.find(a => a.name === 'my_custom_attr');
    expect(custom?.isStandard).toBeUndefined();
  });
});
