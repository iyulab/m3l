# M3L RFC-0001: Derived Fields & Views Extension

> **RFC Status**: Draft (Revised)
> **Author**: UJ (iyulab)
> **Date**: 2026-02-25
> **Revised**: 2026-02-25
> **M3L Version Target**: Next Minor
> **Affects Sections**: 4.4 ~ 4.8 (Advanced Features), New Section for Derived Views

---

## 1. Summary

본 제안은 M3L 스펙에 **Lookup Fields**, **Rollup Fields**, **Derived Views**를 추가하여, 현재 단일 행 내 계산만 가능한 Computed Fields 체계를 **관계 기반 파생 데이터 표현**까지 확장하는 것을 목표로 한다.

이를 통해 M3L은 정적 스키마 정의 언어에서 **데이터 흐름과 파생 구조까지 선언적으로 표현 가능한 모델링 언어**로 진화한다.

---

## 2. Motivation

### 2.1 현재의 한계

M3L 4.4 Computed Fields는 동일 모델 내 필드 간 계산만 지원한다:

```markdown
- subtotal: decimal(12,2) @computed("quantity * unit_price")
```

그러나 실제 데이터 모델링에서는 다음과 같은 **관계 기반 파생 데이터**가 필수적이다:

- 주문 항목에서 제품명을 참조 (Lookup)
- 주문의 총 금액을 항목들로부터 집계 (Rollup)
- 여러 모델을 조합한 대시보드 뷰 (Derived View)

### 2.2 기존 접근의 문제

현재 이런 요구사항은 다음과 같이 처리해야 하는데, 모두 M3L의 선언적 원칙에 맞지 않는다:

| 요구사항 | 현재 처리 방식 | 문제점 |
|---|---|---|
| 제품명 참조 | Behavior에서 수동 처리 | 참조 의도가 불명확, 도구/AI가 데이터 흐름을 파악 불가 |
| 주문 합계 | Computed에 집계 SQL 직접 작성 | 집계 의도가 표현식 안에 묻혀 파서/도구가 식별 불가 |
| 요약 뷰 | M3L 외부에서 정의 | 모델 일관성 상실 |

> **참고**: 문제의 본질은 표현식의 플랫폼 종속성이 아니라 **의도의 불명확성**이다. `@computed("SELECT SUM(...) FROM ...")`처럼 집계 의도가 SQL 표현식에 매몰되면, 파서/코드 생성기/AI 에이전트가 "이 필드는 다른 모델의 데이터를 집계한 것"이라는 사실을 인식할 수 없다. `@lookup`과 `@rollup`은 이 의도를 **선언적으로 드러내는** 것이 핵심 가치다.

### 2.3 선행 사례

이 패턴들은 이미 다양한 플랫폼에서 검증되었다:

- **Airtable**: Lookup, Rollup, Formula를 명확히 구분
- **Notion Database**: Relation → Rollup 체인
- **Power BI / DAX**: Calculated Columns vs Measures
- **SQL**: Computed Columns vs Views vs Materialized Views
- **Entity Framework**: Navigation Properties, Owned Types

---

## 3. Proposal

### 3.1 계산 필드 체계 (Derived Field Hierarchy)

기존 Computed Fields를 포함하여 4단계 파생 필드 체계를 정의한다:

```
┌─────────────────────────────────────────────────────┐
│  Level 0: Stored Fields                             │
│  일반 필드 - 직접 저장되는 값                          │
│  - name: string(200)                                │
├─────────────────────────────────────────────────────┤
│  Level 1: Computed Fields (기존 4.4)                 │
│  동일 행 내 계산                                      │
│  - full_name: string @computed("first + ' ' + last")│
├─────────────────────────────────────────────────────┤
│  Level 2: Lookup Fields (신규)                       │
│  관계를 통한 단일 값 참조                              │
│  - product_name: string @lookup(product_id.name)    │
├─────────────────────────────────────────────────────┤
│  Level 3: Rollup Fields (신규)                       │
│  관계를 통한 집계 계산                                 │
│  - total: decimal @rollup(Items.order_id, sum(amt)) │
├─────────────────────────────────────────────────────┤
│  Level 4: Derived Views (신규)                       │
│  모델 조합 + 필터 + 집계                              │
│  ## OrderSummary ::view                             │
└─────────────────────────────────────────────────────┘
```

