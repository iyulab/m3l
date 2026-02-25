using M3L.Models;

namespace M3L;

/// <summary>
/// Parses a token sequence into a ParsedFile AST.
/// </summary>
public static class Parser
{
    private sealed class ParserState
    {
        public string File { get; set; } = "";
        public string? Namespace { get; set; }
        public object? CurrentElement { get; set; } // ModelNode or EnumNode
        public string? CurrentSection { get; set; }
        public FieldKind CurrentKind { get; set; } = FieldKind.Stored;
        public FieldNode? LastField { get; set; }
        public List<ModelNode> Models { get; } = new();
        public List<EnumNode> Enums { get; } = new();
        public List<ModelNode> Interfaces { get; } = new();
        public List<ModelNode> Views { get; } = new();
        public bool SourceDirectivesDone { get; set; }
    }

    public static ParsedFile ParseString(string content, string file)
    {
        var tokens = Lexer.Lex(content, file);
        return ParseTokens(tokens, file);
    }

    public static ParsedFile ParseTokens(List<Token> tokens, string file)
    {
        var state = new ParserState { File = file };

        foreach (var token in tokens)
            ProcessToken(token, state);

        FinalizeElement(state);

        return new ParsedFile
        {
            Source = file,
            Namespace = state.Namespace,
            Models = state.Models,
            Enums = state.Enums,
            Interfaces = state.Interfaces,
            Views = state.Views,
        };
    }

    private static void ProcessToken(Token token, ParserState state)
    {
        switch (token.Type)
        {
            case TokenType.Namespace: HandleNamespace(token, state); break;
            case TokenType.Model:
            case TokenType.Interface: HandleModelStart(token, state); break;
            case TokenType.Enum: HandleEnumStart(token, state); break;
            case TokenType.View: HandleViewStart(token, state); break;
            case TokenType.Section: HandleSection(token, state); break;
            case TokenType.Field: HandleField(token, state); break;
            case TokenType.NestedItem: HandleNestedItem(token, state); break;
            case TokenType.Blockquote: HandleBlockquote(token, state); break;
            case TokenType.Text: HandleText(token, state); break;
        }
    }

    private static void HandleNamespace(Token token, ParserState state)
    {
        if (token.Data.TryGetValue("kind_section", out var ks) && ks is true)
        {
            HandleSection(token, state);
            return;
        }
        if (state.CurrentElement == null)
            state.Namespace = token.Data.GetValueOrDefault("name") as string;
    }

    private static void HandleModelStart(Token token, ParserState state)
    {
        FinalizeElement(state);
        var data = token.Data;
        var model = new ModelNode
        {
            Name = (string)data["name"]!,
            Label = data.GetValueOrDefault("label") as string,
            NodeType = token.Type == TokenType.Interface ? "interface" : "model",
            Source = state.File,
            Line = token.Line,
            Inherits = GetStringList(data, "inherits"),
            Loc = new SourceLocation(state.File, token.Line, 1),
        };
        // D-C01: Read model-level attributes from data
        if (data.GetValueOrDefault("attributes") is List<Dictionary<string, object?>> modelAttrs)
        {
            model.Attributes = modelAttrs.Select(a => new FieldAttribute
            {
                Name = (string)a["name"]!,
                Args = a.GetValueOrDefault("args") is List<string> argList ? argList.Cast<object>().ToList() : null,
            }).ToList();
        }

        state.CurrentElement = model;
        state.CurrentSection = null;
        state.CurrentKind = FieldKind.Stored;
        state.LastField = null;
        state.SourceDirectivesDone = false;
    }

    private static void HandleEnumStart(Token token, ParserState state)
    {
        FinalizeElement(state);
        var data = token.Data;
        var enumNode = new EnumNode
        {
            Name = (string)data["name"]!,
            Label = data.GetValueOrDefault("label") as string,
            Source = state.File,
            Line = token.Line,
            Description = data.GetValueOrDefault("description") as string,
            Loc = new SourceLocation(state.File, token.Line, 1),
        };
        state.CurrentElement = enumNode;
        state.CurrentSection = null;
        state.CurrentKind = FieldKind.Stored;
        state.LastField = null;
    }

