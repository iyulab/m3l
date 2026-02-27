# M3L Roadmap

**Last updated**: 2026-02-27
**Current version**: v0.4.0

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

### v0.4.0 — Rust Core Centralization (current)
- **Rust single-source parser** (`m3l-core`, 4,500+ LOC)
- **CLI tool** (`m3l parse`, `m3l validate`)
- **All 14 diagnostics** — E001-E010, W001-W004
- **5 binding targets**:
  - C ABI cdylib (`m3l-cabi`)
  - WASM (`m3l-wasm` via wasm-bindgen)
  - Node.js NAPI (`m3l-napi` via napi-rs)
  - C# P/Invoke (`bindings/csharp/`, .NET 8.0)
  - TypeScript wrapper (`bindings/typescript/`, re-exports NAPI)
- **Legacy parsers removed** — `parser/typescript/`, `parser/csharp/` deleted
- **Shared conformance suite** — `spec/conformance/` (14 tests)

---

## Next: v0.5.0 — Release & Distribution

| Task | Priority | Status |
|------|----------|--------|
| npm publish `@iyulab/m3l-napi` (platform binaries) | P0 | Pending |
| npm publish `@iyulab/m3l` (wrapper) | P0 | Pending |
| NuGet publish `M3L.Native` + native DLL | P0 | Pending |
| GitHub Actions CI for Rust workspace | P0 | Pending |
| Cross-compile CI (linux-x64, darwin-x64, darwin-arm64) | P1 | Pending |
| `cargo install m3l-cli` (crates.io publish) | P1 | Pending |
| WASM npm publish `@iyulab/m3l-wasm` | P2 | Pending |

## v0.6.0 — Attribute Registry Value Validation

| Task | Priority | Notes |
|------|----------|-------|
| Validate attribute values against registry definitions | P1 | 등록된 속성의 값 범위/타입 검증 |
| Qualified namespace references in inheritance/fields | P2 | `Auth.User` 정규화 참조 해석 |

## v0.7.0+ — Ecosystem Tools (separate projects)

| Project | Role | Prerequisite |
|---------|------|-------------|
| **m3l-lint** | Schema quality linter (custom rule plugins) | m3l-core stable |
| **m3l-language-server** | LSP integration (VS Code) | m3l-core + m3l-lint |
| **m3l-tools** | diff, migrate, analyze, format | m3l-core + user feedback |

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
