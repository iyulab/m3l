# M3L Roadmap

**Last updated**: 2026-03-03
**Current version**: v0.5.0

---

## Completed

### v0.1.0 — Initial Parsers
- TypeScript parser (`@iyulab/m3l`)
- C# parser (`M3LParser`)
- Basic lexer/parser/resolver pipeline

### v0.2.0 — Syntax Enhancements
- Backtick expressions, fenced code blocks
- View SQL blocks, field blockquote descriptions
- Attribute registry, framework attributes

### v0.3.0 — Validation Diagnostics
- E009 undefined type, E010 relations @reference
- W003 deprecated syntax
- Cross-parser conformance audit (TS/C#)

### v0.4.0 — Rust Core Centralization
- **Rust single-source parser** (`m3l-core`, 4,500+ LOC)
- **CLI tool** (`m3l parse`, `m3l validate`)
- **All 14 diagnostics** — E001-E010, W001-W004
- **5 binding targets**: C ABI, WASM, NAPI, C# P/Invoke, TypeScript wrapper
- **Legacy parsers removed** — `parser/typescript/`, `parser/csharp/` deleted
- **Shared conformance suite** — `spec/conformance/` (14 tests)

### v0.5.0 — Dogfooding & DX Improvements (current)
- **PARSER_VERSION** auto-synced with `Cargo.toml` via `env!()` macro
- **Standard Attributes** — `@pattern`, `@min_length`, `@max_length` added
- **Lint FFI** — `lint()` exposed through C ABI, WASM, NAPI bindings
- **C# strongly-typed AST** — `M3lAstModels.cs` with `ParseToAst()`, `ParseMultiToAst()`
- **TypeScript AST types** — 27 interfaces in `index.d.ts`
- **CLI** — `.m3l`, `.m3l.md`, `.md` file extensions supported
- **CI workflows** — `rust-ci.yml`, `publish-crates.yml`, `publish-npm.yml`, `publish-nuget.yml`, `publish-wasm.yml`

---

## Pending: Release & Distribution

CI workflows are ready. Publish requires configuring GitHub repository secrets.

| Task | Priority | Status | Required Secret |
|------|----------|--------|-----------------|
| GitHub Actions CI for Rust workspace | P0 | **Done** | — |
| Cross-compile CI (linux-x64, darwin-x64, darwin-arm64) | P0 | **Done** | — |
| `cargo publish` m3l-core, m3l-lint, m3l-cli | P0 | Ready | `CARGO_REGISTRY_TOKEN` |
| npm publish `@iyulab/m3l-napi` (platform binaries) | P0 | Ready | `NPM_TOKEN` |
| npm publish `@iyulab/m3l` (TS wrapper) | P0 | Ready | `NPM_TOKEN` |
| NuGet publish `M3L.Native` + native DLL | P0 | Ready | `NUGET_API_KEY` |
| WASM npm publish `@iyulab/m3l-wasm` | P2 | Ready | `NPM_TOKEN` |

**To trigger publish**: Push a VERSION file change to main, or use `workflow_dispatch` in GitHub Actions UI.

## v0.6.0 — Attribute Registry Value Validation

| Task | Priority | Notes |
|------|----------|-------|
| Validate attribute values against registry definitions | P1 | 등록된 속성의 값 범위/타입 검증 |
| Qualified namespace references in inheritance/fields | P2 | `Auth.User` 정규화 참조 해석 |

## v0.7.0+ — Ecosystem Tools

| Project | Role | Status |
|---------|------|--------|
| **m3l-lint** | Schema quality linter | **Done** (4 rules, FFI exposed) |
| **m3l-language-server** | LSP integration (VS Code) | Deferred |
| **TextMate grammar** | VS Code syntax highlighting | Deferred |
| **Benchmarks** | criterion perf data | Deferred |

See: `claudedocs/issues/ISSUE-m3l-20260227-ecosystem-tools-roadmap.md`

---

## Architecture

```
crates/
  m3l-core/           Rust core (lexer → parser → resolver → validator → ffi)
  m3l-cli/            CLI tool (parse / validate)
  m3l-cabi/           C ABI cdylib (extern "C" entry points)
  m3l-wasm/           WASM (wasm-bindgen)
  m3l-napi/           Node.js NAPI (napi-rs)
bindings/
  typescript/         @iyulab/m3l npm wrapper
  csharp/             M3L.Native NuGet (P/Invoke)
spec/
  conformance/        Shared test fixtures
```

## Diagnostics (14/14 implemented)

| Code | Severity | Description |
|------|----------|-------------|
| E001 | Error | Rollup FK missing @reference |
| E002 | Error | Lookup FK missing @reference |
| E003 | Error | Circular import detected |
| E004 | Error | View references non-existent model |
| E005 | Error | Duplicate model/enum name |
| E006 | Error | Duplicate field in model |
| E007 | Error | Unresolved parent in inheritance |
| E008 | Error | Ambiguous reference across namespaces |
| E009 | Error | Undefined type |
| E010 | Error | Relations without @reference |
| W001 | Warning | Field line exceeds 80 chars (strict) |
| W002 | Warning | Object nesting exceeds 3 levels (strict) |
| W003 | Warning | Deprecated syntax |
| W004 | Warning | Lookup chain exceeds 3 hops (strict) |
