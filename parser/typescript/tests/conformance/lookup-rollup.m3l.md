## Customer
- id: identifier @pk
- name: string(100)

## Order
- id: identifier @pk
- customer_id: identifier @fk(Customer.id)
- total: decimal(10,2)

# Lookup
- customer_name: string @lookup(customer_id.name)

# Rollup

## Customer
This would be a separate section but for testing we use a new model definition.

## OrderSummary
- id: identifier @pk
- customer_id: identifier @fk(Customer.id)

# Rollup
- order_count: integer @rollup(Order.customer_id, count)
- total_spent: decimal(12,2) @rollup(Order.customer_id, sum(total))
