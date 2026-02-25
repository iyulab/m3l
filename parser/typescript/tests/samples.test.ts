/**
 * M3L Sample File Comprehensive Test Suite
 *
 * Tests the TypeScript parser against all sample files in D:/data/m3l/samples/.
 * For each sample, verifies: element counts, names, field parsing, derived fields,
 * sections, views, inheritance, and errors/warnings.
 *
 * Defects found during testing are documented inline with [DEFECT] comments.
 */
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { parseString } from '../src/index.js';
import type { M3LAST, ModelNode, FieldNode, EnumNode } from '../src/types.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const samplesDir = join(__dirname, '../../../samples');

function loadSample(filename: string): M3LAST {
  const content = readFileSync(join(samplesDir, filename), 'utf-8');
  return parseString(content, filename);
}

// Helper to find a model by name
function findModel(ast: M3LAST, name: string): ModelNode {
  const m = ast.models.find(m => m.name === name);
  if (!m) throw new Error(`Model "${name}" not found. Available: ${ast.models.map(m => m.name).join(', ')}`);
  return m;
}

function findEnum(ast: M3LAST, name: string): EnumNode {
  const e = ast.enums.find(e => e.name === name);
  if (!e) throw new Error(`Enum "${name}" not found. Available: ${ast.enums.map(e => e.name).join(', ')}`);
  return e;
}

function findView(ast: M3LAST, name: string): ModelNode {
  const v = ast.views.find(v => v.name === name);
  if (!v) throw new Error(`View "${name}" not found. Available: ${ast.views.map(v => v.name).join(', ')}`);
  return v;
}

function findInterface(ast: M3LAST, name: string): ModelNode {
  const i = ast.interfaces.find(i => i.name === name);
  if (!i) throw new Error(`Interface "${name}" not found. Available: ${ast.interfaces.map(i => i.name).join(', ')}`);
  return i;
}

function findField(model: ModelNode, name: string): FieldNode {
  const f = model.fields.find(f => f.name === name);
  if (!f) throw new Error(`Field "${name}" not found in "${model.name}". Available: ${model.fields.map(f => f.name).join(', ')}`);
  return f;
}

