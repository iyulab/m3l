# @iyulab/m3l

[![npm](https://img.shields.io/npm/v/@iyulab/m3l)](https://www.npmjs.com/package/@iyulab/m3l)
[![TypeScript CI](https://github.com/iyulab/m3l/actions/workflows/parser-publish.yml/badge.svg)](https://github.com/iyulab/m3l/actions/workflows/parser-publish.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](../../LICENSE)

M3L (Meta Model Markup Language) parser and CLI — parse `.m3l.md` / `.m3l` files into a structured JSON AST.

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

### Rollup
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

The parser produces an `M3LAST` object:

```typescript
interface M3LAST {
  parserVersion: string;   // Parser package version (semver)
  astVersion: string;      // AST schema version
  project: { name?: string; version?: string };
  sources: string[];
  models: ModelNode[];
  enums: EnumNode[];
  interfaces: ModelNode[];
  views: ModelNode[];
  attributeRegistry: AttributeRegistryEntry[];
  errors: Diagnostic[];
  warnings: Diagnostic[];
}
```

### Key AST types

```typescript
interface FieldNode {
  name: string;
  label?: string;
  type?: string;
  params?: (string | number)[];
  generic_params?: string[];     // map<K,V> -> ["K", "V"]
  nullable: boolean;
  array: boolean;
  arrayItemNullable: boolean;    // string?[] -> true
  kind: 'stored' | 'computed' | 'lookup' | 'rollup';
  default_value?: string;
  description?: string;
  attributes: FieldAttribute[];
  framework_attrs?: CustomAttribute[];
  lookup?: { path: string };
  rollup?: { target: string; fk: string; aggregate: string; field?: string; where?: string };
  computed?: { expression: string };
  enum_values?: EnumValue[];
  fields?: FieldNode[];          // sub-fields for object type
  loc: SourceLocation;
}

interface FieldAttribute {
  name: string;
  args?: (string | number | boolean)[];
  cascade?: string;
  isStandard?: boolean;          // true for M3L standard attributes
  isRegistered?: boolean;        // true for attributes in the registry
}

interface CustomAttribute {
  content: string;               // e.g. "MaxLength(100)"
  raw: string;                   // e.g. "[MaxLength(100)]"
  parsed?: {                     // structured parse result
    name: string;
    arguments: (string | number | boolean)[];
  };
}

interface AttributeRegistryEntry {
  name: string;
  description?: string;
  target: ('field' | 'model')[];
  type: string;
  range?: [number, number];
  required: boolean;
  defaultValue?: string | number | boolean;
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

| Tier | `isStandard` | `isRegistered` | Example |
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

**Warnings (--strict):**
| Code | Description |
|------|-------------|
| M3L-W001 | Field line exceeds 80 characters |
| M3L-W002 | Object nesting exceeds 3 levels |
| M3L-W004 | Lookup chain exceeds 3 hops |

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

### `STANDARD_ATTRIBUTES: Set<string>`

The set of 27 M3L standard attribute names (e.g. `primary`, `unique`, `reference`, `computed`, ...).

### Lower-level API

```typescript
import { lex, parseTokens, resolve, validate } from '@iyulab/m3l';

const tokens = lex(content, 'file.m3l.md');
const parsed = parseTokens(tokens, 'file.m3l.md');
const ast = resolve([parsed]);
const diagnostics = validate(ast, { strict: true });
```

## Compatibility

This TypeScript parser and the [C# parser](../csharp/) produce equivalent AST structures. Both share the same conformance test suite.

| | TypeScript | C# |
|---|---|---|
| Package | `@iyulab/m3l` | `M3LParser` |
| Runtime | Node.js 20+ | .NET 8.0+ |
| AST Version | 1.0 | 1.0 |

Version is managed centrally via the root `VERSION` file.

## License

MIT