    private static void HandleViewStart(Token token, ParserState state)
    {
        FinalizeElement(state);
        var data = token.Data;
        var view = new ModelNode
        {
            Name = (string)data["name"]!,
            Label = data.GetValueOrDefault("label") as string,
            NodeType = "view",
            Source = state.File,
            Line = token.Line,
            Materialized = data.GetValueOrDefault("materialized") is true,
            Loc = new SourceLocation(state.File, token.Line, 1),
        };
        state.CurrentElement = view;
        state.CurrentSection = null;
        state.CurrentKind = FieldKind.Stored;
        state.LastField = null;
        state.SourceDirectivesDone = false;
    }

    private static void HandleSection(Token token, ParserState state)
    {
        var sectionName = (string)token.Data["name"]!;
        if (token.Data.GetValueOrDefault("kind_section") is true)
        {
            if (state.CurrentElement == null) return;
            var lower = sectionName.ToLowerInvariant();
            if (lower.StartsWith("lookup")) state.CurrentKind = FieldKind.Lookup;
            else if (lower.StartsWith("rollup")) state.CurrentKind = FieldKind.Rollup;
            else if (lower.StartsWith("computed")) state.CurrentKind = FieldKind.Computed;
            state.CurrentSection = null;
            state.LastField = null;
            return;
        }

        state.CurrentSection = sectionName;
        state.LastField = null;

        if (sectionName == "Source" && state.CurrentElement is ModelNode { NodeType: "view" })
            state.SourceDirectivesDone = false;
    }

    private static void HandleField(Token token, ParserState state)
    {
        if (state.CurrentElement == null) return;
        var data = token.Data;

        if (state.CurrentElement is EnumNode enumNode)
        {
            HandleEnumValue(data, enumNode);
            return;
        }

        var model = (ModelNode)state.CurrentElement;

        if (data.GetValueOrDefault("is_directive") is true)
        {
            HandleDirective(data, model, token, state);
            return;
        }

        if (state.CurrentSection != null)
        {
            HandleSectionItem(data, model, token, state);
            return;
        }

        var field = BuildFieldNode(data, token, state);
        model.Fields.Add(field);
        state.LastField = field;
    }

    private static void HandleEnumValue(Dictionary<string, object?> data, EnumNode enumNode)
    {
        var val = new EnumValue { Name = (string)data["name"]! };
        var typeName = data.GetValueOrDefault("type_name") as string;

        if (typeName != null && typeName != "enum")
            val.Type = typeName;

        if (data.ContainsKey("default_value"))
            val.Value = data["default_value"];

        var desc = data.GetValueOrDefault("description") as string;
        if (desc != null)
        {
            val.Description = desc;
        }
        else if (typeName != null)
        {
            var strMatch = System.Text.RegularExpressions.Regex.Match(typeName, @"^""(.*)""$");
            if (strMatch.Success)
            {
                val.Description = strMatch.Groups[1].Value;
                val.Type = null;
            }
        }

        enumNode.Values.Add(val);
    }

    private static void HandleDirective(Dictionary<string, object?> data, ModelNode model, Token token, ParserState state)
    {
        var attrs = GetAttrList(data);
        if (attrs.Count == 0) return;
        var attr = attrs[0];
        var attrName = (string)attr["name"]!;
        var loc = new SourceLocation(state.File, token.Line, 1);

        if (attrName == "index")
        {
            model.Sections.Indexes.Add(new Dictionary<string, object?>
            {
                ["type"] = "directive", ["raw"] = data.GetValueOrDefault("raw_content"),
                ["args"] = attr.GetValueOrDefault("args"), ["loc"] = loc
            });
        }
        else if (attrName == "relation")
        {
            model.Sections.Relations.Add(new Dictionary<string, object?>
            {
                ["type"] = "directive", ["raw"] = data.GetValueOrDefault("raw_content"),
                ["args"] = attr.GetValueOrDefault("args"), ["loc"] = loc
            });
        }
        else if (attrName == "behavior")
        {
            model.Sections.Behaviors.Add(new Dictionary<string, object?>
            {
                ["type"] = "directive", ["raw"] = data.GetValueOrDefault("raw_content"),
                ["args"] = attr.GetValueOrDefault("args"), ["loc"] = loc
            });
        }
        else
        {
            if (!model.Sections.Extra.ContainsKey(attrName))
                model.Sections.Extra[attrName] = new();
            model.Sections.Extra[attrName].Add(new Dictionary<string, object?>
            {
                ["raw"] = data.GetValueOrDefault("raw_content"), ["args"] = attr.GetValueOrDefault("args")
            });
        }
    }

