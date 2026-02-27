## BaseModel
- id: identifier @pk
- created_at: timestamp = now()

## User : BaseModel
- name: string(100)
- email: string(320)
