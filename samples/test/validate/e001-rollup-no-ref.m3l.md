# Namespace: test.e001

## OrderItem

- id: identifier @pk
- order_id: identifier
- quantity: integer

---

## Order

- id: identifier @pk
- name: string

### Rollup
- total_qty: integer @rollup(OrderItem.order_id, sum(quantity))
