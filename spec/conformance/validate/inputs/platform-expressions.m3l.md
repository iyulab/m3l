# Namespace: conformance.platform

## Order
- id: identifier @pk
- quantity: integer
- unit_price: decimal(12,2)
- total: decimal(14,2) @computed("ISNULL(quantity, 0) * unit_price")
- year_created: integer @computed_raw("YEAR(GETDATE())", platform: "sqlserver") @computed_raw("YEAR(SYSDATETIME())", platform: "sqlserver")
- label: string @computed_raw("'x'")
