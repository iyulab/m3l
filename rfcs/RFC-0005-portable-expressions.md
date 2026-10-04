# M3L RFC-0005: Portable Expressions

> **RFC Status**: Proposed
> **Author**: UJ (iyulab)
> **Date**: 2026-10-04
> **M3L Version Target**: 0.19.0
> **Affects Sections**: 2.5.5, 4.2 (`@computed`), 4.4 (`@rollup` `where:`), 4.7 (view `### Source` `where`), 10.7, diagnostics table

---

## 1. Summary
Define the expression language that `@computed`, a rollup's `where:` and a view's `where` are
written in: a small, closed subset — literals, field references, operators, `CASE`, and a fixed
list of functions — that every implementation can translate into its own platform's syntax.
An expression outside the subset is not an error; it is platform-specific, and says so with the
escape hatch that already exists for `@computed` (`@computed_raw(…, platform: …)`), now available
in every expression context.

## 2. Motivation
§10.7 already draws the line this RFC needs. It calls `@computed("first_name + ' ' + last_name")`
**"Platform-neutral (recommended)"** and `@computed_raw("YEAR(created_at)", platform: "sqlserver")`
**"Platform-specific (escape hatch)"** — and in the same section says *"M3L does not define an
expression language — the expression content is passed through to the implementation layer."*
Neutral is promised; what is neutral is never said.

The examples show what that costs. Across the specification, plain `@computed` (the neutral
form) uses `DATEDIFF(DAY, …, GETDATE())`, `FORMAT(ordered_at, 'yyyy-MM')`, `ISNULL(…)`,
bracketed `[Name]` identifiers and `+` between strings — all SQL Server syntax that PostgreSQL
rejects — next to `date_add(today(), -30, 'day')`, which is the syntax of no database. And
`where: "is_active = true"` is not even SQL Server: it has no boolean literal, and refuses the
expression as an unknown column named `true`. A model that one implementation turns into working
SQL is, for another, an expression it must either refuse or guess at.

Implementations have already had to choose. One generator passes `@computed` through to SQL Server
unchanged and declines to emit it for PostgreSQL at all, because translating an undefined syntax is
a guess. The same question blocks every derived view that filters or computes on PostgreSQL. The
language is the only place the answer can come from — RFC-0001 left it open as "a separate
sub-specification" (§10, question 6); this is that sub-specification.

## 3. Design

### 3.1 Where expressions appear
| Context | Example |
|---|---|
| `@computed("…")` | `@computed("quantity * unit_price")` |
| a rollup's `where:` | `@rollup(Order.customer_id, count, where: "status = 'paid'")` |
| a view's `### Source` `where` | `- where: "status <> 'cancelled'"` |

Default values (`= now()`, §2.5.5) are outside this RFC: they are single calls, and the functions
they use are listed in §3.4 so the two stay consistent.

### 3.2 Lexical elements
- **Field references**: a field name as declared (`unit_price`). In a view, a field of one of the
  view's relations is written `Relation.field` (`Order.status`). A rollup's `where:` refers to the
  rolled-up model's fields; `$parent.field` keeps its current meaning.
- **Literals**: numbers (`42`, `3.5`, `-1`); strings in single quotes, a quote doubled to escape it
  (`'it''s'`); `true`, `false`, `null`.
- Identifiers are not bracketed or quoted. `[Name]`, `"Name"` and `` `Name` `` are platform syntax.

