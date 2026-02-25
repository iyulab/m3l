using M3L.Models;

namespace M3L;

/// <summary>
/// Resolves and merges multiple parsed file ASTs into a single M3LAST.
/// Handles: inheritance resolution, duplicate detection, reference validation.
/// </summary>
public static class Resolver
{
    /// <summary>AST schema version — bump major on breaking structure changes.</summary>
    public const string AstVersion = "1.0";

    /// <summary>Parser package version — kept in sync with .csproj.</summary>
    public const string ParserVersion = "0.1.1";

    public static M3LAst Resolve(List<ParsedFile> files, ProjectInfo? project = null)
    {
        var errors = new List<Diagnostic>();
        var warnings = new List<Diagnostic>();

        var allModels = new List<ModelNode>();
        var allEnums = new List<EnumNode>();
        var allInterfaces = new List<ModelNode>();
        var allViews = new List<ModelNode>();
        var sources = new List<string>();

        foreach (var file in files)
        {
            sources.Add(file.Source);
            allModels.AddRange(file.Models);
            allEnums.AddRange(file.Enums);
            allInterfaces.AddRange(file.Interfaces);
            allViews.AddRange(file.Views);
        }

        // Build name maps
        var modelMap = new Dictionary<string, ModelNode>();
        var enumMap = new Dictionary<string, EnumNode>();
        var interfaceMap = new Dictionary<string, ModelNode>();
        var allNamedMap = new Dictionary<string, (string Type, string File, int Line)>();

        foreach (var model in allModels)
        {
            CheckDuplicate(model.Name, "model", model.Source, model.Line, allNamedMap, errors);
            modelMap[model.Name] = model;
            allNamedMap[model.Name] = ("model", model.Source, model.Line);
        }

        foreach (var en in allEnums)
        {
            CheckDuplicate(en.Name, "enum", en.Source, en.Line, allNamedMap, errors);
            enumMap[en.Name] = en;
            allNamedMap[en.Name] = ("enum", en.Source, en.Line);
        }

        foreach (var iface in allInterfaces)
        {
            CheckDuplicate(iface.Name, "interface", iface.Source, iface.Line, allNamedMap, errors);
            interfaceMap[iface.Name] = iface;
            allNamedMap[iface.Name] = ("interface", iface.Source, iface.Line);
        }

        foreach (var view in allViews)
        {
            allNamedMap[view.Name] = ("view", view.Source, view.Line);
        }

        // Resolve inheritance
        foreach (var model in allModels)
        {
            ResolveInheritance(model, modelMap, interfaceMap, allNamedMap, errors);
        }

        // Check duplicate field names within each model
        foreach (var model in allModels.Concat(allViews))
        {
            CheckDuplicateFields(model, errors);
        }

        // Detect namespace from first file if available
        var projectInfo = project ?? new ProjectInfo();
        if (string.IsNullOrEmpty(projectInfo.Name))
        {
            var ns = files.FirstOrDefault(f => f.Namespace != null)?.Namespace;
            if (ns != null) projectInfo.Name = ns;
        }

        return new M3LAst
        {
            ParserVersion = ParserVersion,
            AstVersion = AstVersion,
            Project = projectInfo,
            Sources = sources,
            Models = allModels,
            Enums = allEnums,
            Interfaces = allInterfaces,
            Views = allViews,
            Errors = errors,
            Warnings = warnings,
        };
    }

    private static void CheckDuplicate(
        string name, string kind, string source, int line,
        Dictionary<string, (string Type, string File, int Line)> allMap,
        List<Diagnostic> errors)
    {
        if (allMap.TryGetValue(name, out var existing))
        {
            errors.Add(new Diagnostic
            {
                Code = "M3L-E005",
                Severity = "error",
                File = source,
                Line = line,
                Col = 1,
                Message = $"Duplicate {kind} name \"{name}\" (first defined in {existing.File}:{existing.Line})",
            });
        }
    }

