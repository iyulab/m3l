## BacktickExpr
- id: identifier @pk
- profit: decimal @computed(`(price - cost) / price * 100`)
- email: string @validate(pattern(`^[\w.]+@[\w.]+$`))
- raw_val: string @computed_raw(`metadata->>'category'`, platform: "postgresql")
- status: string = "active"
- created_at: timestamp = `now()`
- discount: decimal = `price * 0.9`

## CodeBlockExpr
- id: identifier @pk
- price: decimal(10,2)
- quantity: integer
- total_spent: decimal(12,2)

### Computed
- tier: string @computed
  ```
  CASE
    WHEN total_spent > 10000 THEN 'Gold'
    WHEN total_spent > 5000  THEN 'Silver'
    ELSE 'Bronze'
  END
  ```
- simple: decimal @computed(`price * quantity`)

## FieldBlockquoteDesc
- id: identifier @pk
- email: string(320) @unique @not_null
  > Primary contact email.
  > Used for login and account recovery.
- name: string(100) "Inline description still works"
- notes: text
  > Multi-line field description
  > spanning three lines
  > for demonstration purposes.
