# M3L Parser & CLI — Design Document

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** TypeScript로 M3L 파서 라이브러리와 CLI 도구를 구축하여, `.m3l.md` 파일을 JSON AST로 변환하고 검증한다.

**Architecture:** 마크다운 기반 M3L 문서를 Reader → Lexer → Parser → Resolver → Validator 파이프라인으로 처리. 멀티 파일 프로젝트를 단일 병합 AST로 출력. CLI와 라이브러리 API를 단일 npm 패키지로 제공.

**Tech Stack:** TypeScript, Node.js, vitest, commander (CLI)

---

## Pipeline

```
.m3l.md 파일(들)
    │
    ▼
┌─────────┐    ┌─────────┐    ┌───────────┐    ┌───────────┐
│  Reader  │ →  │  Lexer  │ →  │  Parser   │ →  │ Resolver  │ → JSON AST
│ 파일 수집 │    │ 토큰화   │    │ 단일파일   │    │ 멀티파일   │
│ @import  │    │         │    │ AST 생성   │    │ 병합/검증  │
└─────────┘    └─────────┘    └───────────┘    └───────────┘
                                                      │
                                                      ▼
                                               ┌───────────┐
                                               │ Validator  │
                                               │ 에러/경고   │
                                               └───────────┘
```

- **Reader**: 파일 수집. 단일 파일, 디렉토리 스캔(`**/*.m3l.md`), `m3l.config.yaml` 매니페스트 지원. `@import` 의존성 추적.
- **Lexer**: 마크다운 구조를 M3L 토큰으로 변환. H1(namespace), H2(model/enum/interface/view), H3(section), 리스트(field/directive), 속성(`@`), 타입.
- **Parser**: 파일 단위로 토큰 → AST 노드 변환.
- **Resolver**: 멀티 파일 AST 병합. 상속 해결, 크로스모델 참조 검증, Lookup/Rollup FK 검증, View source 해석.
- **Validator**: 의미 검증(순환 참조, 누락 참조) + `--strict` 스타일 가이드라인.

## CLI Interface

```bash
m3l parse model.m3l.md                    # 단일 파일 → JSON stdout
m3l parse ./models/                       # 디렉토리 스캔
m3l parse                                 # m3l.config.yaml 또는 cwd 스캔
m3l parse ./models/ -o out.json           # 파일 저장

m3l validate model.m3l.md                 # 구조/참조 오류
m3l validate ./models/ --strict           # + 스타일 가이드라인
m3l validate ./models/ --format json      # CI/에디터용 JSON
```

## Project Manifest (optional)

```yaml
# m3l.config.yaml
name: "ecommerce"
version: "1.0"
sources:
  - "common/*.m3l.md"
  - "models/*.m3l.md"
  - "views/*.m3l.md"
```

없으면 `**/*.m3l.md` 자동 스캔.

## Output AST (Flat JSON)

```json
{
  "project": { "name": "ecommerce", "version": "1.0" },
  "sources": ["common/base.m3l.md", "models/product.m3l.md"],
  "models": [
    {
      "name": "Product",
      "type": "model",
      "source": "models/product.m3l.md",
      "line": 3,
      "inherits": ["BaseModel"],
      "description": "Product available for purchase",
      "fields": [
        {
          "name": "name",
          "type": "string",
          "params": [200],
          "nullable": false,
          "kind": "stored",
          "attributes": [{ "name": "unique" }]
        },
        {
          "name": "category_name",
          "type": "string",
          "nullable": true,
          "kind": "lookup",
          "lookup": { "path": "category_id.name" },
          "attributes": []
        }
      ],
      "sections": {
        "indexes": [],
        "relations": [],
        "behaviors": [],
        "metadata": {}
      }
    }
  ],
  "enums": [],
  "interfaces": [],
  "views": [
    {
      "name": "OrderSummary",
      "type": "view",
      "source": "views/dashboard.m3l.md",
      "line": 1,
      "materialized": false,
      "source_def": {
        "from": "Order",
        "joins": [{ "model": "Customer", "on": "Order.customer_id = Customer.id" }],
        "where": "Order.status != 'cancelled'",
        "order_by": "Order.ordered_at desc"
      },
      "fields": []
    }
  ],
  "errors": [],
  "warnings": []
}
```

Field `kind` values: `stored`, `computed`, `lookup`, `rollup`

## Error/Warning Output

**Human (default):**
```
models/order.m3l.md:15:3 error[E001]: @rollup target has no @reference
1 error, 0 warnings in 1 file.
```

**Machine (`--format json`):**
```json
{
  "diagnostics": [
    { "code": "E001", "severity": "error", "file": "...", "line": 15, "col": 3, "message": "..." }
  ],
  "summary": { "errors": 1, "warnings": 0, "files": 1 }
}
```

## Library API

```typescript
import { parse, validate } from '@iyulab/m3l';

const ast = parse('path/to/models/');
const ast = parseString(m3lContent);
const result = validate('path/to/models/', { strict: true });
```

## Directory Structure

```
m3l/
├── README.md
├── rfcs/
├── samples/
└── parser/
    ├── package.json
    ├── tsconfig.json
    ├── vitest.config.ts
    ├── src/
    │   ├── index.ts
    │   ├── cli.ts
    │   ├── types.ts
    │   ├── reader.ts
    │   ├── lexer.ts
    │   ├── parser.ts
    │   ├── resolver.ts
    │   └── validator.ts
    └── tests/
        ├── fixtures/
        ├── lexer.test.ts
        ├── parser.test.ts
        ├── resolver.test.ts
        ├── validator.test.ts
        └── cli.test.ts
```

## Validation Rules

### Errors (always)

| Code | Rule |
|------|------|
| E001 | `@rollup` FK missing `@reference` |
| E002 | `@lookup` path FK missing `@reference` |
| E003 | Circular reference (lookup chain, inheritance) |
| E004 | View `@from` references model not in Source |
| E005 | Duplicate model/field names |
| E006 | `@import` target file not found |
| E007 | Unresolved type/model reference |

### Warnings (`--strict`)

| Code | Rule |
|------|------|
| W001 | Field line length >80 chars |
| W002 | Object nesting >3 levels |
| W003 | `[Attr]` without backtick wrapping |
| W004 | Lookup chain >3 hops |
| W005 | View nesting >2 levels |
| W006 | Inline enum missing `values:` key |
