using System.Runtime.InteropServices;
using System.Text.Json;

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
    /// Parse and return a strongly-typed result.
    /// </summary>
    public static M3lResult<JsonElement>? ParseTyped(string content, string filename)
    {
        var json = Parse(content, filename);
        return JsonSerializer.Deserialize<M3lResult<JsonElement>>(json);
    }

    /// <summary>
    /// Validate and return a strongly-typed result.
    /// </summary>
    public static M3lResult<ValidateResultData>? ValidateTyped(string content, string optionsJson = "{}")
    {
        var json = Validate(content, optionsJson);
        return JsonSerializer.Deserialize<M3lResult<ValidateResultData>>(json);
    }
}

/// <summary>
/// Generic result wrapper from native calls.
/// </summary>
public class M3lResult<T>
{
    public bool Success { get; set; }
    public T? Data { get; set; }
    public string? Error { get; set; }
}

/// <summary>
/// Validation result data.
/// </summary>
public class ValidateResultData
{
    public List<DiagnosticItem> Errors { get; set; } = [];
    public List<DiagnosticItem> Warnings { get; set; } = [];
}

/// <summary>
/// A single diagnostic item.
/// </summary>
public class DiagnosticItem
{
    public string Code { get; set; } = "";
    public string Severity { get; set; } = "";
    public string File { get; set; } = "";
    public int Line { get; set; }
    public int Col { get; set; }
    public string Message { get; set; } = "";
}