    private static void HandleSectionItem(Dictionary<string, object?> data, ModelNode model, Token token, ParserState state)
    {
        var section = state.CurrentSection!;
        var loc = new SourceLocation(state.File, token.Line, 1);

        if (section == "Source" && model.NodeType == "view")
        {
            var name = (string)data["name"]!;
            if (IsSourceDirective(name) && !state.SourceDirectivesDone)
            {
                model.SourceDef ??= new ViewSourceDef();
                SetSourceDirective(model.SourceDef, data);
                return;
            }
            state.SourceDirectivesDone = true;
            var field = BuildFieldNode(data, token, state);
            model.Fields.Add(field);
            state.LastField = field;
            return;
        }

        if (section == "Refresh" && model.NodeType == "view")
        {
            model.Refresh ??= new RefreshDef();
            var name = (string)data["name"]!;
            var typeName = data.GetValueOrDefault("type_name") as string;
            var desc = data.GetValueOrDefault("description") as string;
            if (name == "strategy") model.Refresh.Strategy = typeName ?? "";
            else if (name == "interval") model.Refresh.Interval = desc ?? typeName ?? "";
            return;
        }

        if (section == "Indexes")
        {
            model.Sections.Indexes.Add(new Dictionary<string, object?>
            {
                ["name"] = data["name"], ["label"] = data.GetValueOrDefault("label"), ["loc"] = loc
            });
            state.LastField = new FieldNode { Name = (string)data["name"]! };
            return;
        }

        if (section == "Relations")
        {
            model.Sections.Relations.Add(new Dictionary<string, object?>
            {
                ["raw"] = token.Raw.Trim().TrimStart('-').Trim(), ["loc"] = loc
            });
            return;
        }

        if (section == "Metadata")
        {
            var name = (string)data["name"]!;
            var value = data.GetValueOrDefault("type_name") ?? data.GetValueOrDefault("description");
            model.Sections.Metadata[name] = ParseMetadataValue(value);
            return;
        }

        if (section == "Behaviors")
        {
            model.Sections.Behaviors.Add(new Dictionary<string, object?>
            {
                ["name"] = data["name"], ["raw"] = token.Raw.Trim(), ["loc"] = loc
            });
            return;
        }

        // Generic section — store as section items, NOT as fields
        if (!model.Sections.Extra.ContainsKey(section))
            model.Sections.Extra[section] = new();
        model.Sections.Extra[section].Add(new Dictionary<string, object?>
        {
            ["name"] = data["name"],
            ["raw"] = token.Raw.Trim(),
            ["value"] = data.GetValueOrDefault("type_name") ?? data.GetValueOrDefault("description") ?? data.GetValueOrDefault("raw_value"),
            ["loc"] = loc
        });
    }

