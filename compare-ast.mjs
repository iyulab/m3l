/**
 * M3L AST Conformance Comparison Script
 *
 * Compares TypeScript and C# parser AST JSON outputs for semantic equivalence.
 * Normalizes known naming differences and ignores cosmetic differences
 * (file paths, line numbers, parserVersion, etc.)
 */

import { readFileSync } from 'fs';

// ============================================================
// Configuration: sample pairs
// ============================================================
const SAMPLES = [
  {
    label: '01-ecommerce',
    ts: 'parser/typescript/tests/conformance-output/01-ecommerce.json',
    cs: 'parser/csharp/tests/M3L.Tests/conformance-output/01-ecommerce.json',
  },
  {
    label: '02-blog-cms',
    ts: 'parser/typescript/tests/conformance-output/02-blog-cms.json',
    cs: 'parser/csharp/tests/M3L.Tests/conformance-output/02-blog-cms.json',
  },
  {
    label: '03-types-showcase',
    ts: 'parser/typescript/tests/conformance-output/03-types-showcase.json',
    cs: 'parser/csharp/tests/M3L.Tests/conformance-output/03-types-showcase.json',
  },
  {
    label: '04-multi-file',
    ts: 'parser/typescript/tests/conformance-output/04-multi-file.json',
    cs: 'parser/csharp/tests/M3L.Tests/conformance-output/04-multi-inventory.json',
  },
];

const BASE = 'D:/data/m3l/';

// ============================================================
// Known property name mappings (TS -> CS equivalents)
// snake_case in TS that becomes camelCase in CS
// ============================================================
const PROP_ALIASES = {
  // TS name -> CS name (both directions handled)
  'default_value': 'defaultValue',
  'enum_values': 'enumValues',
  'source_def': 'sourceDef',
  'order_by': 'orderBy',
  'group_by': 'groupBy',
  'type': 'nodeType',          // on model/interface/view nodes
};

// Properties to ignore entirely during comparison
const IGNORE_KEYS = new Set([
  'parserVersion', 'sources', 'loc', 'line', 'col', 'file', 'source',
]);

// ============================================================
// Difference collector
// ============================================================
class DiffCollector {
  constructor(sampleLabel) {
    this.sampleLabel = sampleLabel;
    this.diffs = [];
  }

  add(path, tsVal, csVal, message) {
    this.diffs.push({ path, tsVal, csVal, message });
  }

  get count() {
    return this.diffs.length;
  }

  print() {
    if (this.diffs.length === 0) {
      console.log('    (no differences)');
      return;
    }
    for (const d of this.diffs) {
      console.log(`    DIFF @ ${d.path}`);
      console.log(`      ${d.message}`);
      if (d.tsVal !== undefined) console.log(`      TS: ${fmt(d.tsVal)}`);
      if (d.csVal !== undefined) console.log(`      CS: ${fmt(d.csVal)}`);
    }
  }
}

function fmt(val) {
  if (val === undefined) return '(undefined)';
  if (val === null) return '(null)';
  if (typeof val === 'object') return JSON.stringify(val);
  return String(val);
}

// ============================================================
// Normalization helpers
// ============================================================

/**
 * Normalize a property name from TS snake_case to the canonical form.
 * Returns the "canonical" key name so both sides can be compared.
 */
function canonicalKey(key) {
  // Check if this key has an alias mapping
  if (PROP_ALIASES[key]) return PROP_ALIASES[key];
  return key;
}

/**
 * Build a normalized key map from an object.
 * Returns Map<canonicalKey, { origKey, value }>
 */
function normalizedEntries(obj) {
  const map = new Map();
  for (const [k, v] of Object.entries(obj)) {
    const ck = canonicalKey(k);
    map.set(ck, { origKey: k, value: v });
  }
  return map;
}

// ============================================================
// Deep comparison engine
// ============================================================

