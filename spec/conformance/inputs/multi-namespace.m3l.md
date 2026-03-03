# Namespace: Auth

## User
- id: identifier @pk
- name: string(100)
- email: string(320)

# Namespace: Billing

## Invoice
- id: identifier @pk
- amount: decimal(10,2)
- due_date: date