| Level | 구분 | 범위 | 지시자 | 읽기 전용 |
|---|---|---|---|---|
| 0 | Stored | 직접 저장 | (없음) | No |
| 1 | Computed | 같은 행 | `@computed("...")` | Yes |
| 2 | Lookup | 관계 1건 | `@lookup(...)` | Yes |
| 3 | Rollup | 관계 N건 | `@rollup(...)` | Yes |
| 4 | View | 모델 조합 | `::view` | Yes |

**공통 원칙**: Level 1~4는 모두 **읽기 전용(read-only)**이며, `@persisted`를 명시하지 않는 한 실시간 계산을 기본으로 한다.

---

### 3.2 Lookup Fields

관계(reference)를 따라가서 대상 모델의 필드 값을 가져오는 파생 필드.

#### 3.2.1 기본 문법

```markdown
- fieldName: type @lookup(fk_field.target_field)
```

**단순 참조 (1-hop):**

```markdown
## OrderItem
- product_id: identifier @reference(Product)
- quantity: integer @min(1)

# Lookup fields
- product_name: string @lookup(product_id.name)
- product_sku: string @lookup(product_id.sku)
- product_price: decimal(10,2) @lookup(product_id.price)
```

**체인 참조 (multi-hop):**

```markdown
## OrderItem
- order_id: identifier @reference(Order)

# 2-hop: OrderItem → Order → Customer
- customer_name: string @lookup(order_id.customer_id.name)
- customer_email: string @lookup(order_id.customer_id.email)
```

#### 3.2.2 타입 추론

Lookup 필드의 타입은 원본 필드에서 자동 추론할 수 있다. 타입을 생략하면 원본 타입을 따른다:

```markdown
# 명시적 타입 (권장)
- product_name: string @lookup(product_id.name)

# 타입 추론 (원본 필드 타입을 따름)
- product_name: @lookup(product_id.name)
```

> **권장**: 가독성과 명확성을 위해 타입을 명시한다.

#### 3.2.3 Lookup과 Persistence

자주 조회되는 Lookup 필드는 `@persisted`로 비정규화할 수 있다:

```markdown
# 실시간 참조 (기본)
- product_name: string @lookup(product_id.name)

# 비정규화 저장 (성능 최적화)
- product_name: string @lookup(product_id.name) @persisted
```

`@persisted` Lookup은 원본 변경 시 동기화 전략이 필요하며, 이는 Behavior 또는 구현 레이어에서 처리한다.

#### 3.2.4 확장 포맷

복잡한 Lookup에는 확장 포맷을 사용한다. 기존 Computed Fields 확장 포맷(4.4.2)과 동일한 평탄 구조를 따른다:

```markdown
- customer_name: string
  - lookup: order_id.customer_id.name
  - fallback: "Unknown Customer"
  - description: "Customer name via order reference"
```

> **평탄 구조 원칙**: 확장 포맷에서 속성 키워드(`lookup`, `fallback`, `description`)는 필드 아래 동일 깊이에 나열한다. 기존 Computed 확장의 `- computed: true` + `- formula: "..."` 패턴과 일관된다.

Simple Format과 Extended Format의 대응:

```markdown
# Simple — 한 줄로 완결
- product_name: string @lookup(product_id.name)

# Extended — 복잡한 설정이 필요한 경우
- product_name: string
  - lookup: product_id.name
  - fallback: "N/A"
  - persisted: true
  - description: "Product name from reference"
```

#### 3.2.5 제약 사항

