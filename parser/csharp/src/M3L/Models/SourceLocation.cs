namespace M3L.Models;

/// <summary>Source location for error reporting.</summary>
public record SourceLocation(string File, int Line, int Col);