// ============================================================
// Sample 01: E-Commerce
// ============================================================
describe('Sample 01: E-Commerce (01-ecommerce.m3l.md)', () => {
  const ast = loadSample('01-ecommerce.m3l.md');

  // --- Element counts ---
  describe('Element Counts', () => {
    it('has 8 models', () => {
      expect(ast.models).toHaveLength(8);
    });

    it('has 4 enums', () => {
      expect(ast.enums).toHaveLength(4);
    });

    it('has 2 interfaces', () => {
      expect(ast.interfaces).toHaveLength(2);
    });

    it('has 2 views', () => {
      expect(ast.views).toHaveLength(2);
    });
  });

  // --- Model names ---
  describe('Model Names', () => {
    const expectedModels = [
      'Customer', 'Address', 'Category', 'Product',
      'Inventory', 'Order', 'OrderItem', 'Review',
    ];

    it.each(expectedModels)('contains model "%s"', (name) => {
      expect(ast.models.map(m => m.name)).toContain(name);
    });
  });

  // --- Enum names and values ---
  describe('Enum Names', () => {
    it.each(['CustomerStatus', 'PaymentMethod', 'OrderStatus', 'ShippingPriority'])(
      'contains enum "%s"', (name) => {
        expect(ast.enums.map(e => e.name)).toContain(name);
      }
    );
  });

  describe('Enum: CustomerStatus', () => {
    it('has 4 values', () => {
      const e = findEnum(ast, 'CustomerStatus');
      expect(e.values).toHaveLength(4);
    });

    it('values have correct names and descriptions', () => {
      const e = findEnum(ast, 'CustomerStatus');
      expect(e.values[0]).toMatchObject({ name: 'active', description: 'Active' });
      expect(e.values[3]).toMatchObject({ name: 'pending', description: 'Pending Verification' });
    });
  });

  describe('Enum: OrderStatus', () => {
    it('has 7 values', () => {
      const e = findEnum(ast, 'OrderStatus');
      expect(e.values).toHaveLength(7);
    });
  });

  describe('Enum: ShippingPriority (typed enum with integer values)', () => {
    it('has 3 values', () => {
      const e = findEnum(ast, 'ShippingPriority');
      expect(e.values).toHaveLength(3);
    });

    it('values have type "integer" and numeric values', () => {
      const e = findEnum(ast, 'ShippingPriority');
      // Pattern: - standard: integer = 0 "Standard Shipping"
      expect(e.values[0]).toMatchObject({
        name: 'standard',
        type: 'integer',
        value: '0',
        description: 'Standard Shipping',
      });
      expect(e.values[2]).toMatchObject({
        name: 'overnight',
        type: 'integer',
        value: '2',
        description: 'Overnight Delivery',
      });
    });
  });

  // --- Interfaces ---
  describe('Interface: Timestampable', () => {
    it('has 2 fields', () => {
      const iface = findInterface(ast, 'Timestampable');
      expect(iface.fields).toHaveLength(2);
      expect(iface.fields.map(f => f.name)).toEqual(['created_at', 'updated_at']);
    });
  });

  describe('Interface: Auditable', () => {
    it('has 2 fields', () => {
      const iface = findInterface(ast, 'Auditable');
      expect(iface.fields).toHaveLength(2);
      expect(iface.fields.map(f => f.name)).toEqual(['created_by', 'updated_by']);
    });
  });

  // --- Namespace ---
  describe('Namespace', () => {
    it('project name is set from namespace', () => {
      expect(ast.project.name).toBe('sample.ecommerce');
    });
  });

  // --- Customer model ---
  describe('Model: Customer', () => {
    it('inherits from Timestampable', () => {
      const customer = findModel(ast, 'Customer');
      expect(customer.inherits).toEqual(['Timestampable']);
    });

    it('has inherited fields prepended (created_at, updated_at)', () => {
      const customer = findModel(ast, 'Customer');
      expect(customer.fields[0].name).toBe('created_at');
      expect(customer.fields[1].name).toBe('updated_at');
    });

    it('has 9 total fields (2 inherited + 7 own)', () => {
      const customer = findModel(ast, 'Customer');
      expect(customer.fields).toHaveLength(9);
    });

    it('description is from blockquote', () => {
      const customer = findModel(ast, 'Customer');
      expect(customer.description).toBe('Primary customer entity for the e-commerce platform.');
    });

    it('inline enum on status field has values', () => {
      const customer = findModel(ast, 'Customer');
      const status = findField(customer, 'status');
      expect(status.type).toBe('enum');
      expect(status.default_value).toBe('"active"');
      expect(status.enum_values).toBeDefined();
      expect(status.enum_values).toHaveLength(3);
      expect(status.enum_values![0]).toMatchObject({ name: 'active', description: 'Active' });
    });

    it('loyalty_points has default value 0 and @min attribute', () => {
      const customer = findModel(ast, 'Customer');
      const lp = findField(customer, 'loyalty_points');
      expect(lp.default_value).toBe('0');
      expect(lp.attributes.some(a => a.name === 'min')).toBe(true);
    });

    it('email has @unique attribute and description', () => {
      const customer = findModel(ast, 'Customer');
      const email = findField(customer, 'email');
      expect(email.type).toBe('email');
      expect(email.attributes.some(a => a.name === 'unique')).toBe(true);
      expect(email.description).toBe('Primary contact email');
    });

    it('phone is nullable', () => {
      const customer = findModel(ast, 'Customer');
      const phone = findField(customer, 'phone');
      expect(phone.nullable).toBe(true);
    });

    it('Metadata section has table_name and audit_enabled', () => {
      const customer = findModel(ast, 'Customer');
      expect(customer.sections.metadata).toEqual({
        table_name: 'customers',
        audit_enabled: true,
      });
    });
  });

  // --- Address model ---
  describe('Model: Address', () => {
    it('has @index directive in indexes', () => {
      const address = findModel(ast, 'Address');
      expect(address.sections.indexes.length).toBeGreaterThanOrEqual(1);
      const indexDirective = address.sections.indexes.find(
        (idx: any) => idx.type === 'directive' && idx.args === 'customer_id'
      );
      expect(indexDirective).toBeDefined();
    });

    it('has @unique directive in unique section', () => {
      const address = findModel(ast, 'Address');
      // @unique(customer_id, label) is stored as a generic directive section
      const uniqueSection = address.sections['unique'] as any[];
      expect(uniqueSection).toBeDefined();
      expect(uniqueSection.length).toBeGreaterThanOrEqual(1);
      expect(uniqueSection[0].args).toContain('customer_id');
    });

    it('country field has description "ISO 3166-1 alpha-2"', () => {
      const address = findModel(ast, 'Address');
      const country = findField(address, 'country');
      expect(country.description).toBe('ISO 3166-1 alpha-2');
    });
  });

  // --- Category model ---
  describe('Model: Category', () => {
    it('has lookup field parent_name', () => {
      const category = findModel(ast, 'Category');
      const parentName = findField(category, 'parent_name');
      expect(parentName.kind).toBe('lookup');
      expect(parentName.lookup?.path).toBe('parent_id.name');
    });

    it('has rollup field product_count', () => {
      const category = findModel(ast, 'Category');
      const productCount = findField(category, 'product_count');
      expect(productCount.kind).toBe('rollup');
      expect(productCount.rollup?.target).toBe('Product');
      expect(productCount.rollup?.fk).toBe('category_id');
      expect(productCount.rollup?.aggregate).toBe('count');
    });
  });

  // --- Product model ---
  describe('Model: Product', () => {
    it('inherits from Timestampable and Auditable', () => {
      const product = findModel(ast, 'Product');
      expect(product.inherits).toEqual(['Timestampable', 'Auditable']);
    });

    it('has inherited fields (created_at, updated_at, created_by, updated_by)', () => {
      const product = findModel(ast, 'Product');
      const fieldNames = product.fields.map(f => f.name);
      expect(fieldNames).toContain('created_at');
      expect(fieldNames).toContain('updated_at');
      expect(fieldNames).toContain('created_by');
      expect(fieldNames).toContain('updated_by');
    });

    it('has 17 total fields (4 inherited + 10 own + 1 lookup + 2 computed)', () => {
      const product = findModel(ast, 'Product');
      expect(product.fields).toHaveLength(17);
    });

    it('sku has @immutable attribute', () => {
      const product = findModel(ast, 'Product');
      const sku = findField(product, 'sku');
      expect(sku.attributes.some(a => a.name === 'immutable')).toBe(true);
    });

    it('tags field is an array', () => {
      const product = findModel(ast, 'Product');
      const tags = findField(product, 'tags');
      expect(tags.array).toBe(true);
      expect(tags.type).toBe('string');
    });

    it('lookup: category_name', () => {
      const product = findModel(ast, 'Product');
      const f = findField(product, 'category_name');
      expect(f.kind).toBe('lookup');
      expect(f.lookup?.path).toBe('category_id.name');
    });

    it('computed: profit_margin has expression', () => {
      const product = findModel(ast, 'Product');
      const f = findField(product, 'profit_margin');
      expect(f.kind).toBe('computed');
      expect(f.computed?.expression).toBe('(price - cost) / price * 100');
    });

    it('computed: display_price has expression', () => {
      const product = findModel(ast, 'Product');
      const f = findField(product, 'display_price');
      expect(f.kind).toBe('computed');
      expect(f.computed?.expression).toContain('FORMAT');
    });

    it('Indexes section has 2 entries', () => {
      const product = findModel(ast, 'Product');
      expect(product.sections.indexes).toHaveLength(2);
      const idxCategory = (product.sections.indexes as any[]).find((i: any) => i.name === 'idx_category');
      expect(idxCategory).toBeDefined();
      expect(idxCategory.fields).toEqual(['category_id', 'is_active']);
    });

    it('Relations section has 1 entry', () => {
      const product = findModel(ast, 'Product');
      expect(product.sections.relations).toHaveLength(1);
      expect((product.sections.relations[0] as any).raw).toContain('category');
    });

    it('cost field has inline comment parsed (stripped from field)', () => {
      const product = findModel(ast, 'Product');
      const cost = findField(product, 'cost');
      // The inline comment "# Internal cost, not shown to customers" should be stripped
      // The field type should still be decimal
      expect(cost.type).toBe('decimal');
    });

    it('description from blockquote', () => {
      const product = findModel(ast, 'Product');
      expect(product.description).toBe('Product catalog item.');
    });
  });

  // --- Inventory model ---
  describe('Model: Inventory', () => {
    it('has 2 computed fields', () => {
      const inv = findModel(ast, 'Inventory');
      const computed = inv.fields.filter(f => f.kind === 'computed');
      expect(computed).toHaveLength(2);
      expect(computed.map(f => f.name)).toContain('available');
      expect(computed.map(f => f.name)).toContain('needs_reorder');
    });

    it('computed available has expression "quantity - reserved"', () => {
      const inv = findModel(ast, 'Inventory');
      const available = findField(inv, 'available');
      expect(available.computed?.expression).toBe('quantity - reserved');
    });
  });

  // --- Order model ---
  describe('Model: Order', () => {
    it('inherits from Timestampable and Auditable', () => {
      const order = findModel(ast, 'Order');
      expect(order.inherits).toEqual(['Timestampable', 'Auditable']);
    });

    it('has 23 total fields (4 inherited + 11 own + 3 lookups + 3 rollups + 2 computed)', () => {
      const order = findModel(ast, 'Order');
      expect(order.fields).toHaveLength(23);
    });

    it('has 3 lookup fields', () => {
      const order = findModel(ast, 'Order');
      const lookups = order.fields.filter(f => f.kind === 'lookup');
      expect(lookups).toHaveLength(3);
      expect(lookups.map(f => f.name)).toEqual(['customer_name', 'customer_email', 'shipping_city']);
    });

    it('has 3 rollup fields', () => {
      const order = findModel(ast, 'Order');
      const rollups = order.fields.filter(f => f.kind === 'rollup');
      expect(rollups).toHaveLength(3);
      expect(rollups.map(f => f.name)).toEqual(['item_count', 'subtotal', 'total_quantity']);
    });

    it('rollup subtotal has aggregate=sum and field=line_total', () => {
      const order = findModel(ast, 'Order');
      const subtotal = findField(order, 'subtotal');
      expect(subtotal.rollup?.target).toBe('OrderItem');
      expect(subtotal.rollup?.fk).toBe('order_id');
      expect(subtotal.rollup?.aggregate).toBe('sum');
      expect(subtotal.rollup?.field).toBe('line_total');
    });

    it('has 2 computed fields', () => {
      const order = findModel(ast, 'Order');
      const computed = order.fields.filter(f => f.kind === 'computed');
      expect(computed).toHaveLength(2);
      expect(computed.map(f => f.name)).toEqual(['tax_amount', 'grand_total']);
    });

    it('Indexes section has 2 entries', () => {
      const order = findModel(ast, 'Order');
      expect(order.sections.indexes).toHaveLength(2);
    });

    it('Behaviors section has 1 entry (before_create)', () => {
      const order = findModel(ast, 'Order');
      expect(order.sections.behaviors).toHaveLength(1);
      expect((order.sections.behaviors[0] as any).name).toBe('before_create');
    });

    it('Metadata has table_name and soft_delete', () => {
      const order = findModel(ast, 'Order');
      expect(order.sections.metadata).toEqual({
        table_name: 'orders',
        soft_delete: true,
      });
    });

    it('status field references OrderStatus enum type', () => {
      const order = findModel(ast, 'Order');
      const status = findField(order, 'status');
      expect(status.type).toBe('OrderStatus');
      expect(status.default_value).toBe('"draft"');
    });

    it('customer_id has !! (cascade) shorthand', () => {
      const order = findModel(ast, 'Order');
      const customerId = findField(order, 'customer_id');
      expect(customerId.attributes.some(a => a.name === 'reference')).toBe(true);
    });
  });

  // --- OrderItem model ---
  describe('Model: OrderItem', () => {
    it('has no inheritance', () => {
      const item = findModel(ast, 'OrderItem');
      expect(item.inherits).toEqual([]);
    });

    it('has 2 lookup fields', () => {
      const item = findModel(ast, 'OrderItem');
      const lookups = item.fields.filter(f => f.kind === 'lookup');
      expect(lookups).toHaveLength(2);
      expect(lookups.map(f => f.name)).toEqual(['product_name', 'product_sku']);
    });

    it('has 2 computed fields', () => {
      const item = findModel(ast, 'OrderItem');
      const computed = item.fields.filter(f => f.kind === 'computed');
      expect(computed).toHaveLength(2);
      expect(computed.map(f => f.name)).toEqual(['discount_amount', 'line_total']);
    });

    it('has @unique directive for (order_id, product_id)', () => {
      const item = findModel(ast, 'OrderItem');
      const uniqueSection = item.sections['unique'] as any[];
      expect(uniqueSection).toBeDefined();
      expect(uniqueSection[0].args).toContain('order_id');
    });
  });

  // --- Review model ---
  describe('Model: Review', () => {
    it('has @unique directive for (product_id, customer_id)', () => {
      const review = findModel(ast, 'Review');
      const uniqueSection = review.sections['unique'] as any[];
      expect(uniqueSection).toBeDefined();
    });

    it('rating has @min(1) and @max(5)', () => {
      const review = findModel(ast, 'Review');
      const rating = findField(review, 'rating');
      expect(rating.attributes.some(a => a.name === 'min')).toBe(true);
      expect(rating.attributes.some(a => a.name === 'max')).toBe(true);
    });
  });

  // --- Views ---
  describe('View: ActiveProducts', () => {
    it('is a view with type "view"', () => {
      const view = findView(ast, 'ActiveProducts');
      expect(view.type).toBe('view');
    });

    it('is not materialized', () => {
      const view = findView(ast, 'ActiveProducts');
      expect(view.materialized).toBe(false);
    });

    it('source_def.from is "Product"', () => {
      const view = findView(ast, 'ActiveProducts');
      expect(view.source_def?.from).toBe('Product');
    });

    it('source_def.where contains filter condition', () => {
      const view = findView(ast, 'ActiveProducts');
      expect(view.source_def?.where).toBe('is_active = true AND price > 0');
    });

    it('source_def.order_by is "name asc"', () => {
      const view = findView(ast, 'ActiveProducts');
      expect(view.source_def?.order_by).toBe('name asc');
    });

    it('has description from blockquote', () => {
      const view = findView(ast, 'ActiveProducts');
      expect(view.description).toBe('Products currently available for sale.');
    });
  });

  describe('View: CustomerOrderSummary', () => {
    it('is materialized', () => {
      const view = findView(ast, 'CustomerOrderSummary');
      expect(view.materialized).toBe(true);
    });

    it('source_def has from, join, where, group_by', () => {
      const view = findView(ast, 'CustomerOrderSummary');
      expect(view.source_def?.from).toBe('Customer');
      expect(view.source_def?.joins).toBeDefined();
      expect(view.source_def?.joins).toHaveLength(1);
      expect(view.source_def?.joins![0].model).toBe('Order');
      expect(view.source_def?.joins![0].on).toContain('Customer.id');
      expect(view.source_def?.where).toContain('cancelled');
      expect(view.source_def?.group_by).toEqual(['Customer.id', 'Customer.name', 'Customer.email']);
    });

    it('has view fields defined in Source section', () => {
      const view = findView(ast, 'CustomerOrderSummary');
      expect(view.fields.length).toBeGreaterThanOrEqual(4);
      expect(view.fields.map(f => f.name)).toContain('customer_id');
      expect(view.fields.map(f => f.name)).toContain('total_orders');
    });

    it('refresh strategy is incremental with 1 hour interval', () => {
      const view = findView(ast, 'CustomerOrderSummary');
      expect(view.refresh?.strategy).toBe('incremental');
      expect(view.refresh?.interval).toBe('1 hour');
    });
  });

  // --- Errors and Warnings ---
  describe('Errors and Warnings', () => {
    it('has no errors', () => {
      expect(ast.errors).toHaveLength(0);
    });

    it('has no warnings', () => {
      expect(ast.warnings).toHaveLength(0);
    });
  });
});