### 3.3 Operators and forms
| Kind | Portable form |
|---|---|
| Arithmetic | `+ - * / %` — **numbers only** |
| Comparison | `= <> != < <= > >=` |
| Logic | `AND OR NOT` |
| Null tests | `IS NULL`, `IS NOT NULL` |
| Membership / range | `IN ( … )`, `NOT IN ( … )`, `BETWEEN … AND …` |
| Pattern | `LIKE`, with `%` and `_` (case sensitivity follows the platform's collation) |
| Conditional | `CASE WHEN … THEN … [ELSE …] END` |
| Grouping | `( … )` |

String concatenation is the function `concat(…)`, not an operator: `+` concatenates strings on one
platform and fails on another, `||` the reverse.

### 3.4 Functions
A closed list, written in lower case. Arguments are expressions.

| Function | Meaning |
|---|---|
| `coalesce(a, b, …)` | the first argument that is not null |
| `nullif(a, b)` | null when `a = b`, otherwise `a` |
| `concat(a, b, …)` | the arguments as text, joined; a null argument contributes nothing |
| `lower(s)` · `upper(s)` · `trim(s)` | case and surrounding whitespace |
| `length(s)` | number of characters |
| `substring(s, start, count)` | `count` characters from `start` (1-based) |
| `abs(x)` · `round(x, digits)` · `floor(x)` · `ceiling(x)` | numeric |
| `today()` | the current date |
| `now()` | the current date and time, with offset |
| `date_add(d, amount, unit)` | `d` moved by `amount` units; `unit` is one of `'year'`, `'month'`, `'day'`, `'hour'`, `'minute'`, `'second'` |
| `date_diff(unit, start, end)` | the number of `unit` boundaries crossed from `start` to `end` |
| `year(d)` · `month(d)` · `day(d)` | date parts |
| `generate_uuid()` | a new unique identifier (default values) |

Aggregates (`count`, `sum`, …) are not expression functions: an aggregate over related rows is a
`@rollup`.

### 3.5 Platform-specific expressions
An expression that uses anything outside §3.2–3.4 is platform-specific. That is allowed — it is
how a model reaches a platform feature the subset does not cover — and it is declared, so that a
reader and an implementation both know the expression will not travel:

```markdown
- year_created: integer @computed_raw("YEAR(created_at)", platform: "sqlserver")
- year_created: integer @computed_raw("EXTRACT(YEAR FROM created_at)::int", platform: "postgresql")
```

- `@computed_raw` may appear **once per platform** on a field; an implementation uses the one for
  its platform and reports a field with none for its platform.
- The same form becomes available to the two other contexts: a view's `### Source` takes
  `- where_raw: "…" platform: <name>` (once per platform, instead of `where`), and a rollup takes
  `where_raw:` with `platform:` in place of `where:`.
- `platform` names: `sqlserver`, `postgresql`, `mysql`, `sqlite`. Other names are recorded, not
  checked.

### 3.6 What implementations do
An implementation that emits a portable expression translates it into its platform's syntax —
`true` → `1` on SQL Server, `concat(a, b)` → `CONCAT(a, b)` / `concat(a, b)`,
`date_add(d, -30, 'day')` → `DATEADD(DAY, -30, d)` / `d + INTERVAL '-30 day'`. The mapping is the
implementation's; the meaning is the language's. An implementation that cannot translate an
expression reports it rather than emitting it unchanged.

## 4. Diagnostics
| Code | Severity | Condition |
|---|---|---|
| `M3L-W011` | warning | an expression in a portable context uses something outside the subset — names the construct (a function not in §3.4, a bracketed identifier, `+` next to a string literal) and suggests `@computed_raw`/`where_raw` |
| `M3L-E026` | error | a `@computed_raw`/`where_raw` repeats a platform on the same field or view |

The warning is deliberately not an error: existing models keep building, and a model that
targets one platform can silence it by declaring the expression raw.

## 5. Compatibility
Every model that parses today still parses; no expression changes meaning. Models whose
`@computed` relies on platform syntax start to receive `M3L-W011`, which is the point — the
expression was never portable, and the warning is the first place that is said. The specification
examples that use platform syntax in a neutral context are rewritten in the subset (or marked raw)
in the same change.

## 6. Implementation plan
1. Specification: §10.7 rewritten around §3 above; §2.5.5, §4.2, §4.4, §4.7 examples brought into the
   subset; the diagnostics table. Conformance fixtures for portable and raw expressions.
2. Parser/validator: tokenize expressions in the three contexts against the subset (`M3L-W011`),
   repeatable `@computed_raw`, `where_raw` (`M3L-E026`).
3. Consumers: translators per platform (out of this repository).

## 7. Alternatives considered
- **Leave expressions undefined (status quo).** Every consumer keeps guessing a dialect, and
  multi-platform output stays impossible for any model that computes or filters.
- **Platform SQL only, declared per platform everywhere.** Correct, but turns the common case —
  arithmetic, comparisons, `CASE` — into duplicated text per platform. The subset keeps the common
  case written once and leaves duplication to the cases that need a platform feature.
- **Adopt one existing SQL dialect as the language.** Makes that platform's syntax free and every
  other platform a translation target of an open-ended grammar — the problem this RFC exists to
  close.