- **최대 체인 깊이**: 3-hop까지 권장. 그 이상은 Derived View 사용을 권장한다.
- **순환 참조 금지**: Lookup 체인이 자기 자신으로 돌아오는 경우 파서가 오류를 발생시켜야 한다.
- **Nullable 전파**: 체인 중간에 nullable 참조가 있으면 결과도 nullable이 된다.

```markdown
# category_id가 nullable이므로 결과도 nullable
- category_name: string? @lookup(category_id.name)

# 체인 중간이 nullable: order.customer_id? → 결과 nullable
- customer_name: string? @lookup(order_id.customer_id.name)
```

- **참조 검증**: Lookup 경로의 각 FK 필드에는 `@reference`가 선언되어 있어야 한다. `@lookup(product_id.name)`에서 `product_id` 필드에 `@reference(Product)`가 없으면 파서 오류.

---

### 3.3 Rollup Fields

1:N 관계에서 자식 레코드들을 집계하는 파생 필드.

#### 3.3.1 기본 문법

```markdown
- fieldName: type @rollup(TargetModel.fk_field, aggregate(target_field?))
```

구성 요소:
- **TargetModel**: 집계 대상 모델
- **fk_field**: 대상 모델에서 현재 모델을 참조하는 FK 필드
- **aggregate**: 집계 함수
- **target_field**: 집계 대상 필드 (count에서는 생략 가능)

#### 3.3.2 참조 검증 규칙

Rollup은 관계를 **생성하지 않고 사용**만 한다. 관계의 단일 정의 원칙(Single Source of Truth)은 `@reference`에 위임한다.

검증 규칙:
1. `@rollup(TargetModel.fk_field, ...)`에서 `TargetModel`의 `fk_field`에는 `@reference(현재모델)` 속성이 선언되어 있어야 한다.
2. 해당 `@reference`가 존재하지 않으면 파서가 오류를 발생시킨다.
3. `### Relations` 섹션이 존재하면 파서가 추가 검증에 활용할 수 있으나, 필수는 아니다.

```markdown
## OrderItem
- order_id: identifier @reference(Order)   ← 이 @reference가 있어야

## Order
- item_count: integer @rollup(OrderItem.order_id, count)  ← 이 Rollup이 유효
```

잘못된 예 (파서 오류):

```markdown
## OrderItem
- order_id: identifier    ← @reference 없음

## Order
- item_count: integer @rollup(OrderItem.order_id, count)
  ← ERROR: OrderItem.order_id에 @reference(Order)가 선언되어 있지 않습니다
```

#### 3.3.3 지원 집계 함수

| 함수 | 설명 | 대상 필드 | 결과 타입 |
|---|---|---|---|
| `count` | 레코드 수 | 불필요 | `integer` |
| `sum(field)` | 합계 | 숫자 필드 | 원본 타입 |
| `avg(field)` | 평균 | 숫자 필드 | `decimal` |
| `min(field)` | 최솟값 | 숫자/날짜 | 원본 타입 |
| `max(field)` | 최댓값 | 숫자/날짜 | 원본 타입 |
| `list(field)` | 값 목록 | 모든 타입 | `원본타입[]` |
| `count_distinct(field)` | 고유 값 수 | 모든 타입 | `integer` |

#### 3.3.4 사용 예제

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

#### 3.3.5 조건부 Rollup

집계에 필터 조건을 추가할 수 있다:

```markdown
# 간결 문법
- active_orders: integer @rollup(Order.customer_id, count, where: "status != 'cancelled'")
- paid_total: decimal(12,2) @rollup(Order.customer_id, sum(total_amount), where: "status = 'paid'")
```

확장 포맷 (복잡한 조건):

```markdown
- monthly_revenue: decimal(12,2)
  - rollup: OrderItem.order_id
  - function: sum(subtotal)
  - where: "status != 'cancelled' AND ordered_at >= date_add(today(), -30, 'day')"
  - description: "Revenue from non-cancelled items in the last 30 days"
```

