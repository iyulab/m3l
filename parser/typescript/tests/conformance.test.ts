/**
 * M3L Conformance Test Suite
 *
 * These tests verify the AST output contract that all M3L parser bindings must satisfy.
 * Each test case uses a .m3l.md input file and validates the resulting AST structure.
 *
 * When implementing a new parser binding (e.g., C#, Python), port these tests
 * to ensure cross-binding compatibility.
 */
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { parseString } from '../src/index.js';
import { lex } from '../src/lexer.js';
import { parseTokens } from '../src/parser.js';
import { resolve, AST_VERSION, PARSER_VERSION } from '../src/resolver.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const conformanceDir = join(__dirname, 'conformance');

function loadAndParse(filename: string) {
  const content = readFileSync(join(conformanceDir, filename), 'utf-8');
  return parseString(content, filename);
}

function loadAndParseSingle(filename: string) {
  const content = readFileSync(join(conformanceDir, filename), 'utf-8');
  const tokens = lex(content, filename);
  return parseTokens(tokens, filename);
}

// ============================================================
// 1. AST Metadata
// ============================================================
describe('Conformance: AST Metadata', () => {
  it('AST includes parserVersion and astVersion', () => {
    const ast = loadAndParse('basic-model.m3l.md');
    expect(ast.parserVersion).toBe(PARSER_VERSION);
    expect(ast.astVersion).toBe(AST_VERSION);
    expect(typeof ast.parserVersion).toBe('string');
    expect(typeof ast.astVersion).toBe('string');
  });

  it('AST includes sources array', () => {
    const ast = loadAndParse('basic-model.m3l.md');
    expect(ast.sources).toEqual(['basic-model.m3l.md']);
  });

  it('AST includes errors and warnings arrays', () => {
    const ast = loadAndParse('basic-model.m3l.md');
    expect(Array.isArray(ast.errors)).toBe(true);
    expect(Array.isArray(ast.warnings)).toBe(true);
  });
});

// ============================================================
// 2. Basic Model
// ============================================================
describe('Conformance: Basic Model', () => {
  const ast = loadAndParse('basic-model.m3l.md');

  it('parses one model named "User"', () => {
    expect(ast.models).toHaveLength(1);
    expect(ast.models[0].name).toBe('User');
    expect(ast.models[0].type).toBe('model');
  });

  it('model has 4 fields', () => {
    const fields = ast.models[0].fields;
    expect(fields).toHaveLength(4);
    expect(fields.map(f => f.name)).toEqual(['id', 'name', 'email', 'is_active']);
  });

  it('field types are parsed correctly', () => {
    const fields = ast.models[0].fields;
    expect(fields[0].type).toBe('identifier');
    expect(fields[1].type).toBe('string');
    expect(fields[1].params).toEqual([100]);
    expect(fields[2].type).toBe('string');
    expect(fields[2].params).toEqual([320]);
    expect(fields[3].type).toBe('boolean');
  });

  it('nullable is detected from ? suffix', () => {
    const fields = ast.models[0].fields;
    expect(fields[0].nullable).toBe(false);
    expect(fields[2].nullable).toBe(true); // email: string(320)?
  });

  it('default value is parsed', () => {
    const fields = ast.models[0].fields;
    expect(fields[3].default_value).toBe('true'); // is_active: boolean = true
  });

  it('attributes are parsed with correct names', () => {
    const fields = ast.models[0].fields;
    expect(fields[0].attributes.some(a => a.name === 'pk')).toBe(true);
    expect(fields[1].attributes.some(a => a.name === 'not_null')).toBe(true);
    expect(fields[2].attributes.some(a => a.name === 'unique')).toBe(true);
  });

  it('all fields have loc with file and line', () => {
    for (const field of ast.models[0].fields) {
      expect(field.loc.file).toBe('basic-model.m3l.md');
      expect(typeof field.loc.line).toBe('number');
      expect(field.loc.line).toBeGreaterThan(0);
    }
  });
});

// ============================================================
// 3. Standalone Enum
// ============================================================
describe('Conformance: Standalone Enum', () => {
  const ast = loadAndParse('enum-standalone.m3l.md');

  it('parses one enum named "Status"', () => {
    expect(ast.enums).toHaveLength(1);
    expect(ast.enums[0].name).toBe('Status');
  });

  it('enum has 3 values with descriptions', () => {
    const values = ast.enums[0].values;
    expect(values).toHaveLength(3);
    expect(values[0].name).toBe('active');
    expect(values[0].description).toBe('Active');
    expect(values[1].name).toBe('inactive');
    expect(values[1].description).toBe('Inactive');
    expect(values[2].name).toBe('pending');
    expect(values[2].description).toBe('Pending');
  });

  it('no models, interfaces, or views', () => {
    expect(ast.models).toHaveLength(0);
    expect(ast.interfaces).toHaveLength(0);
    expect(ast.views).toHaveLength(0);
  });
});

// ============================================================
// 4. Inheritance
// ============================================================
describe('Conformance: Inheritance', () => {
  const ast = loadAndParse('inheritance.m3l.md');

  it('parses 2 models', () => {
    expect(ast.models).toHaveLength(2);
  });

  it('User inherits from BaseModel', () => {
    const user = ast.models.find(m => m.name === 'User')!;
    expect(user.inherits).toEqual(['BaseModel']);
  });

  it('User has inherited fields prepended', () => {
    const user = ast.models.find(m => m.name === 'User')!;
    // After inheritance resolution: id, created_at (from BaseModel), then name, email (own)
    expect(user.fields).toHaveLength(4);
    expect(user.fields[0].name).toBe('id');
    expect(user.fields[1].name).toBe('created_at');
    expect(user.fields[2].name).toBe('name');
    expect(user.fields[3].name).toBe('email');
  });

  it('BaseModel retains its own fields only', () => {
    const base = ast.models.find(m => m.name === 'BaseModel')!;
    expect(base.fields).toHaveLength(2);
  });
});

