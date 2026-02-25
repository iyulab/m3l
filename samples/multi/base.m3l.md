# Namespace: sample.multi

> Shared base models and interfaces for multi-file import demonstration.

---

## Timestampable ::interface

- created_at: timestamp = now()
- updated_at: timestamp = now()

## SoftDeletable ::interface

- is_deleted: boolean = false
- deleted_at: timestamp?
- deleted_by: string(100)?

---

## BaseEntity : Timestampable

- id: identifier @pk @generated

---

## Currency ::enum

- usd: "US Dollar"
- eur: "Euro"
- gbp: "British Pound"
- jpy: "Japanese Yen"
- krw: "Korean Won"

## UnitOfMeasure ::enum

- ea: "Each"
- kg: "Kilogram"
- m: "Meter"
- l: "Liter"
- box: "Box"
