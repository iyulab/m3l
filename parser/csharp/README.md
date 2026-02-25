# M3LParser

[![NuGet](https://img.shields.io/nuget/v/M3LParser)](https://www.nuget.org/packages/M3LParser)
[![C# CI](https://github.com/iyulab/m3l/actions/workflows/parser-csharp.yml/badge.svg)](https://github.com/iyulab/m3l/actions/workflows/parser-csharp.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](../../LICENSE)

M3L (Meta Model Markup Language) parser for .NET — parse `.m3l.md` / `.m3l` files into a structured AST.

M3L is a Markdown-based data modeling language. You write data models in readable Markdown, and this parser converts them into a machine-processable AST with full validation.

## Install

```bash
dotnet add package M3LParser
```

**Supported runtimes:** .NET 8.0, .NET 9.0, .NET 10.0

## Quick Start

```csharp
using M3L;

var parser = new M3LParser();

// Parse a file or directory
var ast = await parser.ParseAsync("./models");

// Parse a string
var ast2 = parser.ParseString("""
## User
- name: string(100) @not_null
- email: string(320)? @unique

## UserRole ::enum
- admin: "Administrator"
- user: "Regular User"
""");

Console.WriteLine(ast2.Models.Count);  // 1
Console.WriteLine(ast2.Enums.Count);   // 1

// Validate with diagnostics
var (validated, result) = await parser.ValidateAsync("./models");
foreach (var error in result.Errors)
    Console.WriteLine($"[{error.Code}] {error.Message}");
```

## M3L Syntax

```markdown
# Namespace: myapp

## Author
- name(Author Name): string(100) @not_null @searchable
- bio(Biography): text?
- birth_date: date?

### Rollup
- book_count: integer @rollup(BookAuthor.author_id, count)

> Stores information about book authors.

## BookStatus ::enum
- available: "Available"
- borrowed: "Borrowed"
- reserved: "Reserved"

## Book : BaseModel
- title: string(200) @not_null @searchable
- isbn: string(20) @unique
- status: enum = "available"
  - available: "Available"
  - borrowed: "Borrowed"
- publisher_id: identifier @fk(Publisher.id)

### Lookup
- publisher_name: string @lookup(publisher_id.name)

### Computed
- is_available: boolean @computed("status = 'available' AND quantity > 0")

## OverdueLoans ::view @materialized
> Currently overdue book loans.
### Source
- from: Loan
- where: "due_date < now() AND status = 'ongoing'"
- order_by: due_date asc
- borrower_name: string @lookup(member_id.name)
```

**Key syntax elements:**

| Syntax | Meaning |
|--------|---------|
| `# Title` | Document title or namespace (`# Namespace: domain.example`) |
| `## Name` | Model definition |
| `## Name : Parent` | Model with inheritance |
| `## Name ::enum` | Enum definition |
| `## Name ::interface` | Interface definition |
| `## Name ::view` | Derived view |
| `## @name ::attribute` | Attribute registry entry |
| `- field: type` | Field definition |
| `- field: type?` | Nullable field |
| `- field: type[]` | Array field |
| `- field: type?[]` | Array of nullable items |
| `- field: type = val` | Field with default value |
| `@attr` / `@attr(args)` | Attribute (constraint, index, etc.) |
| `` `[FrameworkAttr]` `` | Custom framework attribute |
| `### Lookup` / `### Rollup` / `### Computed` | Kind section for derived fields |
| `### Section` | Named section (Indexes, Relations, Metadata, etc.) |
| `> text` | Model/element description |
| `"text"` | Inline description on field |

## AST Output

The parser produces an `M3LAst` object:

```csharp
public class M3LAst
{
    public string ParserVersion { get; set; }   // Parser package version (semver)
    public string AstVersion { get; set; }      // AST schema version
    public ProjectInfo Project { get; set; }
    public List<string> Sources { get; set; }
    public List<ModelNode> Models { get; set; }
    public List<EnumNode> Enums { get; set; }
    public List<ModelNode> Interfaces { get; set; }
    public List<ModelNode> Views { get; set; }
    public List<AttributeRegistryEntry> AttributeRegistry { get; set; }
    public List<Diagnostic> Errors { get; set; }
    public List<Diagnostic> Warnings { get; set; }
}
```

### Key AST types

```csharp
public class FieldNode
{
    public string Name { get; set; }
    public string? Type { get; set; }
    public List<string>? GenericParams { get; set; }  // map<K,V> -> ["K", "V"]
    public bool Nullable { get; set; }
    public bool Array { get; set; }
    public bool ArrayItemNullable { get; set; }       // string?[] -> true
    public FieldKind Kind { get; set; }               // Stored, Computed, Lookup, Rollup
    public List<FieldAttribute> Attributes { get; set; }
    public List<CustomAttribute>? FrameworkAttrs { get; set; }
    public LookupDef? Lookup { get; set; }
    public RollupDef? Rollup { get; set; }
    public ComputedDef? Computed { get; set; }
    public List<FieldNode>? Fields { get; set; }      // sub-fields for object type
    // ...
}

public class FieldAttribute
{
    public string Name { get; set; }
    public List<object>? Args { get; set; }
    public string? Cascade { get; set; }
    public bool? IsStandard { get; set; }             // true for M3L standard attributes
    public bool? IsRegistered { get; set; }           // true for attributes in the registry
}

public class CustomAttribute
{
    public string Content { get; set; }               // e.g. "MaxLength(100)"
    public string Raw { get; set; }                   // e.g. "[MaxLength(100)]"
    public ParsedCustomAttribute? Parsed { get; set; } // structured parse result
}

public class AttributeRegistryEntry
{
    public string Name { get; set; }
    public string? Description { get; set; }
    public List<string> Target { get; set; }          // "field", "model"
    public string Type { get; set; }                  // "boolean", "integer", "string"
    public List<int>? Range { get; set; }
    public bool Required { get; set; }
    public object? DefaultValue { get; set; }
}
```

## Attribute Registry

Define custom attributes with validation metadata using `::attribute`:

```markdown
## @pii ::attribute
> Personal identifiable information marker
- target: [field]
- type: boolean
- default: false

## @audit_level ::attribute
> Audit compliance level
- target: [field, model]
- type: integer
- range: [1, 5]
- default: 1
```

Attributes are classified into 3 tiers:

| Tier | `IsStandard` | `IsRegistered` | Example |
|------|-------------|----------------|---------|
| Standard | `true` | — | `@primary`, `@unique`, `@reference` |
| Registered | — | `true` | `@pii`, `@audit_level` (defined via `::attribute`) |
| Unregistered | — | — | `@some_unknown_attr` |

## Validation

The validator checks for semantic errors and style warnings:

**Errors:**

| Code | Description |
|------|-------------|
| M3L-E001 | Rollup FK field missing `@reference` |
| M3L-E002 | Lookup FK field missing `@reference` |
| M3L-E004 | View references non-existent model |
| M3L-E005 | Duplicate model/enum name |
| M3L-E006 | Duplicate field name within model |
| M3L-E007 | Unresolved parent in inheritance |

**Warnings (strict mode):**

| Code | Description |
|------|-------------|
| M3L-W001 | Field line exceeds 80 characters |
| M3L-W002 | Object nesting exceeds 3 levels |
| M3L-W004 | Lookup chain exceeds 3 hops |

## Multi-file Projects

Place `.m3l.md` or `.m3l` files in a directory and parse the directory path. The resolver automatically merges all files, resolves inheritance, and detects cross-file references.

```csharp
var parser = new M3LParser();
var ast = await parser.ParseAsync("./models");

// All models, enums, interfaces, views from all files are merged
Console.WriteLine($"Models: {ast.Models.Count}");
Console.WriteLine($"Errors: {ast.Errors.Count}");
```

## API Reference

### High-level API (`M3LParser`)

```csharp
var parser = new M3LParser();

// Parse files from path (file or directory)
M3LAst ast = await parser.ParseAsync(inputPath);

// Parse content string
M3LAst ast = parser.ParseString(content, filename);

// Parse and validate
(M3LAst ast, ValidateResult result) = await parser.ValidateAsync(inputPath);
(M3LAst ast, ValidateResult result) = parser.ValidateString(content);
```

### Lower-level API

```csharp
using M3L;
using M3L.Models;

// Step-by-step parsing
List<Token> tokens = Lexer.Lex(content, "file.m3l.md");
ParsedFile parsed = Parser.ParseTokens(tokens, "file.m3l.md");
M3LAst ast = Resolver.Resolve(new[] { parsed });
ValidateResult result = Validator.Validate(ast, new ValidateOptions { Strict = true });
```

### Version Info

```csharp
string parserVersion = M3LParser.GetParserVersion(); // from VERSION file
string astVersion = M3LParser.GetAstVersion();       // "1.0"
```

## Compatibility

This C# parser is a direct port of the [TypeScript parser](../typescript/) and produces an equivalent AST structure. Both parsers share the same conformance test suite.

| | TypeScript | C# |
|---|---|---|
| Package | `@iyulab/m3l` | `M3LParser` |
| Runtime | Node.js 20+ | .NET 8.0+ |
| AST Version | 1.0 | 1.0 |

Version is managed centrally via the root `VERSION` file.

## License

MIT
