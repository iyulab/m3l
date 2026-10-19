# Namespace: domain.sales

@import "./inventory.m3l" as inv

## Order : inv.Stamped
- id: identifier @pk
- product_id: identifier @reference(inv.Product)
- product_name: string @lookup(product_id.name)

## ProductSalesProfile ::aspect(inv.Product)
- margin: decimal(5,2)?
