# Namespace: test.e010

## Category

- id: identifier @pk
- name: string

---

## Product

- id: identifier @pk
- name: string
- category_id: identifier

### Relations
- category: >Category via category_id