function deepCompare(tsVal, csVal, path, collector, context = {}) {
  // Both null/undefined
  if (tsVal == null && csVal == null) return;

  // One null, other not
  if (tsVal == null && csVal != null) {
    // Check if csVal is a "zero value" that can be ignored
    if (isEmptyValue(csVal)) return;
    collector.add(path, tsVal, csVal, 'TS is null/undefined but CS has a value');
    return;
  }
  if (tsVal != null && csVal == null) {
    if (isEmptyValue(tsVal)) return;
    collector.add(path, tsVal, csVal, 'TS has a value but CS is null/undefined');
    return;
  }

  // Type mismatch
  if (typeof tsVal !== typeof csVal) {
    // Try numeric coercion (e.g., "0" vs 0)
    if (typeof tsVal === 'number' && typeof csVal === 'string' && String(tsVal) === csVal) return;
    if (typeof tsVal === 'string' && typeof csVal === 'number' && tsVal === String(csVal)) return;
    collector.add(path, tsVal, csVal, `Type mismatch: TS=${typeof tsVal}, CS=${typeof csVal}`);
    return;
  }

  // Primitives
  if (typeof tsVal !== 'object') {
    if (tsVal !== csVal) {
      collector.add(path, tsVal, csVal, 'Value mismatch');
    }
    return;
  }

  // Arrays
  if (Array.isArray(tsVal) && Array.isArray(csVal)) {
    if (tsVal.length !== csVal.length) {
      collector.add(path, `length=${tsVal.length}`, `length=${csVal.length}`, 'Array length mismatch');
      // Compare up to the shorter length
    }
    const len = Math.max(tsVal.length, csVal.length);
    for (let i = 0; i < len; i++) {
      deepCompare(tsVal[i], csVal[i], `${path}[${i}]`, collector, context);
    }
    return;
  }

  if (Array.isArray(tsVal) !== Array.isArray(csVal)) {
    collector.add(path, tsVal, csVal, 'One is array, other is object');
    return;
  }

  // Objects - compare normalized keys
  const tsMap = normalizedEntries(tsVal);
  const csMap = normalizedEntries(csVal);
  const allKeys = new Set([...tsMap.keys(), ...csMap.keys()]);

  for (const ck of allKeys) {
    // Skip ignored keys
    if (IGNORE_KEYS.has(ck)) continue;
    // Also skip the original key names that map to ignored keys
    const tsEntry = tsMap.get(ck);
    const csEntry = csMap.get(ck);

    // Skip if the original key is in IGNORE_KEYS
    if (tsEntry && IGNORE_KEYS.has(tsEntry.origKey)) continue;
    if (csEntry && IGNORE_KEYS.has(csEntry.origKey)) continue;

    const tsV = tsEntry?.value;
    const csV = csEntry?.value;

    // Special handling: 'extra' key in CS sections that's empty
    if (ck === 'extra' && !tsEntry && csEntry && isEmptyValue(csV)) continue;

    // Special handling: 'materialized' key that CS may add but TS omits
    if (ck === 'materialized' && !tsEntry && csEntry) {
      // If CS has materialized: false, it's just a default. Skip.
      if (csV === false) continue;
    }

    // Special handling: 'label' in index entries - CS may add null labels
    if (ck === 'label' && !tsEntry && csEntry && csV === null) continue;

    // Special handling: 'description' may be present in CS but absent in TS, or vice versa
    // This is a real semantic difference - report it

    if (!tsEntry && csEntry) {
      if (!isEmptyValue(csV)) {
        collector.add(`${path}.${ck}`, undefined, csV, `Property only in CS (cs key: "${csEntry.origKey}")`);
      }
      continue;
    }
    if (tsEntry && !csEntry) {
      if (!isEmptyValue(tsV)) {
        collector.add(`${path}.${ck}`, tsV, undefined, `Property only in TS (ts key: "${tsEntry.origKey}")`);
      }
      continue;
    }

    deepCompare(tsV, csV, `${path}.${ck}`, collector, context);
  }
}

function isEmptyValue(val) {
  if (val === null || val === undefined) return true;
  if (val === false) return true;
  if (val === 0) return false; // 0 is meaningful
  if (val === '') return true;
  if (Array.isArray(val) && val.length === 0) return true;
  if (typeof val === 'object' && Object.keys(val).length === 0) return true;
  return false;
}

// ============================================================
// High-level comparison functions
// ============================================================

