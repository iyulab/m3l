## User
- id: identifier @pk
- name: string(100) @not_null
- email: string(320)? @unique
- is_active: boolean = true
