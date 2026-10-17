# Namespace: conformance.registry

## priority ::attribute
> Priority level for task ordering
- target: [field]
- type: number
- range: 1..10

## owner_tag ::attribute
- target: [model]
- type: string
- required: true

## Task
- id: identifier @pk
- level: integer @priority("high")
- rank: integer @priority(42)
- note: string @owner_tag("ops")

## Board @owner_tag
- id: identifier @pk
