using System.Text.Json;
using System.Text.Json.Serialization;

namespace M3L.Native;

// ---------------------------------------------------------------------------
// Source location
// ---------------------------------------------------------------------------

/// <summary>
/// Source location in the M3L file.
/// </summary>
public class SourceLocation
{
    [JsonPropertyName("file")]
    public string File { get; set; } = "";

    [JsonPropertyName("line")]
    public int Line { get; set; }

    [JsonPropertyName("col")]
    public int Col { get; set; }
}

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// <summary>
/// Field kind: stored, computed, lookup, or rollup.
/// </summary>
[JsonConverter(typeof(JsonStringEnumConverter))]
public enum FieldKind
{
    [JsonPropertyName("stored")]
    Stored,

    [JsonPropertyName("computed")]
    Computed,

    [JsonPropertyName("lookup")]
    Lookup,

    [JsonPropertyName("rollup")]
    Rollup,
}

/// <summary>
/// Model type discriminator.
/// </summary>
[JsonConverter(typeof(JsonStringEnumConverter))]
public enum ModelType
{
    [JsonPropertyName("model")]
    Model,

    [JsonPropertyName("enum")]
    Enum,

    [JsonPropertyName("interface")]
    Interface,

    [JsonPropertyName("view")]
    View,
}

/// <summary>
/// Default value type: literal or expression.
/// </summary>
[JsonConverter(typeof(JsonStringEnumConverter))]
public enum DefaultValueType
{
    [JsonPropertyName("literal")]
    Literal,

    [JsonPropertyName("expression")]
    Expression,
}

/// <summary>
/// Diagnostic severity.
/// </summary>
[JsonConverter(typeof(JsonStringEnumConverter))]
public enum DiagnosticSeverity
{
    [JsonPropertyName("error")]
    Error,

    [JsonPropertyName("warning")]
    Warning,
}

// ---------------------------------------------------------------------------
// Project info
// ---------------------------------------------------------------------------

/// <summary>
/// Project metadata from the M3L document.
/// </summary>
public class ProjectInfo
{
    [JsonPropertyName("name")]
    public string? Name { get; set; }

    [JsonPropertyName("version")]
    public string? Version { get; set; }
}

// ---------------------------------------------------------------------------
// Field-related types
// ---------------------------------------------------------------------------

/// <summary>
/// A field attribute (e.g., @required, @unique).
/// </summary>
public class FieldAttribute
{
    [JsonPropertyName("name")]
    public string Name { get; set; } = "";

    [JsonPropertyName("args")]
    public List<JsonElement>? Args { get; set; }

    [JsonPropertyName("cascade")]
    public string? Cascade { get; set; }

    [JsonPropertyName("isStandard")]
    public bool? IsStandard { get; set; }

    [JsonPropertyName("isRegistered")]
    public bool? IsRegistered { get; set; }
}

/// <summary>
/// Parsed representation of a custom (framework) attribute.
/// </summary>
public class CustomAttributeParsed
{
    [JsonPropertyName("name")]
    public string Name { get; set; } = "";

    [JsonPropertyName("arguments")]
    public List<JsonElement> Arguments { get; set; } = [];
}

/// <summary>
/// Custom (framework) attribute attached to a field.
/// </summary>
public class CustomAttribute
{
    [JsonPropertyName("content")]
    public string Content { get; set; } = "";

    [JsonPropertyName("raw")]
    public string Raw { get; set; } = "";

    [JsonPropertyName("parsed")]
    public CustomAttributeParsed? Parsed { get; set; }
}

/// <summary>
/// Lookup definition for a lookup field.
/// </summary>
public class LookupDef
{
    [JsonPropertyName("path")]
    public string Path { get; set; } = "";
}

/// <summary>
/// Rollup definition for a rollup field.
/// </summary>
public class RollupDef
{
    [JsonPropertyName("target")]
    public string Target { get; set; } = "";

    [JsonPropertyName("fk")]
    public string Fk { get; set; } = "";

    [JsonPropertyName("aggregate")]
    public string Aggregate { get; set; } = "";

    [JsonPropertyName("field")]
    public string? Field { get; set; }

    [JsonPropertyName("where")]
    public string? Where { get; set; }
}

/// <summary>
/// Computed field definition.
/// </summary>
public class ComputedDef
{
    [JsonPropertyName("expression")]
    public string Expression { get; set; } = "";

    [JsonPropertyName("platform")]
    public string? Platform { get; set; }
}

/// <summary>
/// Enum member value.
/// </summary>
public class EnumValue
{
    [JsonPropertyName("name")]
    public string Name { get; set; } = "";

    [JsonPropertyName("description")]
    public string? Description { get; set; }

    [JsonPropertyName("type")]
    public string? Type { get; set; }

