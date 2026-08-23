# Contributing to M3L

Thanks for your interest. M3L is a specification plus one canonical parser (`m3l-core`) that every
binding (CLI, Node.js, C#, WASM) shares — the highest-value contributions are usually spec clarity
and parser correctness, since a bug or ambiguity fixed once in `m3l-core` fixes it everywhere at
once.

## Ground rules

- **Read [`docs/design-principles.md`](./docs/design-principles.md) first.** It defines what is
  fixed. A proposal that conflicts with an anchor needs to argue for changing the anchor (as an
  RFC in [`rfcs/`](./rfcs/)), not route around it.
- **Spec changes are RFC-driven.** Anything that extends or changes `docs/specification.md` — new
  syntax, a new field kind, a semantic change to an existing construct — gets an RFC in
  [`rfcs/`](./rfcs/) before or alongside the implementation PR (see `RFC-0001`/`RFC-0002` for the
  expected shape: Summary, Motivation, the concrete spec diff). Small clarifications to the spec's
  own wording, or parser-internal changes with no spec-visible effect, do not need one — use
  judgment, and when unsure, open an issue first.
- **This repo does not generate code.** It parses `.m3l.md` into an AST and stops there (see the
  design anchor "parser, not producer"). A PR that adds SQL/C#/TypeScript emission, migration
  generation, or any other downstream output is out of scope here — that is a consumer's job
  (`mdd-booster` and similar tools consume `m3l-core`'s AST for exactly this).

## Development

Requires a stable Rust toolchain (see `Cargo.toml`'s `edition`) and, for the Node/WASM bindings
specifically, Node.js — but the core workspace builds without either.

```bash
cargo build                                              # build the core workspace
cargo test --workspace --exclude m3l-wasm --exclude m3l-napi   # unit + conformance tests
cargo clippy --workspace --exclude m3l-wasm --exclude m3l-napi -- -D warnings
cargo fmt --all -- --check
cargo run -p m3l-cli -- parse <file.m3l.md>              # exercise the CLI directly
```

`m3l-wasm` and `m3l-napi` are excluded from the default workspace commands because they require
their own toolchains (`wasm-pack`, Node.js) — CI (`.github/workflows/rust-ci.yml`) runs the same
exclusion, so `cargo test --workspace --exclude m3l-wasm --exclude m3l-napi` is what actually gates
a merge, not the unqualified `cargo test --workspace`.

### Conformance fixtures

`spec/conformance/` (`inputs/` + `expected/` + `spec.json`) is the spec's executable form — each
fixture is a `.m3l.md` input paired with the AST it must produce. A spec change that is not backed
by a new or updated fixture here is not actually pinned; anyone could regress it later without a
test noticing. When `m3l-lint`'s rules change, the equivalent lives in `crates/m3l-lint/tests/`.

### Layout

```
crates/
  m3l-core/    # lexer → parser → resolver → validator; the single canonical implementation
  m3l-cli/     # CLI (parse|validate|lint|format|diff|analyze)
  m3l-lint/    # style/quality rule framework, separate from m3l-core's semantic validation
  m3l-cabi/    # C ABI cdylib — what the C# binding (M3L.Native) P/Invokes into
  m3l-wasm/    # wasm-bindgen build for browser/Node WASM consumption
  m3l-napi/    # napi-rs native addon — what @iyulab/m3l wraps
bindings/
  csharp/      # M3L.Native NuGet package wrapper around m3l-cabi
  typescript/  # @iyulab/m3l npm package wrapper around m3l-napi
spec/
  conformance/ # fixture-driven spec pinning (see above)
docs/
  specification.md      # the formal language spec
  design-principles.md  # the anchors — read this first
rfcs/          # accepted/draft spec-extension proposals
samples/       # example .m3l.md files
```

### A bug fixed once, fixed everywhere

Every binding wraps the same `m3l-core` — there is no per-binding parsing logic to keep in sync.
If you find a parsing bug via the CLI, Node.js, or C#, the fix belongs in `m3l-core` (or the shared
`m3l-cabi` surface it exposes), never duplicated into a binding-specific workaround. A binding-level
patch that papers over a core bug is the one pattern this repo actively avoids — see the design
anchor "one parser, many faces".

## Conduct

Be kind, be direct, assume good faith. Disagreements are settled by argument quality against the
design principles, not by volume or seniority.

## License

Contributions are accepted under the project's MIT license.