> **표현식 참고**: `where` 절은 기존 `@computed`와 동일하게 플랫폼별 표현식을 허용한다. M3L은 **의도의 선언**(이것이 조건부 집계임)에 집중하며, 표현식의 구체적 문법은 구현 레이어에 위임한다.

#### 3.3.6 Rollup 체인

Rollup 결과를 다른 Rollup이나 Computed에서 참조할 수 있다:

```markdown
## Customer
- order_count: integer @rollup(Order.customer_id, count)
- total_spent: decimal(12,2) @rollup(Order.customer_id, sum(total_amount))

# Computed using Rollup results
- avg_order_value: decimal(10,2) @computed("total_spent / NULLIF(order_count, 0)")
- customer_tier: string @computed("CASE WHEN total_spent > 10000 THEN 'Gold' WHEN total_spent > 5000 THEN 'Silver' ELSE 'Bronze' END")
```

#### 3.3.7 제약 사항

- Rollup은 **직접적인 1:N 관계**에서만 사용 가능. M:N은 중간 테이블을 경유하여 정의한다.
- 조건부 Rollup의 `where` 절은 대상 모델의 stored/computed 필드만 참조 가능. 다른 Rollup 결과는 참조 불가 (순환 방지).
- `@persisted` Rollup은 Materialized 전략이 필요하며, 구현 레이어에서 갱신 트리거를 정의해야 한다.

---

### 3.4 Derived Views

여러 모델을 조합하여 정의하는 가상 모델. 데이터베이스 View에 대응하며, `::view` 타입 지시자를 사용한다.

#### 3.4.1 기본 문법

```markdown
## ViewName ::view
> 뷰 설명

### Source
- from: PrimaryModel
- join: TargetModel on JoinCondition   # optional
- where: "filter condition"             # optional
- order_by: field direction             # optional
- group_by: [fields]                    # optional (집계 뷰)

- fieldName: type @from(Model.field)
- fieldName: type @rollup(...)
- fieldName: type @computed("...")
```

구성 요소:
- `::view` — 이 모델이 저장 테이블이 아닌 파생 뷰임을 선언
- `### Source` — 데이터 소스 정의 섹션. 기존 `### Indexes`, `### Relations`, `### Metadata` 패턴과 동일한 H3 섹션 형식
- `@from(Model.field)` — 원본 모델의 필드를 매핑

> **섹션 분리 원칙**: View의 `source`, `join`, `where` 등 디렉티브는 필드 정의와 동일한 `- ` 리스트가 아닌, `### Source` 섹션 내에 배치한다. 이는 기존 M3L에서 모델 수준 메타 정보(`### Indexes`, `### Behaviors`, `### Metadata`)를 항상 H3 섹션으로 분리하는 패턴과 일관된다.

#### 3.4.2 단순 뷰 (Single Source)

```markdown
## ActiveProducts ::view
> 판매 가능한 활성 제품 목록

### Source
- from: Product
- where: "is_active = true AND stock_quantity > 0"
- order_by: name asc

- id: identifier @from(Product.id)
- name: string @from(Product.name)
- price: decimal(10,2) @from(Product.price)
- stock: integer @from(Product.stock_quantity)
```

#### 3.4.3 조인 뷰 (Multi Source)

```markdown
## OrderDetail ::view
> 주문 상세 정보 - 주문, 고객, 항목 정보를 결합

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

#### 3.4.4 집계 뷰 (Aggregate View)

```markdown
## CustomerStats ::view
> 고객별 주문 통계

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

#### 3.4.5 Materialized View

성능 최적화를 위해 물리적으로 저장되는 뷰:

```markdown
## MonthlySalesReport ::view @materialized
> 월별 매출 리포트 - 매일 갱신

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

#### 3.4.6 뷰 중첩 (View Nesting)

뷰가 다른 뷰를 데이터 소스로 사용할 수 있다. `### Source`의 `from`에 다른 뷰를 지정한다:

