## Customer
- id: identifier @pk
- name: string(100)

## Order
- id: identifier @pk
- customer_id: identifier @fk(Customer.id)
- amount: decimal(10,2)
- status: string = "pending"

### Lookup
- customer_name: string @lookup(customer_id.name)

### Rollup
- order_total: decimal(12,2) @rollup(Order.customer_id, sum(amount))

### Computed
- display_label: string @computed("status + ' #' + id")