function compareModels(tsModels, csModels, collector) {
  const tsNames = tsModels.map(m => m.name).sort();
  const csNames = csModels.map(m => m.name).sort();

  console.log(`  Models: TS=${tsModels.length}, CS=${csModels.length}`);
  console.log(`    TS names: [${tsNames.join(', ')}]`);
  console.log(`    CS names: [${csNames.join(', ')}]`);

  if (JSON.stringify(tsNames) !== JSON.stringify(csNames)) {
    collector.add('models', tsNames, csNames, 'Model name lists differ');
  }

  // Compare each model by name
  for (const tsModel of tsModels) {
    const csModel = csModels.find(m => m.name === tsModel.name);
    if (!csModel) {
      collector.add(`models.${tsModel.name}`, tsModel.name, undefined, 'Model exists in TS but not CS');
      continue;
    }
    compareModelNode(tsModel, csModel, `models.${tsModel.name}`, collector);
  }
  for (const csModel of csModels) {
    if (!tsModels.find(m => m.name === csModel.name)) {
      collector.add(`models.${csModel.name}`, undefined, csModel.name, 'Model exists in CS but not TS');
    }
  }
}

function compareModelNode(tsModel, csModel, path, collector) {
  // Compare top-level model properties (excluding ignored ones)
  const modelIgnore = new Set([...IGNORE_KEYS, 'fields', 'sections', 'nodeType', 'type']);

  // Compare inherits
  deepCompare(tsModel.inherits || [], csModel.inherits || [], `${path}.inherits`, collector);

  // Compare attributes
  deepCompare(tsModel.attributes || [], csModel.attributes || [], `${path}.attributes`, collector);

  // Compare description
  const tsDesc = tsModel.description;
  const csDesc = csModel.description;
  if ((tsDesc || '') !== (csDesc || '')) {
    if (tsDesc && csDesc) {
      collector.add(`${path}.description`, tsDesc, csDesc, 'Description mismatch');
    } else if (tsDesc && !csDesc) {
      collector.add(`${path}.description`, tsDesc, undefined, 'Description only in TS');
    } else if (!tsDesc && csDesc) {
      collector.add(`${path}.description`, undefined, csDesc, 'Description only in CS');
    }
  }

  // Compare materialized (for views)
  if (tsModel.materialized !== undefined || csModel.materialized !== undefined) {
    const tsM = tsModel.materialized ?? false;
    const csM = csModel.materialized ?? false;
    if (tsM !== csM) {
      collector.add(`${path}.materialized`, tsM, csM, 'Materialized mismatch');
    }
  }

  // Compare source_def / sourceDef (for views)
  const tsSourceDef = tsModel.source_def || tsModel.sourceDef;
  const csSourceDef = csModel.sourceDef || csModel.source_def;
  if (tsSourceDef || csSourceDef) {
    deepCompare(tsSourceDef, csSourceDef, `${path}.sourceDef`, collector);
  }

  // Compare refresh (for views)
  deepCompare(tsModel.refresh, csModel.refresh, `${path}.refresh`, collector);

  // Compare fields
  compareFields(tsModel.fields || [], csModel.fields || [], path, collector);

  // Compare sections
  compareSections(tsModel.sections, csModel.sections, path, collector);
}

function compareFields(tsFields, csFields, modelPath, collector) {
  const path = `${modelPath}.fields`;

  if (tsFields.length !== csFields.length) {
    console.log(`    ${modelPath}: Field count TS=${tsFields.length}, CS=${csFields.length}`);
    const tsNames = tsFields.map(f => f.name);
    const csNames = csFields.map(f => f.name);
    collector.add(path, `count=${tsFields.length} [${tsNames}]`, `count=${csFields.length} [${csNames}]`, 'Field count mismatch');
  }

  // Match fields by name
  const tsFieldMap = new Map(tsFields.map(f => [f.name, f]));
  const csFieldMap = new Map(csFields.map(f => [f.name, f]));

  // Report field names
  const tsNames = tsFields.map(f => f.name);
  const csNames = csFields.map(f => f.name);

  // Check ordering
  const commonNames = tsNames.filter(n => csNames.includes(n));
  const csCommonOrder = csNames.filter(n => tsNames.includes(n));
  if (JSON.stringify(commonNames) !== JSON.stringify(csCommonOrder)) {
    collector.add(`${path}(order)`, commonNames, csCommonOrder, 'Field order differs');
  }

  // Fields only in one side
  for (const name of tsNames) {
    if (!csFieldMap.has(name)) {
      collector.add(`${path}.${name}`, name, undefined, 'Field only in TS');
    }
  }
  for (const name of csNames) {
    if (!tsFieldMap.has(name)) {
      collector.add(`${path}.${name}`, undefined, name, 'Field only in CS');
    }
  }

  // Compare matching fields
  for (const [name, tsField] of tsFieldMap) {
    const csField = csFieldMap.get(name);
    if (!csField) continue;

    const fp = `${path}.${name}`;
    compareField(tsField, csField, fp, collector);
  }
}