    [JsonPropertyName("value")]
    public JsonElement? Value { get; set; }
}

/// <summary>
/// A field in a model, view, or interface.
/// </summary>
public class FieldNode
{
    [JsonPropertyName("name")]
    public string Name { get; set; } = "";

    [JsonPropertyName("label")]
    public string? Label { get; set; }

    [JsonPropertyName("type")]
    public string? Type { get; set; }

    [JsonPropertyName("params")]
    public List<JsonElement>? Params { get; set; }

    [JsonPropertyName("generic_params")]
    public List<string>? GenericParams { get; set; }

    [JsonPropertyName("nullable")]
    public bool Nullable { get; set; }

    [JsonPropertyName("array")]
    public bool Array { get; set; }

    [JsonPropertyName("arrayItemNullable")]
    public bool ArrayItemNullable { get; set; }

    [JsonPropertyName("kind")]
    public FieldKind Kind { get; set; }

    [JsonPropertyName("default_value")]
    public string? DefaultValue { get; set; }

    [JsonPropertyName("default_value_type")]
    public DefaultValueType? DefaultValueType { get; set; }

    [JsonPropertyName("description")]
    public string? Description { get; set; }

    [JsonPropertyName("attributes")]
    public List<FieldAttribute> Attributes { get; set; } = [];

    [JsonPropertyName("framework_attrs")]
    public List<CustomAttribute>? FrameworkAttrs { get; set; }

    [JsonPropertyName("lookup")]
    public LookupDef? Lookup { get; set; }

    [JsonPropertyName("rollup")]
    public RollupDef? Rollup { get; set; }

    [JsonPropertyName("computed")]
    public ComputedDef? Computed { get; set; }

    [JsonPropertyName("enum_values")]
    public List<EnumValue>? EnumValues { get; set; }

    [JsonPropertyName("fields")]
    public List<FieldNode>? Fields { get; set; }

    [JsonPropertyName("loc")]
    public SourceLocation Loc { get; set; } = new();
}

// ---------------------------------------------------------------------------
// View-related types
// ---------------------------------------------------------------------------

/// <summary>
/// Join definition in a view source.
/// </summary>
public class JoinDef
{
    [JsonPropertyName("model")]
    public string Model { get; set; } = "";

    [JsonPropertyName("on")]
    public string On { get; set; } = "";
}

/// <summary>
/// View source definition (FROM, JOIN, WHERE, etc.).
/// </summary>
public class ViewSourceDef
{
    [JsonPropertyName("from")]
    public string? From { get; set; }

    [JsonPropertyName("joins")]
    public List<JoinDef>? Joins { get; set; }

    [JsonPropertyName("where")]
    public string? Where { get; set; }

    [JsonPropertyName("order_by")]
    public string? OrderBy { get; set; }

    [JsonPropertyName("group_by")]
    public List<string>? GroupBy { get; set; }

    [JsonPropertyName("raw_sql")]
    public string? RawSql { get; set; }

    [JsonPropertyName("language_hint")]
    public string? LanguageHint { get; set; }
}

/// <summary>
/// Materialized view refresh strategy.
/// </summary>
public class RefreshDef
{
    [JsonPropertyName("strategy")]
    public string Strategy { get; set; } = "";

    [JsonPropertyName("interval")]
    public string? Interval { get; set; }
}

// ---------------------------------------------------------------------------
// Sections
// ---------------------------------------------------------------------------

/// <summary>
/// Model sections (indexes, relations, behaviors, metadata, plus custom sections).
/// Custom sections are captured via ExtensionData.
/// </summary>
public class Sections
{
    [JsonPropertyName("indexes")]
    public List<JsonElement> Indexes { get; set; } = [];

    [JsonPropertyName("relations")]
    public List<JsonElement> Relations { get; set; } = [];

    [JsonPropertyName("behaviors")]
    public List<JsonElement> Behaviors { get; set; } = [];

    [JsonPropertyName("metadata")]
    public Dictionary<string, JsonElement> Metadata { get; set; } = [];

    [JsonExtensionData]
    public Dictionary<string, JsonElement>? Custom { get; set; }
}

// ---------------------------------------------------------------------------
// Top-level node types
// ---------------------------------------------------------------------------

/// <summary>
/// A model node in the AST.
/// </summary>
public class ModelNode
{
    [JsonPropertyName("name")]
    public string Name { get; set; } = "";

    [JsonPropertyName("label")]
    public string? Label { get; set; }

    [JsonPropertyName("type")]
    public ModelType Type { get; set; }

    [JsonPropertyName("source")]
    public string Source { get; set; } = "";

    [JsonPropertyName("line")]
    public int Line { get; set; }

    [JsonPropertyName("inherits")]
    public List<string> Inherits { get; set; } = [];

