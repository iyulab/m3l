using System.Text.Json;
using System.Text.Json.Serialization;
using M3L;
using M3L.Models;

namespace M3L.Tests;

/// <summary>
/// Parses all M3L sample files and dumps the AST as JSON for comparison.
/// Output is written to the conformance-output/ directory relative to the test project.
/// </summary>
public class DumpSamples
{
    private static readonly string RepoRoot = Path.GetFullPath(
        Path.Combine(AppContext.BaseDirectory, "..", "..", "..", "..", "..", "..", ".."));

    private static readonly string SamplesDir = Path.Combine(RepoRoot, "samples");

    private static readonly string OutputDir = Path.GetFullPath(
        Path.Combine(AppContext.BaseDirectory, "..", "..", "..", "conformance-output"));

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = true,
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
        Converters = { new JsonStringEnumConverter(JsonNamingPolicy.CamelCase) }
    };

    private readonly M3LParser _parser = new();

    private static void EnsureOutputDir()
    {
        if (!Directory.Exists(OutputDir))
            Directory.CreateDirectory(OutputDir);
    }

    private static string ReadSample(string relativePath)
    {
        var path = Path.Combine(SamplesDir, relativePath);
        if (!File.Exists(path))
            throw new FileNotFoundException($"Sample file not found: {path}");
        return File.ReadAllText(path);
    }

    private static void WriteJson(string filename, M3LAst ast)
    {
        EnsureOutputDir();
        var json = JsonSerializer.Serialize(ast, JsonOptions);
        var outputPath = Path.Combine(OutputDir, filename);
        File.WriteAllText(outputPath, json);
        Console.WriteLine($"  Written: {outputPath}");
        Console.WriteLine($"  JSON size: {json.Length:N0} chars");
    }

    private static void PrintSummary(string label, M3LAst ast)
    {
        Console.WriteLine($"\n=== {label} ===");
        Console.WriteLine($"  Parser version: {ast.ParserVersion}");
        Console.WriteLine($"  AST version:    {ast.AstVersion}");
        Console.WriteLine($"  Project:        {ast.Project.Name ?? "(none)"}");
        Console.WriteLine($"  Sources:        {ast.Sources.Count}");
        Console.WriteLine($"  Models:         {ast.Models.Count}");
        Console.WriteLine($"  Enums:          {ast.Enums.Count}");
        Console.WriteLine($"  Interfaces:     {ast.Interfaces.Count}");
        Console.WriteLine($"  Views:          {ast.Views.Count}");
        Console.WriteLine($"  Errors:         {ast.Errors.Count}");
        Console.WriteLine($"  Warnings:       {ast.Warnings.Count}");

        if (ast.Models.Count > 0)
        {
            Console.WriteLine($"  Model names:    {string.Join(", ", ast.Models.Select(m => m.Name))}");
            foreach (var model in ast.Models)
                Console.WriteLine($"    {model.Name}: {model.Fields.Count} fields");
        }

        if (ast.Enums.Count > 0)
            Console.WriteLine($"  Enum names:     {string.Join(", ", ast.Enums.Select(e => e.Name))}");

        if (ast.Interfaces.Count > 0)
            Console.WriteLine($"  Interface names:{string.Join(", ", ast.Interfaces.Select(i => i.Name))}");

        if (ast.Views.Count > 0)
            Console.WriteLine($"  View names:     {string.Join(", ", ast.Views.Select(v => v.Name))}");

        if (ast.Errors.Count > 0)
        {
            Console.WriteLine("  --- Errors ---");
            foreach (var err in ast.Errors)
                Console.WriteLine($"    [{err.Code}] {err.File}:{err.Line} - {err.Message}");
        }

        if (ast.Warnings.Count > 0)
        {
            Console.WriteLine("  --- Warnings ---");
            foreach (var warn in ast.Warnings)
                Console.WriteLine($"    [{warn.Code}] {warn.File}:{warn.Line} - {warn.Message}");
        }
    }

    [Fact]
    public void Dump_01_Ecommerce()
    {
        var content = ReadSample("01-ecommerce.m3l.md");
        var ast = _parser.ParseString(content, "01-ecommerce.m3l.md");

        PrintSummary("01-ecommerce.m3l.md", ast);
        WriteJson("01-ecommerce.json", ast);

        Assert.NotEmpty(ast.Models);
        Assert.True(ast.Models.Count > 0, $"Parsed {ast.Models.Count} models from ecommerce sample");
    }

    [Fact]
    public void Dump_02_BlogCms()
    {
        var content = ReadSample("02-blog-cms.m3l.md");
        var ast = _parser.ParseString(content, "02-blog-cms.m3l.md");

        PrintSummary("02-blog-cms.m3l.md", ast);
        WriteJson("02-blog-cms.json", ast);

        Assert.NotEmpty(ast.Models);
        Assert.True(ast.Models.Count > 0, $"Parsed {ast.Models.Count} models from blog CMS sample");
    }

    [Fact]
    public void Dump_03_TypesShowcase()
    {
        var content = ReadSample("03-types-showcase.m3l.md");
        var ast = _parser.ParseString(content, "03-types-showcase.m3l.md");

        PrintSummary("03-types-showcase.m3l.md", ast);
        WriteJson("03-types-showcase.json", ast);

        Assert.True(ast.Models.Count + ast.Enums.Count + ast.Interfaces.Count > 0,
            "Types showcase should contain models, enums, or interfaces");
    }

    [Fact]
    public async Task Dump_04_MultiInventory()
    {
        // Use ParseAsync with directory path to parse both base.m3l.md and inventory.m3l.md
        var multiDir = Path.Combine(SamplesDir, "multi");
        var ast = await _parser.ParseAsync(multiDir);

        PrintSummary("multi/ (base + inventory)", ast);
        WriteJson("04-multi-inventory.json", ast);

        Assert.True(ast.Sources.Count >= 2, $"Multi-file parse should have >= 2 sources, got {ast.Sources.Count}");
        Assert.NotEmpty(ast.Models);
    }

    [Fact]
    public void OutputDir_Summary()
    {
        // Run after other tests -- print output directory info
        Console.WriteLine($"\nRepo root:   {RepoRoot}");
        Console.WriteLine($"Samples dir: {SamplesDir}");
        Console.WriteLine($"Output dir:  {OutputDir}");

        if (Directory.Exists(OutputDir))
        {
            var files = Directory.GetFiles(OutputDir, "*.json");
            Console.WriteLine($"JSON files in output: {files.Length}");
            foreach (var file in files.OrderBy(f => f))
            {
                var info = new FileInfo(file);
                Console.WriteLine($"  {info.Name} ({info.Length:N0} bytes)");
            }
        }
        else
        {
            Console.WriteLine("Output directory does not exist yet (other tests may not have run).");
        }

        Assert.True(true); // Always passes - informational test
    }
}