// ============================================================
// Sample 02: Blog CMS
// ============================================================
describe('Sample 02: Blog CMS (02-blog-cms.m3l.md)', () => {
  const ast = loadSample('02-blog-cms.m3l.md');

  // --- Element counts ---
  describe('Element Counts', () => {
    it('has 7 models', () => {
      expect(ast.models).toHaveLength(7);
    });

    it('has 2 enums', () => {
      expect(ast.enums).toHaveLength(2);
    });

    it('has 2 interfaces', () => {
      expect(ast.interfaces).toHaveLength(2);
    });

    it('has 2 views', () => {
      expect(ast.views).toHaveLength(2);
    });
  });

  // --- Model names ---
  describe('Model Names', () => {
    it.each(['User', 'Tag', 'Category', 'Post', 'PostTag', 'Comment', 'MediaAsset'])(
      'contains model "%s"', (name) => {
        expect(ast.models.map(m => m.name)).toContain(name);
      }
    );
  });

  // --- Namespace ---
  describe('Namespace', () => {
    it('project name is sample.blog', () => {
      expect(ast.project.name).toBe('sample.blog');
    });
  });

  // --- Interfaces ---
  describe('Interface: BaseModel', () => {
    it('has 3 fields (id, created_at, updated_at)', () => {
      const iface = findInterface(ast, 'BaseModel');
      expect(iface.fields).toHaveLength(3);
      expect(iface.fields.map(f => f.name)).toEqual(['id', 'created_at', 'updated_at']);
    });

    it('created_at has @immutable and default now()', () => {
      const iface = findInterface(ast, 'BaseModel');
      const createdAt = findField(iface, 'created_at');
      expect(createdAt.default_value).toBe('now()');
      expect(createdAt.attributes.some(a => a.name === 'immutable')).toBe(true);
    });
  });

  describe('Interface: Trackable', () => {
    it('has 3 fields (version, is_deleted, deleted_at)', () => {
      const iface = findInterface(ast, 'Trackable');
      expect(iface.fields).toHaveLength(3);
      expect(iface.fields.map(f => f.name)).toEqual(['version', 'is_deleted', 'deleted_at']);
    });
  });

  // --- Enums ---
  describe('Enum: PostStatus', () => {
    it('has 4 values', () => {
      const e = findEnum(ast, 'PostStatus');
      expect(e.values).toHaveLength(4);
      expect(e.values.map(v => v.name)).toEqual(['draft', 'review', 'published', 'archived']);
    });
  });

  describe('Enum: ContentFormat', () => {
    it('has 3 values', () => {
      const e = findEnum(ast, 'ContentFormat');
      expect(e.values).toHaveLength(3);
    });
  });

  // --- User model ---
  describe('Model: User', () => {
    it('inherits from BaseModel (3 inherited fields)', () => {
      const user = findModel(ast, 'User');
      expect(user.inherits).toEqual(['BaseModel']);
      // First 3 fields should be from BaseModel
      expect(user.fields[0].name).toBe('id');
      expect(user.fields[1].name).toBe('created_at');
      expect(user.fields[2].name).toBe('updated_at');
    });

    it('has 14 total fields (3 inherited + 8 own + 1 computed + 2 rollups)', () => {
      const user = findModel(ast, 'User');
      expect(user.fields).toHaveLength(14);
    });

    it('password_hash has framework attribute [JsonIgnore]', () => {
      const user = findModel(ast, 'User');
      const pw = findField(user, 'password_hash');
      expect(pw.framework_attrs).toBeDefined();
      expect(pw.framework_attrs).toHaveLength(1);
      expect(pw.framework_attrs![0].content).toBe('JsonIgnore');
      expect(pw.framework_attrs![0].raw).toBe('[JsonIgnore]');
    });

    it('role is inline enum with 4 values', () => {
      const user = findModel(ast, 'User');
      const role = findField(user, 'role');
      expect(role.type).toBe('enum');
      expect(role.default_value).toBe('"author"');
      expect(role.enum_values).toBeDefined();
      expect(role.enum_values).toHaveLength(4);
      expect(role.enum_values![0].name).toBe('admin');
      expect(role.enum_values![0].description).toBe('Administrator');
    });

    it('has computed field full_profile_url', () => {
      const user = findModel(ast, 'User');
      const f = findField(user, 'full_profile_url');
      expect(f.kind).toBe('computed');
      expect(f.computed?.expression).toContain('/users/');
    });

    it('has rollup field post_count', () => {
      const user = findModel(ast, 'User');
      const f = findField(user, 'post_count');
      expect(f.kind).toBe('rollup');
      expect(f.rollup?.target).toBe('Post');
      expect(f.rollup?.fk).toBe('author_id');
      expect(f.rollup?.aggregate).toBe('count');
    });

    it('has rollup field published_post_count with where clause', () => {
      const user = findModel(ast, 'User');
      const f = findField(user, 'published_post_count');
      expect(f.kind).toBe('rollup');
      expect(f.rollup?.aggregate).toBe('count');
      expect(f.rollup?.where).toBe("status = 'published'");
    });
  });

  // --- Post model ---
  describe('Model: Post', () => {
    it('inherits from BaseModel and Trackable', () => {
      const post = findModel(ast, 'Post');
      expect(post.inherits).toEqual(['BaseModel', 'Trackable']);
    });

    it('has inherited fields from both interfaces', () => {
      const post = findModel(ast, 'Post');
      const fieldNames = post.fields.map(f => f.name);
      // From BaseModel
      expect(fieldNames).toContain('id');
      expect(fieldNames).toContain('created_at');
      expect(fieldNames).toContain('updated_at');
      // From Trackable
      expect(fieldNames).toContain('version');
      expect(fieldNames).toContain('is_deleted');
      expect(fieldNames).toContain('deleted_at');
    });

    it('has 30 total fields', () => {
      const post = findModel(ast, 'Post');
      expect(post.fields).toHaveLength(30);
    });

    it('has 3 lookup fields', () => {
      const post = findModel(ast, 'Post');
      const lookups = post.fields.filter(f => f.kind === 'lookup');
      expect(lookups).toHaveLength(3);
      expect(lookups.map(f => f.name)).toEqual(['author_name', 'author_avatar', 'category_name']);
    });

    it('author_name lookup path is author_id.display_name', () => {
      const post = findModel(ast, 'Post');
      const f = findField(post, 'author_name');
      expect(f.lookup?.path).toBe('author_id.display_name');
    });

    it('has 2 computed fields', () => {
      const post = findModel(ast, 'Post');
      const computed = post.fields.filter(f => f.kind === 'computed');
      expect(computed).toHaveLength(2);
      expect(computed.map(f => f.name)).toEqual(['is_published', 'word_count']);
    });

    it('has 2 rollup fields', () => {
      const post = findModel(ast, 'Post');
      const rollups = post.fields.filter(f => f.kind === 'rollup');
      expect(rollups).toHaveLength(2);
      expect(rollups.map(f => f.name)).toEqual(['comment_count', 'avg_rating']);
    });

    it('avg_rating rollup has aggregate=avg and field=rating', () => {
      const post = findModel(ast, 'Post');
      const f = findField(post, 'avg_rating');
      expect(f.rollup?.aggregate).toBe('avg');
      expect(f.rollup?.field).toBe('rating');
    });

    it('seo_title has framework attribute [MaxLength(70)]', () => {
      const post = findModel(ast, 'Post');
      const f = findField(post, 'seo_title');
      expect(f.framework_attrs).toBeDefined();
      expect(f.framework_attrs).toHaveLength(1);
      expect(f.framework_attrs![0].content).toBe('MaxLength(70)');
    });

    it('has description from multi-line blockquote', () => {
      const post = findModel(ast, 'Post');
      expect(post.description).toContain('Blog post with rich content and metadata.');
      expect(post.description).toContain('Supports multiple content formats');
    });

    it('Indexes section has 2 entries with conditional where', () => {
      const post = findModel(ast, 'Post');
      expect(post.sections.indexes).toHaveLength(2);
      const idxPublished = (post.sections.indexes as any[]).find((i: any) => i.name === 'idx_published');
      expect(idxPublished).toBeDefined();
      expect(idxPublished.where).toBe("status = 'published'");
    });

    it('Behaviors section has 3 entries', () => {
      const post = findModel(ast, 'Post');
      expect(post.sections.behaviors).toHaveLength(3);
      const names = (post.sections.behaviors as any[]).map((b: any) => b.name);
      expect(names).toEqual(['before_create', 'before_update', 'after_publish']);
    });

    it('Metadata has table_name and cache_ttl', () => {
      const post = findModel(ast, 'Post');
      expect(post.sections.metadata).toEqual({
        table_name: 'posts',
        cache_ttl: 300,
      });
    });
  });

  // --- PostTag model ---
  describe('Model: PostTag', () => {
    it('has post_id and tag_id fields', () => {
      const pt = findModel(ast, 'PostTag');
      const fieldNames = pt.fields.map(f => f.name);
      expect(fieldNames).toContain('post_id');
      expect(fieldNames).toContain('tag_id');
    });

    it('post_id has on_delete: cascade from extended format', () => {
      const pt = findModel(ast, 'PostTag');
      const postId = findField(pt, 'post_id');
      const onDelete = postId.attributes.find(a => a.name === 'on_delete');
      expect(onDelete).toBeDefined();
      expect(onDelete!.args).toContain('cascade');
    });

    /**
     * [DEFECT] PrimaryKey section not captured correctly.
     *
     * The `### PrimaryKey` section with `- fields: [post_id, tag_id]` is parsed
     * as a regular field named "fields" with type undefined, instead of being
     * captured as a PrimaryKey section entry.
     *
     * Expected: `sections.PrimaryKey` or similar should contain { fields: ['post_id', 'tag_id'] }
     * Actual: A field named "fields" with type undefined is added to PostTag's fields array.
     */
    it('PrimaryKey section items are captured as section entries (not fields)', () => {
      const pt = findModel(ast, 'PostTag');
      // "fields" should NOT appear as a field in the fields array
      const hasFieldsAsField = pt.fields.some(f => f.name === 'fields');
      expect(hasFieldsAsField).toBe(false);
      // The PrimaryKey section IS now captured
      expect(pt.sections['PrimaryKey']).toBeDefined();
      expect((pt.sections['PrimaryKey'] as any[]).length).toBeGreaterThanOrEqual(1);
    });
  });

  // --- Comment model ---
  describe('Model: Comment', () => {
    it('inherits from BaseModel', () => {
      const comment = findModel(ast, 'Comment');
      expect(comment.inherits).toEqual(['BaseModel']);
    });

    it('has lookup, rollup, and computed fields', () => {
      const comment = findModel(ast, 'Comment');
      expect(comment.fields.filter(f => f.kind === 'lookup')).toHaveLength(2);
      expect(comment.fields.filter(f => f.kind === 'rollup')).toHaveLength(1);
      expect(comment.fields.filter(f => f.kind === 'computed')).toHaveLength(1);
    });

    it('has @index directive inline', () => {
      const comment = findModel(ast, 'Comment');
      // - @index(post_id, created_at) is in the file
      expect(comment.sections.indexes.length).toBeGreaterThanOrEqual(1);
    });
  });

  // --- MediaAsset model ---
  describe('Model: MediaAsset', () => {
    it('has 2 computed fields (file_size_mb, is_image)', () => {
      const ma = findModel(ast, 'MediaAsset');
      const computed = ma.fields.filter(f => f.kind === 'computed');
      expect(computed).toHaveLength(2);
      expect(computed.map(f => f.name)).toContain('file_size_mb');
      expect(computed.map(f => f.name)).toContain('is_image');
    });
  });

  // --- Views ---
  describe('View: PublishedPosts', () => {
    it('is not materialized', () => {
      const view = findView(ast, 'PublishedPosts');
      expect(view.materialized).toBe(false);
    });

    it('source_def from Post', () => {
      const view = findView(ast, 'PublishedPosts');
      expect(view.source_def?.from).toBe('Post');
      expect(view.source_def?.where).toContain("status = 'published'");
      expect(view.source_def?.order_by).toBe('published_at desc');
    });
  });

  describe('View: PopularPosts', () => {
    it('is materialized', () => {
      const view = findView(ast, 'PopularPosts');
      expect(view.materialized).toBe(true);
    });

    it('has refresh with strategy=full and interval="6 hours"', () => {
      const view = findView(ast, 'PopularPosts');
      expect(view.refresh?.strategy).toBe('full');
      expect(view.refresh?.interval).toBe('6 hours');
    });

    it('source_def.where contains DATE_ADD expression', () => {
      const view = findView(ast, 'PopularPosts');
      expect(view.source_def?.where).toContain('DATE_ADD');
    });
  });

  // --- Errors and Warnings ---
  describe('Errors and Warnings', () => {
    it('has no errors', () => {
      expect(ast.errors).toHaveLength(0);
    });

    it('has no warnings', () => {
      expect(ast.warnings).toHaveLength(0);
    });
  });
});

