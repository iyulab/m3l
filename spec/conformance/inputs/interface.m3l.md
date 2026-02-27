## Timestampable ::interface
- created_at: timestamp = now()
- updated_at: timestamp = now()

## Article : Timestampable
- id: identifier @pk
- title: string(200)
- content: text
