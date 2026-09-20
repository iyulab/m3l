# M3L RFC-0004: Model Extension and Base Kinds

> **RFC Status**: Accepted (implemented in 0.13.0)
> **Author**: UJ (iyulab)
> **Date**: 2026-09-20
> **M3L Version Target**: 0.13.0
> **Affects Sections**: 3.4, 10.3, diagnostics table

---

## 1. Summary
Three additions that let a model be built up from more than one source file without the
base file being edited: a file-level owner declaration (`# Prefix:`), a block that adds fields
to a model declared elsewhere (`::extend`), and two model kinds that name a base model
(`::aspect(Base)`, `::subtype(Base)`).

## 2. Motivation
A data model is often delivered as a standard part plus parts written by someone else — an
optional module, or a deployment-specific layer. Today the only way to add a field to a model
is to edit the file that declares it, which makes the standard part impossible to upgrade
independently. `: Parent` inheritance does not help: it copies fields *into a new model*,
it cannot add fields *to an existing one*.

## 3. Design
### 3.1 `# Prefix: <word>`
A file header, sibling of `# Namespace:`. Declares the owner of everything the file declares.
`<word>` matches `[a-z][a-z0-9]*`. Stamped on every model, enum, interface, view and extend
block of the file as `prefix`. Files without the header share one owner: no prefix.
M3L records ownership; it does not enforce a naming policy — that is a generator concern.

### 3.2 `## Target ::extend`
Adds the block's fields to the end of `Target`'s field list, after inherited and own fields,
in source-file order. Each added field carries `origin { prefix, namespace, source }`, and
`Target` carries `extended_by[]`. The block is not a model: it has no name of its own, no
parents, no sections, no model-level directives — fields only (stored and derived kinds alike).

### 3.3 `## Name ::aspect(Base)` and `## Name ::subtype(Base)`
Ordinary models that additionally name one base model: `base { kind, model }`. `aspect` says
"an optional companion of one Base row, existing or not as a unit"; `subtype` says "a kind of
Base". M3L records the declaration and checks that Base exists and is not itself an aspect.
How a generator stores either is not part of the language. A parent list may follow:
`## Name ::aspect(Base) : Timestampable`.

## 4. Diagnostics
E011 extend target not found · E012 extend field name collides · E013 attribute not allowed
in an extend block (`@pk`, `@primary`, `@override`) · E014 extend block declares parents ·
E015 extend block declares something other than fields · E016 `::aspect`/`::subtype` without
a base argument · E017 base model not found · E018 base model is itself an aspect.

## 5. Compatibility
`extend`, `aspect` and `subtype` were previously accepted as generic kinds and landed in the
untyped `extensions` map. A document that used those three words as its own custom kinds
changes meaning. No such use is known.