function compareField(tsField, csField, path, collector) {
  // Compare type
  if (tsField.type !== csField.type) {
    collector.add(`${path}.type`, tsField.type, csField.type, 'Field type mismatch');
  }

  // Compare params
  deepCompare(tsField.params, csField.params, `${path}.params`, collector);

  // Compare nullable
  if (tsField.nullable !== csField.nullable) {
    collector.add(`${path}.nullable`, tsField.nullable, csField.nullable, 'Nullable mismatch');
  }

  // Compare array
  if (tsField.array !== csField.array) {
    collector.add(`${path}.array`, tsField.array, csField.array, 'Array mismatch');
  }

  // Compare arrayItemNullable
  if ((tsField.arrayItemNullable ?? false) !== (csField.arrayItemNullable ?? false)) {
    collector.add(`${path}.arrayItemNullable`, tsField.arrayItemNullable, csField.arrayItemNullable, 'ArrayItemNullable mismatch');
  }

  // Compare kind
  if (tsField.kind !== csField.kind) {
    collector.add(`${path}.kind`, tsField.kind, csField.kind, 'Field kind mismatch');
  }

  // Compare default_value / defaultValue
  const tsDefault = tsField.default_value ?? tsField.defaultValue;
  const csDefault = csField.defaultValue ?? csField.default_value;
  if ((tsDefault ?? null) !== (csDefault ?? null)) {
    // Normalize: some parsers might quote differently
    const tsNorm = tsDefault != null ? String(tsDefault) : null;
    const csNorm = csDefault != null ? String(csDefault) : null;
    if (tsNorm !== csNorm) {
      collector.add(`${path}.defaultValue`, tsDefault, csDefault, 'Default value mismatch');
    }
  }

  // Compare description
  const tsDesc = tsField.description;
  const csDesc = csField.description;
  if ((tsDesc || null) !== (csDesc || null)) {
    if (tsDesc && csDesc) {
      collector.add(`${path}.description`, tsDesc, csDesc, 'Field description mismatch');
    } else if (tsDesc && !csDesc) {
      collector.add(`${path}.description`, tsDesc, undefined, 'Field description only in TS');
    } else if (!tsDesc && csDesc) {
      collector.add(`${path}.description`, undefined, csDesc, 'Field description only in CS');
    }
  }

  // Compare attributes
  deepCompare(tsField.attributes || [], csField.attributes || [], `${path}.attributes`, collector);

  // Compare enum_values / enumValues
  const tsEnumVals = tsField.enum_values || tsField.enumValues;
  const csEnumVals = csField.enumValues || csField.enum_values;
  if (tsEnumVals || csEnumVals) {
    deepCompare(tsEnumVals || [], csEnumVals || [], `${path}.enumValues`, collector);
  }

  // Compare lookup
  deepCompare(tsField.lookup, csField.lookup, `${path}.lookup`, collector);

  // Compare rollup
  deepCompare(tsField.rollup, csField.rollup, `${path}.rollup`, collector);

  // Compare computed
  deepCompare(tsField.computed, csField.computed, `${path}.computed`, collector);

  // Compare format (if present)
  if (tsField.format !== undefined || csField.format !== undefined) {
    if (tsField.format !== csField.format) {
      collector.add(`${path}.format`, tsField.format, csField.format, 'Format mismatch');
    }
  }

  // Compare map_key_type / mapKeyType
  const tsMapKey = tsField.map_key_type ?? tsField.mapKeyType;
  const csMapKey = csField.mapKeyType ?? csField.map_key_type;
  if ((tsMapKey ?? null) !== (csMapKey ?? null)) {
    collector.add(`${path}.mapKeyType`, tsMapKey, csMapKey, 'Map key type mismatch');
  }

  // Compare nested_fields / nestedFields
  const tsNested = tsField.nested_fields ?? tsField.nestedFields;
  const csNested = csField.nestedFields ?? csField.nested_fields;
  if (tsNested || csNested) {
    if (tsNested && csNested) {
      // nested_fields is an array of field-like objects
      if (Array.isArray(tsNested) && Array.isArray(csNested)) {
        compareFields(tsNested, csNested, `${path}.nestedFields`, collector);
      } else {
        deepCompare(tsNested, csNested, `${path}.nestedFields`, collector);
      }
    } else {
      collector.add(`${path}.nestedFields`, tsNested, csNested, 'Nested fields presence mismatch');
    }
  }

  // Compare items (for array of objects)
  const tsItems = tsField.items;
  const csItems = csField.items;
  if (tsItems || csItems) {
    if (tsItems && csItems) {
      deepCompare(tsItems, csItems, `${path}.items`, collector);
    } else {
      collector.add(`${path}.items`, tsItems, csItems, 'Items presence mismatch');
    }
  }
}

