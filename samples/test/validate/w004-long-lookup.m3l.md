# Namespace: test.w004

## Target

- id: identifier @pk
- name: string

---

## Source

- id: identifier @pk
- target_id: identifier @reference(Target)

### Lookup
- deep_val: string @lookup(target_id.a.b.c)
