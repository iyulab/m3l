using M3L.Models;

namespace M3L;

/// <summary>
/// Reads .m3l.md and .m3l files from the file system.
/// </summary>
public static class FileReader
{
    /// <summary>
    /// Read M3L files from a path (file or directory).
    /// If path is a directory, scans for **/*.m3l.md and **/*.m3l files.
    /// </summary>
    public static async Task<List<M3LFile>> ReadM3LFiles(string inputPath)
    {
        var fullPath = Path.GetFullPath(inputPath);

        if (File.Exists(fullPath))
            return [ReadSingleFile(fullPath)];

        if (Directory.Exists(fullPath))
            return await ScanDirectory(fullPath);

        throw new FileNotFoundException($"Path is neither a file nor a directory: {fullPath}");
    }

    /// <summary>
    /// Wrap a string content as an M3LFile.
    /// </summary>
    public static M3LFile ReadM3LString(string content, string filename = "inline.m3l.md")
        => new() { Path = filename, Content = content };

    private static M3LFile ReadSingleFile(string filePath)
        => new() { Path = filePath, Content = File.ReadAllText(filePath) };

    private static Task<List<M3LFile>> ScanDirectory(string dirPath)
    {
        var patterns = new[] { "*.m3l.md", "*.m3l" };
        var files = new List<string>();

        foreach (var pattern in patterns)
        {
            files.AddRange(Directory.GetFiles(dirPath, pattern, SearchOption.AllDirectories));
        }

        var result = files
            .Distinct()
            .OrderBy(f => f)
            .Select(f => ReadSingleFile(f))
            .ToList();

        return Task.FromResult(result);
    }
}

/// <summary>
/// Represents a loaded M3L source file.
/// </summary>
public class M3LFile
{
    public string Path { get; set; } = "";
    public string Content { get; set; } = "";
}