function compareSections(tsSections, csSections, modelPath, collector) {
  const path = `${modelPath}.sections`;

  if (!tsSections && !csSections) return;
  if (!tsSections && csSections) {
    // Check if CS sections is all empty
    const allEmpty = ['indexes', 'relations', 'behaviors'].every(
      k => !csSections[k] || csSections[k].length === 0
    ) && isEmptyValue(csSections.metadata) && isEmptyValue(csSections.extra);
    if (!allEmpty) {
      collector.add(path, undefined, csSections, 'Sections only in CS');
    }
    return;
  }
  if (tsSections && !csSections) {
    const allEmpty = ['indexes', 'relations', 'behaviors'].every(
      k => !tsSections[k] || tsSections[k].length === 0
    ) && isEmptyValue(tsSections.metadata);
    if (!allEmpty) {
      collector.add(path, tsSections, undefined, 'Sections only in TS');
    }
    return;
  }

  // Compare indexes
  compareSectionArray(tsSections.indexes || [], csSections.indexes || [], `${path}.indexes`, collector);

  // Compare relations
  compareSectionArray(tsSections.relations || [], csSections.relations || [], `${path}.relations`, collector);

  // Compare behaviors
  compareSectionArray(tsSections.behaviors || [], csSections.behaviors || [], `${path}.behaviors`, collector);

  // Compare metadata (key-value object)
  deepCompare(tsSections.metadata || {}, csSections.metadata || {}, `${path}.metadata`, collector);

  // Note: CS has 'extra' key, TS does not - we skip empty extras
  if (csSections.extra && !isEmptyValue(csSections.extra)) {
    collector.add(`${path}.extra`, undefined, csSections.extra, 'Extra section only in CS (non-empty)');
  }
}

function compareSectionArray(tsArr, csArr, path, collector) {
  if (tsArr.length !== csArr.length) {
    collector.add(path, `count=${tsArr.length}`, `count=${csArr.length}`, 'Section item count mismatch');
  }

  // Match by name
  const tsMap = new Map(tsArr.map(x => [x.name, x]));
  const csMap = new Map(csArr.map(x => [x.name, x]));

  for (const [name, tsItem] of tsMap) {
    const csItem = csMap.get(name);
    if (!csItem) {
      collector.add(`${path}.${name}`, name, undefined, 'Section item only in TS');
      continue;
    }
    // Compare the item but skip loc, line, col, file, label (CS adds null label)
    const tsClean = { ...tsItem };
    const csClean = { ...csItem };
    delete tsClean.loc; delete csClean.loc;
    delete tsClean.line; delete csClean.line;
    delete tsClean.col; delete csClean.col;
    delete tsClean.file; delete csClean.file;
    delete tsClean.source; delete csClean.source;
    // CS indexes may have 'label: null'
    if (csClean.label === null) delete csClean.label;
    if (tsClean.label === null) delete tsClean.label;

    deepCompare(tsClean, csClean, `${path}.${name}`, collector);
  }

  for (const [name] of csMap) {
    if (!tsMap.has(name)) {
      collector.add(`${path}.${name}`, undefined, name, 'Section item only in CS');
    }
  }
}

