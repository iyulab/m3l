# Namespace: test.analyze

## Base ::interface

- created_at: timestamp

---

## Category

- id: identifier @pk
- name: string
- parent_id: identifier? @reference(Category)?

---

## Product : Base

- id: identifier @pk
- name: string
- category_id: identifier @reference(Category)
- related_id: identifier? @reference(Product)?

---

## Isolated

- id: identifier @pk
- data: text
