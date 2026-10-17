# Namespace: conformance.deprecated

## Author
- id: identifier @pk
- registered: datetime

## Book
- id: identifier @pk
- author_id: identifier @reference(Author) @cascade
- >writer
