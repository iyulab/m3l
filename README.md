# M3L — Meta Model Markup Language

[![TypeScript CI](https://github.com/iyulab/m3l/actions/workflows/parser-publish.yml/badge.svg)](https://github.com/iyulab/m3l/actions/workflows/parser-publish.yml)
[![C# CI](https://github.com/iyulab/m3l/actions/workflows/parser-csharp.yml/badge.svg)](https://github.com/iyulab/m3l/actions/workflows/parser-csharp.yml)
[![npm](https://img.shields.io/npm/v/@iyulab/m3l)](https://www.npmjs.com/package/@iyulab/m3l)
[![NuGet](https://img.shields.io/nuget/v/M3LParser)](https://www.nuget.org/packages/M3LParser)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Markdown-based data modeling language. Write data models in readable Markdown, parse them into machine-processable AST.

```markdown
## User
- name: string(100) @not_null
- email: string(320)? @unique
- role: enum = "user"
  - admin: "Administrator"
  - user: "Regular User"

# Lookup
- department_name: string @lookup(department_id.name)
```

## What This Repo Provides

This repository is the **specification and parser** for M3L. It provides two things:

1. **Language Specification** — the formal definition of M3L syntax, types, and semantics
2. **Parsers** — libraries that read `.m3l.md` / `.m3l` files and produce a structured AST (Abstract Syntax Tree)

**What this repo does NOT do:** code generation, database migration, UI scaffolding, or any other output. Those are the responsibility of consumer applications that depend on the M3L parser to read the AST and generate their own outputs.

```
.m3l.md files ──> [ M3L Parser ] ──> AST (JSON) ──> [ Your App ]
                   ^^^^^^^^^^^^^^                     ^^^^^^^^^^^^
                   this repo                          your repo
                                                      - DB scripts
                                                      - Model classes
                                                      - API endpoints
                                                      - UI components
                                                      - ...
```

## Features

- **Markdown-native** — valid Markdown, renders in any viewer
- **Models, enums, interfaces, views** — full data modeling support
- **Inheritance** — `## Child : Parent` with field resolution
- **Lookup / Rollup / Computed** fields — derived field definitions
- **Derived views** — with source, where, order_by, joins
- **Attribute Registry** — define custom `@` attributes with `::attribute` type indicator
- **3-tier attribute classification** — standard, registered, unregistered
- **Custom framework attributes** — `` `[FrameworkAttr]` `` with structured parsing
- **Validation** — semantic error/warning diagnostics (M3L-E001~E010)
- **Multi-file** — directory scanning with `@import` and `m3l.config.yaml`

## Parsers

| Package | Language | Registry | Status |
|---------|----------|----------|--------|
| [@iyulab/m3l](https://www.npmjs.com/package/@iyulab/m3l) | TypeScript | npm | [![npm](https://img.shields.io/npm/v/@iyulab/m3l)](https://www.npmjs.com/package/@iyulab/m3l) |
| [M3LParser](https://www.nuget.org/packages/M3LParser) | C# | NuGet | [![NuGet](https://img.shields.io/nuget/v/M3LParser)](https://www.nuget.org/packages/M3LParser) |

Both parsers produce equivalent AST structures and share the same conformance test suite.

## Quick Start

### TypeScript / Node.js

```bash
npm install @iyulab/m3l
```

```typescript
import { parse, parseString, validateFiles } from '@iyulab/m3l';

// Parse .m3l.md files into AST
const ast = await parse('./models');

// Validate with diagnostics
const result = await validateFiles('./models', { strict: true });

// AST is JSON-serializable — pass it to your code generator
console.log(JSON.stringify(ast, null, 2));
```

### C# / .NET

```bash
dotnet add package M3LParser
```

```csharp
using M3L;

var parser = new M3LParser();
var ast = await parser.ParseAsync("./models");

Console.WriteLine($"Models: {ast.Models.Count}");
```

### CLI

```bash
npx m3l parse ./models              # Output AST as JSON
npx m3l validate ./models --strict  # Validate with warnings
```

## Documentation

- [M3L Specification](docs/specification.md) — full language spec (syntax, types, grammar)
- [TypeScript Parser](parser/typescript/README.md) — npm package docs
- [C# Parser](parser/csharp/README.md) — NuGet package docs
- [Examples](samples/) — sample M3L files

## Project Structure

```
docs/
  specification.md          # M3L language specification
  plans/                    # Design & implementation plans
  proposals/                # Spec improvement proposals
samples/                    # Example .m3l.md files
parser/
  typescript/               # @iyulab/m3l (npm)
  csharp/                   # M3LParser (NuGet)
```

## License

MIT