    [JsonPropertyName("description")]
    public string? Description { get; set; }

    [JsonPropertyName("attributes")]
    public List<FieldAttribute> Attributes { get; set; } = [];

    [JsonPropertyName("fields")]
    public List<FieldNode> Fields { get; set; } = [];

    [JsonPropertyName("sections")]
    public Sections Sections { get; set; } = new();

    [JsonPropertyName("materialized")]
    public bool? Materialized { get; set; }

    [JsonPropertyName("source_def")]
    public ViewSourceDef? SourceDef { get; set; }

    [JsonPropertyName("refresh")]
    public RefreshDef? Refresh { get; set; }

    [JsonPropertyName("loc")]
    public SourceLocation Loc { get; set; } = new();
}

/// <summary>
/// An enum node in the AST.
/// </summary>
public class EnumNode
{
    [JsonPropertyName("name")]
    public string Name { get; set; } = "";

    [JsonPropertyName("label")]
    public string? Label { get; set; }

    [JsonPropertyName("type")]
    public ModelType Type { get; set; }

    [JsonPropertyName("source")]
    public string Source { get; set; } = "";

    [JsonPropertyName("line")]
    public int Line { get; set; }

    [JsonPropertyName("inherits")]
    public List<string> Inherits { get; set; } = [];

    [JsonPropertyName("description")]
    public string? Description { get; set; }

    [JsonPropertyName("values")]
    public List<EnumValue> Values { get; set; } = [];

    [JsonPropertyName("loc")]
    public SourceLocation Loc { get; set; } = new();
}

// ---------------------------------------------------------------------------
// Attribute registry
// ---------------------------------------------------------------------------

/// <summary>
/// An entry in the attribute registry.
/// </summary>
public class AttributeRegistryEntry
{
    [JsonPropertyName("name")]
    public string Name { get; set; } = "";

    [JsonPropertyName("description")]
    public string? Description { get; set; }

    [JsonPropertyName("target")]
    public List<string> Target { get; set; } = [];

    [JsonPropertyName("type")]
    public string Type { get; set; } = "";

    [JsonPropertyName("range")]
    public List<double>? Range { get; set; }

    [JsonPropertyName("required")]
    public bool Required { get; set; }

    [JsonPropertyName("defaultValue")]
    public JsonElement? DefaultValue { get; set; }
}

// ---------------------------------------------------------------------------
// Diagnostic
// ---------------------------------------------------------------------------

/// <summary>
/// A diagnostic message (error or warning) with source location.
/// </summary>
public class Diagnostic
{
    [JsonPropertyName("code")]
    public string Code { get; set; } = "";

    [JsonPropertyName("severity")]
    public DiagnosticSeverity Severity { get; set; }

    [JsonPropertyName("file")]
    public string File { get; set; } = "";

    [JsonPropertyName("line")]
    public int Line { get; set; }

    [JsonPropertyName("col")]
    public int Col { get; set; }

    [JsonPropertyName("message")]
    public string Message { get; set; } = "";
}

// ---------------------------------------------------------------------------
// Top-level AST
// ---------------------------------------------------------------------------

/// <summary>
/// The complete M3L AST — top-level JSON output from parsing.
/// </summary>
public class M3lAst
{
    [JsonPropertyName("parserVersion")]
    public string ParserVersion { get; set; } = "";

    [JsonPropertyName("astVersion")]
    public string AstVersion { get; set; } = "";

    [JsonPropertyName("project")]
    public ProjectInfo Project { get; set; } = new();

    [JsonPropertyName("sources")]
    public List<string> Sources { get; set; } = [];

    [JsonPropertyName("models")]
    public List<ModelNode> Models { get; set; } = [];

    [JsonPropertyName("enums")]
    public List<EnumNode> Enums { get; set; } = [];

    [JsonPropertyName("interfaces")]
    public List<ModelNode> Interfaces { get; set; } = [];

    [JsonPropertyName("views")]
    public List<ModelNode> Views { get; set; } = [];

    [JsonPropertyName("attributeRegistry")]
    public List<AttributeRegistryEntry> AttributeRegistry { get; set; } = [];

    [JsonPropertyName("errors")]
    public List<Diagnostic> Errors { get; set; } = [];

    [JsonPropertyName("warnings")]
    public List<Diagnostic> Warnings { get; set; } = [];
}

// ---------------------------------------------------------------------------
// Validate result (strongly-typed)
// ---------------------------------------------------------------------------

/// <summary>
/// Validation result with strongly-typed diagnostics.
/// </summary>
public class ValidateResult
{
    [JsonPropertyName("errors")]
    public List<Diagnostic> Errors { get; set; } = [];

    [JsonPropertyName("warnings")]
    public List<Diagnostic> Warnings { get; set; } = [];
}
