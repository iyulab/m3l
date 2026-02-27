## User
- id: identifier @pk
- name: string(100)
- department: string(50)

## ActiveUsers ::view
### Source
- from: User
- where: "is_active = true"
- order_by: "name asc"
