using System.Runtime.InteropServices;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace M3L.Native;

/// <summary>
/// P/Invoke bindings for the Rust m3l-cabi native library.
/// All methods take string inputs and return JSON strings.
/// </summary>
public static class M3lNative
{
    private const string LibName = "m3l_cabi";

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    private static extern IntPtr m3l_parse(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string content,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string filename);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    private static extern IntPtr m3l_parse_multi(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string filesJson);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    private static extern IntPtr m3l_validate(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string content,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string optionsJson);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Ansi)]
    private static extern IntPtr m3l_lint(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string content,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string configJson);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    private static extern void m3l_free_string(IntPtr ptr);

    private static string CallNative(IntPtr ptr)
    {
        if (ptr == IntPtr.Zero)
            return """{"success":false,"error":"Native call returned null"}""";

        try
        {
            string? result = Marshal.PtrToStringUTF8(ptr);
            return result ?? """{"success":false,"error":"Failed to read native string"}""";
        }
        finally
        {
            m3l_free_string(ptr);
        }
    }

    /// <summary>
    /// Parse a single M3L file and return the AST as JSON.
    /// </summary>
    /// <param name="content">M3L markdown text</param>
    /// <param name="filename">Source filename for error reporting</param>
    /// <returns>JSON string with { success, data?, error? }</returns>
    public static string Parse(string content, string filename)
    {
        var ptr = m3l_parse(content, filename);
        return CallNative(ptr);
    }

    /// <summary>
    /// Parse multiple M3L files and return the merged AST as JSON.
    /// </summary>
    /// <param name="filesJson">JSON array of { content, filename } objects</param>
    /// <returns>JSON string with { success, data?, error? }</returns>
    public static string ParseMulti(string filesJson)
    {
        var ptr = m3l_parse_multi(filesJson);
        return CallNative(ptr);
    }

    /// <summary>
    /// Validate M3L content and return diagnostics as JSON.
    /// </summary>
    /// <param name="content">M3L markdown text</param>
    /// <param name="optionsJson">JSON options { strict?, filename? }</param>
    /// <returns>JSON string with { success, data?, error? }</returns>
    public static string Validate(string content, string optionsJson = "{}")
    {
        var ptr = m3l_validate(content, optionsJson);
        return CallNative(ptr);
    }

    /// <summary>
    /// Parse and return a strongly-typed result with raw JsonElement data.
    /// </summary>
    public static M3lResult<JsonElement>? ParseTyped(string content, string filename)
    {
        var json = Parse(content, filename);
        return JsonSerializer.Deserialize<M3lResult<JsonElement>>(json);
    }

    /// <summary>
    /// Parse a single M3L file and return a strongly-typed AST.
    /// </summary>
    public static M3lResult<M3lAst>? ParseToAst(string content, string filename)
    {
        var json = Parse(content, filename);
        return JsonSerializer.Deserialize<M3lResult<M3lAst>>(json, AstJsonOptions);
    }

    /// <summary>
    /// Parse multiple M3L files and return a strongly-typed merged AST.
    /// </summary>
    public static M3lResult<M3lAst>? ParseMultiToAst(string filesJson)
    {
        var json = ParseMulti(filesJson);
        return JsonSerializer.Deserialize<M3lResult<M3lAst>>(json, AstJsonOptions);
    }

    /// <summary>
    /// Validate and return a strongly-typed result.
    /// </summary>
    public static M3lResult<ValidateResultData>? ValidateTyped(string content, string optionsJson = "{}")
    {
        var json = Validate(content, optionsJson);
        return JsonSerializer.Deserialize<M3lResult<ValidateResultData>>(json);
    }

    /// <summary>
    /// Validate and return a strongly-typed result with Diagnostic objects.
    /// </summary>
    public static M3lResult<ValidateResult>? ValidateToResult(string content, string optionsJson = "{}")
    {
        var json = Validate(content, optionsJson);
        return JsonSerializer.Deserialize<M3lResult<ValidateResult>>(json, AstJsonOptions);
    }

    /// <summary>
    /// Lint M3L content and return diagnostics as JSON.
    /// </summary>
    /// <param name="content">M3L markdown text</param>
    /// <param name="configJson">JSON config { rules?: Record&lt;string, "off"|"warn"|"error"&gt; }</param>
    /// <returns>JSON string with { success, data?, error? }</returns>
    public static string Lint(string content, string configJson = "{}")
    {
        var ptr = m3l_lint(content, configJson);
        return CallNative(ptr);
    }

    /// <summary>
    /// Lint and return a strongly-typed result.
    /// </summary>
    public static M3lResult<LintResultData>? LintTyped(string content, string configJson = "{}")
    {
        var json = Lint(content, configJson);
        return JsonSerializer.Deserialize<M3lResult<LintResultData>>(json);
    }

    private static readonly JsonSerializerOptions AstJsonOptions = new()
    {
        PropertyNameCaseInsensitive = false,
        Converters = { new JsonStringEnumConverter(JsonNamingPolicy.CamelCase) },
    };
}

/// <summary>
/// Generic result wrapper from native calls.
/// </summary>
public class M3lResult<T>
{
    [JsonPropertyName("success")]
    public bool Success { get; set; }

    [JsonPropertyName("data")]
    public T? Data { get; set; }

    [JsonPropertyName("error")]
    public string? Error { get; set; }
}

/// <summary>
/// Validation result data.
/// </summary>
public class ValidateResultData
{
    [JsonPropertyName("errors")]
    public List<DiagnosticItem> Errors { get; set; } = [];

    [JsonPropertyName("warnings")]
    public List<DiagnosticItem> Warnings { get; set; } = [];
}

/// <summary>
/// A single diagnostic item.
/// </summary>
public class DiagnosticItem
{
    [JsonPropertyName("code")]
    public string Code { get; set; } = "";

    [JsonPropertyName("severity")]
    public string Severity { get; set; } = "";

    [JsonPropertyName("file")]
    public string File { get; set; } = "";

    [JsonPropertyName("line")]
    public int Line { get; set; }

    [JsonPropertyName("col")]
    public int Col { get; set; }

    [JsonPropertyName("message")]
    public string Message { get; set; } = "";
}

/// <summary>
/// Lint result data.
/// </summary>
public class LintResultData
{
    [JsonPropertyName("diagnostics")]
    public List<LintDiagnosticItem> Diagnostics { get; set; } = [];

    [JsonPropertyName("file_count")]
    public int FileCount { get; set; }
}

/// <summary>
/// A single lint diagnostic item.
/// </summary>
public class LintDiagnosticItem
{
    [JsonPropertyName("rule")]
    public string Rule { get; set; } = "";

    [JsonPropertyName("severity")]
    public string Severity { get; set; } = "";

    [JsonPropertyName("file")]
    public string File { get; set; } = "";

    [JsonPropertyName("line")]
    public int Line { get; set; }

    [JsonPropertyName("col")]
    public int Col { get; set; }

    [JsonPropertyName("message")]
    public string Message { get; set; } = "";
}