```markdown
## ActiveCustomerStats ::view
> 활성 고객만 필터링한 통계

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

> **상속 대신 중첩**: 뷰 확장에 `:` (상속) 문법을 사용하지 않는다. 기존 M3L에서 `:`는 필드 상속을 의미하지만, 뷰 확장은 쿼리 조합(source + filter + projection)에 해당하며 의미가 다르다. SQL의 `CREATE VIEW AS SELECT ... FROM another_view WHERE ...` 패턴과 동일하게, 다른 뷰를 `from`으로 참조하여 명시적으로 중첩한다.

#### 3.4.7 제약 사항

- Derived View는 **읽기 전용**이다. INSERT/UPDATE/DELETE 대상이 될 수 없다.
- View 내에서 다른 View를 `from`으로 참조할 수 있으나, **최대 깊이 2단계**까지 권장한다.
- `@materialized` 뷰는 `### Refresh` 섹션에서 갱신 전략을 명시해야 한다.
- 조인은 **명시적 조건만** 허용한다 (implicit join 불가).
- M3L View는 **선언적 조합**에 집중한다. 복잡한 서브쿼리, UNION, 윈도우 함수 등은 구현 레이어에 위임한다.

---

## 4. Complete Example

다음은 전자상거래 도메인에서 Stored, Computed, Lookup, Rollup, View를 모두 활용하는 종합 예제이다.

```markdown
# E-Commerce Data Model

## BaseModel
- id: identifier @primary @generated
- created_at: timestamp = now()
- updated_at: timestamp = now() @on_update(now())

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

## Category : BaseModel
- name: string(100)
- parent_id: identifier? @reference(Category)
- slug: string(100) @unique

# Rollup
- product_count: integer @rollup(Product.category_id, count)
- active_product_count: integer @rollup(Product.category_id, count, where: "is_active = true")

## Product : BaseModel
- sku: string(50) @unique
- name: string(200)
- description: text
- price: decimal(10,2) @min(0)
- cost: decimal(10,2)?
- stock_quantity: integer = 0
- category_id: identifier? @reference(Category)
- is_active: boolean = true

# Lookup
- category_name: string? @lookup(category_id.name)

# Computed
- profit_margin: decimal(5,2)? @computed("CASE WHEN cost > 0 THEN ((price - cost) / cost) * 100 END")
- is_in_stock: boolean @computed("stock_quantity > 0 AND is_active = true")

## Order : BaseModel
- order_number: string(20) @unique
- customer_id: identifier @reference(Customer)
- status: enum = "pending"
  - pending: "Pending"
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
- is_overdue: boolean @computed("status = 'processing' AND days_since_order > 7")

## OrderItem
- order_id: identifier @reference(Order) @primary(1)
- product_id: identifier @reference(Product) @primary(2)
- quantity: integer @min(1)
- unit_price: decimal(10,2)
- discount: decimal(10,2) = 0

# Lookup
- product_name: string @lookup(product_id.name)
- product_sku: string @lookup(product_id.sku)
- customer_name: string @lookup(order_id.customer_id.name)   # 2-hop

# Computed
- subtotal: decimal(12,2) @computed("quantity * unit_price - discount")

## OrderSummary ::view
> 주문 요약 뷰 - 대시보드용

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
> 고객별 종합 통계

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

## InventoryAlert ::view @materialized
> 재고 부족 알림 뷰 - 1시간마다 갱신

### Source
- from: Product
- where: "is_active = true AND stock_quantity <= 10"
- order_by: stock_quantity asc

### Refresh
- strategy: scheduled
- interval: "hourly"

- sku: string @from(Product.sku)
- name: string @from(Product.name)
- stock: integer @from(Product.stock_quantity)
- category: string? @lookup(Product.category_id.name)
- recent_order_count: integer @rollup(OrderItem.product_id, count, where: "Order.ordered_at >= date_add(today(), -7, 'day')")
```

