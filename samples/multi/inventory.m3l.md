# Namespace: sample.multi.inventory

@import "./base.m3l.md"

> Inventory management module demonstrating multi-file imports,
> cross-file inheritance, and namespace usage.

---

## Warehouse : BaseEntity

- code: string(20) @unique @not_null
- name: string(100) @not_null
- address: string(300)?
- is_active: boolean = true

# Rollup
- total_items: integer @rollup(StockItem.warehouse_id, count)
- total_value: decimal(14,2) @rollup(StockItem.warehouse_id, sum(total_value))

---

## Supplier : BaseEntity, SoftDeletable

- code: string(20) @unique @not_null
- name: string(200) @not_null @searchable
- contact_email: email?
- contact_phone: phone?
- currency: Currency = "usd"
- lead_time_days: integer = 7 @min(0)
- is_active: boolean = true

---

## Item : BaseEntity

> Inventory item master data.

- sku: string(50) @unique @not_null @immutable
- name: string(200) @not_null @searchable
- description: text?
- unit: UnitOfMeasure = "ea"
- category: string(50)?
- min_stock: integer = 0 @min(0) "Minimum stock level for reorder alert"
- is_active: boolean = true

# Rollup
- total_on_hand: integer @rollup(StockItem.item_id, sum(quantity))

---

## StockItem : BaseEntity

> Physical stock in a specific warehouse.

- warehouse_id: identifier @reference(Warehouse)
- item_id: identifier @reference(Item)!
- supplier_id: identifier? @reference(Supplier)?
- quantity: integer = 0 @min(0)
- unit_cost: decimal(10,2) @not_null @min(0)
- batch_number: string(50)?
- expiry_date: date?

# Lookup
- warehouse_name: string @lookup(warehouse_id.name)
- item_name: string @lookup(item_id.name)
- item_sku: string @lookup(item_id.sku)
- supplier_name: string @lookup(supplier_id.name)

# Computed
- total_value: decimal(14,2) @computed("quantity * unit_cost")
- is_expired: boolean @computed("expiry_date IS NOT NULL AND expiry_date < CURRENT_DATE")
- needs_reorder: boolean @computed("quantity <= min_stock")

- @unique(warehouse_id, item_id, batch_number)
- @index(item_id)
- @index(warehouse_id, item_id)

### Relations
- warehouse: >Warehouse via warehouse_id
- item: >Item via item_id
- supplier: >Supplier via supplier_id (optional)

---

## StockMovement : BaseEntity

> Record of stock in/out movements.

- stock_item_id: identifier @reference(StockItem)
- movement_type: enum @not_null
  - in: "Stock In"
  - out: "Stock Out"
  - transfer: "Transfer"
  - adjustment: "Adjustment"
- quantity: integer @not_null "Positive for in, negative for out"
- reference_number: string(50)?
- notes: text?
- moved_by: string(100) @not_null

# Lookup
- item_name: string @lookup(stock_item_id.item_name)

- @index(stock_item_id, created_at)

---

## LowStockAlert ::view

> Items below minimum stock level.

### Source
- from: StockItem
- join: Item on StockItem.item_id = Item.id
- where: "StockItem.quantity <= Item.min_stock AND Item.is_active = true"
- order_by: "StockItem.quantity asc"

- item_sku: string @from(Item.sku)
- item_name: string @from(Item.name)
- warehouse_id: identifier @from(StockItem.warehouse_id)
- current_qty: integer @from(StockItem.quantity)
- min_stock: integer @from(Item.min_stock)
- deficit: integer @computed("Item.min_stock - StockItem.quantity")

---

## WarehouseSummary ::view @materialized

> Aggregated stock value per warehouse.

### Source
- from: StockItem
- join: Warehouse on StockItem.warehouse_id = Warehouse.id
- group_by: [Warehouse.id, Warehouse.name, Warehouse.code]

- warehouse_code: string @from(Warehouse.code)
- warehouse_name: string @from(Warehouse.name)
- item_count: integer @computed("COUNT(DISTINCT StockItem.item_id)")
- total_quantity: integer @computed("SUM(StockItem.quantity)")
- total_value: decimal(14,2) @computed("SUM(StockItem.total_value)")

### Refresh
- strategy: incremental
- interval: "30 minutes"