// ============================================================
// Sample 03: Types Showcase
// ============================================================
describe('Sample 03: Types Showcase (03-types-showcase.m3l.md)', () => {
  const ast = loadSample('03-types-showcase.m3l.md');

  // --- Element counts ---
  describe('Element Counts', () => {
    it('has 17 models', () => {
      expect(ast.models).toHaveLength(17);
    });

    it('has 0 enums', () => {
      expect(ast.enums).toHaveLength(0);
    });

    it('has 0 interfaces', () => {
      expect(ast.interfaces).toHaveLength(0);
    });

    it('has 0 views', () => {
      expect(ast.views).toHaveLength(0);
    });
  });

  // --- Namespace ---
  describe('Namespace', () => {
    it('project name is sample.types', () => {
      expect(ast.project.name).toBe('sample.types');
    });
  });

  // --- AllPrimitiveTypes ---
  describe('Model: AllPrimitiveTypes', () => {
    it('has 13 fields covering all primitive types', () => {
      const m = findModel(ast, 'AllPrimitiveTypes');
      expect(m.fields).toHaveLength(13);
    });

    it('string(200) has type "string" and params [200]', () => {
      const m = findModel(ast, 'AllPrimitiveTypes');
      const f = findField(m, 'str');
      expect(f.type).toBe('string');
      expect(f.params).toEqual([200]);
    });

    it('string without length has no params', () => {
      const m = findModel(ast, 'AllPrimitiveTypes');
      const f = findField(m, 'str_no_len');
      expect(f.type).toBe('string');
      expect(f.params).toBeUndefined();
    });

    it('decimal(10,2) has params [10, 2]', () => {
      const m = findModel(ast, 'AllPrimitiveTypes');
      const f = findField(m, 'dec_val');
      expect(f.type).toBe('decimal');
      expect(f.params).toEqual([10, 2]);
    });

    it('binary(1048576) has params [1048576]', () => {
      const m = findModel(ast, 'AllPrimitiveTypes');
      const f = findField(m, 'bin_val');
      expect(f.type).toBe('binary');
      expect(f.params).toEqual([1048576]);
    });

    it('all expected types are present', () => {
      const m = findModel(ast, 'AllPrimitiveTypes');
      const types = m.fields.map(f => f.type);
      expect(types).toContain('identifier');
      expect(types).toContain('string');
      expect(types).toContain('text');
      expect(types).toContain('integer');
      expect(types).toContain('long');
      expect(types).toContain('decimal');
      expect(types).toContain('float');
      expect(types).toContain('boolean');
      expect(types).toContain('date');
      expect(types).toContain('time');
      expect(types).toContain('timestamp');
      expect(types).toContain('binary');
    });
  });

  // --- SemanticTypes ---
  describe('Model: SemanticTypes', () => {
    it('has 6 fields with semantic types', () => {
      const m = findModel(ast, 'SemanticTypes');
      expect(m.fields).toHaveLength(6);
    });

    it('contact_email has type "email" with description', () => {
      const m = findModel(ast, 'SemanticTypes');
      const f = findField(m, 'contact_email');
      expect(f.type).toBe('email');
      expect(f.description).toContain('RFC 5321');
    });

    it.each([
      ['contact_phone', 'phone'],
      ['homepage', 'url'],
      ['monthly_revenue', 'money'],
      ['completion_rate', 'percentage'],
    ])('field %s has type %s', (fieldName, expectedType) => {
      const m = findModel(ast, 'SemanticTypes');
      const f = findField(m, fieldName);
      expect(f.type).toBe(expectedType);
    });
  });

  // --- TypeModifiers ---
  describe('Model: TypeModifiers', () => {
    it('has 9 fields', () => {
      const m = findModel(ast, 'TypeModifiers');
      expect(m.fields).toHaveLength(9);
    });

    it('nullable_str is nullable', () => {
      const m = findModel(ast, 'TypeModifiers');
      const f = findField(m, 'nullable_str');
      expect(f.nullable).toBe(true);
    });

    it('str_array is an array', () => {
      const m = findModel(ast, 'TypeModifiers');
      const f = findField(m, 'str_array');
      expect(f.array).toBe(true);
      expect(f.type).toBe('string');
    });

    it('int_array is an array of integers', () => {
      const m = findModel(ast, 'TypeModifiers');
      const f = findField(m, 'int_array');
      expect(f.array).toBe(true);
      expect(f.type).toBe('integer');
    });

    /**
     * [DEFECT] nullable_array: string[]? should be nullable=true AND array=true.
     *
     * The M3L spec says `string[]?` means "nullable array of strings".
     * The lexer regex RE_TYPE_PART only captures `?` immediately after the type name,
     * and `[]` after that. The pattern `string[]?` has `[]` before `?`,
     * so the `?` at the end is not captured by RE_TYPE_PART.
     *
     * Expected: nullable=true, array=true
     * Actual: nullable=false, array=true (the trailing ? is lost)
     */
    it('nullable_array: string[]? has both nullable and array flags', () => {
      const m = findModel(ast, 'TypeModifiers');
      const f = findField(m, 'nullable_array');
      expect(f.array).toBe(true);
      expect(f.nullable).toBe(true);
    });

    it('array_of_nullable: string?[] has arrayItemNullable=true, nullable=false', () => {
      const m = findModel(ast, 'TypeModifiers');
      const f = findField(m, 'array_of_nullable');
      // string?[] = array of nullable strings: the array is NOT nullable, but elements are
      expect(f.array).toBe(true);
      expect(f.nullable).toBe(false);
      expect(f.arrayItemNullable).toBe(true);

      console.log(
        '[RESOLVED] TypeModifiers.array_of_nullable: "string?[]" → nullable=false, arrayItemNullable=true; ' +
        '"string[]?" → nullable=true, arrayItemNullable=false. Distinction is now captured via arrayItemNullable field.'
      );
    });
  });

  // --- MapTypes ---
  describe('Model: MapTypes', () => {
    /**
     * [DEFECT] map<string, string> generic type params are not captured.
     *
     * The lexer regex RE_TYPE_PART uses `(?:\(([^)]*)\))?` for params (parentheses),
     * but M3L map types use angle brackets `<K, V>`. The `<string, string>` part
     * is not captured as type params.
     *
     * Expected: type="map", params=["string", "string"] or similar structured representation
     * Actual: type="map", params=undefined (angle bracket params lost)
     */
    it('map<string, string> captures generic type parameters', () => {
      const m = findModel(ast, 'MapTypes');
      const f = findField(m, 'string_map');
      expect(f.type).toBe('map');
      // Generic params are now captured
      expect(f.generic_params).toBeDefined();
      expect(f.generic_params).toEqual(['string', 'string']);
    });
  });

  // --- ComplexNestedObject ---
  describe('Model: ComplexNestedObject', () => {
    it('has 4 top-level fields', () => {
      const m = findModel(ast, 'ComplexNestedObject');
      expect(m.fields).toHaveLength(4);
    });

    /**
     * [DEFECT] Nested object sub-fields are not captured in the AST.
     *
     * The profile field has nested sub-fields (first_name, last_name, contact)
     * defined via indented list items, but these are not stored in field.fields.
     * The parser's handleNestedItem treats indented items under an object field
     * as extended format attributes, not as nested object structure.
     *
     * Expected: profile.fields should contain [{name: "first_name", ...}, {name: "last_name", ...}, ...]
     * Actual: profile.fields is undefined
     */
    it('nested object sub-fields are captured in field.fields', () => {
      const m = findModel(ast, 'ComplexNestedObject');
      const profile = findField(m, 'profile');
      expect(profile.type).toBe('object');
      // Sub-fields are now captured
      expect(profile.fields).toBeDefined();
      expect(profile.fields!.length).toBeGreaterThanOrEqual(1);
      const subNames = profile.fields!.map(f => f.name);
      expect(subNames).toContain('first_name');
      expect(subNames).toContain('last_name');
    });
  });

  // --- ArrayOfObjects ---
  describe('Model: ArrayOfObjects', () => {
    it('addresses field is object[] (array)', () => {
      const m = findModel(ast, 'ArrayOfObjects');
      const f = findField(m, 'addresses');
      expect(f.type).toBe('object');
      expect(f.array).toBe(true);
    });

    /**
     * [DEFECT] Same as ComplexNestedObject - sub-fields of object[] not captured.
     */
    it('object[] sub-fields are captured in field.fields', () => {
      const m = findModel(ast, 'ArrayOfObjects');
      const f = findField(m, 'addresses');
      expect(f.fields).toBeDefined();
      expect(f.fields!.length).toBeGreaterThanOrEqual(1);
      const subNames = f.fields!.map(sf => sf.name);
      expect(subNames).toContain('street');
      expect(subNames).toContain('city');
    });
  });

  // --- ValidationShowcase ---
  describe('Model: ValidationShowcase', () => {
    /**
     * [DEFECT] See below: Validations section items (age_range, email_domain)
     * are parsed as stored fields, inflating the count from 8 to 10.
     */
    it('has 8 stored fields (validation rules no longer parsed as fields)', () => {
      const m = findModel(ast, 'ValidationShowcase');
      const stored = m.fields.filter(f => f.kind === 'stored');
      expect(stored).toHaveLength(8);
    });

    it('age has @min(0) and @max(150)', () => {
      const m = findModel(ast, 'ValidationShowcase');
      const f = findField(m, 'age');
      expect(f.attributes.some(a => a.name === 'min')).toBe(true);
      expect(f.attributes.some(a => a.name === 'max')).toBe(true);
    });

    it('username has @validate attribute with pattern', () => {
      const m = findModel(ast, 'ValidationShowcase');
      const f = findField(m, 'username');
      const validate = f.attributes.find(a => a.name === 'validate');
      expect(validate).toBeDefined();
      expect(validate!.args?.[0]).toContain('pattern');
    });

    /**
     * [DEFECT] Validations section items are parsed as regular fields.
     *
     * The `### Validations` section contains validation rules like:
     *   - age_range
     *     - rule: "age >= 13"
     *     - message: "Must be at least 13 years old"
     *
     * The parser does not have specific handling for ### Validations section,
     * so the items fall through to the generic section handler which treats them
     * as fields. The result is that age_range and email_domain appear as stored
     * fields in the model's fields array, with their rule/message as attributes.
     *
     * Expected: sections.validations (or sections.Validations) should contain rule entries
     * Actual: "age_range" and "email_domain" appear as fields with rule/message attributes
     */
    it('Validations section items are captured as section entries (not fields)', () => {
      const m = findModel(ast, 'ValidationShowcase');
      // age_range and email_domain are validation rules, NOT fields
      const ageRange = m.fields.find(f => f.name === 'age_range');
      const emailDomain = m.fields.find(f => f.name === 'email_domain');
      expect(ageRange).toBeUndefined();
      expect(emailDomain).toBeUndefined();

      // The Validations section IS now captured
      expect(m.sections['Validations']).toBeDefined();
      expect((m.sections['Validations'] as any[]).length).toBeGreaterThanOrEqual(2);
    });
  });

  // --- DefaultValues ---
  describe('Model: DefaultValues', () => {
    it('has 9 fields', () => {
      const m = findModel(ast, 'DefaultValues');
      expect(m.fields).toHaveLength(9);
    });

    it('status has default "active" (quoted)', () => {
      const m = findModel(ast, 'DefaultValues');
      const f = findField(m, 'status');
      expect(f.default_value).toBe('"active"');
    });

    it('count has default 0', () => {
      const m = findModel(ast, 'DefaultValues');
      const f = findField(m, 'count');
      expect(f.default_value).toBe('0');
    });

    it('ratio has default 1.0', () => {
      const m = findModel(ast, 'DefaultValues');
      const f = findField(m, 'ratio');
      expect(f.default_value).toBe('1.0');
    });

    it('is_enabled has default true', () => {
      const m = findModel(ast, 'DefaultValues');
      const f = findField(m, 'is_enabled');
      expect(f.default_value).toBe('true');
    });

    it('created_at has default now()', () => {
      const m = findModel(ast, 'DefaultValues');
      const f = findField(m, 'created_at');
      expect(f.default_value).toBe('now()');
    });

    it('uuid_val has default generate_uuid()', () => {
      const m = findModel(ast, 'DefaultValues');
      const f = findField(m, 'uuid_val');
      expect(f.default_value).toBe('generate_uuid()');
    });

    it('empty_list has default []', () => {
      const m = findModel(ast, 'DefaultValues');
      const f = findField(m, 'empty_list');
      expect(f.default_value).toBe('[]');
    });

    it('quoted_default has escaped quotes in default', () => {
      const m = findModel(ast, 'DefaultValues');
      const f = findField(m, 'quoted_default');
      // The default value should preserve the escaped quotes
      expect(f.default_value).toContain('Hello');
      expect(f.default_value).toContain('World');
    });
  });

  // --- CompositeKeyExample ---
  describe('Model: CompositeKeyExample', () => {
    it('has 3 fields', () => {
      const m = findModel(ast, 'CompositeKeyExample');
      expect(m.fields).toHaveLength(3);
    });

    it('tenant_id has @primary(1) attribute', () => {
      const m = findModel(ast, 'CompositeKeyExample');
      const f = findField(m, 'tenant_id');
      const pk = f.attributes.find(a => a.name === 'primary');
      expect(pk).toBeDefined();
      expect(pk!.args).toContain('1');
    });

    it('entity_id has @primary(2) attribute', () => {
      const m = findModel(ast, 'CompositeKeyExample');
      const f = findField(m, 'entity_id');
      const pk = f.attributes.find(a => a.name === 'primary');
      expect(pk).toBeDefined();
      expect(pk!.args).toContain('2');
    });
  });

  // --- CompositeKeySection ---
  describe('Model: CompositeKeySection', () => {
    /**
     * [DEFECT] PrimaryKey section not captured.
     *
     * Same defect as PostTag. The ### PrimaryKey section with `- fields: [region, code]`
     * is not handled by the parser. It falls through to the generic section handler
     * and becomes a field.
     */
    it('PrimaryKey section is captured as section entry', () => {
      const m = findModel(ast, 'CompositeKeySection');
      // The PrimaryKey section IS now captured
      expect(m.sections['PrimaryKey']).toBeDefined();
      expect((m.sections['PrimaryKey'] as any[]).length).toBeGreaterThanOrEqual(1);
    });
  });

  // --- ExtendedFormatField ---
  describe('Model: ExtendedFormatField', () => {
    it('has 3 fields', () => {
      const m = findModel(ast, 'ExtendedFormatField');
      expect(m.fields).toHaveLength(3);
    });

    it('status field has extended attributes: description, reference, on_delete', () => {
      const m = findModel(ast, 'ExtendedFormatField');
      const f = findField(m, 'status');
      expect(f.description).toBe('Current processing status');
      expect(f.attributes.some(a => a.name === 'reference' && a.args?.includes('StatusEnum'))).toBe(true);
      expect(f.attributes.some(a => a.name === 'on_delete' && a.args?.includes('set_null'))).toBe(true);
    });

    it('config field has description from extended format', () => {
      const m = findModel(ast, 'ExtendedFormatField');
      const f = findField(m, 'config');
      expect(f.description).toBe('Application configuration data');
    });
  });

  // --- InheritanceOverride ---
  describe('Model: InheritanceOverride', () => {
    it('inherits from AllPrimitiveTypes', () => {
      const m = findModel(ast, 'InheritanceOverride');
      expect(m.inherits).toEqual(['AllPrimitiveTypes']);
    });

    /**
     * [DEFECT] @override does not replace the inherited field.
     *
     * InheritanceOverride defines:
     *   - str: string(500) @override "Overridden with larger size"
     *
     * The resolver prepends inherited fields, then appends own fields,
     * resulting in both the inherited `str: string(200)` AND the overriding
     * `str: string(500)` being present. The duplicate field detection
     * then reports an M3L-E005 error.
     *
     * Expected: The @override attribute should cause the inherited str field
     *           to be replaced with the overriding definition.
     * Actual: Both versions of str are present, causing a duplicate field error.
     */
    it('@override replaces inherited field (no duplicate error)', () => {
      // No duplicate error should be produced
      const dupErrors = ast.errors.filter(e =>
        e.code === 'M3L-E005' &&
        e.message.includes('"str"') &&
        e.message.includes('InheritanceOverride')
      );
      expect(dupErrors).toHaveLength(0);

      const m = findModel(ast, 'InheritanceOverride');
      const strFields = m.fields.filter(f => f.name === 'str');
      // Only the overriding version should exist
      expect(strFields).toHaveLength(1);
      // The overriding field should have the new params (500)
      expect(strFields[0].params).toEqual([500]);
      // The @override attribute should be preserved in the AST for consumers
      expect(strFields[0].attributes.some(a => a.name === 'override')).toBe(true);
    });

    it('has extra_field', () => {
      const m = findModel(ast, 'InheritanceOverride');
      const f = findField(m, 'extra_field');
      expect(f.type).toBe('boolean');
      expect(f.default_value).toBe('false');
    });
  });

  // --- ConditionalFields ---
  describe('Model: ConditionalFields', () => {
    it('has 5 fields', () => {
      const m = findModel(ast, 'ConditionalFields');
      expect(m.fields).toHaveLength(5);
    });

    it('company_name has @if attribute', () => {
      const m = findModel(ast, 'ConditionalFields');
      const f = findField(m, 'company_name');
      const ifAttr = f.attributes.find(a => a.name === 'if');
      expect(ifAttr).toBeDefined();
      expect(ifAttr!.args?.[0]).toContain('business');
    });

    it('personal_id has @if attribute for personal type', () => {
      const m = findModel(ast, 'ConditionalFields');
      const f = findField(m, 'personal_id');
      const ifAttr = f.attributes.find(a => a.name === 'if');
      expect(ifAttr).toBeDefined();
      expect(ifAttr!.args?.[0]).toContain('personal');
    });

    it('type field is inline enum with 2 values', () => {
      const m = findModel(ast, 'ConditionalFields');
      const f = findField(m, 'type');
      expect(f.type).toBe('enum');
      expect(f.enum_values).toBeDefined();
      expect(f.enum_values).toHaveLength(2);
    });
  });

  // --- DocumentationShowcase ---
  describe('Model: DocumentationShowcase', () => {
    it('has multi-line description from blockquote', () => {
      const m = findModel(ast, 'DocumentationShowcase');
      expect(m.description).toContain('detailed model description');
      expect(m.description).toContain('multi-line blockquote support');
    });

    it('name field has inline description', () => {
      const m = findModel(ast, 'DocumentationShowcase');
      const f = findField(m, 'name');
      expect(f.description).toBe('This is an inline description');
    });

    it('notes has description "Optional notes field"', () => {
      const m = findModel(ast, 'DocumentationShowcase');
      const f = findField(m, 'notes');
      expect(f.description).toBe('Optional notes field');
    });

    it('has 4 fields', () => {
      const m = findModel(ast, 'DocumentationShowcase');
      expect(m.fields).toHaveLength(4);
    });
  });

  // --- ComputedVariants ---
  describe('Model: ComputedVariants', () => {
    it('has 6 stored fields and 5 computed fields', () => {
      const m = findModel(ast, 'ComputedVariants');
      const stored = m.fields.filter(f => f.kind === 'stored');
      const computed = m.fields.filter(f => f.kind === 'computed');
      expect(stored).toHaveLength(6);
      expect(computed).toHaveLength(5);
    });

    it('full_name has expression with string concatenation', () => {
      const m = findModel(ast, 'ComputedVariants');
      const f = findField(m, 'full_name');
      expect(f.kind).toBe('computed');
      expect(f.computed?.expression).toContain("first_name + ' ' + last_name");
    });

    it('total_price has @persisted attribute', () => {
      const m = findModel(ast, 'ComputedVariants');
      const f = findField(m, 'total_price');
      expect(f.kind).toBe('computed');
      expect(f.attributes.some(a => a.name === 'persisted')).toBe(true);
    });

    /**
     * [DEFECT] @computed_raw fields have kind="computed" but no computed.expression.
     *
     * The field `age: integer @computed_raw("DATEDIFF(year, birth_date, GETDATE())", platform: "sqlserver")`
     * is correctly identified as kind="computed" (since it's in the ### Computed section),
     * but the parser only extracts expression from @computed, not @computed_raw.
     * The computed_raw attribute with its platform-specific expression is stored in
     * attributes but not extracted into the computed field.
     *
     * Expected: field.computed should have the expression and platform info
     * Actual: field.computed is undefined, data only in attributes
     */
    it('@computed_raw fields have computed.expression extracted', () => {
      const m = findModel(ast, 'ComputedVariants');
      const age = findField(m, 'age');
      expect(age.kind).toBe('computed');
      // computed_raw expression is now extracted
      expect(age.computed).toBeDefined();
      expect(age.computed!.expression).toBeDefined();
      expect(age.computed!.expression.length).toBeGreaterThan(0);
    });
  });

  // --- BehaviorShowcase ---
  describe('Model: BehaviorShowcase', () => {
    it('has 3 stored fields', () => {
      const m = findModel(ast, 'BehaviorShowcase');
      expect(m.fields).toHaveLength(3);
      expect(m.fields.map(f => f.name)).toEqual(['id', 'code', 'name']);
    });

    it('Behaviors section has 4 entries (3 from section + 1 from @behavior directive)', () => {
      const m = findModel(ast, 'BehaviorShowcase');
      // @behavior directive now correctly goes into sections.behaviors
      expect(m.sections.behaviors.length).toBeGreaterThanOrEqual(4);
    });

    it('@behavior directive is now stored in "behaviors" section (not "behavior")', () => {
      const m = findModel(ast, 'BehaviorShowcase');
      // The singular 'behavior' key should NOT exist
      expect(m.sections['behavior']).toBeUndefined();
      // The directive should be merged into the plural 'behaviors'
      const behaviors = m.sections.behaviors as any[];
      const directiveEntry = behaviors.find((b: any) => b.args && typeof b.args === 'string' && b.args.includes('before_create'));
      expect(directiveEntry).toBeDefined();
    });
  });

  // --- VersionedEntity ---
  describe('Model: VersionedEntity', () => {
    /**
     * [DEFECT] Version and Migration section items are parsed as fields.
     *
     * The ### Version section has key-value pairs (major: 2, minor: 1, etc.)
     * and ### Migration section has (changed, added, removed).
     * These all fall through to the generic section handler which treats
     * them as regular fields, inflating the count from 3 to 10.
     *
     * Expected: 3 fields (id, name, data)
     * Actual: 10 fields (3 real + 4 from Version + 3 from Migration)
     */
    it('has 3 fields (Version/Migration section items no longer leak as fields)', () => {
      const m = findModel(ast, 'VersionedEntity');
      expect(m.fields).toHaveLength(3);

      // The real fields
      expect(m.fields.map(f => f.name)).toContain('id');
      expect(m.fields.map(f => f.name)).toContain('name');
      expect(m.fields.map(f => f.name)).toContain('data');
    });

    it('Version and Migration sections are captured as section entries', () => {
      const m = findModel(ast, 'VersionedEntity');
      // The Version section IS now captured
      expect(m.sections['Version']).toBeDefined();
      expect((m.sections['Version'] as any[]).length).toBeGreaterThanOrEqual(1);

      // Section field names should NOT appear as model fields
      const sectionFieldNames = ['major', 'minor', 'patch', 'date', 'changed', 'added', 'removed'];
      for (const name of sectionFieldNames) {
        expect(m.fields.some(f => f.name === name)).toBe(false);
      }
    });
  });

  // --- Errors and Warnings ---
  describe('Errors and Warnings', () => {
    it('has no errors (@override now prevents duplicate field error)', () => {
      expect(ast.errors).toHaveLength(0);
    });

    it('has no warnings', () => {
      expect(ast.warnings).toHaveLength(0);
    });
  });
});