    private static void HandleNestedItem(Token token, ParserState state)
    {
        if (state.CurrentElement == null) return;
        var data = token.Data;
        var key = data.GetValueOrDefault("key") as string;
        var value = data.GetValueOrDefault("value") as string;

        if (state.CurrentElement is EnumNode enumNode)
        {
            if (key != null)
            {
                var val = new EnumValue { Name = key };
                if (value != null)
                {
                    var strMatch = System.Text.RegularExpressions.Regex.Match(value, @"^""(.*)""$");
                    if (strMatch.Success) val.Description = strMatch.Groups[1].Value;
                    else val.Value = value;
                }
                enumNode.Values.Add(val);
            }
            return;
        }

        var model = (ModelNode)state.CurrentElement;

        // Indexes section nested
        if (state.CurrentSection == "Indexes" && state.LastField != null)
        {
            var last = model.Sections.Indexes.LastOrDefault();
            if (last is Dictionary<string, object?> dict && key != null)
                dict[key] = ParseNestedValue(value ?? "");
            return;
        }

        // Field nested items
        if (state.LastField != null)
        {
            var field = state.LastField;

            if (key == "values" && value == null)
            {
                field.EnumValues ??= new();
                return;
            }

            if (field.EnumValues != null && key != null)
            {
                var strMatch = value != null ? System.Text.RegularExpressions.Regex.Match(value, @"^""(.*)""$") : null;
                field.EnumValues.Add(new EnumValue
                {
                    Name = key,
                    Description = strMatch?.Success == true ? strMatch.Groups[1].Value : null,
                    Value = strMatch?.Success != true ? value : null,
                });
                return;
            }

            if (field.Type == "enum" && key != null && (value == null || !value.Contains(':')))
            {
                field.EnumValues ??= new();
                var strMatch = value != null ? System.Text.RegularExpressions.Regex.Match(value, @"^""(.*)""$") : null;
                field.EnumValues.Add(new EnumValue
                {
                    Name = key,
                    Description = strMatch?.Success == true ? strMatch.Groups[1].Value : null,
                });
                return;
            }

            // D-003: Sub-field for object type parent
            if (key != null && value != null && field.Type == "object")
            {
                field.Fields ??= new();
                var pseudoData = new Dictionary<string, object?> { ["name"] = key };
                // Quick type parse
                var subTypeMatch = new System.Text.RegularExpressions.Regex(@"^([\w]+)(?:<([^>]+)>)?(?:\(([^)]*)\))?(\?)?(\[\])?").Match(value);
                if (subTypeMatch.Success)
                {
                    pseudoData["type_name"] = subTypeMatch.Groups[1].Value;
                    if (subTypeMatch.Groups[3].Success && subTypeMatch.Groups[3].Value.Length > 0)
                        pseudoData["type_params"] = subTypeMatch.Groups[3].Value.Split(',').Select(s => s.Trim()).ToList();
                    pseudoData["nullable"] = subTypeMatch.Groups[4].Success && subTypeMatch.Groups[4].Value == "?";
                    pseudoData["array"] = subTypeMatch.Groups[5].Success && subTypeMatch.Groups[5].Value == "[]";
                }
                var subField = BuildFieldNode(pseudoData, token, state);
                field.Fields.Add(subField);
                state.LastField = subField;
                return;
            }

            if (key != null)
            {
                ApplyExtendedAttribute(field, key, value ?? "");
                return;
            }
        }

        // Source section nested for views
        if (state.CurrentSection == "Source" && model.NodeType == "view" && key != null && model.SourceDef != null)
            SetSourceDirective(model.SourceDef, new Dictionary<string, object?> { ["name"] = key, ["type_name"] = value });
    }

    private static void HandleBlockquote(Token token, ParserState state)
    {
        if (state.CurrentElement == null) return;
        var text = (string)token.Data["text"]!;

        if (state.CurrentElement is ModelNode model)
            model.Description = model.Description != null ? model.Description + "\n" + text : text;
        else if (state.CurrentElement is EnumNode enumNode)
            enumNode.Description = enumNode.Description != null ? enumNode.Description + "\n" + text : text;
    }

    private static void HandleText(Token token, ParserState state)
    {
        if (state.CurrentElement is ModelNode model && model.Fields.Count == 0)
        {
            var text = token.Data.GetValueOrDefault("text") as string ?? "";
            if (text.Length > 0 && model.Description == null)
                model.Description = text;
        }
    }

    private static void FinalizeElement(ParserState state)
    {
        if (state.CurrentElement is EnumNode enumNode)
            state.Enums.Add(enumNode);
        else if (state.CurrentElement is ModelNode model)
        {
            switch (model.NodeType)
            {
                case "interface": state.Interfaces.Add(model); break;
                case "view": state.Views.Add(model); break;
                default: state.Models.Add(model); break;
            }
        }
        state.CurrentElement = null;
        state.CurrentSection = null;
        state.CurrentKind = FieldKind.Stored;
        state.LastField = null;
    }

    // --- Helpers ---