---

## 5. Spec Integration Plan

기존 M3L 스펙에 다음과 같이 통합한다:

### 5.1 섹션 구조 변경

```
4. Advanced Features
   4.1 Composite Key Definition        (기존 유지)
   4.2 Comments and Documentation      (기존 유지)
   4.3 Behavior Definition             (기존 유지)
   4.4 Computed Fields                 (기존 유지, "Row-Level" 범위 명시 추가)
   4.5 Lookup Fields                   ★ 신규
   4.6 Rollup Fields                   ★ 신규
   4.7 Derived Views                   ★ 신규
   4.8 Conditional Fields              (기존 4.5 → 번호 변경)
   4.9 Complex Data Structures         (기존 4.6 → 번호 변경)
   4.10 Validation Rules               (기존 4.7 → 번호 변경)
   4.11 Templates and Generics         (기존 4.8 → 번호 변경)
```

### 5.2 기존 섹션 수정 사항

**섹션 1.3 Expression Patterns** — Derived Field 패턴 추가:

```markdown
- **Derived Patterns**: Cross-model expressions for lookup, aggregation, and view composition
  - Lookup: `@lookup(fk_field.target_field)`
  - Rollup: `@rollup(Target.fk, aggregate(field))`
  - View: `## ViewName ::view`
```

**섹션 2.4 Data Type Notation** — 타입 추론 규칙 추가:

```markdown
#### 2.4.5 Derived Field Type Inference
- Lookup: follows the source field type
- Rollup: determined by aggregate function (count → integer, sum → source type, avg → decimal)
- Computed: inferred from expression or explicitly specified
```

**섹션 3.1 Enum Definition의 `::enum`과 병렬** — `::view` 타입 지시자를 Special Elements 또는 Introduction에서 소개:

```markdown
Type Indicators:
- `::enum` — Enumeration type
- `::interface` — Interface type
- `::view` — Derived view type (read-only, virtual)
```

### 5.3 신규 속성 목록

| 속성 | 적용 대상 | 설명 |
|---|---|---|
| `@lookup(path)` | Field | 관계 참조로 값 가져오기 |
| `@rollup(target, fn)` | Field | 관계 집계 |
| `@from(Model.field)` | View Field | 원본 모델 필드 매핑 |
| `@materialized` | View | 물리적 저장 뷰 |
| `@persisted` | Lookup/Rollup | 비정규화 저장 (기존 Computed에도 사용) |

---

## 6. Design Decisions & Rationale

### 6.1 왜 `@lookup`과 `@rollup`을 분리하는가?

`@computed`에 SQL을 직접 작성하면 기술적으로 동일한 결과를 얻을 수 있다. 그러나 분리하는 이유는:

1. **의도의 명확성**: "참조"와 "집계"는 의미적으로 다른 연산이다
2. **도구 지원**: 파서/코드 생성기가 최적의 구현을 선택할 수 있음
3. **AI 친화성**: 에이전트가 데이터 흐름을 정확히 이해할 수 있음
4. **플랫폼 최적화**: 구현 레이어가 Lookup은 JOIN으로, Rollup은 서브쿼리나 트리거로 각각 최적화 가능

> **플랫폼 독립성에 대한 참고**: M3L의 표현식(`@computed`, `where` 절 등)은 현재 플랫폼별 문법을 허용한다. `@lookup`과 `@rollup`의 핵심 가치는 표현식의 플랫폼 독립성이 아니라, **"이것이 참조/집계입니다"라는 의도를 선언적으로 드러내는 것**이다. 표현식의 플랫폼 독립성은 향후 M3L Expression Language로 별도 제안할 수 있다.

### 6.2 왜 `::view`를 별도 모델 타입으로 정의하는가?

Rollup 필드만으로도 많은 집계를 처리할 수 있지만, View가 필요한 이유는:

1. **다중 모델 조합**: 3개 이상 모델을 조인하는 경우 개별 Rollup으로는 표현 불가
2. **필터링된 관점**: 특정 조건의 데이터만 보여주는 "슬라이스" 정의
3. **성능 최적화**: Materialized View로 미리 계산된 결과 저장
4. **관심사 분리**: 저장 모델과 표현 모델의 명확한 구분

### 6.3 왜 View 복잡도를 제한하는가?

M3L View는 의도적으로 SQL View보다 제한적이다:

- 서브쿼리, UNION, 윈도우 함수 불가
- 조인은 명시적 조건만 허용
- 뷰 중첩 최대 2단계

이는 M3L이 **쿼리 언어가 아닌 모델링 언어**라는 정체성을 유지하기 위함이다. 복잡한 데이터 처리 로직은 구현 레이어(Stored Procedure, Application Logic 등)에 위임한다.

### 6.4 왜 `@from`과 `@lookup`을 구분하는가?

둘 다 "다른 모델의 필드 참조"이지만 작동 맥락이 다르다:

- **`@lookup`**: 모델 필드로서, FK 경로를 선언하면 런타임이 자동으로 조인을 수행한다. 1:1 관계의 단일 값 참조에 적합하다. 필드 하나로 완결되는 **인라인 네비게이션**.
- **`@from`**: View 필드로서, `### Source` 섹션에서 이미 선언된 조인의 결과를 매핑한다. 다중 조인, 복잡한 조건의 조합에 적합하다. 소스 정의와 필드 선택이 분리된 **명시적 프로젝션**.

