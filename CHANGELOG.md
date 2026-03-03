# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

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
- Formatter duplicates inherited fields on re-format (tracked in claudedocs)

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

[Unreleased]: https://github.com/iyulab/m3l/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/iyulab/m3l/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/iyulab/m3l/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/iyulab/m3l/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/iyulab/m3l/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/iyulab/m3l/releases/tag/v0.1.0
