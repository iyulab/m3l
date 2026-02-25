import { describe, it, expect } from 'vitest';
import { readFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { parseString } from '../src/index.js';

const __dirname = dirname(fileURLToPath(import.meta.url));

describe('integration', () => {
  it('should parse the full library sample', () => {
    const content = readFileSync(
      join(__dirname, '../../../samples/sample_library.m3l.md'),
      'utf-8'
    );
    const ast = parseString(content, 'sample_library.m3l.md');

    // Should have multiple models
    expect(ast.models.length).toBeGreaterThanOrEqual(8);

    // Check specific model names exist
    const modelNames = ast.models.map(m => m.name);
    expect(modelNames).toContain('Author');
    expect(modelNames).toContain('Book');
    expect(modelNames).toContain('Member');
    expect(modelNames).toContain('Loan');

    // Views: OverdueLoans, MemberActivity, PopularBooks
    expect(ast.views).toHaveLength(3);
    const viewNames = ast.views.map(v => v.name);
    expect(viewNames).toContain('OverdueLoans');
    expect(viewNames).toContain('MemberActivity');
    expect(viewNames).toContain('PopularBooks');

    // Check a specific lookup field on Book
    const book = ast.models.find(m => m.name === 'Book')!;
    expect(book).toBeDefined();
    const publisherLookup = book.fields.find(f => f.name === 'publisher_name');
    expect(publisherLookup).toBeDefined();
    expect(publisherLookup!.kind).toBe('lookup');
    expect(publisherLookup!.lookup?.path).toBe('publisher_id.name');

    // Check a rollup field on Author
    const author = ast.models.find(m => m.name === 'Author')!;
    const bookCount = author.fields.find(f => f.name === 'book_count');
    expect(bookCount).toBeDefined();
    expect(bookCount!.kind).toBe('rollup');
    expect(bookCount!.rollup?.target).toBe('BookAuthor');

    // Check computed field on Book
    const isAvailable = book.fields.find(f => f.name === 'is_available');
    expect(isAvailable).toBeDefined();
    expect(isAvailable!.kind).toBe('computed');

    // Check materialized view
    const popular = ast.views.find(v => v.name === 'PopularBooks')!;
    expect(popular.materialized).toBe(true);
    expect(popular.refresh?.strategy).toBe('scheduled');

    // Check view source
    const overdue = ast.views.find(v => v.name === 'OverdueLoans')!;
    expect(overdue.source_def?.from).toBe('Loan');
    expect(overdue.fields.length).toBeGreaterThan(0);

    // Check inline enum on Book.status
    const statusField = book.fields.find(f => f.name === 'status');
    expect(statusField).toBeDefined();
    expect(statusField!.type).toBe('enum');
    expect(statusField!.enum_values!.length).toBeGreaterThan(0);

    // Check model description (blockquote)
    expect(book.description).toBe('Stores book information.');

    // Project name from namespace
    expect(ast.project.name).toBe('Library Management System');
  });

  it('should produce valid JSON output', () => {
    const content = readFileSync(
      join(__dirname, '../../../samples/sample_library.m3l.md'),
      'utf-8'
    );
    const ast = parseString(content, 'sample_library.m3l.md');

    // Should be serializable to JSON
    const json = JSON.stringify(ast);
    const parsed = JSON.parse(json);
    expect(parsed.models).toBeDefined();
    expect(parsed.views).toBeDefined();
  });

  it('should parse inheritance correctly in library sample', () => {
    const content = readFileSync(
      join(__dirname, '../../../samples/sample_library.m3l.md'),
      'utf-8'
    );
    const ast = parseString(content, 'sample_library.m3l.md');

    // Author inherits BaseModel which inherits Timestampable
    const author = ast.models.find(m => m.name === 'Author')!;
    const fieldNames = author.fields.map(f => f.name);
    // Should have inherited fields: created_at, updated_at (from Timestampable), id (from BaseModel)
    expect(fieldNames).toContain('created_at');
    expect(fieldNames).toContain('id');
    // Plus own fields
    expect(fieldNames).toContain('name');
  });
});
