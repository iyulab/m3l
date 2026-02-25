using System.Text.Json.Serialization;

namespace M3L.Models;

public enum FieldKind { Stored, Computed, Lookup, Rollup }

public class FieldAttribute
{
    public string Name { get; set; } = "";
    public List<object>? Args { get; set; }
    public string? Cascade { get; set; }
}

public class CustomAttribute
{
    /// <summary>Content inside brackets, e.g. "MaxLength(100)"</summary>
    public string Content { get; set; } = "";
    /// <summary>Original text including brackets, e.g. "[MaxLength(100)]"</summary>
    public string Raw { get; set; } = "";
}

public class EnumValue
{
    public string Name { get; set; } = "";
    public string? Description { get; set; }
    public string? Type { get; set; }
    public object? Value { get; set; }
}

public class FieldNode
{
    public string Name { get; set; } = "";
    public string? Label { get; set; }
    public string? Type { get; set; }
    public List<string>? GenericParams { get; set; }
    public List<object>? Params { get; set; }
    public bool Nullable { get; set; }
    public bool Array { get; set; }
    public bool ArrayItemNullable { get; set; }
    public FieldKind Kind { get; set; } = FieldKind.Stored;
    public string? DefaultValue { get; set; }
    public string? Description { get; set; }
    public List<FieldAttribute> Attributes { get; set; } = new();
    public List<CustomAttribute>? FrameworkAttrs { get; set; }
    public LookupDef? Lookup { get; set; }
    public RollupDef? Rollup { get; set; }
    public ComputedDef? Computed { get; set; }
    public List<EnumValue>? EnumValues { get; set; }
    public List<FieldNode>? Fields { get; set; }
    public SourceLocation Loc { get; set; } = new("", 0, 0);
}

public class LookupDef
{
    public string Path { get; set; } = "";
}

public class RollupDef
{
    public string Target { get; set; } = "";
    public string Fk { get; set; } = "";
    public string Aggregate { get; set; } = "";
    public string? Field { get; set; }
    public string? Where { get; set; }
}

public class ComputedDef
{
    public string Expression { get; set; } = "";
    public string? Platform { get; set; }
}

public class ViewSourceDef
{
    public string From { get; set; } = "";
    public List<JoinDef>? Joins { get; set; }
    public string? Where { get; set; }
    public string? OrderBy { get; set; }
    public List<string>? GroupBy { get; set; }
}

public class JoinDef
{
    public string Model { get; set; } = "";
    public string On { get; set; } = "";
}

public class RefreshDef
{
    public string Strategy { get; set; } = "";
    public string? Interval { get; set; }
}

public class SectionData
{
    public List<object> Indexes { get; set; } = new();
    public List<object> Relations { get; set; } = new();
    public List<object> Behaviors { get; set; } = new();
    public Dictionary<string, object?> Metadata { get; set; } = new();
    [JsonExtensionData]
    public Dictionary<string, object?> Extra { get; set; } = new();
}

public class ModelNode
{
    public string Name { get; set; } = "";
    public string? Label { get; set; }
    public string NodeType { get; set; } = "model"; // model, enum, interface, view
    public string Source { get; set; } = "";
    public int Line { get; set; }
    public List<string> Inherits { get; set; } = new();
    public string? Description { get; set; }
    public List<FieldAttribute> Attributes { get; set; } = new();
    public List<FieldNode> Fields { get; set; } = new();
    public SectionData Sections { get; set; } = new();
    public bool Materialized { get; set; }
    public ViewSourceDef? SourceDef { get; set; }
    public RefreshDef? Refresh { get; set; }
    public SourceLocation Loc { get; set; } = new("", 0, 0);
}

public class EnumNode
{
    public string Name { get; set; } = "";
    public string? Label { get; set; }
    public string Source { get; set; } = "";
    public int Line { get; set; }
    public string? Description { get; set; }
    public List<EnumValue> Values { get; set; } = new();
    public SourceLocation Loc { get; set; } = new("", 0, 0);
}

public class ProjectInfo
{
    public string? Name { get; set; }
    public string? Version { get; set; }
}

public class Diagnostic
{
    public string Code { get; set; } = "";
    public string Severity { get; set; } = "error";
    public string File { get; set; } = "";
    public int Line { get; set; }
    public int Col { get; set; } = 1;
    public string Message { get; set; } = "";
}

public class ParsedFile
{
    public string Source { get; set; } = "";
    public string? Namespace { get; set; }
    public List<ModelNode> Models { get; set; } = new();
    public List<EnumNode> Enums { get; set; } = new();
    public List<ModelNode> Interfaces { get; set; } = new();
    public List<ModelNode> Views { get; set; } = new();
}

public class M3LAst
{
    public string ParserVersion { get; set; } = "";
    public string AstVersion { get; set; } = "";
    public ProjectInfo Project { get; set; } = new();
    public List<string> Sources { get; set; } = new();
    public List<ModelNode> Models { get; set; } = new();
    public List<EnumNode> Enums { get; set; } = new();
    public List<ModelNode> Interfaces { get; set; } = new();
    public List<ModelNode> Views { get; set; } = new();
    public List<Diagnostic> Errors { get; set; } = new();
    public List<Diagnostic> Warnings { get; set; } = new();
}

public class ValidateOptions
{
    public bool Strict { get; set; }
}

public class ValidateResult
{
    public List<Diagnostic> Errors { get; set; } = new();
    public List<Diagnostic> Warnings { get; set; } = new();
}
