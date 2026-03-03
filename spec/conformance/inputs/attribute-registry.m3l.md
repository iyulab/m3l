## auditable ::attribute
> Marks a field as requiring audit tracking
- target: [field]
- type: boolean
- default: false

## User
- id: identifier @pk
- name: string(100) @auditable
- email: string(320)
