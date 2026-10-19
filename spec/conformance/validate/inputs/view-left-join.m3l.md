## Customer
- id: identifier @pk
- name: string(100)

## Order
- id: identifier @pk
- customer_id: identifier? @reference(Customer)

## OrderSummary ::view
### Source
- from: Order
- left_join: Customer on Order.customer_id = Customer.id

- order_id: identifier @from(Order.id)
- customer_name: string(100) @from(Customer.name)
- customer_id: identifier? @from(Customer.id)

## PaidOrder ::view
### Source
- from: Order
- join: Customer on Order.customer_id = Customer.id

- customer_name: string(100) @from(Customer.name)
