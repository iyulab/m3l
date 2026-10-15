# Namespace: conformance.renames

## Stamped ::interface
- created_at: timestamp @renamed_from(made_at)

## Customer : Stamped @renamed_from(Client)
- id: identifier @pk
- display_name: string(200) @renamed_from(name)
- tier_code: string(20) @renamed_from(level, tier)
- total: decimal(12,2) @computed(`1`) @persisted @renamed_from(sum)

## Supplier @renamed_from(Vendor, Seller)
- id: identifier @pk
- code: string(20)

## Order @renamed_from(Customer)
- id: identifier @pk
- note: string(100) @renamed_from(memo)
- memo: string(100) @renamed_from(remark)
- remark_text: string(100) @renamed_from(remark)
- status: string(20) @renamed_from(state, state)
- label: string(20) @renamed_from(label)
- ref: string(20) @renamed_from(other.ref)
- tag: string(20) @renamed_from()
- shown: string(20) @computed(`note`) @renamed_from(display)

## Invoice @renamed_from(Client)
- id: identifier @pk

## CustomerList ::view @renamed_from(ClientList)

### Source
- from: Customer
- order_by: "display_name asc"

## Phase ::enum
- open: "Open" @renamed_from(started)
- closed: "Closed"
