## Product
- id: identifier @pk

## Audit ::aspect
- reviewed: boolean = false

## Stray ::aspect(Nowhere)
- note: string?

## Note ::aspect(Product)
- text: string?

## Remark ::subtype(Note)
- extra: string?