    private static void ResolveInheritance(
        ModelNode model,
        Dictionary<string, ModelNode> modelMap,
        Dictionary<string, ModelNode> interfaceMap,
        Dictionary<string, (string Type, string File, int Line)> allNamedMap,
        List<Diagnostic> errors)
    {
        if (model.Inherits.Count == 0) return;

        var inheritedFields = new List<FieldNode>();
        var resolved = new HashSet<string>();
        var visiting = new HashSet<string>();

        void CollectFields(string name, ModelNode fromModel)
        {
            if (resolved.Contains(name) || visiting.Contains(name)) return;
            visiting.Add(name);

            ModelNode? parent = null;
            if (!modelMap.TryGetValue(name, out parent))
                interfaceMap.TryGetValue(name, out parent);

            if (parent == null)
            {
                if (!allNamedMap.ContainsKey(name))
                {
                    errors.Add(new Diagnostic
                    {
                        Code = "M3L-E007",
                        Severity = "error",
                        File = fromModel.Source,
                        Line = fromModel.Line,
                        Col = 1,
                        Message = $"Unresolved inheritance reference \"{name}\" in model \"{fromModel.Name}\"",
                    });
                }
                visiting.Remove(name);
                return;
            }

            // Recursively resolve parent's parents first
            foreach (var grandparent in parent.Inherits)
            {
                CollectFields(grandparent, fromModel);
            }

            // Add parent's own fields
            foreach (var field in parent.Fields)
            {
                if (!inheritedFields.Any(f => f.Name == field.Name))
                {
                    inheritedFields.Add(CloneField(field));
                }
            }

            visiting.Remove(name);
            resolved.Add(name);
        }

        foreach (var parentName in model.Inherits)
        {
            CollectFields(parentName, model);
        }

        // Handle @override: child fields with @override replace inherited fields
        var overrideNames = new HashSet<string>();
        foreach (var ownField in model.Fields)
        {
            var overrideAttr = ownField.Attributes.FindIndex(a => a.Name == "override");
            if (overrideAttr >= 0)
            {
                overrideNames.Add(ownField.Name);
                ownField.Attributes.RemoveAt(overrideAttr);
            }
        }

        // Remove overridden inherited fields
        inheritedFields.RemoveAll(f => overrideNames.Contains(f.Name));

        // Prepend inherited fields before model's own fields
        if (inheritedFields.Count > 0)
        {
            inheritedFields.AddRange(model.Fields);
            model.Fields = inheritedFields;
        }
    }

    private static FieldNode CloneField(FieldNode f) => new()
    {
        Name = f.Name,
        Label = f.Label,
        Type = f.Type,
        GenericParams = f.GenericParams != null ? new List<string>(f.GenericParams) : null,
        Params = f.Params != null ? new List<object>(f.Params) : null,
        Nullable = f.Nullable,
        Array = f.Array,
        ArrayItemNullable = f.ArrayItemNullable,
        Kind = f.Kind,
        DefaultValue = f.DefaultValue,
        Description = f.Description,
        Attributes = new List<FieldAttribute>(f.Attributes.Select(a => new FieldAttribute
        {
            Name = a.Name,
            Args = a.Args != null ? new List<object>(a.Args) : null,
        })),
        FrameworkAttrs = f.FrameworkAttrs,
        Lookup = f.Lookup,
        Rollup = f.Rollup,
        Computed = f.Computed,
        EnumValues = f.EnumValues,
        Fields = f.Fields,
        Loc = f.Loc,
    };

    private static void CheckDuplicateFields(ModelNode model, List<Diagnostic> errors)
    {
        var seen = new Dictionary<string, FieldNode>();
        foreach (var field in model.Fields)
        {
            if (seen.TryGetValue(field.Name, out var existing))
            {
                errors.Add(new Diagnostic
                {
                    Code = "M3L-E005",
                    Severity = "error",
                    File = field.Loc.File,
                    Line = field.Loc.Line,
                    Col = 1,
                    Message = $"Duplicate field name \"{field.Name}\" in {model.NodeType} \"{model.Name}\" (first at line {existing.Loc.Line})",
                });
            }
            else
            {
                seen[field.Name] = field;
            }
        }
    }
}
