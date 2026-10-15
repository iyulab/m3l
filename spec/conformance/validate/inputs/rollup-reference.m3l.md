# Namespace: conformance.validate

## Order
- id: identifier @pk
- item_count: integer @rollup(OrderItem.order_id, count)

## OrderItem
- id: identifier @pk
- order_id: identifier
