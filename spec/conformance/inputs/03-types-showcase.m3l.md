# Namespace: sample.types

> Comprehensive showcase of all M3L data types, modifiers,
> validation rules, complex structures, and edge cases.

---

## AllPrimitiveTypes

> Every primitive type in M3L.

- id: identifier @pk @generated
- str: string(200)
- str_no_len: string
- txt: text
- int_val: integer
- lng_val: long
- dec_val: decimal(10,2)
- flt_val: float
- bool_val: boolean
- dt_val: date
- tm_val: time
- ts_val: timestamp
- bin_val: binary(1048576)

---

## SemanticTypes

> Semantic shorthand types that expand to primitives.

- id: identifier @pk
- contact_email: email "Expands to string(320) with RFC 5321 validation"
- contact_phone: phone "Expands to string(20) with E.164 validation"
- homepage: url "Expands to string(2048) with RFC 3986 validation"
- monthly_revenue: money "Expands to decimal(19,4)"
- completion_rate: percentage "Expands to decimal(5,2), range 0-100"

---

## TypeModifiers

> Nullable, array, and combined modifiers.

- id: identifier @pk
- required_str: string(100) @not_null
- nullable_str: string(100)?
- str_array: string[]
- int_array: integer[]
- nullable_array: string[]?
- array_of_nullable: string?[]
- nullable_ts: timestamp?
- required_bool: boolean = false

---

## MapTypes

> Map type variations.

- id: identifier @pk
- string_map: map<string, string>
- config: map<string, integer>

---

## ComplexNestedObject

> Object types with nested structures (up to 3 levels).

- id: identifier @pk
- profile: object
  - first_name: string(50) @not_null
  - last_name: string(50) @not_null
  - contact: object
    - email: email
    - phone: phone?
    - social: object
      - twitter: string(50)?
      - linkedin: url?
- preferences: json "Unstructured settings data"
- metadata: json

---

## ArrayOfObjects

> Array fields containing structured objects.

- id: identifier @pk
- addresses: object[]
  - street: string(200)
  - city: string(100)
  - country: string(2)
  - is_primary: boolean = false
- scores: integer[]
- tags: string[]

---

## ValidationShowcase

> Demonstrates all validation attributes and patterns.

- id: identifier @pk
- age: integer @min(0) @max(150) "Age in years"
- rating: decimal(3,2) @min(0) @max(5) "Star rating"
- username: string(30) @validate(pattern("^[a-zA-Z0-9_]+$")) @not_null
- email: email @not_null @unique
- percentage: decimal(5,2) @min(0) @max(100)
- positive_int: integer @min(1)
- short_code: string(10) @not_null

### Validations
- age_range
  - rule: "age >= 13"
  - message: "Must be at least 13 years old"
- email_domain
  - rule: "email LIKE '%@company.com'"
  - message: "Must use company email"

---

## DefaultValues

> Various default value patterns.

- id: identifier @pk @generated
- status: string(20) = "active"
- count: integer = 0
- ratio: float = 1.0
- is_enabled: boolean = true
- created_at: timestamp = now()
- uuid_val: identifier = generate_uuid()
- empty_list: string[] = []
- discount_price: decimal(10,2) = `price * 0.9`
- quoted_default: string(50) = "Hello \"World\""

---

## CompositeKeyExample

> Demonstrates composite primary key definition.

- tenant_id: identifier @primary(1)
- entity_id: identifier @primary(2)
- data: text

---

## CompositeKeySection

> Composite key via ### PrimaryKey section.

- region: string(10)
- code: string(20)
- name: string(100)

### PrimaryKey
- fields: [region, code]

---

## ExtendedFormatField

> Fields using extended (multi-line) format.

- id: identifier @pk
- status: string(20)
  - description: "Current processing status"
  - reference: StatusEnum
  - on_delete: set_null
- config: json
  - description: "Application configuration data"

---

## InheritanceOverride : AllPrimitiveTypes

> Override inherited field definition.

- str: string(500) @override "Overridden with larger size"
- extra_field: boolean = false

---

## ConditionalFields

> Fields with conditional visibility.

- id: identifier @pk
- type: enum = "personal"
  - personal: "Personal Account"
  - business: "Business Account"
- company_name: string(200)? @if(type == "business")
- tax_id: string(20)? @if(type == "business")
- personal_id: string(20)? @if(type == "personal")

---

## DocumentationShowcase

> This blockquote provides a detailed model description.
> It spans multiple lines to demonstrate multi-line blockquote support.

- id: identifier @pk
- name: string(100) "This is an inline description"
- code: string(10) # This is an inline comment
- notes: text? "Optional notes field" # Internal: used for admin purposes
- email: string(320) @unique
  > Primary contact email address.
  > Used for login, notifications, and password recovery.

<!-- Hidden comment: This model is used for documentation testing -->

---

## ComputedVariants

> All types of computed fields.

- id: identifier @pk
- first_name: string(50)
- last_name: string(50)
- price: decimal(10,2)
- tax_rate: decimal(5,4)
- birth_date: date

### Computed
- full_name: string @computed("first_name + ' ' + last_name")
- tax_amount: decimal(10,2) @computed(`price * tax_rate`)
- total_price: decimal(10,2) @computed("price + tax_amount") @persisted
- age: integer @computed_raw("DATEDIFF(year, birth_date, GETDATE())", platform: "sqlserver")
- pg_age: integer @computed_raw("EXTRACT(YEAR FROM AGE(birth_date))", platform: "postgresql")

---

## BehaviorShowcase

> Demonstrates behavior definitions.

- id: identifier @pk @generated
- code: string(20) @unique
- name: string(100)

- @behavior(before_create, generate_code)

### Behaviors
- before_create: validate_uniqueness
  - condition: always
- after_create: send_notification
  - condition: "name IS NOT NULL"
- before_delete: check_dependencies

---

## VersionedEntity

> Schema versioning demonstration.

- id: identifier @pk
- name: string(100)
- data: json

### Version
- major: 2
- minor: 1
- patch: 0
- date: "2026-01-15"

### Migration (v1.0 → v2.0)
- changed: "name from string(50) to string(100)"
- added: "data field"
- removed: "legacy_code field"