    private static FieldNode BuildFieldNode(Dictionary<string, object?> data, Token token, ParserState state)
    {
        var attrs = ParseAttributes(GetAttrList(data));
        var kind = state.CurrentKind;

        var lookupAttr = attrs.Find(a => a.Name == "lookup");
        var rollupAttr = attrs.Find(a => a.Name == "rollup");
        var computedAttr = attrs.Find(a => a.Name == "computed");
        var computedRawAttr = attrs.Find(a => a.Name == "computed_raw");

        if (lookupAttr != null) kind = FieldKind.Lookup;
        else if (rollupAttr != null) kind = FieldKind.Rollup;
        else if (computedAttr != null) kind = FieldKind.Computed;
        else if (computedRawAttr != null) kind = FieldKind.Computed;

        var fwRaw = data.GetValueOrDefault("framework_attrs") as List<string>;
        List<CustomAttribute>? fwAttrs = null;
        if (fwRaw is { Count: > 0 })
            fwAttrs = fwRaw.Select(raw => new CustomAttribute
            {
                Content = raw.TrimStart('[').TrimEnd(']'),
                Raw = raw,
            }).ToList();

        var field = new FieldNode
        {
            Name = (string)data["name"]!,
            Label = data.GetValueOrDefault("label") as string,
            Type = data.GetValueOrDefault("type_name") as string,
            GenericParams = data.GetValueOrDefault("type_generic_params") as List<string>,
            Params = ParseTypeParams(data.GetValueOrDefault("type_params")),
            Nullable = data.GetValueOrDefault("nullable") is true,
            Array = data.GetValueOrDefault("array") is true,
            ArrayItemNullable = data.GetValueOrDefault("arrayItemNullable") is true,
            Kind = kind,
            DefaultValue = data.GetValueOrDefault("default_value") as string,
            Description = data.GetValueOrDefault("description") as string,
            Attributes = attrs,
            FrameworkAttrs = fwAttrs,
            Loc = new SourceLocation(state.File, token.Line, 1),
        };

        if (lookupAttr?.Args is [string lookupArg, ..])
            field.Lookup = new LookupDef { Path = lookupArg };

        if (rollupAttr?.Args is [string rollupArg, ..])
            field.Rollup = ParseRollupArgs(rollupArg);

        if (computedAttr?.Args is [string computedArg, ..])
            field.Computed = new ComputedDef { Expression = computedArg.Trim('"', '\'') };

        if (computedRawAttr?.Args is [string computedRawArg, ..])
            field.Computed = new ComputedDef { Expression = computedRawArg.Trim('"', '\'') };

        return field;
    }

    private static List<FieldAttribute> ParseAttributes(List<Dictionary<string, object?>> rawAttrs)
    {
        return rawAttrs.Select(a => new FieldAttribute
        {
            Name = (string)a["name"]!,
            Args = a.GetValueOrDefault("args") is string s ? new List<object> { s } : null,
        }).ToList();
    }

    private static List<object>? ParseTypeParams(object? raw)
    {
        if (raw is not List<string> list || list.Count == 0) return null;
        return list.Select<string, object>(p =>
        {
            if (int.TryParse(p, out var n)) return n;
            return p;
        }).ToList();
    }

    private static RollupDef ParseRollupArgs(string argsStr)
    {
        var parts = SplitRollupArgs(argsStr);
        var targetFk = parts.Count > 0 ? parts[0] : "";
        var dotIdx = targetFk.IndexOf('.');
        var target = dotIdx >= 0 ? targetFk[..dotIdx] : targetFk;
        var fk = dotIdx >= 0 ? targetFk[(dotIdx + 1)..] : "";

        string aggregate = "";
        string? field = null;

        if (parts.Count > 1)
        {
            var aggPart = parts[1].Trim();
            var aggMatch = System.Text.RegularExpressions.Regex.Match(aggPart, @"^(\w+)(?:\((\w+)\))?$");
            if (aggMatch.Success)
            {
                aggregate = aggMatch.Groups[1].Value;
                field = aggMatch.Groups[2].Success ? aggMatch.Groups[2].Value : null;
            }
            else aggregate = aggPart;
        }

        string? where = null;
        for (int i = 2; i < parts.Count; i++)
        {
            var part = parts[i].Trim();
            var wm = System.Text.RegularExpressions.Regex.Match(part, @"^where:\s*""(.*)""$");
            if (wm.Success) where = wm.Groups[1].Value;
        }

        return new RollupDef { Target = target, Fk = fk, Aggregate = aggregate, Field = field, Where = where };
    }

