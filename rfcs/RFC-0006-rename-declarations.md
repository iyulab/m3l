# M3L RFC-0006: Rename Declarations

> **RFC Status**: Proposed
> **Author**: UJ (iyulab)
> **Date**: 2026-10-15
> **M3L Version Target**: assigned on acceptance (a minor release)
> **Affects Sections**: 2.2 (model definition), 2.3 (field definition), 2.5 (attributes), 3.1.7 (inline enums — exclusion), the attribute catalog, diagnostics table

---

## 1. Summary
Give a model and a stored field a way to say what they used to be called:
`@renamed_from(OldName)`. The declaration states an identity — *this* `Customer` is the model that was
`Client` — which a model has no other way to express. It is a fact about the model's history, carried
in the parsed form, checked by the parser against the rest of the model, and left for implementations
that produce or change storage to act on.

## 2. Motivation
A model is a description of the present. When an author renames `Client` to `Customer`, two versions
of the model differ by one name, and that difference reads identically as two other changes: «a model
was removed, and an unrelated model was added». Nothing in either version separates the cases — the
identity the author means exists only in the author's head.

The two readings have opposite consequences for data. An implementation that turns the difference into
storage changes must pick one, and the only safe pick without more information is the second: the old
storage is dropped (or refused, or left behind) and the new storage starts empty. Guessing the first
from shape — same fields, one name out, one name in — is worse when wrong: rows land in storage they do
not belong to, and nothing downstream reports it.

Database refactoring practice treats a rename as a named, data-preserving refactoring, not a removal
plus an addition. The model is the only layer where the fact «this is a rename» exists — it is where
the author made the change. M3L has no vocabulary to write it down, so every rename today leaves the
model and is restated by hand somewhere the model cannot check.

## 3. Design

### 3.1 The declaration
`@renamed_from` takes one or more names, **oldest first**, and appears where the renamed element is
declared:

```markdown
## Customer @renamed_from(Client)
- id: identifier @pk
- display_name: string(200) @renamed_from(name)
- tier_code: string(20) @renamed_from(level, tier)
```

- On a **model header** (`##`) it names the model's former names.
- On a **field** it names the field's former names within the same model.
- Several names are a history: `@renamed_from(level, tier)` reads «`level` became `tier`, and `tier`
  became `tier_code`». The order is the order the renames happened, which is the order an
  implementation replays them in for storage that is still at the oldest name.

A former name is a name, never a reference. `@reference(Client)` does not resolve to `Customer`; every
reference to a renamed element is updated in the same change, exactly as without this RFC. The
declaration adds history, not an alias.

### 3.2 Where it applies — and where it does not

| Element | Applies | Why |
|---|---|---|
| Model (`##`, including `::aspect` and `::subtype`) | ✅ | A model is stored under its name |
| Stored field | ✅ | A field is stored under its name |
| Field inherited from a parent | on the parent | The field is declared once; its rename is declared where it is declared, and holds for every model that inherits it |
| Field declared in `::extend` | ✅ on that field | The extend block declares it |
| `::extend` header | ❌ | An extend block names its target (§3.2 of RFC-0004); renaming the target is the target's own header |
| View (`::view`, also `@materialized`) | ❌ | A view's rows are derived from its sources — a renamed view is a new definition, rebuilt, with no data of its own to carry |
| Derived field without storage (`@computed`, `@lookup`, `@rollup`, view fields) | ❌ | No stored value carries over |
| `@computed(…) @persisted` | ✅ | It has storage; renaming keeps it instead of rebuilding it. Recomputable, so the declaration is optional rather than necessary |
| Enum value | ❌ (out of scope) | Renaming a value rewrites stored *data*, not a storage name — a data migration, not a rename (§7) |
| Relation name (`### Relations`, `@relation`) | ❌ | A navigation name is not stored; changing it changes no data |

A declaration in a place marked ❌ is an error rather than ignored: an author who writes it expects it
to act, and silence would hide that it does not.

### 3.3 Lifetime
The language **does not require** a declaration to be removed, and does not define a point after which
it is stale.

- A model cannot know which of its databases — development copies, test environments, every
  installation in production — are already past a rename. Only the author (or the tooling that applies
  changes and records what it applied) knows that.
- Keeping a declaration is harmless when the implementation applying it records what it already did
  — the second application finds nothing to do. Removing it too early is the failure the declaration
  exists to prevent: a storage that is still at the old name reads the next model as a removal plus an
  addition again.

