## Customer
- id: identifier @pk

## Order
- id: identifier @pk
- customer_id: identifier @reference(Customer)
- nowhere_id: identifier @reference(Nowhere)
- fk_nowhere_id: identifier @fk(Nowhere.id)
- category_id: identifier @reference(external://taxonomy.Category)
