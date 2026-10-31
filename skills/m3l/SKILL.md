---
name: m3l
description: Write, check and compare data models in M3L (.m3l.md, Markdown-based data modeling) with the m3l CLI — parse to a JSON AST, validate with coded diagnostics, lint, format, analyze dependencies, and diff two versions of a model into structured changes. Use when asked to author or change an M3L model, find out why a model does not validate, review what changed between two model versions, or read a model's structure as JSON.
---

# M3L: authoring and checking data models

M3L is Markdown with data-modeling semantics on top. A model is a `.m3l.md` file (or a directory of them) that stays
readable in any Markdown viewer. `m3l` turns it into a JSON AST and tells you what is wrong with it. It generates
nothing else — code, SQL and migrations are what a consumer builds from the AST.

## Prerequisites

`m3l` on PATH (`cargo install m3l-cli`). Every command takes a file or a directory (default: the current directory);
a directory is read with its `@import`s and `m3l.config.yaml`. The syntax is in `docs/specification.md` — read the
section for the construct you are writing instead of guessing; this skill covers the loop around it.

## The loop

1. **Edit** the `.m3l.md` file(s).
2. **Validate**: `m3l validate ./models` — exit `0` when there are no errors, `1` when there are. Each diagnostic has a
   stable code (`M3L-E…` errors, `M3L-W…` warnings), the catalog is specification §10.5. Fix by code, not by message
   text. `--strict` adds style guidelines.
3. **Lint**: `m3l lint ./models` — style and quality rules (naming, conventions). It reports; its exit code stays `0`
   for warnings, so read the output.
4. **Format**: `m3l format ./models` prints the standardized text to **stdout** — it does not rewrite the files.
   Redirect it yourself when you want the change, and review it first.
5. **Review a change**: `m3l diff old.m3l.md new.m3l.md --format json` (below).

## Machine contracts (parse these, not console text)

- `m3l parse ./models` — the AST as JSON (`-o file` to write it). It carries `parserVersion` and `astVersion`; a
  declaration's `loc` says where it was written.
- `m3l validate ./models --format json` — `{diagnostics: [{code, severity, message, file, line, col}], summary:
  {errors, warnings, files}}`.
- `m3l lint ./models --format json` — `{diagnostics: [{rule, severity, message, file, line, col}], …}`;
  `--format sarif` is available for code-scanning tools.
- `m3l diff a b --format json` — `{version, differs, summary, changes[]}`. Each change is
  `{op: added | removed | modified | renamed, scope: declaration | field | project | attributeRegistry, …}` with `kind`
  / `name` (declaration), `declaration` / `name` (field), `from` / `to` (rename), and for `modified` a `details` list of
  `{path, change: added | removed | changed, before?, after?}` carrying the AST's own JSON. Exit codes: `0` same, `1`
  differ, `2` could not compare. Branch on `version`; fields are only added within one.
- `m3l analyze ./models` — the dependency graph (`--format mermaid` default, or `dot`).

## Reading a diff

A change that touches stored data — a dropped `@unique`, a shortened `string(200)`, a type change, a removed field —
shows up as `removed` / `changed`; say so before anyone applies it downstream. A rename is only a rename when the model
says so with `@renamed_from(OldName)` on the declaration or field; without it the same edit reads as one removal plus
one addition, and a consumer cannot tell a rename from a data-losing drop. Reordering fields, attributes or enum values
changes nothing.

## Pitfalls

- A line that is not a field is not an error to the parser, so a malformed line can disappear from the AST without a
  diagnostic. After a hand edit, `m3l parse` the file and check that the field you wrote is in the output.
- `validate` finding nothing does not mean the model says what you meant — it means it is well-formed. Read the AST for
  the declarations you changed.
- Do not hand-write the AST or edit generated JSON; the `.m3l.md` file is the source.
