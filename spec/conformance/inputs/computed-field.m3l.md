## Product
- id: identifier @pk
- price: decimal(10,2)
- quantity: integer

### Computed
- total_value: decimal(12,2) @computed("price * quantity")
