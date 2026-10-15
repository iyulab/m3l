# Namespace: sample.extended

## Line
> Every field on one line (§2.3.1).

- id: identifier @pk
- price: decimal(10,2)
- qty: integer
- code: string(20)? @unique @renamed_from(legacy_code, old_code)
- tags: map<string,integer>
- label: string @check("len(label) > 0", name: "nonempty")
- active: boolean @default(false)
- total: decimal(12,2) @computed("price * qty") @persisted
- doubled: integer @computed("qty * 2")
- line_name: string @lookup(id.name)

## Extended
> The same fields in the extended format (§2.3.2) — the same AST, apart from the model's name.

- id
  - type: identifier
  - pk: true
- price
  - type: decimal(10,2)
- qty
  - type: integer
- code
  - type: string(20)?
  - unique: true
  - index: false
  - renamed_from: legacy_code, old_code
- tags
  - type: map<string,integer>
- label
  - type: string
  - check: "len(label) > 0", name: "nonempty"
- active
  - type: boolean
  - default: false
- total
  - type: decimal(12,2)
  - computed: true
  - formula: "price * qty"
  - persisted: true
- doubled
  - type: integer
  - computed: "qty * 2"
- line_name
  - type: string
  - lookup: id.name