The recommendation to authors is therefore: **keep a declaration until every storage built from the
model is known to be past it, then remove it in a change of its own.** A long history is legitimate;
`@renamed_from(a, b, c)` on a field that was renamed three times is a correct statement.

### 3.4 Parsed form
The names arrive as their own key, like `label`: `renamed_from: ["Client"]` on the model node,
`renamed_from: ["level", "tier"]` on the field node, absent when not declared. The attribute itself is
not repeated in the node's attribute list — one fact, one place (the same rule the relationship
entries follow, §3.2.4 «One parsed form for every spelling»). Names are kept as written; namespace
qualification (§2.1) is not allowed in a former name, because a rename stays within its namespace.

### 3.5 What implementations do
An implementation that produces or changes storage from the model may use the declaration to emit a
rename instead of a removal plus an addition — in the order the history gives, and only for a former
name whose storage still exists. One that does not act on it states so; it must not silently treat the
declaration as an ordinary attribute and drop it. Moving a model between namespaces, and changing a
storage-wide prefix (`# Prefix:`), are not renames in this RFC's sense (§7).

## 4. Validation
New errors (codes assigned at acceptance; the next free error code at the time of writing is
`M3L-E028`):

| Rule | Reported when |
|---|---|
| Former name collides with a current name | A model's former name is the name of a model that exists in the same namespace now; a field's former name is the name of a field the model has now (its own, inherited, or added by `::extend`). The old name cannot both be gone and be present |
| Former name claimed twice | Two models in one namespace, or two fields of one model, list the same former name. Two elements cannot both have been the same element |
| Repeated or circular history | One declaration lists a name twice, or lists the element's current name |
| Declaration where nothing is stored | `@renamed_from` on a view, an `::extend` header, a derived field without storage, an enum value, or a relation (§3.2) |
| Not a name | An argument that is empty, qualified, or not an identifier |

No warnings are proposed. In particular a model that changes shape without a declaration is not
reported: the parser sees one version of the model, and a rename is a difference between two.

## 5. Compatibility
Additive. `@renamed_from` is not used anywhere in the specification or its examples today; a model that
used it as a custom attribute (§2.5.6) now has its meaning defined, and is checked against §4. The parsed
form gains an optional key on model and field nodes; consumers that ignore unknown keys are unaffected.

## 6. Implementation plan
1. Specification: the declaration in §2.2 and §2.3, the catalog row, the exclusions of §3.2, the
   lifetime guidance of §3.3, the diagnostics. Conformance cases for each rule in §4 and for the parsed
   form of a multi-step history.
2. Parser/validator: the `renamed_from` key on model and field nodes; the rules of §4, including the
   inheritance and `::extend` cases.
3. Consumers: acting on the declaration (out of this repository).

## 7. Open questions
- **History order.** Oldest first reads as a timeline and matches replay order; newest first puts the
  name an author most recently saw next to the current one. This draft takes oldest first.
- **Moving between namespaces.** `domain.sales.Client` → `domain.crm.Customer` is a rename for storage
  that is keyed by namespace and a no-op for storage that is not. This draft leaves it out (former names
  are unqualified); a later revision could allow a qualified former name for exactly this case.
- **Prefix changes.** A changed `# Prefix:` renames every model's storage at once. A file-level
  declaration (`# Prefix: crm @renamed_from(sales)`) is the natural shape; left out until there is a
  case for it.
- **Enum values.** A value rename (`Pending` → `Waiting`) is a data change on every row holding the
  value. If it is wanted, it is a separate declaration with data semantics — not this one.
- **Spelling.** `@renamed_from` against shorter `@was` / `@formerly`. This draft prefers the name that
  says what happened and from which direction.

## 8. Alternatives considered
- **Leave renames out of the model (status quo).** Every rename is restated by hand outside the model,
  where nothing checks it against the model that caused it, and forgetting it loses data.
- **Infer renames from shape.** Two versions with one model out and one in, with matching fields, look
  like a rename. They also look like a removal plus an unrelated addition, and a wrong inference moves
  data into the wrong place without a report. A declaration is cheap; a wrong guess is not.
- **A stable identifier per element** (`@id("c0f1…")` on every model and field, compared across
  versions). Detects every rename without a history, but puts an opaque token on every line of every
  model to serve a rare change, and breaks when an element is copied. The declaration costs nothing
  until a rename happens.
- **A separate rename file next to the model.** Keeps the model free of history, but splits one change
  across two files that can disagree, and loses the check against the model's current names (§4).
