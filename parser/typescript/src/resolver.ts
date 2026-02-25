import type {
  ParsedFile,
  M3LAST,
  ModelNode,
  EnumNode,
  FieldNode,
  Diagnostic,
  ProjectInfo,
} from './types.js';

/** AST schema version — bump major on breaking structure changes */
export const AST_VERSION = '1.0';

/** Parser package version — kept in sync with package.json */
export const PARSER_VERSION = '0.1.1';

/**
 * Resolve and merge multiple parsed file ASTs into a single M3LAST.
 * Handles: inheritance resolution, duplicate detection, reference validation.
 */
export function resolve(
  files: ParsedFile[],
  project?: ProjectInfo
): M3LAST {
  const errors: Diagnostic[] = [];
  const warnings: Diagnostic[] = [];

  // Collect all elements from all files
  const allModels: ModelNode[] = [];
  const allEnums: EnumNode[] = [];
  const allInterfaces: ModelNode[] = [];
  const allViews: ModelNode[] = [];
  const sources: string[] = [];

  for (const file of files) {
    sources.push(file.source);
    allModels.push(...file.models);
    allEnums.push(...file.enums);
    allInterfaces.push(...file.interfaces);
    allViews.push(...file.views);
  }

  // Build name maps
  const modelMap = new Map<string, ModelNode>();
  const enumMap = new Map<string, EnumNode>();
  const interfaceMap = new Map<string, ModelNode>();
  const allNamedMap = new Map<string, { type: string; loc: { file: string; line: number } }>();

  // Check for duplicate model names
  for (const model of allModels) {
    checkDuplicate(model.name, 'model', model, modelMap, allNamedMap, errors);
    modelMap.set(model.name, model);
    allNamedMap.set(model.name, { type: 'model', loc: { file: model.source, line: model.line } });
  }

  for (const en of allEnums) {
    checkDuplicate(en.name, 'enum', en, enumMap, allNamedMap, errors);
    enumMap.set(en.name, en);
    allNamedMap.set(en.name, { type: 'enum', loc: { file: en.source, line: en.line } });
  }

  for (const iface of allInterfaces) {
    checkDuplicate(iface.name, 'interface', iface, interfaceMap, allNamedMap, errors);
    interfaceMap.set(iface.name, iface);
    allNamedMap.set(iface.name, { type: 'interface', loc: { file: iface.source, line: iface.line } });
  }

  for (const view of allViews) {
    allNamedMap.set(view.name, { type: 'view', loc: { file: view.source, line: view.line } });
  }

  // Resolve inheritance
  for (const model of allModels) {
    resolveInheritance(model, modelMap, interfaceMap, allNamedMap, errors);
  }

  // Check duplicate field names within each model
  for (const model of [...allModels, ...allViews]) {
    checkDuplicateFields(model, errors);
  }

  // Detect namespace from first file if available
  const projectInfo: ProjectInfo = project || {};
  if (!projectInfo.name) {
    const ns = files.find(f => f.namespace)?.namespace;
    if (ns) projectInfo.name = ns;
  }

  return {
    parserVersion: PARSER_VERSION,
    astVersion: AST_VERSION,
    project: projectInfo,
    sources,
    models: allModels,
    enums: allEnums,
    interfaces: allInterfaces,
    views: allViews,
    errors,
    warnings,
  };
}

function checkDuplicate<T extends { name: string; source: string; line: number }>(
  name: string,
  kind: string,
  item: T,
  map: Map<string, T>,
  allMap: Map<string, { type: string; loc: { file: string; line: number } }>,
  errors: Diagnostic[]
): void {
  const existing = allMap.get(name);
  if (existing) {
    errors.push({
      code: 'M3L-E005',
      severity: 'error',
      file: item.source,
      line: item.line,
      col: 1,
      message: `Duplicate ${kind} name "${name}" (first defined in ${existing.loc.file}:${existing.loc.line})`,
    });
  }
}

function resolveInheritance(
  model: ModelNode,
  modelMap: Map<string, ModelNode>,
  interfaceMap: Map<string, ModelNode>,
  allNamedMap: Map<string, { type: string; loc: { file: string; line: number } }>,
  errors: Diagnostic[]
): void {
  if (model.inherits.length === 0) return;

  const inheritedFields: FieldNode[] = [];
  const resolved = new Set<string>();
  const visiting = new Set<string>();

  function collectFields(name: string, fromModel: ModelNode): void {
    if (resolved.has(name) || visiting.has(name)) return;
    visiting.add(name);

    const parent = modelMap.get(name) || interfaceMap.get(name);
    if (!parent) {
      if (!allNamedMap.has(name)) {
        errors.push({
          code: 'M3L-E007',
          severity: 'error',
          file: fromModel.source,
          line: fromModel.line,
          col: 1,
          message: `Unresolved inheritance reference "${name}" in model "${fromModel.name}"`,
        });
      }
      visiting.delete(name);
      return;
    }

    // Recursively resolve parent's parents first
    for (const grandparent of parent.inherits) {
      collectFields(grandparent, fromModel);
    }

    // Add parent's own fields
    for (const field of parent.fields) {
      if (!inheritedFields.some(f => f.name === field.name)) {
        inheritedFields.push({ ...field });
      }
    }

    visiting.delete(name);
    resolved.add(name);
  }

  for (const parentName of model.inherits) {
    collectFields(parentName, model);
  }

  // Handle @override: child fields with @override replace inherited fields
  const overrideNames = new Set<string>();
  for (const ownField of model.fields) {
    const overrideIdx = ownField.attributes.findIndex(a => a.name === 'override');
    if (overrideIdx >= 0) {
      overrideNames.add(ownField.name);
      // Remove @override attribute from the child field
      ownField.attributes.splice(overrideIdx, 1);
    }
  }

  // Remove inherited fields that are overridden
  const filteredInherited = inheritedFields.filter(f => !overrideNames.has(f.name));

  // Prepend inherited fields before model's own fields
  if (filteredInherited.length > 0) {
    model.fields = [...filteredInherited, ...model.fields];
  }
}

function checkDuplicateFields(model: ModelNode, errors: Diagnostic[]): void {
  const seen = new Map<string, FieldNode>();
  for (const field of model.fields) {
    const existing = seen.get(field.name);
    if (existing) {
      errors.push({
        code: 'M3L-E005',
        severity: 'error',
        file: field.loc.file,
        line: field.loc.line,
        col: 1,
        message: `Duplicate field name "${field.name}" in ${model.type} "${model.name}" (first at line ${existing.loc.line})`,
      });
    } else {
      seen.set(field.name, field);
    }
  }
}
