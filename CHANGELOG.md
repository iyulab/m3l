# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added
- **Relationship notation is parsed rather than passed on as text.** §3.2.2 and §3.2.4 define
  `>name`, `<name`, `<>name` and the arrow spellings, with an optional cardinality after a colon.
  Until now the parser kept the whole line — direction, colon and cardinality included — so every
  consumer that wanted any part of it had to parse that string again, and one of them did. Each
  entry of `sections.relations` now carries `direction`, plus `name` (short spellings) or `target`
  (arrow spellings), plus `cardinality` when a colon supplied one. `raw` is unchanged and still
  holds the source line, so nothing that reads it has to change at once. What follows the colon is
  carried through as written — the specification does not close that set, and policing it here
  would make the parser refuse a value the language later allows.

### Fixed
- **The notation no longer becomes a field when it is written among a model's fields.** The line
  arrived as a field whose *name* was the entire source line, with no type and classified as
  stored — usable neither as a relationship nor as a field. It is now read as a relationship
  wherever it appears, and reported as `M3L-W009` because `### Relations` is where §3.2.3 places
  it. A warning rather than an error: the meaning is unambiguous, and refusing the document would
  buy nothing. Only a line that begins with the notation is affected; the check keys on those
  leading characters alone, so ordinary field lines are untouched.

## [0.7.2] - 2026-09-06

### Fixed
- **An unquoted attribute argument containing a colon is no longer rewritten.** The tokenizer
  treated any such argument as a `key: value` pair and rebuilt it as `"{key}: {value}"`, so a URI
  scheme lost its separator — §5.2's own example, `@reference(external://taxonomy.Category)`,
  reached the AST as `external: //taxonomy.Category`, and a clock time like `23:59:59` came out as
  `23: 59:59`. The value now arrives spelled the way the document spelled it. Reading a
  `key: value` shape out of an argument belongs to whichever attribute defines one, and the only
  attribute that does — `@computed_raw`'s `platform:` — already reads the raw spelling through a
  pattern that tolerates any spacing or quoting, so it is unaffected. Quoting the argument was a
  workaround; it is no longer needed, and both spellings now yield the same value.
- **Unwrapping a quoted value no longer eats quote characters that belong to it.** Five places
  stripped the wrapping delimiters by character *set*, so the trim kept going once the outer
  delimiter was gone: `@computed_raw("metadata->>'category'", platform: "postgresql")` reached the
  AST as `metadata->>'category` — a SQL expression missing its closing quote — and the same
  happened to `@computed` expressions, metadata strings, nested values, and extended-format
  descriptions ending in `'` or `"`. Exactly one matched pair is removed now; a quote character at
  either end that has no partner is part of the value.

## [0.7.1] - 2026-09-03

### Fixed
- **`@rollup`'s documented `where:` clause (§4.6.5, Conditional Rollup) is now actually captured.**
  The generic attribute-argument tokenizer already strips a string argument's enclosing quotes
  before `RollupDef` gets built from it, but the `where:` extractor's regex still required them —
  so `where_clause` silently stayed `None` for every rollup filter ever written, regardless of
  syntax. `m3l parse`/`validate` accepted the documented syntax without error the whole time,
  which is what made this easy to miss downstream.

## [0.7.0] - 2026-08-31

### Changed
- **`@visibility(level)` removed from the standard attribute catalog.** §10.8.3, §2.2.4, and
  §2.5.4's example gave it three mutually contradictory definitions (target, value domain, and
  spelling all disagreed) and no real `.m3l.md` document in this workspace ever used it —
  `@public`/`@private` (model-level, already documented in §2.2.4) are the only visibility
  attributes M3L defines. `@visibility(...)` still parses as a custom/extension attribute; it
  just no longer reports `is_standard: true` in the AST. §10.8.3 now also lists `@public`/
  `@private` directly, closing a separate gap where the catalog omitted them entirely.

- **`FieldAttribute.argsQuoted` and `FieldNode.defaultValueQuoted`/`defaultValueBacktick`
  (new, optional AST fields).** `AttrArgValue` is `#[serde(untagged)]`, so `@reference("Category")`
  and `@reference(Category)` previously parsed to the identical bare string — nothing in the AST
  recorded whether the source had quoted it. `argsQuoted` carries that per-argument, and the two
  `defaultValue*` flags do the same for a field's `= value`: `defaultValueQuoted` for a `"..."`
  literal, `defaultValueBacktick` for a `` `...` `` expression. All three are additive and absent
  by default (omitted from JSON when not applicable), so existing consumers are unaffected.

### Fixed
- **`m3l format` no longer mispositions or drops an array's nullable markers.** `Type?[]?` has
  two independent markers — a leading `?` for item-nullable, a trailing `?` (after `[]`) for
  array-nullable — and the formatter previously only ever emitted the leading position, so
  `string[]?` (array-nullable) came back as `string?[]` (item-nullable, a different field) and
  `string?[]` lost its marker outright on a second format pass.
- **`m3l format` now reproduces quoted attribute arguments and quoted/backtick default values
  on a second pass instead of silently dropping their delimiters.** Previously every string
  argument and every default value was re-emitted bare, so `@reference("Category")` came back
  unquoted, and a backtick-wrapped expression default (`` = `price * qty` ``) lost its backticks
  outright — which, once re-parsed as a bareword, truncated the default at the first non-word
  character (`= price`) instead of staying `price * qty`. The formatter now consults the new
  `argsQuoted`/`defaultValueQuoted`/`defaultValueBacktick` AST fields to restore each one's
  original delimiter. `m3l format`'s round-trip is now idempotent — `format(input) ==
  format(format(input))` — end to end.

### Added
- **M3L-W008** — a custom `::attribute` registry entry marked `required: true` now warns when
  it's used without an explicit argument (e.g. bare `@priority` rather than `@priority(5)`).
  Settles the semantics §10.8.7 previously left open: `required` means an explicit argument is
  mandatory at each usage, not that every matching field/model must carry the attribute —
  `default` remains the value assumed for a *non*-required attribute used bare.
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
  `default`) and how it interacts with M3L-W005/W006/W007/W008 — previously implemented and
  tested but absent from the specification. §10.5.2's diagnostic catalog now lists W005–W008.

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

[Unreleased]: https://github.com/iyulab/m3l/compare/v0.7.2...HEAD
[0.7.2]: https://github.com/iyulab/m3l/compare/v0.7.1...v0.7.2
[0.7.1]: https://github.com/iyulab/m3l/compare/v0.7.0...v0.7.1
[0.7.0]: https://github.com/iyulab/m3l/compare/v0.6.1...v0.7.0
[0.6.1]: https://github.com/iyulab/m3l/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/iyulab/m3l/compare/v0.5.1...v0.6.0
[0.5.1]: https://github.com/iyulab/m3l/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/iyulab/m3l/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/iyulab/m3l/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/iyulab/m3l/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/iyulab/m3l/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/iyulab/m3l/releases/tag/v0.1.0