// ============================================================
// 5. Lookup and Rollup
// ============================================================
describe('Conformance: Lookup and Rollup', () => {
  const ast = loadAndParse('lookup-rollup.m3l.md');

  it('Order has a lookup field', () => {
    const order = ast.models.find(m => m.name === 'Order')!;
    const lookup = order.fields.find(f => f.kind === 'lookup');
    expect(lookup).toBeDefined();
    expect(lookup!.name).toBe('customer_name');
    expect(lookup!.lookup?.path).toBe('customer_id.name');
  });

  it('OrderSummary has rollup fields', () => {
    const summary = ast.models.find(m => m.name === 'OrderSummary')!;
    const rollups = summary.fields.filter(f => f.kind === 'rollup');
    expect(rollups).toHaveLength(2);

    const orderCount = rollups.find(f => f.name === 'order_count')!;
    expect(orderCount.rollup?.target).toBe('Order');
    expect(orderCount.rollup?.fk).toBe('customer_id');
    expect(orderCount.rollup?.aggregate).toBe('count');

    const totalSpent = rollups.find(f => f.name === 'total_spent')!;
    expect(totalSpent.rollup?.target).toBe('Order');
    expect(totalSpent.rollup?.aggregate).toBe('sum');
    expect(totalSpent.rollup?.field).toBe('total');
  });

  it('@fk attribute is parsed with args', () => {
    const order = ast.models.find(m => m.name === 'Order')!;
    const fkField = order.fields.find(f => f.name === 'customer_id')!;
    const fkAttr = fkField.attributes.find(a => a.name === 'fk');
    expect(fkAttr).toBeDefined();
    expect(fkAttr!.args).toBeDefined();
  });
});

// ============================================================
// 6. View
// ============================================================
describe('Conformance: View', () => {
  const ast = loadAndParse('view.m3l.md');

  it('parses one view named "ActiveUsers"', () => {
    expect(ast.views).toHaveLength(1);
    expect(ast.views[0].name).toBe('ActiveUsers');
    expect(ast.views[0].type).toBe('view');
  });

  it('view has source_def with from, where, order_by', () => {
    const view = ast.views[0];
    expect(view.source_def).toBeDefined();
    expect(view.source_def!.from).toBe('User');
    expect(view.source_def!.where).toBe('is_active = true');
    expect(view.source_def!.order_by).toBe('name asc');
  });
});

// ============================================================
// 7. Framework Attributes (CustomAttribute)
// ============================================================
describe('Conformance: Framework Attributes', () => {
  const ast = loadAndParse('framework-attrs.m3l.md');

  it('framework_attrs are structured as CustomAttribute[]', () => {
    const account = ast.models.find(m => m.name === 'Account')!;
    const password = account.fields.find(f => f.name === 'password')!;

    expect(password.framework_attrs).toBeDefined();
    expect(password.framework_attrs).toHaveLength(2);

    // Each item should have content and raw
    const first = password.framework_attrs![0];
    expect(first.content).toBe('DataType(DataType.Password)');
    expect(first.raw).toBe('[DataType(DataType.Password)]');

    const second = password.framework_attrs![1];
    expect(second.content).toBe('JsonIgnore');
    expect(second.raw).toBe('[JsonIgnore]');
  });

  it('fields without framework attrs have undefined framework_attrs', () => {
    const account = ast.models.find(m => m.name === 'Account')!;
    const displayName = account.fields.find(f => f.name === 'display_name')!;
    expect(displayName.framework_attrs).toBeUndefined();
  });
});

// ============================================================
// 8. Computed Field
// ============================================================
describe('Conformance: Computed Field', () => {
  const ast = loadAndParse('computed-field.m3l.md');

  it('computed field has kind="computed" and expression', () => {
    const product = ast.models.find(m => m.name === 'Product')!;
    const computed = product.fields.find(f => f.name === 'total_value')!;

    expect(computed.kind).toBe('computed');
    expect(computed.computed).toBeDefined();
    expect(computed.computed!.expression).toBe('price * quantity');
  });
});

// ============================================================
// 9. Interface
// ============================================================
describe('Conformance: Interface', () => {
  const ast = loadAndParse('interface.m3l.md');

  it('parses one interface named "Timestampable"', () => {
    expect(ast.interfaces).toHaveLength(1);
    expect(ast.interfaces[0].name).toBe('Timestampable');
    expect(ast.interfaces[0].type).toBe('interface');
  });

  it('Article inherits interface fields', () => {
    const article = ast.models.find(m => m.name === 'Article')!;
    // Inherited: created_at, updated_at; Own: id, title, content
    expect(article.fields).toHaveLength(5);
    expect(article.fields[0].name).toBe('created_at');
    expect(article.fields[1].name).toBe('updated_at');
    expect(article.fields[2].name).toBe('id');
  });
});

// ============================================================
// 10. FieldAttribute.args typing
// ============================================================
describe('Conformance: Attribute args typing', () => {
  it('attribute args are string | number | boolean (not unknown)', () => {
    const ast = loadAndParse('basic-model.m3l.md');
    const fields = ast.models[0].fields;
    for (const field of fields) {
      for (const attr of field.attributes) {
        if (attr.args) {
          for (const arg of attr.args) {
            expect(['string', 'number', 'boolean']).toContain(typeof arg);
          }
        }
      }
    }
  });
});
