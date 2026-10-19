# Namespace: orders

## Order
- id: identifier @pk
- product_id: identifier @reference(Product)
- sales_product_id: identifier @reference(sales.Product)
- sales_name: string @lookup(sales_product_id.name)

## OrderView ::view
### Source
- from: Product

- id: identifier @from(Product.id)

## SalesOrderView ::view
### Source
- from: sales.Product

- id: identifier @from(sales.Product.id)
