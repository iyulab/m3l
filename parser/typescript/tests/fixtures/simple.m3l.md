# Library System

## Author
- name(Author Name): string(100) @not_null @idx
- bio(Biography): text?

> Stores author information.

## BookStatus ::enum
- available: "Available"
- borrowed: "Borrowed"
- reserved: "Reserved"