원칙: **Lookup은 네비게이션(탐색), From은 프로젝션(투영).**

```markdown
# Lookup — FK를 따라가서 자동 조인
## OrderItem
- product_id: identifier @reference(Product)
- product_name: string @lookup(product_id.name)
  ↑ "product_id를 따라 Product.name으로 탐색(navigate)"

# @from — 이미 선언된 조인에서 필드 선택
## OrderDetail ::view
### Source
- from: Order
- join: Product on OrderItem.product_id = Product.id

- product_name: string @from(Product.name)
  ↑ "조인된 Product에서 name을 투영(project)"
```

### 6.5 왜 View에서 상속(`:`) 대신 중첩(`from`)을 사용하는가?

기존 M3L에서 `:` 문법은 **필드 상속**을 의미한다:

```markdown
## Product : BaseModel    ← BaseModel의 필드를 물려받음
```

뷰 확장은 필드 상속이 아니라 **쿼리 조합**(source + filter + projection)이다. 동일한 `:` 문법에 다른 의미를 부여하면 혼란을 야기하므로, SQL의 `CREATE VIEW AS SELECT ... FROM another_view WHERE ...` 패턴과 동일하게 `from`으로 명시적 중첩을 사용한다:

```markdown
## ActiveCustomerStats ::view
### Source
- from: CustomerStats                 ← 다른 뷰를 데이터 소스로 참조
- where: "total_orders > 0"           ← 추가 필터
```

### 6.6 왜 View 디렉티브를 `### Source` 섹션으로 분리하는가?

기존 M3L에서 `- ` 리스트 항목은 항상 필드 정의다. View의 `source`, `join`, `where` 등 디렉티브를 동일한 `- ` 리스트에 혼합하면 파서가 필드와 디렉티브를 구분할 수 없다.

기존 M3L은 모델 수준 메타 정보를 항상 H3 섹션으로 분리한다:

```markdown
### Indexes       ← 인덱스 정의
### Behaviors     ← 동작 정의
### Relations     ← 관계 정의
### Metadata      ← 메타데이터
### Source        ← 뷰 소스 정의 (신규, 동일 패턴)
### Refresh       ← Materialized 갱신 전략 (신규, 동일 패턴)
```

이 패턴을 따르면 파서 구현이 단순하고 기존 문법과 완전히 일관된다.

---

