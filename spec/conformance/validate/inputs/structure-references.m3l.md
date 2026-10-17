# Namespace: conformance.structure

## Product
- id: identifier @pk
- name: string(100)
- name: string(200)
- price: money_amount

## ProductNames ::view
### Source
- from: Product

- id: identifier @from(Product.id)
- label: string @from(Catalog.name)

## GhostView ::view
### Source
- from: Missing