function compareEnums(tsEnums, csEnums, collector) {
  const tsNames = (tsEnums || []).map(e => e.name).sort();
  const csNames = (csEnums || []).map(e => e.name).sort();

  console.log(`  Enums: TS=${tsNames.length}, CS=${csNames.length}`);
  console.log(`    TS names: [${tsNames.join(', ')}]`);
  console.log(`    CS names: [${csNames.join(', ')}]`);

  if (JSON.stringify(tsNames) !== JSON.stringify(csNames)) {
    collector.add('enums', tsNames, csNames, 'Enum name lists differ');
  }

  for (const tsEnum of (tsEnums || [])) {
    const csEnum = (csEnums || []).find(e => e.name === tsEnum.name);
    if (!csEnum) {
      collector.add(`enums.${tsEnum.name}`, tsEnum.name, undefined, 'Enum only in TS');
      continue;
    }
    // Compare values
    deepCompare(tsEnum.values || [], csEnum.values || [], `enums.${tsEnum.name}.values`, collector);
  }
  for (const csEnum of (csEnums || [])) {
    if (!(tsEnums || []).find(e => e.name === csEnum.name)) {
      collector.add(`enums.${csEnum.name}`, undefined, csEnum.name, 'Enum only in CS');
    }
  }
}

function compareInterfaces(tsInterfaces, csInterfaces, collector) {
  const tsNames = (tsInterfaces || []).map(i => i.name).sort();
  const csNames = (csInterfaces || []).map(i => i.name).sort();

  console.log(`  Interfaces: TS=${tsNames.length}, CS=${csNames.length}`);
  console.log(`    TS names: [${tsNames.join(', ')}]`);
  console.log(`    CS names: [${csNames.join(', ')}]`);

  if (JSON.stringify(tsNames) !== JSON.stringify(csNames)) {
    collector.add('interfaces', tsNames, csNames, 'Interface name lists differ');
  }

  for (const tsIface of (tsInterfaces || [])) {
    const csIface = (csInterfaces || []).find(i => i.name === tsIface.name);
    if (!csIface) {
      collector.add(`interfaces.${tsIface.name}`, tsIface.name, undefined, 'Interface only in TS');
      continue;
    }
    compareModelNode(tsIface, csIface, `interfaces.${tsIface.name}`, collector);
  }
}

function compareViews(tsViews, csViews, collector) {
  const tsNames = (tsViews || []).map(v => v.name).sort();
  const csNames = (csViews || []).map(v => v.name).sort();

  console.log(`  Views: TS=${tsNames.length}, CS=${csNames.length}`);
  console.log(`    TS names: [${tsNames.join(', ')}]`);
  console.log(`    CS names: [${csNames.join(', ')}]`);

  if (JSON.stringify(tsNames) !== JSON.stringify(csNames)) {
    collector.add('views', tsNames, csNames, 'View name lists differ');
  }

  for (const tsView of (tsViews || [])) {
    const csView = (csViews || []).find(v => v.name === tsView.name);
    if (!csView) {
      collector.add(`views.${tsView.name}`, tsView.name, undefined, 'View only in TS');
      continue;
    }
    compareModelNode(tsView, csView, `views.${tsView.name}`, collector);
  }
}

// ============================================================
// Top-level comparison
// ============================================================

function compareAST(tsAst, csAst, label) {
  const collector = new DiffCollector(label);

  // Compare project
  deepCompare(tsAst.project, csAst.project, 'project', collector);

  // Compare astVersion
  if (tsAst.astVersion !== csAst.astVersion) {
    collector.add('astVersion', tsAst.astVersion, csAst.astVersion, 'AST version mismatch');
  }

  // Compare models
  compareModels(tsAst.models || [], csAst.models || [], collector);

  // Compare enums
  compareEnums(tsAst.enums, csAst.enums, collector);

  // Compare interfaces
  compareInterfaces(tsAst.interfaces, csAst.interfaces, collector);

  // Compare views
  compareViews(tsAst.views, csAst.views, collector);

  // Compare errors
  deepCompare(tsAst.errors || [], csAst.errors || [], 'errors', collector);

  // Compare warnings
  deepCompare(tsAst.warnings || [], csAst.warnings || [], 'warnings', collector);

  return collector;
}

// ============================================================
// Per-model field summary
// ============================================================

