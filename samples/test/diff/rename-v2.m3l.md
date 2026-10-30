# Namespace: test.diff

## CustomerOrder @renamed_from(SalesOrder)

- id: identifier @pk
- order_code: string(20) @renamed_from(code)
- total: integer @renamed_from(amount)
