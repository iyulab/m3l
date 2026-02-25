# Iyulab.M3L

M3L (Meta Model Markup Language) parser for .NET — parse `.m3l.md` files into a structured AST.

M3L is a Markdown-based data modeling language. You write data models in readable Markdown, and this parser converts them into a machine-processable AST with full validation.

## Install

```bash
dotnet add package Iyulab.M3L
```

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

# Rollup
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

# Lookup
- publisher_name: string @lookup(publisher_id.name)

# Computed
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
| `- field: type` | Field definition |
| `- field: type?` | Nullable field |
| `- field: type[]` | Array field |
| `- field: type = val` | Field with default value |
| `@attr` / `@attr(args)` | Attribute (constraint, index, etc.) |
| `# Lookup` / `# Rollup` / `# Computed` | Kind section for derived fields |
| `### Section` | Named section (Indexes, Relations, Metadata, etc.) |
| `> text` | Model/element description |
| `"text"` | Inline description on field |

## AST Output

The parser produces an `M3LAst` object:

```csharp
public class M3LAst
{
    public string ParserVersion { get; set; }
    public string AstVersion { get; set; }
    public ProjectInfo Project { get; set; }
    public List<string> Sources { get; set; }
    public List<ModelNode> Models { get; set; }
    public List<EnumNode> Enums { get; set; }
    public List<ModelNode> Interfaces { get; set; }
    public List<ModelNode> Views { get; set; }
    public List<Diagnostic> Errors { get; set; }
    public List<Diagnostic> Warnings { get; set; }
}
```

Each `ModelNode` contains fields, sections (indexes, relations, metadata), inheritance info, and source locations for error reporting.

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

Place `.m3l.md` files in a directory and parse the directory path. The resolver automatically merges all files, resolves inheritance, and detects cross-file references.

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
string parserVersion = M3LParser.GetParserVersion(); // "0.1.0"
string astVersion = M3LParser.GetAstVersion();       // "1.0"
```

## Compatibility

This C# parser is a direct port of the [TypeScript parser](../typescript/) and produces an equivalent AST structure. Both parsers share the same conformance test suite.

| | TypeScript | C# |
|---|---|---|
| Package | `@iyulab/m3l` | `Iyulab.M3L` |
| Runtime | Node.js 18+ | .NET 10+ |
| AST Version | 1.0 | 1.0 |
| Parser Version | 0.1.0 | 0.1.0 |

## License

MIT
