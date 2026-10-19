# Namespace: sample.views

## Customer
- id: identifier @pk
- name: string(100)

## Order
- id: identifier @pk
- customer_id: identifier? @reference(Customer)
- total: decimal(12,2)

## CustomerProfile
- customer_id: identifier @pk @reference(Customer)
- tier: string(20)

## OrderSummary ::view
> Every order, with its customer where it has one

### Source
- from: Order
- left_join: Customer on Order.customer_id = Customer.id
- left_join: CustomerProfile on Customer.id = CustomerProfile.customer_id

- order_id: identifier @from(Order.id)
- total: decimal(12,2) @from(Order.total)
- customer_name: string(100)? @from(Customer.name)
- tier: string(20)? @from(CustomerProfile.tier)

## PaidOrder ::view
> Only orders that have a customer

### Source
- from: Order
- join: Customer on Order.customer_id = Customer.id

- order_id: identifier @from(Order.id)
- customer_name: string(100) @from(Customer.name)
