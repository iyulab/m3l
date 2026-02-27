# Namespace: test.lint.refs

## Target0

- id: identifier @pk

## Target1

- id: identifier @pk

## Target2

- id: identifier @pk

## Target3

- id: identifier @pk

## Target4

- id: identifier @pk

## Target5

- id: identifier @pk

---

## Complex

- id: identifier @pk
- ref_0: identifier @reference(Target0)
- ref_1: identifier @reference(Target1)
- ref_2: identifier @reference(Target2)
- ref_3: identifier @reference(Target3)
- ref_4: identifier @reference(Target4)
- ref_5: identifier @reference(Target5)
- name: string
