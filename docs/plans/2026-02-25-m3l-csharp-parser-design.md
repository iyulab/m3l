# M3L C# Parser Design

## Overview

Port the TypeScript M3L parser to C# as a NuGet package, following iyulab .NET conventions. The C# parser provides the same pipeline (Lexer → Parser → Resolver → Validator) and produces an identical AST structure.

## Project Structure

```
parser/csharp/
  M3L.sln
  src/
    M3L/                        # Library (NuGet: M3L)
      Lexer.cs                  # Markdown → Token[]
      Parser.cs                 # Token[] → ParsedFile
      Resolver.cs               # ParsedFile[] → M3LAst (merge + inheritance)
      Validator.cs              # M3LAst → Diagnostic[] (semantic checks)
      M3LParser.cs              # High-level public API
      FileReader.cs             # File/directory scanning (.m3l.md, .m3l)
      Models/
        Token.cs                # Token, TokenType
        Ast.cs                  # M3LAst, ParsedFile, ProjectInfo
        FieldNode.cs            # FieldNode, FieldAttribute, FieldKind
        ModelNode.cs            # ModelNode, ViewSourceDef, SectionData
        EnumNode.cs             # EnumNode, EnumValue
        Diagnostic.cs           # Diagnostic, DiagnosticSeverity
        ValidateOptions.cs      # ValidateOptions, ValidateResult
    M3L.Cli/                    # CLI tool (dotnet tool: m3l)
      Program.cs                # Entry point
      ParseCommand.cs           # m3l parse [path] [-o file]
      ValidateCommand.cs        # m3l validate [path] [--strict] [--format json]
  tests/
    M3L.Tests/                  # xUnit tests
      LexerTests.cs
      ParserTests.cs
      ResolverTests.cs
      ValidatorTests.cs
      CliTests.cs
      IntegrationTests.cs
      Fixtures/
        simple.m3l.md
        multi/a.m3l.md
        multi/b.m3l.md
        mdd/mes.m3l
        mdd/raybox.m3l
```

## Tech Stack

| Item | Choice | Rationale |
|------|--------|-----------|
| Target | net10.0 | iyulab standard |
| NuGet package | M3L | iyulab naming (no prefix) |
| CLI framework | System.CommandLine | iyulab standard |
| Console output | Spectre.Console | iyulab standard |
| Test framework | xUnit + FluentAssertions | iyulab standard |
| JSON serialization | System.Text.Json | Built-in, no extra dependency |
| File globbing | Microsoft.Extensions.FileSystemGlobbing | Standard library |
| YAML config | YamlDotNet | .NET YAML standard |
| Code style | Nullable enabled, TreatWarningsAsErrors | iyulab standard |

## Public API

```csharp
namespace M3L;

/// High-level parsing API
public static class M3LParser
{
    /// Parse M3L files from a file or directory path
    public static Task<M3LAst> ParseAsync(string path);

    /// Parse M3L content string
    public static M3LAst ParseString(string content, string filename = "inline.m3l.md");

    /// Parse and validate M3L files
    public static Task<ValidateResult> ValidateAsync(
        string path, ValidateOptions? options = null);
}

/// Lower-level API access
public static class Lexer
{
    public static List<Token> Lex(string content, string file);
}

public static class Parser
{
    public static ParsedFile Parse(List<Token> tokens, string file);
    public static ParsedFile ParseString(string content, string file);
}

public static class Resolver
{
    public static M3LAst Resolve(List<ParsedFile> files, ProjectInfo? project = null);
}

public static class Validator
{
    public static ValidateResult Validate(M3LAst ast, ValidateOptions? options = null);
}
```

## Pipeline

Identical to TypeScript implementation:

```
.m3l.md files
     │
     ▼
  Lexer.Lex()          ← Regex-based line tokenizer
     │                    Handles: H1/H2/H3, fields, blockquotes,
     │                    nested items, directives, @import
     ▼
  Parser.Parse()       ← State machine: tokens → single-file AST
     │                    Tracks: currentElement, currentSection,
     │                    currentKind (stored/lookup/rollup/computed)
     ▼
  Resolver.Resolve()   ← Multi-file merge
     │                    Inheritance resolution, duplicate detection (E005),
     │                    unresolved reference detection (E007)
     ▼
  Validator.Validate() ← Semantic validation
     │                    E001-E007 errors, W001-W004 warnings (strict)
     ▼
  M3LAst               ← Final output (JSON-serializable)
```

## Key Implementation Notes

### Lexer
- Same regex patterns as TypeScript
- Positional scanner for `parseTypeAndAttrs` with balanced paren support
- `findBalancedParen()` and `findClosingQuote()` helpers
- Special case for enum values: `NAME "description"` (no colon)

### Parser
- State machine with `ParserState` tracking current element/section/kind
- Handles: models, enums, interfaces, views, sections, fields, nested items
- Rollup args parsing with `splitRollupArgs()` (balanced comma splitting)
- Source directives for views (from, where, order_by, group_by, join)

### Resolver
- Merge multiple ParsedFile into single M3LAst
- Inheritance: copy parent fields to child models
- Detect duplicate names (E005) and unresolved parents (E007)

### Validator
- E001: Rollup FK missing @reference
- E002: Lookup FK missing @reference
- E004: View references non-existent model
- E005: Duplicate model/enum name
- E006: Duplicate field within model
- E007: Unresolved parent in inheritance
- W001-W004: Strict mode style warnings

## CLI Commands

```bash
# Parse and output JSON AST
m3l parse ./models
m3l parse ./models/user.m3l.md -o ast.json

# Validate
m3l validate ./models
m3l validate ./models --strict --format json
```

## CI/CD

GitHub Actions workflow at `.github/workflows/parser-csharp-publish.yml`:
- Trigger: `parser/csharp/**` changes on main
- Jobs: build → test → pack → publish to NuGet
- Secret: `NUGET_API_KEY`

## Test Coverage

Mirror TypeScript test suite:
- Lexer: 26+ tests (token types, field parsing, balanced parens)
- Parser: 19+ tests (models, enums, views, sections, directives)
- Resolver: 8+ tests (merge, inheritance, duplicates)
- Validator: 10+ tests (E001-E007, W001-W004)
- CLI: 5+ tests (parse/validate commands)
- Integration: 3+ tests (full library sample)
- MDD: 45+ tests (real-world MES data)
