# Namespace: test.format

## Timestampable ::interface

- created_at: timestamp = now()
- updated_at: timestamp = now()

---

## Status ::enum

- active: "Active"
- inactive: "Inactive"

---

## Customer : Timestampable

- id: identifier @pk @generated
- name: string(100) @not_null
- email: email @unique
- profile: object
  - bio: text?
  - avatar_url: url?

---

## ActiveCustomers ::view

### Source
- from: Customer
- where: "is_active = true"
