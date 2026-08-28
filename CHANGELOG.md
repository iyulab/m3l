# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added
- **M3L-W007** — a custom `::attribute` registry entry declares which of `field`/`model` it may
  be used on; using it in a context its `target` doesn't list is now a validator warning, the
  same way a type or range mismatch already was.
- Registry attribute usages on a **model header** (`## Name @attr`) are now validated at all —
  type (M3L-W005), range (M3L-W006), and target (M3L-W007) previously checked only field-level
  usages.
- M3L-W005 now also catches a `boolean`-typed registry attribute given a non-boolean argument;
  previously only `number`/`string` mismatches were checked.

### Documentation
- §10.8.7 documents the `::attribute` custom registry syntax (`target`/`type`/`range`/`required`/
  `default`) and how it interacts with M3L-W005/W006/W007 — previously implemented and tested but
  absent from the specification. §10.5.2's diagnostic catalog now lists W005–W007.

## [0.6.1] - 2026-08-01

### Documentation
- **Specification status markers now state their own scope** (§1.6, new). Every
  `Status:` blockquote reports how much of a section the reference parser and validator
  cover — and nothing else. Code generation and value-level enforcement are outside all
  of them, so a construct can be `Implemented` here and consumed by nothing. §4.7 says
  so on its own line, having been read as a promise that views are generated.
- **§10.4.2 says what each `Expands To` bound is sized against.** For `email`, `url`,
  `money`, and `percentage` it bounds the value as written, so the bound may be adopted
  on its own. `phone` cannot: E.164 is a normalization, and `string(20)` fits `+` plus
  15 digits — not the same number written with separators, a spelled-out prefix, or an
  extension. The section now carries a status blockquote (the parser expands neither
  column) and tabulates which combinations of the two columns stand up.

## [0.6.0] - 2026-07-19

### Added
- **Enum value attributes** — individual enum values may carry `@name` / `@name(args)`
  attributes, matching what fields have always supported (`- legacy: "이관" @system`).
  M3L assigns them no meaning; consumers interpret. Exposed as `EnumValue.attributes`
  in the AST (omitted when absent) and `EnumValue.Attributes` in the C# bindings.
  Specification §3.1.8.
- `ResolveOptions` / `resolve_with()` — `inline_inherited: false` returns an AST that
  mirrors the source document instead of its semantic closure. Used by the formatter.

### Fixed
- Attributes written **after** a label are no longer dropped. `parse_type_and_attrs`
  scanned for attributes only ahead of the description, but `name: "label" @attr` is
  the canonical enum value shape — and reads naturally for fields too.
- Formatter no longer deletes enum value `type`, stored `value`, and attributes, nor
  whole inline enums, on `m3l format`.
- Formatter no longer duplicates inherited fields on every re-format (it emitted the
  `: Parent` header *and* the inlined parent fields). Resolves the long-standing
  Known Issue below; `format_preserves_ast` now runs unignored.
- Multi-line descriptions are emitted as blockquote continuation lines instead of a
  trailing `#` comment, which put a raw newline mid-line and orphaned every line
  after the first.
- Inline enum values no longer lose their label to leftover attribute text
  (`"이관" @system` was stored verbatim as the value).
- `update-version.ps1` now updates intra-workspace Cargo dependency specs. Patch
  bumps inside a minor never tripped them, so the first minor bump broke the build.

### Removed
- `compare-ast.mjs` — referenced `parser/typescript/` and `parser/csharp/` paths that
  no longer exist, and was called from no script or workflow.

### Known Issues
- `format_idempotent` remains ignored: the AST does not retain source spelling for
  attribute argument expressions (`@computed('/users/' + username)`), the
  nullable-array marker (`string?[]`), or default value quoting, so the formatter
  cannot reproduce them. Needs a decision on how much raw text the AST should keep.
- Changelog entries for 0.5.2–0.5.6 were never written; this file jumps 0.5.1 → 0.6.0.

## [0.5.1] - 2026-03-03

### Added
- `@pattern`, `@min_length`, `@max_length` Standard Attributes for string validation
- Lint API exposed through FFI bindings (C ABI, WASM, NAPI) — `lint()` function
- Strongly-typed C# AST models (`M3lAstModels.cs`) with `ParseToAst()`, `ParseMultiToAst()`, `ValidateToResult()`
- 27 TypeScript AST interfaces in `index.d.ts` for full type safety
- 8 new conformance test fixtures (empty file, deep nesting, duplicate fields, etc.) — 14→22
- `m3l.config.yaml` schema documented in specification §5.3
- Implementation status markers on all spec sections (Implemented/Partial/Planned)
- Format idempotency and AST preservation tests (`#[ignore]` pending formatter bug)

### Fixed
- `PARSER_VERSION` constant now auto-syncs with `Cargo.toml` via `env!("CARGO_PKG_VERSION")`

### Known Issues
- Formatter duplicates inherited fields on re-format

## [0.5.0] - 2026-02-27

### Added
- CLI support for `.m3l`, `.m3l.md`, and `.md` file extensions
- Design Principles section in README
- Publish workflows for crates.io, npm, and NuGet (CI)

### Fixed
- Non-namespace H1 lines no longer emit Namespace token
- CI: drop musl target, use macos-latest, add fail-fast:false
- CI: skip already-published crates in publish-crates workflow

## [0.4.0] - 2026-02-15

### Added
- **Rust single-source parser** (`m3l-core`, 4,500+ LOC) replacing legacy TS/C# parsers
- **CLI tool** — `m3l parse`, `m3l validate`, `m3l lint`, `m3l format`, `m3l diff`, `m3l analyze`
- **Lint framework** (`m3l-lint`) with configurable rules
- **5 binding targets**: C ABI cdylib, WASM, Node.js NAPI, C# P/Invoke, TypeScript wrapper
- All 14 diagnostics implemented (E001–E010, W001–W004)
- Shared conformance test suite (`spec/conformance/`, 14 fixtures)
- Multi-file parsing with `@import` and `m3l.config.yaml`

### Removed
- Legacy TypeScript parser (`parser/typescript/`)
- Legacy C# parser (`parser/csharp/`)

## [0.3.0] - 2026-01-20

### Added
- E009 undefined type validation
- E010 relations `@reference` validation
- W003 deprecated syntax warning
- Cross-parser conformance audit (TypeScript/C#)

## [0.2.0] - 2025-12-15

### Added
- Backtick expressions and fenced code blocks
- View SQL blocks
- Field blockquote descriptions
- Attribute registry with `::attribute` type indicator
- Framework attributes with structured parsing

## [0.1.0] - 2025-11-01

### Added
- Initial TypeScript parser (`@iyulab/m3l`)
- Initial C# parser (`M3LParser`)
- Basic lexer/parser/resolver pipeline

[Unreleased]: https://github.com/iyulab/m3l/compare/v0.6.1...HEAD
[0.6.1]: https://github.com/iyulab/m3l/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/iyulab/m3l/compare/v0.5.1...v0.6.0
[0.5.1]: https://github.com/iyulab/m3l/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/iyulab/m3l/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/iyulab/m3l/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/iyulab/m3l/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/iyulab/m3l/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/iyulab/m3l/releases/tag/v0.1.0
