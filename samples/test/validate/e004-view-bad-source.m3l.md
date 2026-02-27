# Namespace: test.e004

## Product

- id: identifier @pk
- name: string

---

## GhostView ::view

### Source
- from: NonExistentModel
- where: "active = true"
