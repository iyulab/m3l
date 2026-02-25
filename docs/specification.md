# Meta Model Markup Language (M3L) Specification

## Table of Contents
1. [Introduction](#1-introduction)
   1. [Core Principles](#11-core-principles)
   2. [Document Structure](#12-document-structure)
   3. [Expression Patterns](#13-expression-patterns)
   4. [Notation Principles](#14-notation-principles)
   5. [Markdown Rendering Principles](#15-markdown-rendering-principles)
2. [Basic Syntax](#2-basic-syntax)
   1. [Namespace Definition](#21-namespace-definition)
   2. [Model Definition](#22-model-definition)
   3. [Field Definition](#23-field-definition)
   4. [Data Type Notation](#24-data-type-notation)
   5. [Attribute Notation](#25-attribute-notation)
3. [Special Elements](#3-special-elements)
   1. [Enum Definition](#31-enum-definition)
   2. [Relationship Definition](#32-relationship-definition)
   3. [Index Definition](#33-index-definition)
   4. [Inheritance and Interfaces](#34-inheritance-and-interfaces)
   5. [Metadata Definition](#35-metadata-definition)
4. [Advanced Features](#4-advanced-features)
   1. [Composite Key Definition](#41-composite-key-definition)
   2. [Comments and Documentation](#42-comments-and-documentation)
   3. [Behavior Definition](#43-behavior-definition)
   4. [Computed Fields](#44-computed-fields)
   5. [Lookup Fields](#45-lookup-fields)
   6. [Rollup Fields](#46-rollup-fields)
   7. [Derived Views](#47-derived-views)
   8. [Conditional Fields](#48-conditional-fields)
   9. [Complex Data Structures](#49-complex-data-structures)
   10. [Validation Rules](#410-validation-rules)
   11. [Templates and Generics](#411-templates-and-generics)
5. [External References](#5-external-references)
   1. [Imports](#51-imports)
   2. [External Schema References](#52-external-schema-references)
6. [Versioning and Migration](#6-versioning-and-migration)
   1. [Schema Versioning](#61-schema-versioning)
   2. [Migration Notation](#62-migration-notation)
7. [Complete Examples](#7-complete-examples)
   1. [Content Management Example](#71-content-management-example)
   2. [Order Processing Example](#72-order-processing-example)
8. [Extensions](#8-extensions)
   1. [Enhanced Documentation](#81-enhanced-documentation)
   2. [Basic Constraints](#82-basic-constraints)
   3. [Advanced Field Types](#83-advanced-field-types)
   4. [Cascade Behavior for Foreign Keys](#84-cascade-behavior-for-foreign-keys)
9. [Best Practices and Anti-patterns](#9-best-practices-and-anti-patterns)
   1. [Recommended Practices](#91-recommended-practices)
   2. [Anti-patterns to Avoid](#92-anti-patterns-to-avoid)
   3. [Naming Conventions](#93-naming-conventions)
10. [Appendix](#10-appendix)
    1. [Terminology](#101-terminology)
    2. [Mapping to Implementation](#102-mapping-to-implementation)
    3. [Formal Grammar (PEG)](#103-formal-grammar-peg)
    4. [Type Catalog](#104-type-catalog)
    5. [Error Catalog](#105-error-catalog)

## 1. Introduction

M3L (Meta Model Markup Language) is a markdown-based language for defining data models and schemas. It is platform-independent, not bound to any specific programming language or database system, and aims to express data structures and relationships clearly and concisely.

### 1.1 Core Principles

- **Conciseness**: Providing maximum expressiveness with minimal syntax
- **Readability**: Easy to understand with a structure close to natural language
- **Platform Independence**: Not bound to any specific programming language or database
- **Flexibility**: Offering various expression styles for different contexts while maintaining consistency
- **Extensibility**: Flexible structure accommodating various data modeling requirements
- **Compatibility**: M3L documents are valid markdown files that render as meaningful, readable documentation in standard viewers (GitHub, Notion, Obsidian, etc.) without requiring a dedicated M3L parser. Syntax choices must not break or degrade the viewing experience in common markdown renderers
- **AI-Friendly**: Structured format that AI agents can easily interpret and process

### 1.2 Document Structure

In M3L, markdown header levels define document structure with specific meanings:

- **# (H1)**: Document title and namespace
  - Example: `# Domain Data Model` or `# Namespace: domain.example`
  - Recommendation: Only one per document

- **## (H2)**: Main element definitions
  - Defines core elements like models, enums, interfaces
  - Example: `## Person` or `## StatusValue ::enum`

- **### (H3)**: Sections within models
  - Divides sections for indexes, behaviors, metadata, etc.
  - Example: `### Indexes`, `### Relations`, `### Metadata`

### 1.3 Expression Patterns

M3L provides multiple ways to express the same concepts:

- **Simple Format**: Concise one-line expressions (recommended)
  - Example: `- name: string(200) @required`

- **Extended Format**: Multi-line expressions for complex definitions
  - Example: Using nested items for validation rules
  ```markdown
  - email
    - type: string(100)
    - unique: true
    - validate: email
  ```

- **Section Format**: Definitions grouped within dedicated sections (for related elements)
  - Example: Using `### Relations` to group all relationships
  ```markdown
  ### Relations
  - >author
    - target: Person
    - from: author_id
  ```

- **Derived Patterns**: Cross-model expressions for lookup, aggregation, and view composition
  - Lookup: `@lookup(fk_field.target_field)` — Reference a value through a relationship
  - Rollup: `@rollup(Target.fk, aggregate(field))` — Aggregate across a 1:N relationship
  - View: `## ViewName ::view` — Compose multiple models into a read-only virtual model

### 1.4 Notation Principles

- **Not Null by Default**: All fields are considered non-null by default
- **Explicit Null Allowance**: Use `?` suffix after the type to allow null values
- **Conciseness**: Use the most concise notation unless special cases require otherwise
- **Meaning-Centered**: M3L focuses on the meaning and relationships of data rather than implementation details
- **Flexibility**: Provides various expression styles to choose the most appropriate for each situation
- **Consistency**: Similar concepts are expressed in similar ways throughout a document
- **Progressive Disclosure**: Simple elements can be expressed simply, complex elements have dedicated constructs

### 1.5 Markdown Rendering Principles

M3L files serve dual purposes: machine-parseable schema definitions and human-readable documentation. The following principles govern syntax decisions:

1. **No Rendering Breakage**: M3L syntax must not produce broken links, unintended formatting, or layout corruption in major markdown viewers
2. **Visual Semantics**: Different structural elements (fields, enums, sections) should be visually distinguishable when rendered
3. **Scannable Layout**: Documents should be navigable via standard markdown features (TOC generation, header anchors, search)
4. **Graceful Density**: Single lines should remain readable without horizontal scrolling on typical screen widths (~100 characters)
5. **Standard Elements Only**: M3L uses only standard markdown elements (headers, lists, blockquotes, inline code, horizontal rules, HTML comments). No extended markdown syntax (footnotes, definition lists, task lists) is required.

#### 1.5.1 Markdown-Safe Characters

The following table shows M3L special characters and their markdown safety status:

| Character | M3L Usage | Markdown Meaning | Safety |
|---|---|---|---|
| `#` | Inline comment | Header | ⚠️ Safe in list items only |
| `@` | Attribute prefix | Email (in some parsers) | ✅ Safe |
| `?` | Nullable suffix | — | ✅ Safe |
| `!` | NO ACTION cascade | Bold/emphasis (in combos) | ✅ Safe |
| `!!` | RESTRICT cascade | — | ✅ Safe |
| `>` | Blockquote docs | Blockquote | ✅ Intentional |
| `[]` | Framework attrs | Link syntax | 🔴 Wrap in backticks |
| `()` | Type parameters | Link URL | ✅ Safe in list context |
| `::` | Type indicator | — | ✅ Safe |
| `<>` | Many-to-many rel | HTML tag | ⚠️ May be stripped |
| `---` | Model separator | Horizontal rule | ✅ Intentional |
| `` ` `` | Framework attr wrap | Inline code | ✅ Intentional |

**`<>` Notation**: The many-to-many relationship symbol `<>` may be interpreted as an HTML tag in some viewers and stripped from output. In rendered documentation contexts, prefer the explicit Relations section format:

```markdown
### Relations
- tags
  - target: Tag
  - cardinality: many-to-many
```

## 2. Basic Syntax

### 2.1 Namespace Definition

Namespaces logically group models and schemas.

```markdown
# Namespace: domain.example
```

Or

```markdown
# Domain Data Model
```

#### 2.1.1 Nested Namespaces

Nested namespaces can be defined using dot notation:

```markdown
# Namespace: domain.example.inventory
```

#### 2.1.2 Multiple Namespaces in One Document

For multiple namespaces in a single document:

```markdown
# Namespace: domain.common

# Namespace: domain.specific
```

> **Recommendation**: Prefer one namespace per document for clarity.

#### 2.1.3 Namespace Resolution Rules

**Rule 1 — Membership**: A model belongs to the nearest `# Namespace:` declaration above it. Models without a namespace declaration belong to the default namespace (`_default`).

```markdown
# Namespace: domain.sales

## Order          ← domain.sales.Order
## OrderItem      ← domain.sales.OrderItem

# Namespace: domain.inventory

## Product        ← domain.inventory.Product
```

**Rule 2 — Intra-document resolution**: Short names are resolved by searching the same namespace first, then the entire document. If not found, a parser error is raised.

```markdown
# Namespace: domain.sales

## Order
- product_id: identifier @reference(Product)
  # 1st: Search domain.sales.Product → not found
  # 2nd: Search entire document → domain.inventory.Product found → resolved
```

**Rule 3 — Cross-document resolution**: References to models in other files require `@import`. Use qualified names or aliases.

```markdown
@import "inventory/models.m3l" as inventory

## Order
- product_id: identifier @reference(inventory.Product)
```

**Rule 4 — Ambiguity resolution**: If the same short name exists in multiple namespaces and the reference is ambiguous, the parser raises an error. Use a qualified name to resolve.

```markdown
# Error: Product exists in both domain.sales and domain.inventory
- product_id: identifier @reference(Product)  # AMBIGUOUS ERROR

# Fix: Use qualified name
- product_id: identifier @reference(domain.inventory.Product)
```

### 2.2 Model Definition

Models are the basic units of data structure in M3L.

#### 2.2.1 Basic Model Definition Syntax

```markdown
## ModelName(Label)

Brief description of the model.

- field1: type @attribute
- field2: type @attribute
```

Components:
- **ModelName**: Identifier for the model (required)
- **Label**: Display name for the model (in parentheses, optional)
- **Description**: Description of the model (optional)
- **Fields**: List of fields composing the model (required)

Example:

```markdown
## Product(Product)

A product represents an item available for purchase.

- id: identifier @primary
- name: string(200)
- price: decimal(10, 2)
```

#### 2.2.2 Model Definition with Inheritance

Models can inherit from other models or interfaces:

```markdown
## Product : BaseModel, Searchable

A product represents an item available for purchase.

- name: string(200)
- price: decimal(10, 2)
```

#### 2.2.3 Model Metadata

Model-level metadata can be defined:

```markdown
## Product

- name: string(200)
- price: decimal(10, 2)

### Metadata
- domain: "retail"
- versioning: true
```

#### 2.2.4 Model Visibility and Access Control

Model visibility can be defined:

```markdown
## Product @public

## SystemSettings @private
```

#### 2.2.5 Model Separation

Models are primarily separated by `##` headers. For documents with complex or lengthy models, horizontal rules (`---`) may be used between models to provide additional visual separation:

```markdown
## Product : BaseModel
- name: string(200)
- price: decimal(10,2) @min(0)

### Relations
- @relation(category, -> Category, from: category_id)

---

## Order : BaseModel
- order_number: string(20) @unique
- customer_id: identifier @reference(Customer)
```

**Usage Guidelines:**
- Horizontal rules are optional — omit them for short, simple models
- Place the rule between the last content of one model and the next model's `##` header
- Use consistently within a document: either all model boundaries have rules, or none do
- Do NOT use horizontal rules within a model (between fields and sections)
- The M3L parser treats horizontal rules as whitespace (ignored)

### 2.3 Field Definition

Fields define individual attributes of a model.

#### 2.3.1 Basic Field Format (Recommended)

```markdown
- fieldName: type @attribute @attribute(value) "description"
```

Components:
- **fieldName**: Name of the field used as an identifier
- **type**: Data type of the field (required)
- **attribute**: Additional information starting with @ (optional)
- **description**: Field description in quotes (optional)

Example:

```markdown
- id: identifier @primary
- name: string(200) @searchable "Product name"
- price: decimal(10, 2) @min(0) "Price in USD"
- category_id: identifier? @reference(Category) "Optional category"
- created_at: timestamp = now() "Creation timestamp"
```

#### 2.3.1.1 Shortened Type Notations

For common data patterns, M3L provides convenient shorthand types:

```markdown
- email: email @unique           # Equivalent to: string(320) with email validation
- phone: phone                   # Equivalent to: string(20) with phone validation
- url: url                       # Equivalent to: string(2048) with URL validation
- money: money                   # Equivalent to: decimal(19,4)
- percentage: percentage         # Equivalent to: decimal(5,2) with 0-100 range
```

#### 2.3.2 Extended Field Format (For Complex Cases Only)

**Use Basic Format whenever possible.** Extended format is only for very complex field definitions:

```markdown
- fieldName
  - type: type
  - attribute1: value
  - attribute2: true
  - description: "Detailed description"
```

Example:

```markdown
- email
  - type: string(100)
  - unique: true
  - index: true
  - description: "Primary contact email address"
  - validate: email
```

**Prefer Basic Format:** `- email: email @unique @index "Primary contact email"`

#### 2.3.3 Field Metadata

Field-specific metadata can be added:

```markdown
- title: string(200)
  - metadata:
    - importance: 2.0
    - display_priority: 1
```

#### 2.3.4 Line Length Guidelines

To maintain readability in markdown viewers, field definitions should follow these guidelines:

**Short Form (recommended, ~80 characters or fewer):**
Use when a field has up to 3 attributes.

```markdown
- email: string(320) @unique @index "Primary contact email"
- price: decimal(10,2) @min(0)
```

**Threshold: When to Switch to Extended Form**
Switch to extended format when any of these apply:
- Total line length exceeds approximately 80 characters
- Field has more than 3 `@` attributes
- Description text is longer than a short phrase
- Complex validation or business rules need explanation

**Extended Form (for complex definitions):**

```markdown
- blocked_user_id: identifier
  - reference: User
  - on_delete: no_action
  - index: true
  - searchable: true
  - description: "The user who has been blocked from this account"
```

### 2.4 Data Type Notation

M3L provides consistent rules for denoting data types:

#### 2.4.1 Basic Type Notation
- Type names are written in lowercase: `string`, `integer`, `boolean`, etc.
- Type parameters are written in parentheses: `string(100)`, `decimal(10,2)`
- Implementation types vary based on the platform or language used

#### 2.4.2 Nullable Types
- Add a question mark (`?`) after the type to indicate nullable types: `string?`, `integer?`
- Fields not explicitly marked are considered required by default

#### 2.4.3 Array Types
- Add square brackets (`[]`) after the type to indicate array types: `string[]`, `integer[]`
- Arrays of nullable items: `string?[]` (array of nullable strings)
- Nullable arrays: `string[]?` (a nullable array of strings)

#### 2.4.4 Common Type Usage Examples
```markdown
- username: string(50)      # Required string with max length 50
- bio: text?                # Nullable text
- tags: string[]            # Array of strings
- amount: decimal(10,2)     # Decimal number with precision 10, scale 2
- is_active: boolean = true # Boolean with default value true
```

#### 2.4.5 Derived Field Type Inference

Derived fields can infer their type from the source:

- **Lookup**: Follows the source field type. `@lookup(product_id.name)` inherits the type of `Product.name`.
- **Rollup**: Determined by aggregate function — `count` → `integer`, `sum` → source type, `avg` → `decimal`.
- **Computed**: Inferred from expression or explicitly specified.

```markdown
# Explicit type (recommended)
- product_name: string @lookup(product_id.name)

# Type inference (follows source field type)
- product_name: @lookup(product_id.name)
```

### 2.5 Attribute Notation

In M3L, the `@` prefix is used to define attributes of fields or models. These can be interpreted as constraints, validations, behaviors, etc.

#### 2.5.1 Attribute Notation Rules

- All attributes start with the `@` symbol
- Attributes can be placed at the end of field definitions or on separate lines
- Attributes can have values or be used standalone

#### 2.5.2 Attribute Formats

1. **Standalone Attributes**: `@attributeName`
   ```markdown
   - username: string(50) @unique
   ```

2. **Attributes with Values**: `@attributeName(value)`
   ```markdown
   - description: text @description("User profile description")
   ```

3. **Default Value Attributes**: `= value` (recommended) or `@default(value)`
   ```markdown
   - status: string = "active"
   - count: integer @default(0)
   ```

4. **Attribute-Only Lines**:
   ```markdown
   - @meta(version, "1.0")
   ```

#### 2.5.3 Attribute Usage Examples

```markdown
- id: identifier @primary @generated
- username: string(50) @unique @searchable
- age: integer = 18 @min(13) @max(120)
- parent_id: identifier? @reference(Category)
```

#### 2.5.4 Attribute Precedence

When attributes might conflict, the precedence order is:
1. Explicit field-level attributes
2. Inherited attributes
3. Model-level default attributes

```markdown
## ModelWithDefaults
- @default_attribute(visibility, hidden)

- field1: string  # Gets default visibility: hidden
- field2: string @visibility(visible)  # Overrides default
```

### 2.5.5 Advanced Default Value Expressions

Default values can be more than simple constants. M3L supports various forms of default value expressions.

#### 2.5.5.1 System Functions as Default Values

```markdown
- created_at: timestamp = now()
- uuid: string = generate_uuid()
- expiry_date: date = date_add(today(), 1, 'year')
```

#### 2.5.5.2 Computed Values as Defaults

```markdown
- discount_price: decimal(10, 2) = price * 0.9
```

#### 2.5.5.3 Conditional Default Values

```markdown
- status: string = if(is_verified, "active", "pending")
```

#### 2.5.6 Custom Framework Attributes

M3L allows custom framework-specific attributes using square brackets wrapped in backticks. The backtick wrapping prevents markdown renderers from misinterpreting brackets as link syntax:

```markdown
- password: string(100) `[DataType(DataType.Password)]` `[JsonIgnore]`
- created_at: timestamp = now() `[Insert("@now")]`
```

These attributes are rendered as inline code in markdown viewers, visually distinguishing them from M3L's native `@` attributes. The parser strips backticks and processes the inner `[...]` content.

> **Migration**: Existing `[Attr]` syntax (without backticks) remains supported for backward compatibility. The backtick-wrapped form is recommended for all new documents.

## 3. Special Elements

M3L uses type indicators after `::` to define special element types:

- `::enum` — Enumeration type (predefined set of values)
- `::interface` — Interface type (shared field definitions)
- `::view` — Derived view type (read-only, virtual model composed from other models)

### 3.1 Enum Definition

Enums provide a predefined set of values that can be used for specific fields.

#### 3.1.1 Standalone Enum Definition

```markdown
## EnumName ::enum
- value1: "Description of value1"
- value2: "Description of value2"
- value3: "Description of value3"
```

Example:

```markdown
## UserStatus ::enum
- active: "Account in normal use"
- suspended: "Temporarily suspended account"
- inactive: "Account not used for a long time"
- banned: "Permanently banned account"
```

#### 3.1.2 Enum Definition with Namespace

```markdown
## Namespace.EnumName ::enum
- value1: "Description of value1"
- value2: "Description of value2"
```

Example:

```markdown
## Order.Status ::enum
- pending: "Order has been created"
- processing: "Order is being processed"
- shipped: "Order has been shipped"
- delivered: "Order has been delivered"
- cancelled: "Order has been cancelled"
```

#### 3.1.3 Enum Definition with Value Types

```markdown
## EnumName ::enum
- value1: integer = 1 "Description of value1"
- value2: integer = 2 "Description of value2"
- value3: integer = 3 "Description of value3"
```

Example:

```markdown
## OrderStatus ::enum
- pending: integer = 100 "Pending order"
- processing: integer = 200 "Order being processed"
- completed: integer = 300 "Completed order"
- cancelled: integer = 400 "Cancelled order"
```

#### 3.1.4 Grouping Enum Values

```markdown
## EnumName ::enum
- group1
  - value1: "Description of value1"
  - value2: "Description of value2"
- group2
  - value3: "Description of value3"
  - value4: "Description of value4"
```

Example:

```markdown
## ProductStatus ::enum
- active
  - available: "In stock and available for purchase"
  - backorder: "Can be ordered but shipping delayed"
- inactive
  - discontinued: "No longer produced"
  - coming_soon: "Not yet available for sale"
```

#### 3.1.5 Enum Values with Special Characters

Values with special characters or spaces should be enclosed in quotes:

```markdown
## Grade ::enum
- "A+": "Excellent"
- "A": "Very Good"
- "B+": "Good"
- "Not Yet Evaluated": "Awaiting evaluation"
```

#### 3.1.6 Enum Inheritance

Enums can inherit values from other enums:

```markdown
## BasicStatus ::enum
- active: "Active"
- inactive: "Inactive"

## UserStatus ::enum : BasicStatus
- suspended: "Suspended"
- banned: "Banned"
```

#### 3.1.7 Inline Enum Definition

For enums used only within a single field, define values inline. The `values:` key is recommended to visually distinguish enum values from field attributes in the extended format:

**Recommended (with `values:` key):**
```markdown
- status: enum = "active"
  - values:
    - active: "Active"
    - inactive: "Inactive"
    - suspended: "Suspended"

- priority: enum = "medium"
  - values:
    - low: "Low priority"
    - medium: "Medium priority"
    - high: "High priority"
    - critical: "Critical - immediate attention"
```

**Also valid (without `values:` key):**
```markdown
- status: enum = "active"
  - active: "Active"
  - inactive: "Inactive"
  - suspended: "Suspended"
```

> The `values:` key serves as a visual landmark that distinguishes "enum values follow" from extended format attributes (`type:`, `unique:`, etc.). Both forms are valid; the `values:` form is recommended for clarity in rendered markdown.

#### 3.1.8 Enum Collision Rules

**Rule 1**: Inline enums are scoped to their field. They cannot be referenced by other fields.

```markdown
## Order
- status: enum = "pending"       # This enum is only valid for Order.status
  - pending: "Pending"
  - paid: "Paid"

## Shipment
- status: enum = "preparing"     # Separate inline enum for Shipment.status
  - preparing: "Preparing"
  - shipped: "Shipped"
```

**Rule 2**: To reuse enum values across fields, extract them as a standalone enum.

```markdown
## OrderStatus ::enum
- pending: "Pending"
- paid: "Paid"

## Order
- status: OrderStatus = "pending"

## OrderHistory
- previous_status: OrderStatus
```

**Rule 3**: Inline enums are anonymous — they do not conflict with standalone enum names. When a field type references a standalone enum name, it refers to the standalone definition; `enum` as a type creates an anonymous inline enum.

```markdown
- status: OrderStatus = "pending"   # References standalone OrderStatus enum
- status: enum = "pending"          # Defines an anonymous inline enum
```

### 3.2 Relationship Definition

Relationships define connections between models.

#### 3.2.1 Field Level Relationships (Recommended for Simple References)

```markdown
- author_id: identifier @reference(Person)          # CASCADE (default)
- blocked_user_id: identifier @reference(User)!    # NO ACTION (prevent deletion)
- reviewed_by_id?: identifier @reference(User)?    # SET NULL (nullable field)
```

#### 3.2.1.1 CASCADE Behavior

M3L uses a 2-tier system for specifying cascade behavior:

**Tier 1 — Symbol Syntax (Simple Cases, Recommended):**

| Syntax | Behavior | Description |
|---|---|---|
| `@reference(Model)` | Automatic | Nullable → SET NULL, Non-nullable → CASCADE |
| `@reference(Model)!` | NO ACTION | Prevent deletion of referenced record |
| `@reference(Model)?` | SET NULL | Set to null on deletion (requires nullable field) |
| `@reference(Model)!!` | RESTRICT | Strict prevention of deletion |

```markdown
- author_id: identifier @reference(User)      # Automatic → CASCADE (non-nullable)
- reviewer_id?: identifier @reference(User)    # Automatic → SET NULL (nullable)
- blocker_id: identifier @reference(User)!     # Explicit: NO ACTION
- admin_id: identifier @reference(User)!!      # Explicit: RESTRICT
```

**Tier 2 — Extended Format (Complex Cases):**

For cases requiring `on_update` or fine-grained control, use the extended field format:

```markdown
- author_id: identifier
  - reference: User
  - on_delete: cascade
  - on_update: cascade
```

**Automatic Decision Logic:**
When no explicit symbol is provided, CASCADE behavior is determined by field nullability:
- **Nullable FK** → SET NULL (safe cleanup)
- **Non-nullable FK** → CASCADE (strong relationship)

> **Deprecated syntax**: The standalone attribute forms (`@cascade`, `@no_action`, `@set_null`, `@restrict`) and the parameter form (`@cascade(CASCADE)`, `@cascade(NO-ACTION)`) are deprecated. Use the symbol suffix or extended format instead. Parsers should emit a warning for deprecated forms.

#### 3.2.2 Model Level Relationships (Single Line)

```markdown
- @relation(products, <- Product.category_id) "Products in this category"
- @relation(parent, -> Category, from: parent_id) "Parent category"
```

#### 3.2.3 Relationship Section (Recommended for Multiple Relationships)

```markdown
### Relations
- >author
  - target: Person
  - from: author_id
  - on_delete: restrict
  
- <comments
  - target: Comment.post_id
```

#### 3.2.4 Relationship Types

Relationship Notation:
- `>target` or `-> Target`: "To" relationship (this model references Target)
- `<target` or `<- Target`: "From" relationship (Target references this model)

Cardinality can be specified:
```markdown
- >category: one-to-one
- <posts: one-to-many
- <>tags: many-to-many
```

#### 3.2.5 Relationship Attributes

Relationships can have additional attributes:

```markdown
- >author
  - target: Person
  - from: author_id
  - on_delete: restrict
  - on_update: cascade
  - load: eager
  - order_by: created_at desc
```

#### 3.2.6 Role Separation: @reference vs ### Relations

**`@reference` — Defines the relationship (Source of Truth):**
Every FK field must have `@reference` to declare the relationship's existence and target. This is the single source of truth for relationship definitions.

**`### Relations` — Supplements with metadata:**
The Relations section provides supplementary information (loading strategy, ordering, reverse naming) but does not define new relationships. A Relations entry without a corresponding `@reference` on a FK field is a parser error.

**`@relation` — Declares reverse relationships:**
Used to name and describe the reverse side of a relationship.

```markdown
## Post
- author_id: identifier @reference(Person)  # Defines the relationship

### Relations
- >author                                   # Supplements author_id's @reference
  - load: eager
  - order_by: created_at desc
- @relation(comments, <- Comment.post_id)   # Names the reverse relationship
```

**Parser rules:**
- If `### Relations` defines a relationship that has no matching `@reference` FK field → parser error
- If `### Relations` metadata contradicts `@reference` → parser error

### 3.3 Index Definition

Indexes are used to optimize query performance.

#### 3.3.1 Field Level Indexes (Recommended for Single Column Indexes)

```markdown
- customer_id: identifier @reference(Customer) @index
```

#### 3.3.2 Model Level Indexes (Single Line)

```markdown
- @index(customer_id, order_date, name: "customer_orders") "For customer orders"
- @unique(order_id, product_id) "Ensures uniqueness of products per order"
```

#### 3.3.3 Index Section (Recommended for Multiple Indexes)

```markdown
### Indexes
- customer_orders(For customer order lookup)
  - fields: [customer_id, order_date]
  - unique: false

- status_date(For order lookup by status)
  - fields: [status, order_date]
  - unique: false
```

### 3.3.4 Unique Constraints

Unique constraints ensure that individual fields or combinations of fields maintain uniqueness.

#### 3.3.4.1 Single Field Unique Constraints

Defined at the field level using the `@unique` attribute:

```markdown
- username: string(50) @unique
- email: string(100) @unique @index
```

Or using framework-specific notation:

```markdown
- normalized_email: string(256) [UQ]
```

#### 3.3.4.2 Multi-Column Unique Constraints

To ensure the uniqueness of a combination of multiple columns, defined at the model level:

```markdown
- @unique(column1, column2) "Ensures combined uniqueness of these columns"
```

For example:
```markdown
- @unique(tenant_id, username) "Ensures username uniqueness within each tenant"
```

### 3.4 Inheritance and Interfaces

Inheritance provides a mechanism for sharing common attributes between models.

#### 3.4.1 Interface Definition

```markdown
## Timestampable ::interface
- created_at: timestamp = now()
- updated_at: timestamp = now() @on_update(now())
```

With descriptive comment:

```markdown
## Timestampable ::interface # Common timestamp fields
- created_at: timestamp = now()
- updated_at: timestamp = now() @on_update(now())
```

#### 3.4.2 Base Model Definition

```markdown
## BaseModel : Timestampable
- id: identifier @primary @generated
```

#### 3.4.3 Model with Inheritance

```markdown
## Product : BaseModel, Timestampable
- name: string(200)
```

#### 3.4.4 Multiple Inheritance

Models can inherit from multiple interfaces. All referenced interfaces must be defined elsewhere in the document or imported:

```markdown
## BlogPost : BaseModel, Timestampable, Trackable
```

#### 3.4.5 Inheritance Conflict Resolution

When inheriting conflicting fields:

```markdown
## ContentRevision : ContentBase
- updated_at: timestamp @override  # Explicitly overrides the field from base
```

### 3.5 Metadata Definition

Metadata provides additional information about the model itself or implementation details.

#### 3.5.1 Single Line Approach

```markdown
- @meta(version, "1.0") "Schema version"
- @meta(domain, "sales") "Business domain"
```

#### 3.5.2 Metadata Section (Recommended for Multiple Metadata)

```markdown
### Metadata
- version: "1.0"
- domain: "sales"
- validFrom: "2023-01-01"
- owner: "Data Team"
```

## 4. Advanced Features

### 4.1 Composite Key Definition

Composite keys are primary keys composed of two or more fields.

```markdown
## OrderItem
- order_id: identifier @reference(Order) @primary(1)
- product_id: identifier @reference(Product) @primary(2)
- quantity: integer = 1
```

Multi-line format:

```markdown
## OrderItem
### PrimaryKey
- fields: [order_id, product_id]
```

### 4.2 Comments and Documentation

M3L provides multiple documentation mechanisms with distinct purposes, ordered by preference:

#### 4.2.1 Description Strings `"text"` — Primary Documentation

Rendered as part of the field definition. Recommended for all user-facing descriptions:

```markdown
- email: string(320) @unique "Primary contact email"
- name: string(200) "Product display name, shown in catalog"
```

#### 4.2.2 Blockquotes `>` — Model and Section Documentation

Rendered as styled quote blocks. Use for multi-line descriptions, usage notes, and design rationale:

```markdown
## User
> User account information for the platform.
> Supports both Google OAuth and email authentication.

- username: string(50) @unique @index
  > Unique identifier used for login
```

#### 4.2.3 Header Comments `#` — Inline Model Annotations

Brief annotations appended to model headers:

```markdown
## Timestampable ::interface # Common timestamp fields
```

#### 4.2.4 Inline Comments `#` — Developer Notes

Not intended for end-user documentation. Use sparingly for technical notes that aid schema developers:

```markdown
- cache_key: string(100)  # TTL: 3600s, invalidated on profile update
- tenant_id: identifier @index  # Shard key for multi-tenancy
```

**Guidelines:**
- Prefer description strings (`"..."`) over `#` comments for field explanations
- Use `#` comments only for technical/implementation notes that should not appear in generated documentation
- `#` comments must appear after all field attributes on the same line
- Avoid `#` at the beginning of a line outside of header contexts

#### 4.2.5 Hidden Comments `<!-- -->` — Development Notes

Completely hidden in rendered output. Use for TODOs, temporary notes, and parser directives:

```markdown
<!-- TODO: Migrate to composite key after v2.0 release -->
<!-- This model is deprecated, see UserV2 -->
```

### 4.3 Behavior Definition

Behaviors define events and actions associated with a model.

#### 4.3.1 Single Line Approach

```markdown
- @behavior(before_create, generate_id) "Generate ID before creation"
```

#### 4.3.2 Behavior Section (Recommended for Multiple Behaviors)

```markdown
### Behaviors
- before_create
  - action: generate_id
  - condition: always
  
- after_update
  - action: notify_changes
  - condition: status_changed
```

### 4.4 Computed Fields (Row-Level)

Computed fields derive their values from expressions and other fields **within the same row**, providing calculated columns in the database. For cross-model derivations, see [Lookup Fields (4.5)](#45-lookup-fields) and [Rollup Fields (4.6)](#46-rollup-fields).

#### 4.4.1 Basic Computed Fields

**Simple Format (Recommended):**
```markdown
- full_name: string @computed("first_name + ' ' + last_name")
- age: integer @computed("DATEDIFF(YEAR, birth_date, GETDATE())")
- total_amount: decimal(12,2) @computed("quantity * unit_price")
- DisplayName: string @computed("ISNULL([Name], 'Anonymous User')")
```

**With Persistence (Performance Optimization):**
```markdown
- total_amount: decimal(12,2) @computed("quantity * unit_price") @persisted
- search_vector: string @computed("name + ' ' + description") @persisted
- display_name: string @computed("COALESCE([nickname], [first_name] + ' ' + [last_name])") @persisted
```

**Practical Example:**
```markdown
## User
- Id: identifier @primary
- Name: string(100)
- Email: string(320) @unique
- DisplayName: string @computed("ISNULL([Name], 'Anonymous User')")
- SearchText: string @computed("[Name] + ' ' + [Email]") @persisted @index
```

#### 4.4.2 Advanced Computed Fields

**Extended Format (Complex Cases Only):**
```markdown
- total_price: decimal(12,2)
  - computed: true
  - formula: "quantity * unit_price * (1 - discount_rate)"
  - persisted: true
  - description: "Final price after quantity and discount"
```

#### 4.4.3 Computed Field Types

**String Operations:**
```markdown
- display_name: string @computed("COALESCE(nickname, first_name + ' ' + last_name)")
- slug: string @computed("LOWER(REPLACE(title, ' ', '-'))") @persisted
```

**Numeric Calculations:**
```markdown
- bmi: decimal(5,2) @computed("weight / (height * height)")
- tax_amount: decimal(10,2) @computed("subtotal * tax_rate")
- profit_margin: decimal(5,2) @computed("((selling_price - cost_price) / cost_price) * 100")
```

**Date/Time Calculations:**
```markdown
- age: integer @computed("DATEDIFF(YEAR, birth_date, GETDATE())")
- days_since_created: integer @computed("DATEDIFF(DAY, created_at, GETDATE())")
- is_recent: boolean @computed("created_at > DATEADD(DAY, -30, GETDATE())")
```

**Conditional Calculations:**
```markdown
- status_label: string @computed("CASE WHEN is_active = 1 THEN 'Active' ELSE 'Inactive' END")
- discount_tier: string @computed("CASE WHEN total_purchases > 1000 THEN 'Gold' WHEN total_purchases > 500 THEN 'Silver' ELSE 'Bronze' END")
```

#### 4.4.4 Computed Field Best Practices

**Performance Considerations:**
```markdown
# Frequently queried complex calculations: use @persisted
- search_text: string @computed("title + ' ' + description + ' ' + tags") @persisted

# Simple calculations: keep as runtime computed
- full_name: string @computed("first_name + ' ' + last_name")

# Computed fields that need indexing: @persisted + @index
- age_group: string @computed("CASE WHEN age < 18 THEN 'Minor' WHEN age < 65 THEN 'Adult' ELSE 'Senior' END") @persisted @index
```

**Type Inference:**
```markdown
- age: integer @computed("DATEDIFF(YEAR, birth_date, GETDATE())")  # integer type auto-inferred
- total: decimal @computed("price * quantity")                     # decimal type auto-inferred
- is_adult: boolean @computed("age >= 18")                         # boolean type auto-inferred
```

#### 4.4.5 Platform-Specific Expressions

For expressions that require platform-specific syntax, use `@computed_raw` to explicitly mark the platform dependency:

```markdown
# Platform-neutral (recommended — use @computed)
- full_name: string @computed("first_name + ' ' + last_name")
- total: decimal(12,2) @computed("quantity * unit_price")

# Platform-specific (use @computed_raw)
- json_val: string @computed_raw("metadata->>'category'", platform: "postgresql")
- year_created: integer @computed_raw("YEAR(created_at)", platform: "sqlserver")
- json_extract: string @computed_raw("JSON_EXTRACT(metadata, '$.category')", platform: "mysql")
```

> **Note**: `@computed` expressions are opaque strings passed through to the implementation layer. M3L does not define an expression language. `@computed_raw` provides an explicit signal that the expression is not portable. See [10.7 Platform-Specific Expressions](#107-platform-specific-expressions) for details.

### 4.5 Lookup Fields

Lookup fields reference a value from a related model by following a foreign key relationship. They are **read-only** and computed at runtime unless `@persisted` is specified.

#### 4.5.1 Basic Lookup

```markdown
- fieldName: type @lookup(fk_field.target_field)
```

**Single-hop reference:**

```markdown
## OrderItem
- product_id: identifier @reference(Product)
- quantity: integer @min(1)

# Lookup fields
- product_name: string @lookup(product_id.name)
- product_sku: string @lookup(product_id.sku)
- product_price: decimal(10,2) @lookup(product_id.price)
```

**Multi-hop chain reference:**

```markdown
## OrderItem
- order_id: identifier @reference(Order)

# 2-hop: OrderItem → Order → Customer
- customer_name: string @lookup(order_id.customer_id.name)
- customer_email: string @lookup(order_id.customer_id.email)
```

#### 4.5.2 Lookup with Persistence

Frequently accessed Lookup fields can be denormalized with `@persisted`:

```markdown
# Realtime reference (default)
- product_name: string @lookup(product_id.name)

# Denormalized storage (performance optimization)
- product_name: string @lookup(product_id.name) @persisted
```

`@persisted` Lookup fields require a synchronization strategy when the source changes, handled by Behaviors or the implementation layer.

#### 4.5.3 Extended Format

For complex Lookup definitions, use the extended format following the same flat structure as Computed Fields (4.4.2):

```markdown
- customer_name: string
  - lookup: order_id.customer_id.name
  - fallback: "Unknown Customer"
  - description: "Customer name via order reference"
```

Simple Format and Extended Format correspondence:

```markdown
# Simple — single line
- product_name: string @lookup(product_id.name)

# Extended — when additional settings are needed
- product_name: string
  - lookup: product_id.name
  - fallback: "N/A"
  - persisted: true
  - description: "Product name from reference"
```

#### 4.5.4 Lookup Constraints

- **Maximum chain depth**: Up to 3-hop recommended. Beyond that, use Derived Views (4.7).
- **No circular references**: The parser must raise an error if a Lookup chain loops back to itself.
- **Nullable propagation**: If any reference in the chain is nullable, the result is also nullable.
- **Reference validation**: Each FK field in the Lookup path must have a `@reference` attribute declared. `@lookup(product_id.name)` requires `product_id` to have `@reference(Product)`, otherwise the parser raises an error.

```markdown
# category_id is nullable → result is nullable
- category_name: string? @lookup(category_id.name)

# Chain intermediate is nullable: order.customer_id? → result nullable
- customer_name: string? @lookup(order_id.customer_id.name)
```

### 4.6 Rollup Fields

Rollup fields aggregate values from child records in a 1:N relationship. They are **read-only** and computed at runtime unless `@persisted` is specified.

#### 4.6.1 Basic Rollup

```markdown
- fieldName: type @rollup(TargetModel.fk_field, aggregate(target_field?))
```

Components:
- **TargetModel**: The model to aggregate from
- **fk_field**: The foreign key field in the target model that references the current model
- **aggregate**: The aggregation function
- **target_field**: The field to aggregate (optional for `count`)

#### 4.6.2 Reference Validation

Rollup **consumes** relationships but does not **define** them. The Single Source of Truth for relationships is `@reference`.

Validation rules:
1. In `@rollup(TargetModel.fk_field, ...)`, the `fk_field` in `TargetModel` must have `@reference(CurrentModel)` declared.
2. If the `@reference` does not exist, the parser must raise an error.
3. If a `### Relations` section exists, the parser may use it for additional validation, but it is not required.

```markdown
## OrderItem
- order_id: identifier @reference(Order)   # This @reference must exist

## Order
- item_count: integer @rollup(OrderItem.order_id, count)  # Valid: OrderItem.order_id has @reference(Order)
```

Invalid example (parser error):

```markdown
## OrderItem
- order_id: identifier    # No @reference

## Order
- item_count: integer @rollup(OrderItem.order_id, count)
  # ERROR: OrderItem.order_id does not have @reference(Order) declared
```

#### 4.6.3 Supported Aggregate Functions

| Function | Description | Target Field | Result Type |
|---|---|---|---|
| `count` | Record count | Not required | `integer` |
| `sum(field)` | Sum | Numeric field | Source type |
| `avg(field)` | Average | Numeric field | `decimal` |
| `min(field)` | Minimum | Numeric/Date | Source type |
| `max(field)` | Maximum | Numeric/Date | Source type |
| `list(field)` | Value list | Any type | `source_type[]` |
| `count_distinct(field)` | Distinct count | Any type | `integer` |

#### 4.6.4 Usage Examples

```markdown
## Order
- id: identifier @primary
- customer_id: identifier @reference(Customer)
- status: enum = "pending"
  - pending: "Pending"
  - paid: "Paid"
  - shipped: "Shipped"
  - cancelled: "Cancelled"

# Rollup fields
- item_count: integer @rollup(OrderItem.order_id, count)
- total_amount: decimal(12,2) @rollup(OrderItem.order_id, sum(subtotal))
- avg_item_price: decimal(10,2) @rollup(OrderItem.order_id, avg(unit_price))
- max_quantity: integer @rollup(OrderItem.order_id, max(quantity))
- product_names: string[] @rollup(OrderItem.order_id, list(product_name))
```

```markdown
## Customer
- id: identifier @primary
- name: string(100)

# Rollup fields
- order_count: integer @rollup(Order.customer_id, count)
- total_spent: decimal(12,2) @rollup(Order.customer_id, sum(total_amount))
- last_order_date: timestamp? @rollup(Order.customer_id, max(ordered_at))
- unique_products: integer @rollup(Order.customer_id, count_distinct(product_id))
```

#### 4.6.5 Conditional Rollup

Filters can be added to aggregation:

```markdown
# Simple format
- active_orders: integer @rollup(Order.customer_id, count, where: "status != 'cancelled'")
- paid_total: decimal(12,2) @rollup(Order.customer_id, sum(total_amount), where: "status = 'paid'")
```

Extended format (for complex conditions):

```markdown
- monthly_revenue: decimal(12,2)
  - rollup: OrderItem.order_id
  - function: sum(subtotal)
  - where: "status != 'cancelled' AND ordered_at >= date_add(today(), -30, 'day')"
  - description: "Revenue from non-cancelled items in the last 30 days"
```

> **Expression note**: The `where` clause follows the same platform-specific expression convention as `@computed`. M3L focuses on **declaring the intent** (that this is a conditional aggregation), while the specific expression syntax is delegated to the implementation layer.

#### 4.6.6 Rollup Chaining

Rollup results can be referenced by other Rollup or Computed fields:

```markdown
## Customer
- order_count: integer @rollup(Order.customer_id, count)
- total_spent: decimal(12,2) @rollup(Order.customer_id, sum(total_amount))

# Computed using Rollup results
- avg_order_value: decimal(10,2) @computed("total_spent / NULLIF(order_count, 0)")
- customer_tier: string @computed("CASE WHEN total_spent > 10000 THEN 'Gold' WHEN total_spent > 5000 THEN 'Silver' ELSE 'Bronze' END")
```

#### 4.6.7 Rollup Constraints

- Rollup is only available on **direct 1:N relationships**. For M:N, define through the intermediate table.
- The `where` clause in conditional Rollup can only reference stored/computed fields of the target model. Referencing other Rollup results is not allowed (prevents circular dependencies).
- `@persisted` Rollup requires a materialization strategy; update triggers must be defined in the implementation layer.

### 4.7 Derived Views

Derived Views are virtual models composed from multiple models. They correspond to database Views and use the `::view` type indicator.

#### 4.7.1 Basic Syntax

```markdown
## ViewName ::view
> View description

### Source
- from: PrimaryModel
- join: TargetModel on JoinCondition   # optional
- where: "filter condition"             # optional
- order_by: field direction             # optional
- group_by: [fields]                    # optional (aggregate views)

- fieldName: type @from(Model.field)
- fieldName: type @rollup(...)
- fieldName: type @computed("...")
```

Components:
- `::view` — Declares this model as a derived view, not a stored table
- `### Source` — Data source definition section, following the same H3 section pattern as `### Indexes`, `### Relations`, and `### Metadata`
- `@from(Model.field)` — Maps a field from the source model

> **Section separation principle**: View directives (`from`, `join`, `where`, etc.) are placed inside a `### Source` section, separate from field definitions. This follows the existing M3L pattern where model-level meta information is always separated into H3 sections.

> **`@from` vs `@lookup`**: Both reference fields from other models but work in different contexts. `@lookup` is an inline navigation — it declares an FK path and the runtime auto-joins. `@from` is an explicit projection — it selects from models already joined in `### Source`. Principle: **Lookup navigates, From projects.**

#### 4.7.2 Simple View (Single Source)

```markdown
## ActiveProducts ::view
> Active products available for sale

### Source
- from: Product
- where: "is_active = true AND stock_quantity > 0"
- order_by: name asc

- id: identifier @from(Product.id)
- name: string @from(Product.name)
- price: decimal(10,2) @from(Product.price)
- stock: integer @from(Product.stock_quantity)
```

#### 4.7.3 Join View (Multi Source)

```markdown
## OrderDetail ::view
> Order detail information - combines order, customer, and item data

### Source
- from: Order
- join: Customer on Order.customer_id = Customer.id
- join: OrderItem on Order.id = OrderItem.order_id
- join: Product on OrderItem.product_id = Product.id

- order_number: string @from(Order.order_number)
- ordered_at: timestamp @from(Order.ordered_at)
- status: string @from(Order.status)
- customer_name: string @from(Customer.name)
- customer_email: string @from(Customer.email)
- product_name: string @from(Product.name)
- quantity: integer @from(OrderItem.quantity)
- unit_price: decimal(10,2) @from(OrderItem.unit_price)
- subtotal: decimal(12,2) @from(OrderItem.subtotal)
```

#### 4.7.4 Aggregate View

```markdown
## CustomerStats ::view
> Per-customer order statistics

### Source
- from: Customer
- group_by: [Customer.id, Customer.name, Customer.email]

- customer_name: string @from(Customer.name)
- email: string @from(Customer.email)
- total_orders: integer @rollup(Order.customer_id, count)
- total_spent: decimal(12,2) @rollup(Order.customer_id, sum(total_amount))
- last_order: timestamp? @rollup(Order.customer_id, max(ordered_at))
- avg_order_value: decimal(10,2) @computed("total_spent / NULLIF(total_orders, 0)")
- tier: string @computed("CASE WHEN total_spent > 10000 THEN 'Gold' WHEN total_spent > 5000 THEN 'Silver' ELSE 'Bronze' END")
```

#### 4.7.5 Materialized View

Physically stored views for performance optimization:

```markdown
## MonthlySalesReport ::view @materialized
> Monthly sales report - refreshed daily

### Source
- from: Order
- where: "status IN ('paid', 'shipped', 'delivered')"
- group_by: [year_month]

### Refresh
- strategy: scheduled
- interval: "daily 02:00"

- year_month: string @computed("FORMAT(ordered_at, 'yyyy-MM')")
- order_count: integer @rollup(Order.customer_id, count)
- total_revenue: decimal(12,2) @rollup(Order.customer_id, sum(total_amount))
- unique_customers: integer @computed("COUNT(DISTINCT customer_id)")
```

#### 4.7.6 View Nesting

A view can use another view as its data source by specifying it in the `from` directive:

```markdown
## ActiveCustomerStats ::view
> Customer statistics filtered to active customers only

### Source
- from: CustomerStats
- where: "total_orders > 0 AND last_order > date_add(today(), -365, 'day')"

- customer_name: string @from(CustomerStats.customer_name)
- email: string @from(CustomerStats.email)
- total_orders: integer @from(CustomerStats.total_orders)
- total_spent: decimal(12,2) @from(CustomerStats.total_spent)
- last_order: timestamp? @from(CustomerStats.last_order)
- churn_risk: string @computed("CASE WHEN last_order < date_add(today(), -180, 'day') THEN 'High' WHEN last_order < date_add(today(), -90, 'day') THEN 'Medium' ELSE 'Low' END")
```

> **Nesting instead of inheritance**: View extension does not use the `:` (inheritance) syntax. In M3L, `:` means field inheritance (`## Product : BaseModel`), but view extension is query composition (source + filter + projection) — a semantically different operation. Following the SQL pattern `CREATE VIEW AS SELECT ... FROM another_view WHERE ...`, views are nested explicitly via `from`.

#### 4.7.7 View Constraints

- Derived Views are **read-only**. They cannot be targets of INSERT/UPDATE/DELETE operations.
- Views can reference other views via `from`, but **maximum nesting depth of 2** is recommended.
- `@materialized` views must specify a refresh strategy in the `### Refresh` section.
- Joins require **explicit conditions only** (no implicit joins).
- M3L Views focus on **declarative composition**. Complex subqueries, UNION, window functions, etc. are delegated to the implementation layer.

### 4.8 Conditional Fields

Fields that exist or are required only under certain conditions.

#### 4.8.1 Basic Conditional Fields

```markdown
- expiry_date: date @if(status == "temporary")
```

#### 4.8.2 Complex Conditional Fields

```markdown
- company_name: string(100)
  - required: "account_type == 'business'"
  - visible: "account_type == 'business'"
```

### 4.9 Complex Data Structures

Defining complex nested data structures.

#### 4.9.1 Object Types

```markdown
- address: object
  - street: string(100)
  - city: string(50)
  - postal_code: string(20)
  - country: string(2)
```

#### 4.9.2 Array of Objects

```markdown
- addresses: object[]
  - type: string  # Each object in the array has these fields
  - street: string
  - city: string
  - country: string
```

#### 4.9.3 Map Types

```markdown
- preferences: map<string, string>
```

With specified structure:

```markdown
- translations: map<string, object>
  - key_format: "[a-z]{2}"  # Language code format
  - value:
    - title: string
    - description: string
```

#### 4.9.4 Nesting Depth Guidelines

Object nesting should not exceed 3 levels for readability in markdown viewers. Deeper structures should be extracted into separate models.

**Acceptable (3 levels):**

```markdown
- shipping: object
  - address: object
    - street: string(200)
    - city: string(100)
    - postal_code: string(20)
  - method: string
```

**Avoid (4+ levels):**

```markdown
- config: object
  - display: object
    - theme: object
      - colors: object          # Level 4 — extract to separate model
        - primary: string
        - secondary: string
```

**Refactored:**

```markdown
## ThemeColors
- primary: string(7)
- secondary: string(7)
- accent: string(7)

## DisplayConfig
- theme_colors: ThemeColors     # Reference instead of deep nesting
- font_size: integer = 14
```

### 4.10 Validation Rules

Rules for validating field values.

#### 4.10.1 Basic Validation

```markdown
- username: string(50) @validate(pattern("[a-zA-Z0-9_]+"))
```

#### 4.10.2 Multiple Validation Rules

```markdown
- email: string
  - validate:
    - required: true
    - format: email
    - unique: true
```

#### 4.10.3 Cross-Field Validation

```markdown
### Validations
- password_match:
  - rule: "password == password_confirmation"
  - message: "Passwords do not match"
  - trigger: [create, password_update]
```

### 4.11 Templates and Generics

Defining reusable templates with generic parameters.

#### 4.11.1 Generic Types

```markdown
## List<T>
- items: T[]
- count: integer @computed("items.length")
```

#### 4.11.2 Using Generic Types

```markdown
## ProductList : List<Product>
- category: string
```

## 5. External References

### 5.1 Imports

Importing definitions from other M3L files.

```markdown
@import "common/base_models.m3l"
@import "common/interfaces.m3l" as interfaces
```

- **Relative paths** (`./`, `../`): Resolved from the current file's directory
- **Package paths** (no prefix): Resolved from the project root
- **Aliases** (`as name`): Qualify references with the alias (e.g., `interfaces.Timestampable`)
- **Without alias**: All exported models are available by short name in the current scope
- **Circular imports**: Detected by the parser and raised as error `M3L-E003`
- **Diamond dependency**: Same file imported through multiple paths is loaded once

See [10.6 Import Resolution](#106-import-resolution) for detailed rules.

### 5.2 External Schema References

Referencing external schemas or models.

```markdown
- category_id: identifier @reference(external://taxonomy.Category)
```

## 6. Versioning and Migration

### 6.1 Schema Versioning

Version information for the schema.

```markdown
# Domain Data Model
### Version
- major: 2
- minor: 1
- patch: 3
- date: 2023-10-15
```

### 6.2 Migration Notation

Defining changes between schema versions.

```markdown
### Migration (v1.0 → v2.0)
- changed:
  - Person.email: @unique (added)
  - Content.body: text → rich_text (type change)
- added:
  - Person.phone: string(20)?
- removed:
  - Person.fax
```

## 7. Complete Examples

### 7.1 Content Management Example

```markdown
# Content Management Data Model

## Timestampable ::interface # Common timestamp fields
- created_at: timestamp = now()
- updated_at: timestamp = now() @on_update(now())

## Trackable ::interface # Common tracking fields
- created_by: identifier? @reference(Person)
- updated_by: identifier? @reference(Person)

## BaseModel : Timestampable
- id: identifier @primary @generated

## Person : BaseModel, Trackable
- username: string(50) @unique @searchable
  > Unique identifier used for login
- email: string(100) @unique @searchable
- password: string(100)
- first_name: string(50)
- last_name: string(50)
- bio: text?
- profile_image: string?(255)
- status: enum = "active"
  - active: "Active account"
  - suspended: "Suspended"
  - inactive: "Inactive account"
- last_login: timestamp?
- full_name: string @computed("first_name + ' ' + last_name")

- @index(email, username, name: "login_info")

- @relation(contents, <- Content.author_id) "Content created by person"
- @relation(comments, <- Comment.author_id) "Comments written by person"

## Content : BaseModel, Trackable
- title: string(200) @searchable
- slug: string(200) @unique
  > Content identifier used in URL
- body: text @searchable
- summary: text?
- status: enum = "draft"
  - draft: "Draft"
  - published: "Published"
  - scheduled: "Scheduled"
  - archived: "Archived"
- author_id: identifier @reference(Person) @index
- category_id: identifier? @reference(Category) @index
- view_count: integer = 0
- published_at: timestamp?
- tags: string[]?

- @relation(author, -> Person, from: author_id) "Author of this content"
- @relation(category, -> Category, from: category_id) "Category of this content"
- @relation(comments, <- Comment.content_id) "Comments on this content"

### Metadata
- searchable: true
- archive_after: 730  # Days
```

### 7.2 Order Processing Example

```markdown
# Order Processing Data Model

## Category : BaseModel
- name: string(100) @unique
- parent_id: identifier? @reference(Category)
- description: text?

## Customer : BaseModel
- name: string(100)
- email: string(320) @unique
- tier: string = "Bronze"
- is_active: boolean = true

# Rollup fields
- order_count: integer @rollup(Order.customer_id, count)
- total_spent: decimal(12,2) @rollup(Order.customer_id, sum(total_amount))
- last_order_date: timestamp? @rollup(Order.customer_id, max(ordered_at))

# Computed from Rollup
- avg_order_value: decimal(10,2) @computed("total_spent / NULLIF(order_count, 0)")
- computed_tier: string @computed("CASE WHEN total_spent > 10000 THEN 'Gold' WHEN total_spent > 5000 THEN 'Silver' ELSE 'Bronze' END")

## Product : BaseModel
- sku: string(50) @unique
- name: string(200)
- description: text
- price: decimal(10, 2) @min(0)
- sale_price: decimal(10, 2)?
- cost: decimal(10, 2)?
- stock_quantity: integer = 0
- category_id: identifier? @reference(Category)
- is_active: boolean = true
- specifications: object
  - weight: decimal(8, 2)?
  - dimensions: object?
    - length: decimal(8, 2)
    - width: decimal(8, 2)
    - height: decimal(8, 2)
  - color: string?
  - material: string?
- images: object[]
  - url: string
  - alt: string
  - is_primary: boolean = false
- tags: string[]?

# Lookup
- category_name: string? @lookup(category_id.name)

# Computed
- profit_margin: decimal(5,2)? @computed("CASE WHEN cost > 0 THEN ((price - cost) / cost) * 100 END")
- is_in_stock: boolean @computed("stock_quantity > 0 AND is_active = true")

- @index(name, sku, name: "product_search", fulltext: true)

- @relation(category, -> Category, from: category_id) "Category this product belongs to"
- @relation(order_items, <- OrderItem.product_id) "Order items containing this product"

## Order : BaseModel
- order_number: string(20) @unique
- customer_id: identifier @reference(Customer)
- status: enum = "pending"
  - pending: "Pending payment"
  - paid: "Paid"
  - processing: "Processing"
  - shipped: "Shipped"
  - delivered: "Delivered"
  - cancelled: "Cancelled"
- shipping_address: object
  - street: string
  - city: string
  - state: string
  - postal_code: string
  - country: string(2)
- billing_address: object?
  - street: string
  - city: string
  - state: string
  - postal_code: string
  - country: string(2)
- payment_method: string
- notes: text?
- ordered_at: timestamp = now()
- shipped_at: timestamp?

# Lookup
- customer_name: string @lookup(customer_id.name)
- customer_email: string @lookup(customer_id.email)

# Rollup
- item_count: integer @rollup(OrderItem.order_id, count)
- total_amount: decimal(12,2) @rollup(OrderItem.order_id, sum(subtotal))

# Computed
- days_since_order: integer @computed("DATEDIFF(DAY, ordered_at, GETDATE())")

- @relation(customer, -> Customer, from: customer_id) "Customer who placed this order"
- @relation(items, <- OrderItem.order_id) "Items in this order"

## OrderItem
- order_id: identifier @reference(Order) @primary(1)
- product_id: identifier @reference(Product) @primary(2)
- quantity: integer @min(1)
- unit_price: decimal(10, 2)
- discount: decimal(10, 2) = 0

# Lookup
- product_name: string @lookup(product_id.name)
- product_sku: string @lookup(product_id.sku)
- customer_name: string @lookup(order_id.customer_id.name)  # 2-hop

# Computed
- subtotal: decimal(12, 2) @computed("quantity * unit_price - discount")

- @relation(order, -> Order, from: order_id) "Order containing this item"
- @relation(product, -> Product, from: product_id) "Product in this order item"

## OrderSummary ::view
> Order summary view for dashboard

### Source
- from: Order
- join: Customer on Order.customer_id = Customer.id
- where: "Order.status != 'cancelled'"
- order_by: Order.ordered_at desc

- order_number: string @from(Order.order_number)
- ordered_at: timestamp @from(Order.ordered_at)
- status: string @from(Order.status)
- customer_name: string @from(Customer.name)
- customer_tier: string @from(Customer.tier)
- item_count: integer @rollup(OrderItem.order_id, count)
- total_amount: decimal(12,2) @rollup(OrderItem.order_id, sum(subtotal))

## CustomerDashboard ::view
> Per-customer comprehensive statistics

### Source
- from: Customer
- where: "is_active = true"
- group_by: [Customer.id, Customer.name, Customer.email, Customer.tier]

- name: string @from(Customer.name)
- email: string @from(Customer.email)
- tier: string @from(Customer.tier)
- order_count: integer @rollup(Order.customer_id, count)
- total_spent: decimal(12,2) @rollup(Order.customer_id, sum(total_amount))
- active_orders: integer @rollup(Order.customer_id, count, where: "status IN ('pending', 'paid', 'processing')")
- last_order: timestamp? @rollup(Order.customer_id, max(ordered_at))
- avg_order_value: decimal(10,2) @computed("total_spent / NULLIF(order_count, 0)")
```

## 8. Extensions

M3L can be extended with minimal complexity while maintaining readability and focus on table specifications.

### 8.1 Enhanced Documentation

Improved documentation using markdown blockquotes for better readability:

#### 8.1.1 Model Documentation

```markdown
## User
> User account information for the platform
> Supports both Google OAuth and email authentication

- Id: identifier @primary
- Email: string(320) @unique
- Name: string(100)
```

#### 8.1.2 Field Comments

Use inline comments for field descriptions:

```markdown
- Email: string(320) @unique  # Primary contact email
- CreatedAt: timestamp = now()  # Account creation timestamp
```

### 8.2 Basic Constraints

Simple field-level constraints for common validation needs:

```markdown
## Product
- Price: decimal(10,2) @min(0)  # Must be positive
- Stock: integer @min(0) @max(9999)  # Inventory limits
- Name: string(200) @required  # Cannot be null or empty
```

### 8.3 Advanced Field Types

Support for modern data types:

```markdown
## UserProfile
- Id: identifier @primary
- Settings: json  # JSON configuration data
- Tags: string[]  # Array of strings
- Metadata: object  # Structured data
  - Theme: string
  - Language: string(5)
  - Timezone: string(50)
```

### 8.4 Cascade Behavior for Foreign Keys

M3L provides comprehensive syntax for controlling foreign key cascade behavior with automatic decision logic:

#### 8.4.1 Symbol-Based Cascade Notation

```markdown
## Block
> User blocking system with cascade protection

- Id: identifier @primary
- BlockerId: identifier @reference(User)!      # NO ACTION - prevents User deletion
- BlockedUserId: identifier @reference(User)!  # NO ACTION - prevents User deletion
- Reason?: string(500)                         # Optional blocking reason
- CreatedAt: timestamp = now()

## Report
> Content reporting system with mixed cascade behavior

- Id: identifier @primary
- ReporterId: identifier @reference(User)!     # NO ACTION - preserve reporter
- TargetType: ReportTargetType
- TargetId: identifier
- Status: ReportStatus = "Pending"
- ReviewedBy?: identifier @reference(User)     # Automatic → SET NULL (nullable)
- SystemAdmin: identifier @reference(User)!!   # RESTRICT - critical system constraint

## Post
> Blog post with automatic cascade decisions

- Id: identifier @primary
- AuthorId: identifier @reference(User)        # Automatic → CASCADE (non-nullable)
- CategoryId?: identifier @reference(Category) # Automatic → SET NULL (nullable)
- ModeratedBy?: identifier @reference(User)    # Automatic → SET NULL (nullable)
```

#### 8.4.2 Cascade Behavior Guidelines

**Automatic Decision (recommended)**: Let M3L decide based on field nullability
```markdown
- AuthorId: identifier @reference(User)        # Non-nullable → CASCADE
- CategoryId?: identifier @reference(Category) # Nullable → SET NULL
```

**CASCADE**: Use for parent-child relationships where child data becomes meaningless without parent
```markdown
- AuthorId: identifier @reference(User) @cascade        # Delete posts when author deleted
- CategoryId: identifier @reference(Category) @cascade  # Delete products when category deleted
```

**NO ACTION (!)**: Use for important reference data that should be preserved
```markdown
- CreatedBy: identifier @reference(User)! @no_action      # Preserve audit trail
- ModeratorId: identifier @reference(User)! @no_action    # Prevent moderator deletion
```

**SET NULL (?)**: Use for optional relationships where record should survive parent deletion
```markdown
- ReviewedBy?: identifier @reference(User)? @set_null    # Clear reviewer on deletion
- AssignedTo?: identifier @reference(User)? @set_null    # Clear assignment on deletion
```

**RESTRICT (!!)**: Use for critical system constraints that must never be deleted
```markdown
- SystemAdminId: identifier @reference(User)!! @restrict  # Critical system constraint
- RootCategory: identifier @reference(Category)!! @restrict # Foundation data
```

## 9. Best Practices and Anti-patterns

### 9.1 Recommended Practices

#### 9.1.1 Organization

- Group related models in the same M3L document
- Put commonly used interfaces and base models in separate files
- Organize fields in a logical order (identifiers first, then core data, then metadata)
- Use sections for complex models with many relationships

#### 9.1.2 Documentation

- Include a description for every model
- Document complex fields, particularly those with business rules
- Use comments to explain non-obvious relationships or constraints
- Provide examples for fields with specific formats

#### 9.1.3 Expression Patterns

- Use the simplest expression pattern that adequately captures the metadata
- Use one-line format for simple fields
- Use model-level attributes for relationships and indexes when possible
- Use section format only when needed for groups of related items
- Be consistent in your chosen pattern throughout a document

#### 9.1.4 Extension Usage

- Use storage hints for multi-tier architectures
- Define business constraints close to field definitions

#### 9.1.5 Cascade Behavior Best Practices

- **Leverage automatic decisions** for simple relationships - nullable fields automatically get SET NULL, non-nullable get CASCADE
- **Use NO ACTION (!)** for audit trail preservation, user blocking systems, and critical reference data
- **Use SET NULL (?)** for optional assignments where the record should survive reference deletion
- **Use RESTRICT (!!)** for critical system constraints that should never be deleted
- **Document cascade decisions** with inline comments explaining the business rationale
- **Test cascade scenarios** thoroughly, especially for complex multi-table relationships
- **Avoid cascade conflicts** by using NO ACTION for circular or complex reference patterns

**Automatic Decision Examples:**
```markdown
# Automatic behavior based on nullability
- AuthorId: identifier @reference(User)      # Non-nullable → CASCADE
- ReviewedBy?: identifier @reference(User)   # Nullable → SET NULL
```

**Syntax Choice Guidelines:**
```markdown
# Simple cases: Use symbol syntax (Tier 1)
- CreatedBy: identifier @reference(User)     # Automatic (CASCADE for non-nullable)
- ReviewedBy: identifier @reference(User)!   # NO ACTION
- AssignedTo?: identifier @reference(User)   # Automatic (SET NULL for nullable)
- SystemAdmin: identifier @reference(User)!! # RESTRICT

# Complex cases: Use extended format (Tier 2)
- author_id: identifier
  - reference: User
  - on_delete: cascade
  - on_update: cascade
```

**Anti-patterns to avoid:**
```markdown
# BAD: Multiple CASCADE paths to same table can cause conflicts
- BlockerId: identifier @reference(User)     # AUTO → CASCADE
- BlockedUserId: identifier @reference(User) # AUTO → CASCADE - can cause conflicts!

# GOOD: Use NO ACTION for blocking/audit systems
- BlockerId: identifier @reference(User)! @no_action     # NO ACTION - preserve blocking data
- BlockedUserId: identifier @reference(User)! @no_action # NO ACTION - preserve blocking data

# MIXED: Different business logic for same reference
- UserId: identifier @reference(User)! @cascade     # User data can be deleted
- CreatedBy: identifier @reference(User)! @no_action # Creator audit preserved
- SystemRole: identifier @reference(User)!! @restrict # Critical system constraint
```
- Apply security attributes consistently across models
- Use events sparingly for critical business state changes

## 10. Appendix

### 10.1 Terminology

| Term | Definition |
|---|---|
| **Model** | A data structure definition, analogous to a database table or class |
| **Field** | An attribute of a model, analogous to a column or property |
| **Enum** | A predefined set of named values |
| **Interface** | A shared set of fields that models can inherit |
| **View** | A virtual, read-only model composed from other models |
| **Lookup** | A derived field that references a value from a related model via FK |
| **Rollup** | A derived field that aggregates values from child records (1:N) |
| **Computed** | A derived field whose value is calculated from an expression |
| **Namespace** | A logical grouping for models and schemas |
| **Attribute** | An `@`-prefixed annotation on fields or models |

### 10.2 Mapping to Implementation

| M3L Concept | SQL | ORM (C#/Java) | NoSQL |
|---|---|---|---|
| Model | Table | Entity class | Collection |
| Field | Column | Property | Field |
| `identifier @primary` | `PRIMARY KEY` | `[Key]` / `@Id` | `_id` |
| `@reference(Model)` | `FOREIGN KEY` | Navigation property | Embedded ref / DBRef |
| `::enum` | `ENUM` / Lookup table | `enum` type | String field |
| `::view` | `CREATE VIEW` | Query object / DTO | Aggregation pipeline |
| `@computed` | Computed column / Generated | `[DatabaseGenerated]` | Virtual field |
| `@lookup` | `JOIN` expression | Lazy/Eager load | `$lookup` |
| `@rollup` | Subquery / Aggregate | LINQ aggregate | `$group` |
| `@materialized` | Materialized view | Cached query | On-demand materialization |

### 10.3 Formal Grammar (PEG)

This section defines the normative grammar for M3L using PEG (Parsing Expression Grammar) notation. The grammar establishes the canonical token order and structure that all conforming parsers must follow.

#### 10.3.1 Document Structure

```peg
Document       ← Namespace? (Import / HRule / ModelDef / EnumDef / InterfaceDef / ViewDef)*
Namespace      ← '# Namespace:' _ QualifiedName NL
                / '#' _ FreeText NL
Import         ← '@import' _ QuotedString (_ 'as' _ Identifier)? NL
HRule          ← '---' '-'* NL
```

#### 10.3.2 Model and Element Definitions

```peg
ModelDef       ← '## ' ModelName Inheritance? ModelAttrs? NL
                  Description? (FieldDef / SectionDef / RelationLine / IndexLine / MetaLine)*
EnumDef        ← '## ' Identifier ('(' Label ')')? _ '::enum' Inheritance? NL
                  Description? EnumValue+
InterfaceDef   ← '## ' Identifier ('(' Label ')')? _ '::interface' NL
                  Description? FieldDef+
ViewDef        ← '## ' Identifier ('(' Label ')')? _ '::view' ViewAttrs? NL
                  Description? SourceSection? FieldDef* SectionDef*

ModelName      ← Identifier ('(' Label ')')?
Inheritance    ← _ ':' _ Identifier (',' _ Identifier)*
ModelAttrs     ← (_ '@' Identifier ('(' AttrParams ')')?)+
ViewAttrs      ← (_ '@materialized')?
```

#### 10.3.3 Field Definition

The canonical token order for field definitions is **fixed**. Deviations from this order are parser errors.

```peg
FieldDef       ← '- ' FieldName ':' _ TypeExpr DefaultValue? Attributes? Description? InlineComment? NL
               / '- ' FieldName NL ExtendedField+
               / '- ' FieldName ':' _ TypeExpr NL ExtendedField+

FieldName      ← Identifier ('(' Label ')')?
TypeExpr       ← BaseType TypeParams? Nullable? Array?
BaseType       ← 'string' / 'integer' / 'decimal' / 'boolean' / 'text' / 'timestamp'
               / 'date' / 'time' / 'identifier' / 'enum' / 'object' / 'json' / 'binary'
               / 'long' / 'float' / 'email' / 'phone' / 'url' / 'money' / 'percentage'
               / 'map' '<' TypeExpr ',' _ TypeExpr '>'
               / Identifier
TypeParams     ← '(' Number (',' _ Number)* ')'
Nullable       ← '?'
Array          ← '[]'
DefaultValue   ← _ '=' _ Expression
Attributes     ← (_ Attribute)+
Attribute      ← '@' AttrName ('(' AttrParams ')')? CascadeSuffix?
CascadeSuffix  ← '!' / '!!' / '?'
Description    ← _ '"' [^"]* '"'
InlineComment  ← _ '#' [^\n]*
```

**Token order**: `name: type = default @attrs "description" # comment`

This matches the parser implementation's positional scanner, which processes tokens left-to-right in this exact order.

#### 10.3.4 Sections and Directives

```peg
SectionDef     ← '### ' SectionName NL SectionContent+
SectionName    ← 'Source' / 'Indexes' / 'Relations' / 'Metadata' / 'Behaviors'
               / 'Validations' / 'PrimaryKey' / 'Refresh' / 'Version' / 'Migration' _ FreeText

SourceSection  ← '### Source' NL SourceDirective+ FieldDef*
SourceDirective← '- ' ('from' / 'join' / 'where' / 'order_by' / 'group_by') ':' _ FreeText NL

EnumValue      ← '- ' Identifier ':' _ QuotedString NL
               / '- ' Identifier ':' _ TypeExpr _ '=' _ Expression _ QuotedString? NL
               / '- ' QuotedString ':' _ QuotedString NL

ExtendedField  ← '  - ' Identifier ':' _ FreeText NL
Description    ← '>' _ FreeText NL
```

#### 10.3.5 Primitives

```peg
Identifier     ← [a-zA-Z_] [a-zA-Z0-9_]*
QualifiedName  ← Identifier ('.' Identifier)*
Label          ← [^)]+
FreeText       ← [^\n]+
QuotedString   ← '"' [^"]* '"'
Number         ← [0-9]+
Expression     ← QuotedString / FunctionCall / Number / 'true' / 'false' / Identifier
FunctionCall   ← Identifier '(' FreeText? ')'
AttrName       ← Identifier
AttrParams     ← BalancedContent
BalancedContent← ([^()] / '(' BalancedContent ')')*
_              ← [ \t]+
NL             ← '\n' / '\r\n'
```

#### 10.3.6 Encoding and Whitespace

- **Encoding**: UTF-8 required
- **Line endings**: LF (`\n`) is canonical; CRLF is accepted and normalized to LF
- **Indentation**: Extended format uses 2-space indentation (4 spaces also accepted; tabs not recommended)
- **Inter-token spacing**: 1+ spaces/tabs between tokens, normalized to single space

### 10.4 Type Catalog

The following table defines all official M3L types. Types not listed here are treated as model references.

#### 10.4.1 Primitive Types

| Type | Parameters | Description | Example |
|---|---|---|---|
| `string` | `(maxLength)` | Length-limited string | `string(200)` |
| `text` | — | Unlimited-length string | `text` |
| `integer` | — | 32-bit integer | `integer` |
| `long` | — | 64-bit integer | `long` |
| `decimal` | `(precision, scale)` | Fixed-point number | `decimal(10,2)` |
| `float` | — | Floating-point number | `float` |
| `boolean` | — | True/false | `boolean` |
| `date` | — | Date without time | `date` |
| `time` | — | Time without date | `time` |
| `timestamp` | — | Date + time + timezone | `timestamp` |
| `identifier` | — | Unique ID (UUID or platform-specific) | `identifier` |
| `binary` | `(maxSize)?` | Binary data | `binary(1048576)` |

#### 10.4.2 Semantic Types (Shorthands)

| Type | Expands To | Implicit Validation |
|---|---|---|
| `email` | `string(320)` | RFC 5321 email format |
| `phone` | `string(20)` | E.164 phone format |
| `url` | `string(2048)` | RFC 3986 URL format |
| `money` | `decimal(19,4)` | Non-negative |
| `percentage` | `decimal(5,2)` | 0–100 range |

#### 10.4.3 Structural Types

| Type | Parameters | Description | Example |
|---|---|---|---|
| `object` | — | Nested structure (inline field definition required) | `object` + sub-fields |
| `json` | — | Unstructured JSON data (no schema) | `json` |
| `enum` | — | Enumeration (inline or standalone) | `enum` + value list |
| `map` | `<keyType, valueType>` | Key-value mapping | `map<string, string>` |

> **`object` vs `json`**: `object` has inline schema (fields defined below it); `json` is schema-free arbitrary JSON data.

#### 10.4.4 Type Modifiers

| Modifier | Position | Description | Example |
|---|---|---|---|
| `?` | After type | Nullable | `string?` |
| `[]` | After type | Array | `string[]` |
| `?[]` | After type | Array of nullable elements | `string?[]` |
| `[]?` | After type | Nullable array | `string[]?` |

#### 10.4.5 Deprecated Types

| Deprecated | Replacement | Notes |
|---|---|---|
| `datetime` | `timestamp` | Parser should emit warning and treat as `timestamp` |

### 10.5 Error Catalog

Conforming parsers should use these error codes for consistent diagnostics.

#### 10.5.1 Errors

| Code | Message Template | Description |
|---|---|---|
| `M3L-E001` | Rollup FK `{field}` missing `@reference` | Rollup references a FK field that lacks `@reference` |
| `M3L-E002` | Lookup FK `{field}` missing `@reference` | Lookup references a FK field that lacks `@reference` |
| `M3L-E003` | Circular import detected: {chain} | Import graph contains a cycle |
| `M3L-E004` | View references non-existent model `{model}` | View `from` or `join` targets undefined model |
| `M3L-E005` | Duplicate model/enum name `{name}` | Same name defined multiple times |
| `M3L-E006` | Duplicate field `{field}` in model `{model}` | Same field name within a model |
| `M3L-E007` | Unresolved parent `{parent}` in inheritance | `## Child : Parent` where Parent is not defined |
| `M3L-E008` | Ambiguous model reference `{name}` in namespaces {ns1}, {ns2} | Short name exists in multiple namespaces |
| `M3L-E009` | Undefined type `{type}` | Type not in catalog and not a known model/enum |
| `M3L-E010` | Relations entry without matching `@reference` | `### Relations` defines relationship with no FK `@reference` |

#### 10.5.2 Warnings

| Code | Message Template | Description |
|---|---|---|
| `M3L-W001` | Field `{field}` has no description | Strict mode: field lacks documentation |
| `M3L-W002` | Model `{model}` has no description | Strict mode: model lacks documentation |
| `M3L-W003` | Deprecated syntax: `{syntax}` | Use of deprecated cascade/type syntax |
| `M3L-W004` | Line length exceeds 100 characters | Readability guideline exceeded |

### 10.6 Import Resolution

#### 10.6.1 Path Resolution

```markdown
# Relative path (from current file)
@import "./common/base.m3l"
@import "../shared/interfaces.m3l"

# Package path (from project root)
@import "common/base.m3l"
```

- Relative paths start with `./` or `../`
- Package paths are resolved from the project root (determined by `.m3lroot` file, `m3l.config.yaml`, or CLI option)

#### 10.6.2 Aliases

```markdown
@import "common/base.m3l" as base

## Product : base.BaseModel
- name: string(200)
```

Without an alias, all models from the imported file are exposed directly in the current scope.

#### 10.6.3 Circular Import Detection

Parsers must build an import graph and raise `M3L-E003` if a cycle is detected.

#### 10.6.4 Diamond Dependency

If the same file is imported through multiple paths, it is loaded only once (de-duplication).

### 10.7 Platform-Specific Expressions

For cases where `@computed` expressions require platform-specific functions, use `@computed_raw` to explicitly mark platform dependency:

```markdown
# Platform-neutral (recommended)
- full_name: string @computed("first_name + ' ' + last_name")

# Platform-specific (escape hatch)
- json_val: string @computed_raw("metadata->>'category'", platform: "postgresql")
- year_created: integer @computed_raw("YEAR(created_at)", platform: "sqlserver")
```

`@computed_raw` expressions are not guaranteed to be portable across platforms. The `platform` parameter is informational metadata for code generators.

> **Note**: `@computed` expressions are treated as opaque strings by the parser. M3L does not define an expression language — the expression content is passed through to the implementation layer. `@computed_raw` provides an explicit signal that the expression is platform-specific.