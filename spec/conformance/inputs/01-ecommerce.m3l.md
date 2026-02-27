# Namespace: sample.ecommerce

> E-Commerce data model demonstrating core M3L features:
> models, enums, references, lookups, rollups, indexes, and sections.

---

## Timestampable ::interface

- created_at: timestamp = now()
- updated_at: timestamp = now()

## Auditable ::interface

- created_by: string(100)
- updated_by: string(100)

---

## CustomerStatus ::enum

- active: "Active"
- inactive: "Inactive"
- suspended: "Suspended"
- pending: "Pending Verification"

## PaymentMethod ::enum

- credit_card: "Credit Card"
- bank_transfer: "Bank Transfer"
- paypal: "PayPal"
- cash: "Cash on Delivery"

## OrderStatus ::enum

- draft: "Draft"
- confirmed: "Confirmed"
- processing: "Processing"
- shipped: "Shipped"
- delivered: "Delivered"
- cancelled: "Cancelled"
- returned: "Returned"

## ShippingPriority ::enum

- standard: integer = 0 "Standard Shipping"
- express: integer = 1 "Express Shipping"
- overnight: integer = 2 "Overnight Delivery"

---

## Customer : Timestampable @public

> Primary customer entity for the e-commerce platform.

- id: identifier @pk @generated
- email: email @unique
  > Primary contact email.
  > Used for login, notifications, and account recovery.
- name: string(100) @not_null @searchable
- phone: phone?
- status: enum = "active"
  - active: "Active"
  - inactive: "Inactive"
  - suspended: "Suspended"
- loyalty_points: integer = 0 @min(0)
- is_verified: boolean = false

### Metadata
- table_name: customers
- audit_enabled: true

---

## Address : Timestampable

> Shipping and billing address.

- id: identifier @pk @generated
- customer_id: identifier @reference(Customer)
- label: string(50) = "Home" "Address label (Home, Office, etc.)"
- street: string(200) @not_null
- city: string(100) @not_null
- state: string(100)?
- postal_code: string(20) @not_null
- country: string(2) @not_null "ISO 3166-1 alpha-2"
- is_default: boolean = false

- @index(customer_id)
- @unique(customer_id, label)

---

## Category : Timestampable

- id: identifier @pk @generated
- name: string(100) @not_null @searchable
- slug: string(100) @unique
- parent_id: identifier? @reference(Category)?
- sort_order: integer = 0
- is_active: boolean = true

### Lookup
- parent_name: string @lookup(parent_id.name)

### Rollup
- product_count: integer @rollup(Product.category_id, count)

---

## Product : Timestampable, Auditable @public

> Product catalog item.

- id: identifier @pk @generated
- sku: string(50) @unique @not_null @immutable "Stock Keeping Unit"
- name: string(200) @not_null @searchable
- description: text?
- category_id: identifier @reference(Category)
- price: decimal(10,2) @not_null @min(0)
- cost: decimal(10,2)? @min(0) # Internal cost, not shown to customers
- weight: float? "Weight in kg"
- is_active: boolean = true
- tags: string[]

### Lookup
- category_name: string @lookup(category_id.name)

### Computed
- profit_margin: decimal(5,2) @computed(`(price - cost) / price * 100`)
- display_price: string @computed("FORMAT(price, '$#,##0.00')")

### Indexes
- idx_category
  - fields: [category_id, is_active]
- idx_price
  - fields: [price]

### Relations
- category: >Category via category_id

---

## Inventory : Timestampable

- id: identifier @pk @generated
- product_id: identifier @reference(Product)! @unique
- quantity: integer = 0 @min(0)
- reserved: integer = 0 @min(0)
- reorder_point: integer = 10
- reorder_qty: integer = 50

### Computed
- available: integer @computed("quantity - reserved")
- needs_reorder: boolean @computed("available <= reorder_point")

---

## Order : Timestampable, Auditable

> Customer order with items, totals, and status tracking.

- id: identifier @pk @generated
- order_number: string(20) @unique @generated
- customer_id: identifier @reference(Customer)!!
- status: OrderStatus = "draft"
- shipping_address_id: identifier? @reference(Address)?
- billing_address_id: identifier? @reference(Address)?
- payment_method: PaymentMethod?
- notes: text?
- ordered_at: timestamp?
- shipped_at: timestamp?
- delivered_at: timestamp?

### Lookup
- customer_name: string @lookup(customer_id.name)
- customer_email: email @lookup(customer_id.email)
- shipping_city: string @lookup(shipping_address_id.city)

### Rollup
- item_count: integer @rollup(OrderItem.order_id, count)
- subtotal: decimal(12,2) @rollup(OrderItem.order_id, sum(line_total))
- total_quantity: integer @rollup(OrderItem.order_id, sum(quantity))

### Computed
- tax_amount: decimal(12,2) @computed("subtotal * 0.1")
- grand_total: decimal(12,2) @computed(`subtotal + tax_amount`)

### Indexes
- idx_customer_date
  - fields: [customer_id, ordered_at]
- idx_status
  - fields: [status]

### Behaviors
- before_create: generate_order_number

### Metadata
- table_name: orders
- soft_delete: true

---

## OrderItem

- id: identifier @pk @generated
- order_id: identifier @reference(Order)
- product_id: identifier @reference(Product)!
- quantity: integer @not_null @min(1) @max(9999)
- unit_price: decimal(10,2) @not_null
- discount_pct: decimal(5,2) = 0 @min(0) @max(100)

### Lookup
- product_name: string @lookup(product_id.name)
- product_sku: string @lookup(product_id.sku)

### Computed
- discount_amount: decimal(10,2) @computed("unit_price * quantity * discount_pct / 100")
- line_total: decimal(12,2) @computed(`unit_price * quantity - discount_amount`)

- @unique(order_id, product_id)

---

## Review : Timestampable

- id: identifier @pk @generated
- product_id: identifier @reference(Product)
- customer_id: identifier @reference(Customer)
- rating: integer @not_null @min(1) @max(5)
- title: string(200)?
- body: text?
- is_verified: boolean = false

- @unique(product_id, customer_id)

---

## ActiveProducts ::view

> Products currently available for sale.

### Source
- from: Product
- where: "is_active = true AND price > 0"
- order_by: "name asc"

---

## CustomerOrderSummary ::view @materialized

> Aggregated customer order statistics.

### Source
```sql
FROM Customer c
JOIN Order o ON c.id = o.customer_id
WHERE o.status != 'cancelled'
GROUP BY c.id, c.name, c.email
```

- customer_id: identifier @from(Customer.id)
- customer_name: string @from(Customer.name)
- total_orders: integer @computed(`COUNT(o.id)`)
- total_spent: decimal(12,2) @computed(`SUM(o.grand_total)`)
- avg_order_value: decimal(10,2) @computed(`AVG(o.grand_total)`)
- last_order_date: timestamp @computed(`MAX(o.ordered_at)`)

### Refresh
- strategy: incremental
- interval: "1 hour"
