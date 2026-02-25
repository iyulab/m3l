import { describe, it, expect } from 'vitest';
import { readFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { parseString, STANDARD_ATTRIBUTES } from '../src/index.js';
import { lex } from '../src/lexer.js';
import { parseTokens } from '../src/parser.js';
import { resolve } from '../src/resolver.js';

const __dirname = dirname(fileURLToPath(import.meta.url));

describe('integration', () => {
  it('should parse the full ecommerce sample', () => {
    const content = readFileSync(
      join(__dirname, '../../../samples/01-ecommerce.m3l.md'),
      'utf-8'
    );
    const ast = parseString(content, '01-ecommerce.m3l.md');

    // Models: Customer, Address, Category, Product, Inventory, Order, OrderItem, Review
    expect(ast.models.length).toBeGreaterThanOrEqual(8);

    const modelNames = ast.models.map(m => m.name);
    expect(modelNames).toContain('Customer');
    expect(modelNames).toContain('Product');
    expect(modelNames).toContain('Order');
    expect(modelNames).toContain('OrderItem');

    // Enums: CustomerStatus, PaymentMethod, OrderStatus, ShippingPriority
    expect(ast.enums.length).toBeGreaterThanOrEqual(4);
    const enumNames = ast.enums.map(e => e.name);
    expect(enumNames).toContain('OrderStatus');

    // Interfaces: Timestampable, Auditable
    expect(ast.interfaces).toHaveLength(2);

    // Views: ActiveProducts, CustomerOrderSummary
    expect(ast.views).toHaveLength(2);
    const viewNames = ast.views.map(v => v.name);
    expect(viewNames).toContain('ActiveProducts');
    expect(viewNames).toContain('CustomerOrderSummary');

    // Lookup field on Product
    const product = ast.models.find(m => m.name === 'Product')!;
    const categoryLookup = product.fields.find(f => f.name === 'category_name');
    expect(categoryLookup).toBeDefined();
    expect(categoryLookup!.kind).toBe('lookup');
    expect(categoryLookup!.lookup?.path).toBe('category_id.name');

    // Rollup field on Order
    const order = ast.models.find(m => m.name === 'Order')!;
    const itemCount = order.fields.find(f => f.name === 'item_count');
    expect(itemCount).toBeDefined();
    expect(itemCount!.kind).toBe('rollup');
    expect(itemCount!.rollup?.target).toBe('OrderItem');

    // Computed field on Product
    const profitMargin = product.fields.find(f => f.name === 'profit_margin');
    expect(profitMargin).toBeDefined();
    expect(profitMargin!.kind).toBe('computed');

    // Materialized view
    const summary = ast.views.find(v => v.name === 'CustomerOrderSummary')!;
    expect(summary.materialized).toBe(true);
    expect(summary.refresh?.strategy).toBe('incremental');

    // View source
    const activeProducts = ast.views.find(v => v.name === 'ActiveProducts')!;
    expect(activeProducts.source_def?.from).toBe('Product');

    // Model description (blockquote)
    expect(product.description).toBe('Product catalog item.');

    // Project name from namespace
    expect(ast.project.name).toBe('sample.ecommerce');
  });

  it('should produce valid JSON output', () => {
    const content = readFileSync(
      join(__dirname, '../../../samples/01-ecommerce.m3l.md'),
      'utf-8'
    );
    const ast = parseString(content, '01-ecommerce.m3l.md');

    const json = JSON.stringify(ast);
    const parsed = JSON.parse(json);
    expect(parsed.models).toBeDefined();
    expect(parsed.views).toBeDefined();
  });

  it('should support full extensibility scenario: registry + isStandard + isRegistered + custom attrs', () => {
    const registrySource = [
      '## @pii ::attribute',
      '> Personal identifiable information marker',
      '- target: [field]',
      '- type: boolean',
      '- default: false',
      '',
      '## @audit_level ::attribute',
      '> Audit compliance level (1-5)',
      '- target: [field, model]',
      '- type: integer',
      '- range: [1, 5]',
      '- required: false',
      '- default: 1',
    ].join('\n');

    const modelSource = [
      '## Customer @audit_level(3)',
      '> CRM customer information',
      '- id: identifier @primary @generated',
      '- name: string(100) @searchable',
      '- email: email @unique @pii',
      '- ssn: string(11) @pii `[JsonIgnore]`',
      '- score: float `[Range(0, 100)]`',
    ].join('\n');

    const regTokens = lex(registrySource, 'registry.m3l.md');
    const regFile = parseTokens(regTokens, 'registry.m3l.md');
    const modelTokens = lex(modelSource, 'model.m3l.md');
    const modelFile = parseTokens(modelTokens, 'model.m3l.md');
    const ast = resolve([regFile, modelFile]);

    // 1. Registry entries parsed correctly
    expect(ast.attributeRegistry).toHaveLength(2);
    expect(ast.attributeRegistry[0].name).toBe('pii');
    expect(ast.attributeRegistry[1].name).toBe('audit_level');
    expect(ast.attributeRegistry[1].range).toEqual([1, 5]);

    const customer = ast.models.find(m => m.name === 'Customer')!;

    // 2. Model-level attribute: isStandard=undefined, isRegistered=true
    const modelAudit = customer.attributes.find(a => a.name === 'audit_level');
    expect(modelAudit).toBeDefined();
    expect(modelAudit!.isStandard).toBeUndefined();
    expect(modelAudit!.isRegistered).toBe(true);
    // Note: model-level args are currently stored as string[] from lexer
    expect(modelAudit!.args).toBeDefined();

    // 3. Standard attributes: isStandard=true, isRegistered=undefined
    const primary = customer.fields[0].attributes.find(a => a.name === 'primary');
    expect(primary?.isStandard).toBe(true);
    expect(primary?.isRegistered).toBeUndefined();

    const generated = customer.fields[0].attributes.find(a => a.name === 'generated');
    expect(generated?.isStandard).toBe(true);

    // 4. Registered extension attributes: isStandard=undefined, isRegistered=true
    const piiOnEmail = customer.fields[2].attributes.find(a => a.name === 'pii');
    expect(piiOnEmail?.isStandard).toBeUndefined();
    expect(piiOnEmail?.isRegistered).toBe(true);

    const piiOnSsn = customer.fields[3].attributes.find(a => a.name === 'pii');
    expect(piiOnSsn?.isRegistered).toBe(true);

    // 5. Custom framework attributes: parsed structure
    const jsonIgnore = customer.fields[3].framework_attrs?.find(a => a.content === 'JsonIgnore');
    expect(jsonIgnore).toBeDefined();
    expect(jsonIgnore!.parsed?.name).toBe('JsonIgnore');
    expect(jsonIgnore!.parsed?.arguments).toEqual([]);

    const rangeAttr = customer.fields[4].framework_attrs?.find(a => a.content === 'Range(0, 100)');
    expect(rangeAttr).toBeDefined();
    expect(rangeAttr!.parsed?.name).toBe('Range');
    expect(rangeAttr!.parsed?.arguments).toEqual([0, 100]);

    // 6. STANDARD_ATTRIBUTES export is available and has expected entries
    expect(STANDARD_ATTRIBUTES.has('primary')).toBe(true);
    expect(STANDARD_ATTRIBUTES.has('pii')).toBe(false);
  });

  it('should parse inheritance correctly in ecommerce sample', () => {
    const content = readFileSync(
      join(__dirname, '../../../samples/01-ecommerce.m3l.md'),
      'utf-8'
    );
    const ast = parseString(content, '01-ecommerce.m3l.md');

    // Product inherits Timestampable and Auditable
    const product = ast.models.find(m => m.name === 'Product')!;
    const fieldNames = product.fields.map(f => f.name);
    // Inherited fields from Timestampable
    expect(fieldNames).toContain('created_at');
    expect(fieldNames).toContain('updated_at');
    // Inherited fields from Auditable
    expect(fieldNames).toContain('created_by');
    expect(fieldNames).toContain('updated_by');
    // Own fields
    expect(fieldNames).toContain('id');
    expect(fieldNames).toContain('name');
    expect(fieldNames).toContain('price');
  });
});
