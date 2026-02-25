using M3L.Models;

namespace M3L;

/// <summary>
/// High-level API for parsing M3L files into a merged AST.
/// </summary>
public class M3LParser
{
    /// <summary>
    /// Parse M3L files from a path (file or directory) into a merged AST.
    /// </summary>
    public async Task<M3LAst> ParseAsync(string inputPath, ProjectInfo? project = null)
    {
        var files = await FileReader.ReadM3LFiles(inputPath);

        if (files.Count == 0)
            throw new InvalidOperationException($"No .m3l.md or .m3l files found at: {inputPath}");

        var parsedFiles = files.Select(f => Parser.ParseString(f.Content, f.Path)).ToList();
        return Resolver.Resolve(parsedFiles, project);
    }

    /// <summary>
    /// Parse M3L content string into AST.
    /// </summary>
    public M3LAst ParseString(string content, string filename = "inline.m3l.md")
    {
        var parsed = Parser.ParseString(content, filename);
        return Resolver.Resolve([parsed]);
    }

    /// <summary>
    /// Parse M3L files and validate the AST.
    /// </summary>
    public async Task<(M3LAst Ast, ValidateResult Validation)> ValidateAsync(
        string inputPath, ValidateOptions? options = null, ProjectInfo? project = null)
    {
        var ast = await ParseAsync(inputPath, project);
        var result = Validator.Validate(ast, options);
        return (ast, result);
    }

    /// <summary>
    /// Parse M3L content string and validate the AST.
    /// </summary>
    public (M3LAst Ast, ValidateResult Validation) ValidateString(
        string content, ValidateOptions? options = null, string filename = "inline.m3l.md")
    {
        var ast = ParseString(content, filename);
        var result = Validator.Validate(ast, options);
        return (ast, result);
    }

    /// <summary>
    /// Returns the AST schema version.
    /// </summary>
    public static string GetAstVersion() => Resolver.AstVersion;

    /// <summary>
    /// Returns the parser package version.
    /// </summary>
    public static string GetParserVersion() => Resolver.ParserVersion;
}
