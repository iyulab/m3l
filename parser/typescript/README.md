# @iyulab/m3l

M3L (Meta Model Markup Language) parser and CLI — parse `.m3l.md` files into a structured JSON AST.

M3L is a Markdown-based data modeling language. You write data models in readable Markdown, and this parser converts them into a machine-processable AST with full validation.

## Install

```bash
npm install @iyulab/m3l
```

## Quick Start

### As a Library

```typescript
import { parse, parseString, validateFiles } from '@iyulab/m3l';

// Parse a file or directory
const ast = await parse('./models');

// Parse a string
const ast2 = parseString(`
## User
- name: string(100) @not_null
- email: string(320)? @unique

## UserRole ::enum
- admin: "Administrator"
- user: "Regular User"
`);

console.log(ast2.models);  // [{ name: 'User', fields: [...], ... }]
console.log(ast2.enums);   // [{ name: 'UserRole', values: [...], ... }]

// Validate with diagnostics
const { ast: validated, errors, warnings } = await validateFiles('./models');
```

### As a CLI

```bash
# Parse and output JSON AST
npx m3l parse ./models

# Parse a single file
npx m3l parse ./models/user.m3l.md -o ast.json

# Validate
npx m3l validate ./models

# Validate with strict style checks and JSON output
npx m3l validate ./models --strict --format json
```

## M3L Syntax

```markdown
# Library System

## Author
- name(Author Name): string(100) @not_null @idx
- bio(Biography): text?
- birth_date: date?

# Rollup
- book_count: integer @rollup(BookAuthor.author_id, count)

> Stores information about book authors.

## BookStatus ::enum "Status of a book"
- available: "Available"
- borrowed: "Borrowed"
- reserved: "Reserved"

## Book : BaseModel
- title: string(200) @not_null @idx
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

The parser produces an `M3LAST` object:

```typescript
interface M3LAST {
  project: { name?: string; version?: string };
  sources: string[];         // parsed file paths
  models: ModelNode[];       // models and interfaces
  enums: EnumNode[];         // enum definitions
  interfaces: ModelNode[];   // interface definitions
  views: ModelNode[];        // derived views
  errors: Diagnostic[];      // parse/resolve errors
  warnings: Diagnostic[];    // validation warnings
}
```

Each `ModelNode` contains fields, sections (indexes, relations, metadata), inheritance info, and source locations for error reporting.

## Validation

The validator checks for semantic errors and style warnings:

**Errors:**
| Code | Description |
|------|-------------|
| E001 | Rollup FK field missing `@reference` |
| E002 | Lookup FK field missing `@reference` |
| E004 | View references non-existent model |
| E005 | Duplicate model/enum name |
| E006 | Duplicate field name within model |
| E007 | Unresolved parent in inheritance |

**Warnings (--strict):**
| Code | Description |
|------|-------------|
| W001 | Model has no fields |
| W002 | Model has no description |
| W003 | Field missing label |
| W004 | Enum has no values |

## Multi-file Projects

Place `.m3l.md` or `.m3l` files in a directory and parse the directory path. The resolver automatically merges all files, resolves inheritance, and detects cross-file references.

Optionally, create an `m3l.config.yaml`:

```yaml
name: my-project
version: 1.0.0
sources:
  - "models/**/*.m3l.md"
  - "shared/*.m3l"
```

## API Reference

### `parse(inputPath: string): Promise<M3LAST>`

Parse M3L files from a file or directory into a merged AST.

### `parseString(content: string, filename?: string): M3LAST`

Parse an M3L content string into an AST.

### `validateFiles(inputPath: string, options?): Promise<{ ast, errors, warnings }>`

Parse and validate M3L files. Options: `{ strict?: boolean }`.

### Lower-level API

```typescript
import { lex, parseTokens, resolve, validate } from '@iyulab/m3l';

const tokens = lex(content, 'file.m3l.md');
const parsed = parseTokens(tokens, 'file.m3l.md');
const ast = resolve([parsed]);
const diagnostics = validate(ast, { strict: true });
```

## License

MIT
