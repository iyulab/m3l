# Design Principles

These are the anchors of M3L. They are deliberately few, and deliberately stable: syntax details and
implementation choices may change freely, but a change to this document is a change to what the
project *is* and belongs in an RFC (see `rfcs/`), not a routine PR.

Everything not fixed here is an open implementation decision, to be settled when the code forces the
question — not before.

---

## 1. M3L is Markdown, not a language that borrows Markdown's clothes

A `.m3l.md` file is valid Markdown first. Headings are headings, lists are lists, blockquotes are
blockquotes — M3L layers schema semantics on top of standard Markdown constructs, it does not invent
a new grammar that happens to be embeddable in a `.md` file extension.

Consequences:

- Every M3L file renders as meaningful documentation in GitHub, VS Code, Obsidian, or any other
  Markdown viewer, with zero special tooling. The model *is* its own readable spec.
- If a construct looks like standard Markdown, it must behave like standard Markdown — a
  Markdown-literate reader's expectation is the contract, not an implementation detail to violate for
  parser convenience.
- Unrecognized Markdown is tolerated, not corrupted. The parser is a consumer of Markdown semantics,
  never their owner.

Anti-goal: M3L will not grow syntax that only makes sense inside a specialized editor or that breaks
rendering in a generic Markdown viewer. `RFC-0002` (Markdown Viewer Compatibility) exists because a
prior direction leaned toward the opposite and was pulled back.

## 2. Parser, not producer

This repository draws one hard boundary: `.m3l.md` in, AST (JSON) out. It does not generate SQL, C#,
TypeScript, migrations, or any other downstream artifact.

```
.m3l.md files ──> [ M3L Parser ] ──> AST (JSON) ──> [ Your App ]
                   ^^^^^^^^^^^^^^                     ^^^^^^^^^^^^
                   this repo                          your repo
```

Consequences:

- A consumer (`mdd-booster` and similar tools) owns everything downstream of the AST — what gets
  generated, in what shape, for which target. M3L never needs to know those consumers exist, and a
  change here is never justified by "because a specific downstream tool wants it" — see the anchor
  below on layering.
- The spec and the parser can evolve on their own release cadence, unblocked by any single consumer's
  roadmap.

Anti-goal: M3L will not become a code generator, a migration tool, or a modeling *platform*. If a
proposal's real payload is "and then it emits X", it belongs in a consumer, not here.

## 3. One parser, many faces

`m3l-core` is the single canonical implementation. The CLI, the C# binding (`M3L.Native`, via
`m3l-cabi`), the Node.js binding (`@iyulab/m3l`, via `m3l-napi`), and the WASM build all wrap the same
core — none of them re-implement parsing, resolution, or validation independently.

Consequences:

- A parsing bug fixed in `m3l-core` is fixed for every binding at once. A binding-level workaround for
  a core bug is exactly the failure mode this anchor exists to prevent.
- New language bindings are additive wrappers around `m3l-core`/`m3l-cabi`, never parallel
  implementations — a second parser is not on the table regardless of target platform.

## 4. The spec is pinned by fixtures, not by prose alone

`docs/specification.md` states what M3L means. `spec/conformance/` (paired `inputs/`/`expected/`
fixtures) states what M3L *actually does*, checked on every CI run. A spec clause with no
corresponding fixture is a claim nobody is currently verifying.

Consequences:

- A spec change (new syntax, a semantic change to an existing construct) ships with a conformance
  fixture in the same PR — the RFC that proposed the change is not sufficient on its own.
- `m3l-lint`'s style/quality rules are validated the same way, in `crates/m3l-lint/tests/` — lint
  rules are opinions about code quality, not language semantics, so they are pinned separately from
  `spec/conformance/`, not folded into it.

## 5. Extension is RFC-driven, not PR-driven

A change to `docs/specification.md`'s syntax or semantics starts as an RFC in `rfcs/` — Summary,
Motivation, the concrete spec diff — before or alongside implementation. This is not bureaucracy for
its own sake: M3L's value is that every binding and every consumer shares one grammar, so a syntax
change is a breaking change for everyone downstream at once, and deserves the same deliberateness a
public API's breaking change would get in any other project.

Small clarifications to the spec's existing wording, or parser-internal changes with no spec-visible
effect, do not need an RFC — use judgment.

---

## What this document is not

It is not the specification. `docs/specification.md` is the formal grammar, type system, and
semantics — read it for what M3L syntax *is*. This document only answers *why the project draws its
boundaries where it does*. When an implementation question arises that the spec doesn't settle, the
test is: *which answer better serves the five anchors above?* If the anchors don't decide it, it's a
genuine free choice; make it, record it in an RFC if it is consequential enough to affect the spec,
move on.
