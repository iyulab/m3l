import { describe, it, expect } from 'vitest';
import { readFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { parseString } from '../src/index.js';

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
