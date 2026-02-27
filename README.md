# M3L — Meta Model Markup Language

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Markdown-based data modeling language. Write data models in readable Markdown, parse them into machine-processable AST.

```markdown
## User
- name: string(100) @not_null
- email: string(320)? @unique
  > Primary contact email.
  > Used for login and account recovery.
- role: enum = "user"
  - admin: "Administrator"
  - user: "Regular User"
- created_at: timestamp = `now()`

# Lookup
- department_name: string @lookup(department_id.name)

# Computed
- display_name: string @computed(`name + ' <' + email + '>'`)
```

## What This Repo Provides

This repository is the **specification and parser** for M3L. It provides:

1. **Language Specification** — the formal definition of M3L syntax, types, and semantics
2. **Rust Core Parser** — single canonical implementation (`m3l-core`)
3. **CLI Tool** — `m3l parse`, `m3l validate`, `m3l lint`, `m3l format`, `m3l diff`, `m3l analyze`
4. **Lint Framework** — configurable style & quality rules (`m3l-lint`)
5. **Language Bindings** — Node.js (NAPI), C# (P/Invoke), WASM

**What this repo does NOT do:** code generation, database migration, UI scaffolding, or any other output. Those are the responsibility of consumer applications that depend on the M3L parser to read the AST.

```
.m3l.md files ──> [ M3L Parser ] ──> AST (JSON) ──> [ Your App ]
                   ^^^^^^^^^^^^^^                     ^^^^^^^^^^^^
                   this repo                          your repo
```

## Features

- **Markdown-native** — valid Markdown, renders in any viewer
- **Models, enums, interfaces, views** — full data modeling support
- **Inheritance** — `## Child : Parent` with field resolution
- **Lookup / Rollup / Computed** fields — derived field definitions
- **Backtick expressions** — `` @computed(`expr`) ``, `` = `now()` `` for unambiguous expressions
- **Multi-line computed** — fenced code blocks for complex expressions (CASE, multi-line SQL)
- **Derived views** — with source, where, order_by, joins, or raw SQL code blocks
- **Field blockquote descriptions** — `  > multi-line docs` attached to fields
- **Attribute Registry** — define custom `@` attributes with `::attribute` type indicator
- **3-tier attribute classification** — standard, registered, unregistered
- **Custom framework attributes** — `` `[FrameworkAttr]` `` with structured parsing
- **Validation** — 14 semantic diagnostics (M3L-E001~E010, W001~W004)
- **Multi-file** — directory scanning with `@import` and `m3l.config.yaml`

## Bindings

All bindings share the same Rust core parser (`m3l-core`), ensuring identical behavior.

| Binding | Platform | Package |
|---------|----------|---------|
| CLI | Any (native binary) | `m3l parse`, `validate`, `lint`, `format`, `diff`, `analyze` |
| Node.js | NAPI native addon | `@iyulab/m3l` |
| C# | P/Invoke (.NET 8.0+) | `M3L.Native` |
| WASM | Browser / Node.js | `@iyulab/m3l-wasm` |

## Quick Start

### CLI

```bash
cargo install m3l-cli

m3l parse ./models                  # Output AST as JSON
m3l validate ./models --strict      # Validate with diagnostics
m3l validate ./models --format json # Machine-readable output
m3l lint ./models                   # Style & quality checks
m3l lint ./models --format sarif    # SARIF 2.1.0 output (GitHub Code Scanning)
m3l format ./models                 # Standardize M3L formatting
m3l diff old.m3l.md new.m3l.md      # Compare two schemas
m3l analyze ./models                # Dependency graph (Mermaid)
m3l analyze ./models --format dot   # Dependency graph (DOT/Graphviz)
```

### Node.js

```bash
npm install @iyulab/m3l
```

```javascript
const { parse, validate } = require('@iyulab/m3l');

const result = JSON.parse(parse('## User\n- name: string', 'user.m3l.md'));
console.log(result.data.models[0].name); // "User"

const diag = JSON.parse(validate('## User\n- name: unknown_type', '{}'));
console.log(diag.data.errors); // [{ code: "M3L-E009", ... }]
```

### C# / .NET

```bash
dotnet add package M3L.Native
```

```csharp
using M3L.Native;

var json = M3lNative.Parse("## User\n- name: string", "user.m3l.md");
var result = M3lNative.ParseTyped("## User\n- name: string", "user.m3l.md");
Console.WriteLine(result?.Success); // True
```

## Documentation

- [M3L Specification](docs/specification.md) — full language spec (syntax, types, grammar)
- [Examples](samples/) — sample M3L files

## Project Structure

```
crates/
  m3l-core/           # Rust core parser (lexer → parser → resolver → validator)
  m3l-cli/            # CLI tool (parse, validate, lint, format, diff, analyze)
  m3l-lint/           # Lint framework (naming, model-size, similar-fields, relation-complexity)
  m3l-cabi/           # C ABI cdylib (for P/Invoke / ctypes)
  m3l-wasm/           # WASM (wasm-bindgen)
  m3l-napi/           # Node.js native addon (napi-rs)
bindings/
  typescript/         # @iyulab/m3l npm package wrapper
  csharp/             # M3L.Native NuGet package (P/Invoke)
spec/
  conformance/        # Shared test fixtures (inputs + expected JSON)
docs/
  specification.md    # M3L language specification
samples/              # Example .m3l.md files
```

## License

MIT
