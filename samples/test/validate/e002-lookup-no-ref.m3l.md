# Namespace: test.e002

## Order

- id: identifier @pk
- customer_id: identifier
- name: string

### Lookup
- cust_name: string @lookup(customer_id.name)