    private static List<string> SplitRollupArgs(string argsStr)
    {
        var parts = new List<string>();
        var current = "";
        int depth = 0;
        bool inQuote = false;
        char quoteChar = '\0';

        foreach (var ch in argsStr)
        {
            if (inQuote) { current += ch; if (ch == quoteChar) inQuote = false; continue; }
            if (ch is '"' or '\'') { inQuote = true; quoteChar = ch; current += ch; continue; }
            if (ch == '(') { depth++; current += ch; continue; }
            if (ch == ')') { depth--; current += ch; continue; }
            if (ch == ',' && depth == 0) { parts.Add(current.Trim()); current = ""; continue; }
            current += ch;
        }
        if (current.Trim().Length > 0) parts.Add(current.Trim());
        return parts;
    }

    private static bool IsSourceDirective(string name) =>
        name is "from" or "where" or "order_by" or "group_by" or "join";

    private static void SetSourceDirective(ViewSourceDef def, Dictionary<string, object?> data)
    {
        var name = (string)data["name"]!;
        var typeName = data.GetValueOrDefault("type_name") as string;
        var desc = data.GetValueOrDefault("description") as string;
        var rawValue = data.GetValueOrDefault("raw_value") as string;
        var value = desc ?? rawValue ?? typeName ?? "";

        switch (name)
        {
            case "from": def.From = value; break;
            case "where": def.Where = value; break;
            case "order_by": def.OrderBy = value; break;
            case "group_by": def.GroupBy = ParseArrayValue(value); break;
            case "join":
                def.Joins ??= new();
                var joinParts = value.Split(new[] { " on " }, 2, StringSplitOptions.None);
                def.Joins.Add(new JoinDef
                {
                    Model = joinParts[0].Trim(),
                    On = joinParts.Length > 1 ? joinParts[1].Trim() : "",
                });
                break;
        }
    }

    private static List<string> ParseArrayValue(string value)
    {
        var cleaned = value.TrimStart('[').TrimEnd(']');
        return cleaned.Split(',').Select(s => s.Trim()).Where(s => s.Length > 0).ToList();
    }

    private static object? ParseMetadataValue(object? value)
    {
        if (value is not string str) return value;
        var unquoted = str.Trim('"', '\'');
        if (int.TryParse(unquoted, out var n)) return n;
        if (double.TryParse(unquoted, System.Globalization.NumberStyles.Float,
            System.Globalization.CultureInfo.InvariantCulture, out var d)) return d;
        if (unquoted == "true") return true;
        if (unquoted == "false") return false;
        return unquoted;
    }

    private static object ParseNestedValue(string value)
    {
        var str = value.Trim();
        if (str.StartsWith('[') && str.EndsWith(']')) return ParseArrayValue(str);
        if (str == "true") return true;
        if (str == "false") return false;
        if (int.TryParse(str, out var n)) return n;
        return str.Trim('"', '\'');
    }

    private static void ApplyExtendedAttribute(FieldNode field, string key, string value)
    {
        var parsed = ParseNestedValue(value);
        switch (key)
        {
            case "type":
                field.Type = value.Replace("?", "").Replace("[]", "");
                if (value.EndsWith('?')) field.Nullable = true;
                if (value.EndsWith("[]")) field.Array = true;
                break;
            case "description":
                field.Description = parsed is string s ? s : parsed.ToString();
                break;
            case "reference":
                field.Attributes.Add(new FieldAttribute { Name = "reference", Args = new() { value } });
                break;
            case "on_delete":
                field.Attributes.Add(new FieldAttribute { Name = "on_delete", Args = new() { value } });
                break;
            default:
                field.Attributes.Add(new FieldAttribute { Name = key, Args = new() { parsed } });
                break;
        }
    }

    private static List<Dictionary<string, object?>> GetAttrList(Dictionary<string, object?> data)
    {
        if (data.GetValueOrDefault("attributes") is List<Dictionary<string, object?>> list) return list;
        return new();
    }

    private static List<string> GetStringList(Dictionary<string, object?> data, string key)
    {
        if (data.GetValueOrDefault(key) is List<string> list) return list;
        return new();
    }
}
