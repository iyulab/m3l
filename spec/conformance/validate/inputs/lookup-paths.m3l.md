# Namespace: conformance.validate

## Region
- id: identifier @pk
- name: string(100)

## Customer
- id: identifier @pk
- region_id: identifier @reference(Region)
- loose_id: identifier
- region_name: string @lookup(region_id.name)
- loose_name: string @lookup(loose_id.name)
- missing: string @lookup(region_id.nothing)

## Category
- id: identifier @pk
- parent_id: identifier? @reference(Category)
- root_name: string? @lookup(parent_id.root_name)
