using M3L.Models;

namespace M3L;

/// <summary>
/// Validates a resolved M3L AST for semantic errors and style warnings.
/// </summary>
public static class Validator
{
    public static ValidateResult Validate(M3LAst ast, ValidateOptions? options = null)
    {
        options ??= new ValidateOptions();
        var errors = new List<Diagnostic>(ast.Errors);
        var warnings = new List<Diagnostic>(ast.Warnings);

        var allModels = ast.Models.Concat(ast.Views).ToList();
        var modelMap = new Dictionary<string, ModelNode>();
        foreach (var m in allModels)
            modelMap[m.Name] = m;

        // M3L-E001: @rollup FK missing @reference
        foreach (var model in allModels)
        {
            foreach (var field in model.Fields)
            {
                if (field.Kind == FieldKind.Rollup && field.Rollup != null)
                    ValidateRollupReference(field, model, modelMap, errors);
            }
        }

        // M3L-E002: @lookup path FK missing @reference
        foreach (var model in allModels)
        {
            foreach (var field in model.Fields)
            {
                if (field.Kind == FieldKind.Lookup && field.Lookup != null)
                    ValidateLookupReference(field, model, modelMap, errors);
            }
        }

        // M3L-E004: View @from references model not found
        foreach (var view in ast.Views)
        {
            if (view.SourceDef?.From is { Length: > 0 } fromModel)
            {
                if (!modelMap.ContainsKey(fromModel))
                {
                    errors.Add(new Diagnostic
                    {
                        Code = "M3L-E004",
                        Severity = "error",
                        File = view.Source,
                        Line = view.Line,
                        Col = 1,
                        Message = $"View \"{view.Name}\" references model \"{fromModel}\" which is not defined",
                    });
                }
            }
        }

        // M3L-E006: Duplicate field names
        foreach (var model in allModels)
        {
            var seen = new HashSet<string>();
            foreach (var field in model.Fields)
            {
                if (!seen.Add(field.Name))
                {
                    errors.Add(new Diagnostic
                    {
                        Code = "M3L-E006",
                        Severity = "error",
                        File = field.Loc.File,
                        Line = field.Loc.Line,
                        Col = 1,
                        Message = $"Duplicate field name \"{field.Name}\" in {model.NodeType} \"{model.Name}\"",
                    });
                }
            }
        }

        // Strict mode warnings
        if (options.Strict)
        {
            foreach (var model in allModels)
            {
                foreach (var field in model.Fields)
                {
                    CheckFieldLineLength(field, warnings);

                    // M3L-W004: Lookup chain > 3 hops
                    if (field.Kind == FieldKind.Lookup && field.Lookup != null)
                    {
                        var hops = field.Lookup.Path.Split('.').Length;
                        if (hops > 3)
                        {
                            warnings.Add(new Diagnostic
                            {
                                Code = "M3L-W004",
                                Severity = "warning",
                                File = field.Loc.File,
                                Line = field.Loc.Line,
                                Col = 1,
                                Message = $"Lookup chain \"{field.Lookup.Path}\" exceeds 3 hops ({hops} hops)",
                            });
                        }
                    }
                }

                // M3L-W002: Object nesting > 3 levels
                CheckNestingDepth(model.Fields, 1, model, warnings);
            }
        }

        return new ValidateResult { Errors = errors, Warnings = warnings };
    }

    private static void ValidateRollupReference(
        FieldNode field, ModelNode model,
        Dictionary<string, ModelNode> modelMap,
        List<Diagnostic> errors)
    {
        var rollup = field.Rollup!;
        if (!modelMap.TryGetValue(rollup.Target, out var targetModel))
            return;

        var fkField = targetModel.Fields.FirstOrDefault(f => f.Name == rollup.Fk);
        if (fkField == null) return;

        var hasReference = fkField.Attributes.Any(a => a.Name is "reference" or "fk");
        if (!hasReference)
        {
            errors.Add(new Diagnostic
            {
                Code = "M3L-E001",
                Severity = "error",
                File = field.Loc.File,
                Line = field.Loc.Line,
                Col = 1,
                Message = $"@rollup on \"{field.Name}\" targets \"{rollup.Target}.{rollup.Fk}\" which has no @reference or @fk attribute",
            });
        }
    }

    private static void ValidateLookupReference(
        FieldNode field, ModelNode model,
        Dictionary<string, ModelNode> modelMap,
        List<Diagnostic> errors)
    {
        var lookupPath = field.Lookup!.Path;
        var segments = lookupPath.Split('.');
        if (segments.Length < 2) return;

        var fkFieldName = segments[0];
        var fkField = model.Fields.FirstOrDefault(f => f.Name == fkFieldName);
        if (fkField == null) return;

        var hasReference = fkField.Attributes.Any(a => a.Name is "reference" or "fk");
        if (!hasReference)
        {
            errors.Add(new Diagnostic
            {
                Code = "M3L-E002",
                Severity = "error",
                File = field.Loc.File,
                Line = field.Loc.Line,
                Col = 1,
                Message = $"@lookup on \"{field.Name}\" references FK \"{fkFieldName}\" which has no @reference or @fk attribute",
            });
        }
    }

    private static void CheckFieldLineLength(FieldNode field, List<Diagnostic> warnings)
    {
        int len = 2 + field.Name.Length;
        if (field.Label != null) len += field.Label.Length + 2;
        if (field.Type != null)
        {
            len += 2 + field.Type.Length;
            if (field.Params != null) len += string.Join(",", field.Params).Length + 2;
        }
        if (field.Nullable) len += 1;
        if (field.DefaultValue != null) len += 3 + field.DefaultValue.Length;
        foreach (var attr in field.Attributes)
        {
            len += 2 + attr.Name.Length;
            if (attr.Args != null) len += string.Join(",", attr.Args).Length + 2;
        }
        if (field.Description != null) len += 3 + field.Description.Length;

        if (len > 80)
        {
            warnings.Add(new Diagnostic
            {
                Code = "M3L-W001",
                Severity = "warning",
                File = field.Loc.File,
                Line = field.Loc.Line,
                Col = 1,
                Message = $"Field \"{field.Name}\" line length (~{len} chars) exceeds 80 character guideline",
            });
        }
    }

    private static void CheckNestingDepth(
        List<FieldNode> fields, int depth,
        ModelNode model, List<Diagnostic> warnings)
    {
        foreach (var field in fields)
        {
            if (field.Fields is { Count: > 0 })
            {
                if (depth >= 3)
                {
                    warnings.Add(new Diagnostic
                    {
                        Code = "M3L-W002",
                        Severity = "warning",
                        File = field.Loc.File,
                        Line = field.Loc.Line,
                        Col = 1,
                        Message = $"Object nesting depth exceeds 3 levels at field \"{field.Name}\" in \"{model.Name}\"",
                    });
                }
                CheckNestingDepth(field.Fields, depth + 1, model, warnings);
            }
        }
    }
}