function printFieldSummary(tsModels, csModels, label) {
  console.log(`\n  --- Per-Model Field Summary ---`);
  const allNames = new Set([
    ...(tsModels || []).map(m => m.name),
    ...(csModels || []).map(m => m.name),
  ]);
  for (const name of allNames) {
    const tsModel = (tsModels || []).find(m => m.name === name);
    const csModel = (csModels || []).find(m => m.name === name);
    if (!tsModel || !csModel) {
      console.log(`  ${name}: MISSING in ${!tsModel ? 'TS' : 'CS'}`);
      continue;
    }
    const tsFields = tsModel.fields || [];
    const csFields = csModel.fields || [];
    const tsFNames = tsFields.map(f => f.name);
    const csFNames = csFields.map(f => f.name);
    const match = tsFields.length === csFields.length && JSON.stringify(tsFNames) === JSON.stringify(csFNames);
    const marker = match ? 'OK' : 'MISMATCH';
    console.log(`  ${name}: fields TS=${tsFields.length} CS=${csFields.length} [${marker}]`);
    if (!match) {
      const onlyInTs = tsFNames.filter(n => !csFNames.includes(n));
      const onlyInCs = csFNames.filter(n => !tsFNames.includes(n));
      if (onlyInTs.length) console.log(`    Only in TS: ${onlyInTs.join(', ')}`);
      if (onlyInCs.length) console.log(`    Only in CS: ${onlyInCs.join(', ')}`);
    }
    // Show field types and kinds for matching fields
    for (const tsF of tsFields) {
      const csF = csFields.find(f => f.name === tsF.name);
      if (!csF) continue;
      const issues = [];
      if (tsF.type !== csF.type) issues.push(`type: TS=${tsF.type} CS=${csF.type}`);
      if (tsF.kind !== csF.kind) issues.push(`kind: TS=${tsF.kind} CS=${csF.kind}`);
      if (tsF.nullable !== csF.nullable) issues.push(`nullable: TS=${tsF.nullable} CS=${csF.nullable}`);
      if (tsF.array !== csF.array) issues.push(`array: TS=${tsF.array} CS=${csF.array}`);
      if (issues.length > 0) {
        console.log(`    ${tsF.name}: ${issues.join('; ')}`);
      }
    }
  }
}

// ============================================================
// Main
// ============================================================

function main() {
  console.log('='.repeat(80));
  console.log('M3L AST Conformance Comparison: TypeScript vs C# Parser');
  console.log('='.repeat(80));
  console.log();

  let totalDiffs = 0;
  const summaries = [];

  for (const sample of SAMPLES) {
    console.log('-'.repeat(80));
    console.log(`SAMPLE: ${sample.label}`);
    console.log('-'.repeat(80));

    let tsAst, csAst;
    try {
      tsAst = JSON.parse(readFileSync(BASE + sample.ts, 'utf8'));
    } catch (e) {
      console.log(`  ERROR loading TS file: ${e.message}`);
      continue;
    }
    try {
      csAst = JSON.parse(readFileSync(BASE + sample.cs, 'utf8'));
    } catch (e) {
      console.log(`  ERROR loading CS file: ${e.message}`);
      continue;
    }

    console.log(`  TS file: ${sample.ts}`);
    console.log(`  CS file: ${sample.cs}`);
    console.log();

    const collector = compareAST(tsAst, csAst, sample.label);

    // Print field summary
    printFieldSummary(tsAst.models, csAst.models, sample.label);

    // Also for views
    if ((tsAst.views?.length || 0) > 0 || (csAst.views?.length || 0) > 0) {
      console.log(`\n  --- Per-View Field Summary ---`);
      printFieldSummary(tsAst.views, csAst.views, sample.label + ' (views)');
    }

    console.log();
    console.log(`  === DIFFERENCES (${collector.count}) ===`);
    collector.print();
    console.log();

    totalDiffs += collector.count;
    summaries.push({ label: sample.label, count: collector.count });
  }

  // Final summary
  console.log('='.repeat(80));
  console.log('SUMMARY');
  console.log('='.repeat(80));
  for (const s of summaries) {
    const status = s.count === 0 ? 'PASS' : 'FAIL';
    console.log(`  ${s.label}: ${s.count} difference(s) [${status}]`);
  }
  console.log(`  ${'─'.repeat(40)}`);
  console.log(`  TOTAL: ${totalDiffs} difference(s) across all samples`);
  console.log();
  if (totalDiffs === 0) {
    console.log('  All samples are semantically equivalent!');
  } else {
    console.log(`  ${totalDiffs} semantic difference(s) found. Review above for details.`);
  }
}

main();