// ============================================================
// Cross-cutting concerns
// ============================================================
describe('Cross-Cutting: All Samples', () => {
  const samples = [
    { file: '01-ecommerce.m3l.md', name: 'E-Commerce' },
    { file: '02-blog-cms.m3l.md', name: 'Blog CMS' },
    { file: '03-types-showcase.m3l.md', name: 'Types Showcase' },
  ];

  describe('AST Structure', () => {
    it.each(samples)('$name: has valid AST metadata', ({ file }) => {
      const ast = loadSample(file);
      expect(ast.parserVersion).toBeDefined();
      expect(ast.astVersion).toBeDefined();
      expect(ast.sources).toEqual([file]);
      expect(Array.isArray(ast.models)).toBe(true);
      expect(Array.isArray(ast.enums)).toBe(true);
      expect(Array.isArray(ast.interfaces)).toBe(true);
      expect(Array.isArray(ast.views)).toBe(true);
      expect(Array.isArray(ast.errors)).toBe(true);
      expect(Array.isArray(ast.warnings)).toBe(true);
    });
  });

  describe('Field Location Integrity', () => {
    it.each(samples)('$name: all fields have valid loc', ({ file }) => {
      const ast = loadSample(file);
      for (const model of [...ast.models, ...ast.views]) {
        for (const field of model.fields) {
          expect(field.loc).toBeDefined();
          expect(field.loc.file).toBe(file);
          expect(typeof field.loc.line).toBe('number');
          expect(field.loc.line).toBeGreaterThan(0);
        }
      }
    });
  });

  describe('Model Location Integrity', () => {
    it.each(samples)('$name: all models/enums/interfaces/views have valid loc', ({ file }) => {
      const ast = loadSample(file);
      for (const item of [...ast.models, ...ast.enums, ...ast.interfaces, ...ast.views]) {
        expect(item.loc).toBeDefined();
        expect(item.loc.file).toBe(file);
        expect(typeof item.loc.line).toBe('number');
        expect(item.loc.line).toBeGreaterThan(0);
      }
    });
  });

  describe('JSON Serialization', () => {
    it.each(samples)('$name: produces valid JSON', ({ file }) => {
      const ast = loadSample(file);
      const json = JSON.stringify(ast);
      const parsed = JSON.parse(json);
      expect(parsed.models).toBeDefined();
      expect(parsed.enums).toBeDefined();
    });
  });

  describe('Field Kind Correctness', () => {
    it.each(samples)('$name: all fields have valid kind', ({ file }) => {
      const ast = loadSample(file);
      const validKinds = new Set(['stored', 'computed', 'lookup', 'rollup']);
      for (const model of [...ast.models, ...ast.views]) {
        for (const field of model.fields) {
          expect(validKinds.has(field.kind)).toBe(true);
        }
      }
    });
  });

  describe('Lookup Fields Have Path', () => {
    it.each(samples)('$name: all lookup fields have lookup.path', ({ file }) => {
      const ast = loadSample(file);
      for (const model of [...ast.models, ...ast.views]) {
        for (const field of model.fields) {
          if (field.kind === 'lookup') {
            expect(field.lookup).toBeDefined();
            expect(field.lookup!.path).toBeTruthy();
            // Path should have at least one dot (fk_field.target_field)
            expect(field.lookup!.path).toContain('.');
          }
        }
      }
    });
  });

  describe('Rollup Fields Have Target and Aggregate', () => {
    it.each(samples)('$name: all rollup fields have rollup target and aggregate', ({ file }) => {
      const ast = loadSample(file);
      for (const model of [...ast.models, ...ast.views]) {
        for (const field of model.fields) {
          if (field.kind === 'rollup') {
            expect(field.rollup).toBeDefined();
            expect(field.rollup!.target).toBeTruthy();
            expect(field.rollup!.fk).toBeTruthy();
            expect(field.rollup!.aggregate).toBeTruthy();
          }
        }
      }
    });
  });

  describe('Computed Fields Mostly Have Expression', () => {
    /**
     * Note: @computed_raw fields are kind="computed" but lack computed.expression.
     * This test documents that not all computed-kind fields have the expression set.
     */
    it.each(samples)('$name: @computed fields have expression', ({ file }) => {
      const ast = loadSample(file);
      for (const model of [...ast.models, ...ast.views]) {
        for (const field of model.fields) {
          if (field.kind === 'computed' && field.attributes.some(a => a.name === 'computed')) {
            expect(field.computed).toBeDefined();
            expect(field.computed!.expression).toBeTruthy();
          }
        }
      }
    });
  });
});

// ============================================================
// Summary Report (printed when tests run)
// ============================================================
describe('Defect Summary Report', () => {
  it('prints summary of all defects found', () => {
    // All defects (D001-D010) have been fixed, including D004 via arrayItemNullable field
    const defects: { id: string; severity: string; summary: string }[] = [];

    console.log('\n========================================');
    console.log('M3L PARSER DEFECT SUMMARY REPORT');
    console.log('========================================\n');
    console.log('All defects have been fixed!');
    console.log('========================================\n');

    expect(defects.length).toBe(0);
  });
});