## 7. Migration & Backward Compatibility

### 7.1 하위 호환성

본 제안은 **순수 추가(additive)**이다. 기존 M3L 문서는 수정 없이 유효하다:

- 신규 속성(`@lookup`, `@rollup`, `@from`, `@materialized`)은 기존 속성과 충돌하지 않음
- `::view` 타입 지시자는 기존 `::enum`, `::interface`와 동일한 패턴
- 기존 Computed Fields는 변경 없이 유지

### 7.2 파서 영향

M3L 파서는 다음을 추가로 처리해야 한다:

1. `@lookup(path)` 속성 파싱 및 FK `@reference` 검증
2. `@rollup(target, function, where?)` 속성 파싱 및 FK `@reference` 검증
3. `::view` 타입 지시자와 `### Source` 섹션 내 `from`, `join`, `where`, `group_by`, `order_by` 디렉티브
4. `@from(Model.field)` 속성 파싱 — `### Source`에서 선언된 모델만 참조 가능
5. `@materialized` 속성과 `### Refresh` 섹션
6. Lookup 체인 깊이 검증 (최대 3-hop 권장)
7. 순환 참조 탐지 (Lookup 체인, Rollup 의존)

---

## 8. Open Questions

향후 논의가 필요한 사항:

1. **Lookup 기본값**: Lookup 대상이 null이거나 삭제된 경우의 fallback 정책을 `@lookup` 문법에 포함할 것인가, 별도 속성으로 분리할 것인가?

2. **Rollup 성능 힌트**: `@persisted`외에 캐싱 전략이나 갱신 빈도를 M3L 수준에서 표현할 필요가 있는가?

3. **View 권한**: `::view`에 접근 제어(`@public`, `@private`)를 적용할 것인가? 기존 모델의 접근 제어(2.2.4)와 동일하게 처리할 수 있는가?

4. **코드 생성 표준**: Lookup/Rollup/View의 플랫폼별 코드 생성 가이드라인을 M3L 스펙에 포함할 것인가, 별도 문서로 관리할 것인가?

5. **Rollup의 Rollup**: Rollup 필드를 다른 모델의 Rollup에서 참조할 수 있는가? 허용한다면 순환 탐지 로직이 복잡해진다.

6. **M3L Expression Language**: `@computed`, `@rollup(where:)`, View의 `where` 등 모든 표현식 컨텍스트에서 플랫폼 독립적 표현을 제공하는 별도 서브스펙이 필요한가?

---

## 9. Revision History

| Date | Change | Rationale |
|------|--------|-----------|
| 2026-02-25 | Initial draft | - |
| 2026-02-25 | (A) View 디렉티브를 `### Source` 섹션으로 분리 | 기존 `### Indexes/Relations/Metadata` 패턴과 일관성 |
| 2026-02-25 | (B) Rollup 참조 검증 규칙 추가 (3.3.2) | `@reference` 기반 SSOT 원칙 명시 |
| 2026-02-25 | (C) Motivation 재프레이밍 (2.2) | "플랫폼 종속성" → "의도의 불명확성"으로 핵심 논점 수정 |
| 2026-02-25 | (D) View 상속(`:`) 제거, 뷰 중첩(`from`) 도입 (3.4.6) | `:` 문법의 의미 충돌 해소 |
| 2026-02-25 | (E) Design Decision 6.4 추가 | `@from` vs `@lookup` 구분 근거 문서화 |
| 2026-02-25 | (F) 확장 포맷 평탄 구조 통일 (3.2.4, 3.3.5) | 기존 Computed 확장 패턴과 일관성 |

---

## 10. References

- M3L Specification (Current): `README.md`
- Airtable Field Types: Lookup, Rollup, Formula
- Notion Database Relations & Rollups
- SQL Standard: CREATE VIEW, Materialized Views
- Entity Framework: Navigation Properties, Computed Columns
- DAX (Power BI): Calculated Columns vs Measures
