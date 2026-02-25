import type {
  M3LAST,
  ModelNode,
  FieldNode,
  Diagnostic,
} from './types.js';

export interface ValidateOptions {
  strict?: boolean;
}

export interface ValidateResult {
  errors: Diagnostic[];
  warnings: Diagnostic[];
}

/**
 * Validate a resolved M3L AST for semantic errors and style warnings.
 */
export function validate(ast: M3LAST, options: ValidateOptions = {}): ValidateResult {
  const errors: Diagnostic[] = [...ast.errors];
  const warnings: Diagnostic[] = [...ast.warnings];

  const allModels = [...ast.models, ...ast.views];
  const modelMap = new Map<string, ModelNode>();
  for (const m of allModels) {
    modelMap.set(m.name, m);
  }

  // E001: @rollup FK missing @reference
  for (const model of allModels) {
    for (const field of model.fields) {
      if (field.kind === 'rollup' && field.rollup) {
        validateRollupReference(field, model, modelMap, errors);
      }
    }
  }

  // E002: @lookup path FK missing @reference
  for (const model of allModels) {
    for (const field of model.fields) {
      if (field.kind === 'lookup' && field.lookup) {
        validateLookupReference(field, model, modelMap, errors);
      }
    }
  }

  // E004: View @from references model not found
  for (const view of ast.views) {
    if (view.source_def?.from) {
      if (!modelMap.has(view.source_def.from)) {
        errors.push({
          code: 'E004',
          severity: 'error',
          file: view.source,
          line: view.line,
          col: 1,
          message: `View "${view.name}" references model "${view.source_def.from}" which is not defined`,
        });
      }
    }
  }

  // E005: Duplicate field names (already checked in resolver, but re-check for safety)
  for (const model of allModels) {
    const seen = new Set<string>();
    for (const field of model.fields) {
      if (seen.has(field.name)) {
        errors.push({
          code: 'E005',
          severity: 'error',
          file: field.loc.file,
          line: field.loc.line,
          col: 1,
          message: `Duplicate field name "${field.name}" in ${model.type} "${model.name}"`,
        });
      }
      seen.add(field.name);
    }
  }

  // Strict mode warnings
  if (options.strict) {
    for (const model of allModels) {
      for (const field of model.fields) {
        // W001: Field line length > 80 chars
        // We check the source loc raw length — approximate using field attributes count
        checkFieldLineLength(field, model, warnings);

        // W003: Framework attrs without backtick (already processed in lexer, skip)

        // W004: Lookup chain > 3 hops
        if (field.kind === 'lookup' && field.lookup) {
          const hops = field.lookup.path.split('.').length;
          if (hops > 3) {
            warnings.push({
              code: 'W004',
              severity: 'warning',
              file: field.loc.file,
              line: field.loc.line,
              col: 1,
              message: `Lookup chain "${field.lookup.path}" exceeds 3 hops (${hops} hops)`,
            });
          }
        }
      }

      // W002: Object nesting > 3 levels
      checkNestingDepth(model.fields, 1, model, warnings);
    }

    // W006: Inline enum missing values: key
    for (const model of allModels) {
      for (const field of model.fields) {
        if (field.type === 'enum' && field.enum_values && field.enum_values.length > 0) {
          // The lexer/parser would have marked whether values: key was used
          // For now, we check based on presence — if enum_values exist without
          // the values: wrapper, the parser still collects them.
          // This warning is informational for style.
          // We'll rely on a flag set during parsing in the future.
        }
      }
    }
  }

  return { errors, warnings };
}

function validateRollupReference(
  field: FieldNode,
  model: ModelNode,
  modelMap: Map<string, ModelNode>,
  errors: Diagnostic[]
): void {
  const rollup = field.rollup!;
  const targetModel = modelMap.get(rollup.target);
  if (!targetModel) {
    // Target model doesn't exist — this is E007 (already caught in resolver)
    return;
  }

  // Check that the FK field in the target model has @reference or @fk
  const fkField = targetModel.fields.find(f => f.name === rollup.fk);
  if (!fkField) {
    // FK field doesn't exist in target
    return;
  }

  const hasReference = fkField.attributes.some(
    a => a.name === 'reference' || a.name === 'fk'
  );
  if (!hasReference) {
    errors.push({
      code: 'E001',
      severity: 'error',
      file: field.loc.file,
      line: field.loc.line,
      col: 1,
      message: `@rollup on "${field.name}" targets "${rollup.target}.${rollup.fk}" which has no @reference or @fk attribute`,
    });
  }
}

function validateLookupReference(
  field: FieldNode,
  model: ModelNode,
  modelMap: Map<string, ModelNode>,
  errors: Diagnostic[]
): void {
  const lookupPath = field.lookup!.path;
  const segments = lookupPath.split('.');
  if (segments.length < 2) return;

  const fkFieldName = segments[0];

  // Find the FK field in the current model
  const fkField = model.fields.find(f => f.name === fkFieldName);
  if (!fkField) return; // Missing field — different issue

  const hasReference = fkField.attributes.some(
    a => a.name === 'reference' || a.name === 'fk'
  );
  if (!hasReference) {
    errors.push({
      code: 'E002',
      severity: 'error',
      file: field.loc.file,
      line: field.loc.line,
      col: 1,
      message: `@lookup on "${field.name}" references FK "${fkFieldName}" which has no @reference or @fk attribute`,
    });
  }
}

function checkFieldLineLength(
  field: FieldNode,
  model: ModelNode,
  warnings: Diagnostic[]
): void {
  // Reconstruct approximate line length
  let len = 2 + field.name.length; // "- name"
  if (field.label) len += field.label.length + 2; // "(label)"
  if (field.type) {
    len += 2 + field.type.length; // ": type"
    if (field.params) len += field.params.join(',').length + 2;
  }
  if (field.nullable) len += 1;
  if (field.default_value) len += 3 + field.default_value.length;
  for (const attr of field.attributes) {
    len += 2 + attr.name.length; // " @name"
    if (attr.args) len += attr.args.join(',').length + 2;
  }
  if (field.description) len += 3 + field.description.length;

  if (len > 80) {
    warnings.push({
      code: 'W001',
      severity: 'warning',
      file: field.loc.file,
      line: field.loc.line,
      col: 1,
      message: `Field "${field.name}" line length (~${len} chars) exceeds 80 character guideline`,
    });
  }
}

function checkNestingDepth(
  fields: FieldNode[],
  depth: number,
  model: ModelNode,
  warnings: Diagnostic[]
): void {
  for (const field of fields) {
    if (field.fields && field.fields.length > 0) {
      if (depth >= 3) {
        warnings.push({
          code: 'W002',
          severity: 'warning',
          file: field.loc.file,
          line: field.loc.line,
          col: 1,
          message: `Object nesting depth exceeds 3 levels at field "${field.name}" in "${model.name}"`,
        });
      }
      checkNestingDepth(field.fields, depth + 1, model, warnings);
    }
  }
}
