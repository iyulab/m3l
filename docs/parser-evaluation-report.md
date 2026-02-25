# M3L Parser Evaluation Report

**평가일**: 2026-02-25
**대상**: TypeScript parser (`@iyulab/m3l` 0.1.0), C# parser (`M3LParser` 0.1.0)
**테스트 기반**: `samples/01-ecommerce.m3l.md`, `samples/02-blog-cms.m3l.md`, `samples/03-types-showcase.m3l.md`

---

## 1. 테스트 요약

| 항목 | TypeScript | C# |
|---|---|---|
| 테스트 수 | 227 | 142 |
| 통과 | 227 (100%) | 141 (99.3%) |
| 실패 | 0 (결함은 테스트 내 주석으로 문서화) | 1 (의도적 결함 탐지 테스트) |
| 테스트 프레임워크 | Vitest | xUnit (net8.0/net9.0/net10.0) |
| 실행 시간 | ~1.4s | ~1.1s (per framework) |

C# 파서는 TypeScript의 직접 포팅이므로 **동일한 결함 세트**를 공유한다.
3개 프레임워크(net8.0/net9.0/net10.0) 모두 동일한 결과(141/142 pass).

---

## 2. 기능 적합도 매트릭스

### 2.1 핵심 기능

| 기능 | TS | C# | 비고 |
|---|---|---|---|
| Namespace 파싱 | ✅ | ✅ | `# Namespace: domain` 정상 |
| Model 정의 (`## Name`) | ✅ | ✅ | |
| Enum 정의 (`::enum`) | ✅ | ✅ | 값, 설명, 타입 값 모두 정상 |
| Interface 정의 (`::interface`) | ✅ | ✅ | |
| View 정의 (`::view`) | ✅ | ✅ | Source, Join, Where, OrderBy, GroupBy |
| 단일 상속 | ✅ | ✅ | `## B : A` |
| 다중 상속 | ✅ | ✅ | `## C : A, B` |
| 전이적 상속 | ✅ | ✅ | A → B → C 필드 전파 |
| Lookup 필드 | ✅ | ✅ | `@lookup(fk.field)` → path 분해 |
| Rollup 필드 | ✅ | ✅ | target, FK, aggregate, where 파싱 |
| Computed 필드 | ✅ | ✅ | `@computed(expr)` |
| Materialized View | ✅ | ✅ | `@materialized` + `### Refresh` |
| Framework 속성 | ✅ | ✅ | `` `[JsonIgnore]` `` → CustomAttribute |
| Blockquote 설명 | ✅ | ✅ | `> text` → description |
| 인라인 설명 | ✅ | ✅ | `"text"` → description |
| 기본값 | ✅ | ✅ | 문자열, 숫자, boolean, 함수호출 |
| Kind 섹션 | ✅ | ✅ | `# Lookup`, `# Rollup`, `# Computed` |
| 명명된 섹션 | ✅ | ✅ | `### Indexes`, `### Metadata`, etc. |
| Multi-file Resolve | ✅ | ✅ | 여러 파일 병합, 중복 탐지 |
| Validation | ✅ | ✅ | E001~E007, W001~W004 |

### 2.2 타입 시스템

| 타입 | TS | C# | 비고 |
|---|---|---|---|
| Primitive (string, integer, ...) | ✅ | ✅ | 12종 모두 정상 |
| Semantic (email, phone, url, ...) | ✅ | ✅ | 5종 모두 정상 |
| 타입 파라미터 `(N)`, `(N,M)` | ✅ | ✅ | string(200), decimal(10,2) |
| Nullable `?` | ✅ | ✅ | |
| Array `[]` | ✅ | ✅ | |
| Nullable array `[]?` | ⚠️ | ⚠️ | **결함 D003** |
| Map `map<K,V>` | ⚠️ | ⚠️ | **결함 D006** |
| Object 중첩 | ⚠️ | ⚠️ | **결함 D007** |
| Inline enum | ✅ | ✅ | 하위 항목으로 enum 값 정의 |

---

## 3. 결함 목록

총 **7개 고유 결함** 발견 (양쪽 파서 공통).

### HIGH — 기능 결손

| ID | 결함 | 영향 | 위치 |
|---|---|---|---|
| **D-001** | `@override`가 상속 필드를 교체하지 않음 | 자식 모델이 부모 필드를 재정의하면 `M3L-E005` 중복 에러 발생. 상속 오버라이드 시나리오가 불가능. | Resolver — `resolveInheritance` |

### MEDIUM — 데이터 손실

| ID | 결함 | 영향 | 위치 |
|---|---|---|---|
| **D-002** | `map<K,V>` 제네릭 타입 파라미터 미캡처 | `map<string, string>`에서 type.base="map"만 남고 K,V 정보 소실. Code generator가 map 키/값 타입을 알 수 없음. | Lexer — `ReTypePart` regex |
| **D-003** | 중첩 Object 하위 필드가 구조화되지 않음 | `profile: object` 하위에 `contact`, `preferences` 등 서브필드가 `FieldNode.fields`에 반영 안 됨. 깊은 중첩 구조 표현 불가. | Parser — `handleNestedItem` |
| **D-004** | `@computed_raw` 표현식 미캡처 | `@computed_raw("SQL expr")` 사용 시 `computed.expression`이 null. 플랫폼 종속 계산 표현식 소실. | Parser — `buildFieldNode` |
| **D-005** | 커스텀 섹션 항목이 모델 필드로 혼입 | `### PrimaryKey`, `### Validations`, `### Version`, `### Migration` 등 미인식 섹션의 항목이 일반 필드로 파싱됨. | Parser — 섹션 폴스루 처리 |

### LOW — 경미한 데이터 불일치

| ID | 결함 | 영향 | 위치 |
|---|---|---|---|
| **D-006** | `string[]?` nullable 플래그 소실 | `type[]?` 구문에서 `?`가 `[]` 뒤에 올 때 nullable 미인식. 드문 사용 패턴이나 spec에 정의됨. | Lexer — 타입 파싱 regex |
| **D-007** | `@behavior` 지시자와 섹션의 키 불일치 | `@behavior(event, action)` → `sections["behavior"]`, `### Behaviors` → `sections["behaviors"]`. 단수/복수 불일치. | Parser — 지시자 핸들링 |

### C# 전용 추가 결함

| ID | 결함 | 영향 | 위치 |
|---|---|---|---|
| **D-C01** | Model-level 속성(`@public`, `@private` 등) 누락 | Lexer가 `token.Data["attributes"]`에 파싱하지만 Parser의 `HandleModelStart`가 읽지 않음. `ModelNode`에 `Attributes` 프로퍼티 없음. | Parser — `HandleModelStart` |

> **참고**: D-C01은 TypeScript에는 해당하지 않을 수 있으나, C# 포팅 과정에서 누락된 부분임.

---

## 4. 파서 간 동등성 평가

### 4.1 AST 구조 호환성

| 항목 | 일치 여부 | 비고 |
|---|---|---|
| 모델 이름/개수 | ✅ | |
| 필드 이름/타입/속성 | ✅ | |
| Enum 값/설명 | ✅ | |
| View 소스/조인/필터 | ✅ | |
| Lookup/Rollup/Computed 구조 | ✅ | |
| 상속 필드 순서 | ✅ | 상속 필드가 앞, 자체 필드가 뒤 |
| 에러/경고 코드 | ✅ | 동일 진단 코드 |
| Source Location | ⚠️ | line/column 존재하나, 정밀도 미검증 |

### 4.2 API 호환성

| API | TypeScript | C# | 동등 |
|---|---|---|---|
| `parseString(content, filename)` | ✅ | `ParseString(content, filename)` | ✅ |
| `parse(inputPath)` | ✅ | `ParseAsync(inputPath)` | ✅ |
| `validate(inputPath)` | ✅ | `ValidateAsync(inputPath)` | ✅ |
| `getParserVersion()` | "0.1.0" | "0.1.0" | ✅ |
| `getAstVersion()` | "1.0" | "1.0" | ✅ |

---

## 5. 개선 권고

### 5.1 즉시 수정 (v0.1.1)

| 우선순위 | 결함 | 작업량 | 설명 |
|---|---|---|---|
| 🔴 P1 | D-001 `@override` | 중 | Resolver에서 `@override` 속성 감지 시 상속 필드 교체 로직 추가 |
| 🟠 P2 | D-005 섹션 폴스루 | 소 | 미인식 섹션을 `sections[]` 배열에 저장, 필드로 변환하지 않음 |
| 🟠 P2 | D-C01 Model 속성 누락 | 소 | C# `ModelNode`에 `Attributes` 추가, `HandleModelStart`에서 읽기 |

### 5.2 단기 개선 (v0.2.0)

| 우선순위 | 결함 | 작업량 | 설명 |
|---|---|---|---|
| 🟠 P2 | D-002 `map<K,V>` | 중 | Lexer 타입 regex에 `<...>` 제네릭 파라미터 캡처 추가 |
| 🟠 P2 | D-003 Object 중첩 | 대 | Parser에서 indent 기반 트리 구조 빌드, `FieldNode.fields` 채우기 |
| 🟠 P2 | D-004 `@computed_raw` | 소 | `@computed_raw` → `computed.expression` 매핑, `isRaw=true` 설정 |

### 5.3 장기 개선

| 항목 | 설명 |
|---|---|
| D-006 `[]?` 순서 | Lexer 타입 regex를 `(\[\])?(\?)?` ↔ `(\?)?(\[\])` 양쪽 허용으로 확장 |
| D-007 behavior 키 통일 | 지시자와 섹션 모두 `"behaviors"` (복수형)로 통일 |
| Conformance Test Set | 양쪽 파서가 동일 입력에 대해 동일 JSON 출력을 생성하는지 자동 검증 |
| Source Location 정밀도 | endLine/endColumn 정확성 검증 테스트 추가 |

---

## 6. 종합 점수

| 평가 항목 | TypeScript | C# | 기준 |
|---|---|---|---|
| **기본 파싱** | 95/100 | 94/100 | 모델, 필드, 타입, 속성 파싱 정확도 |
| **상속/해석** | 85/100 | 85/100 | override 미지원으로 감점 |
| **파생 필드** | 90/100 | 90/100 | computed_raw 미캡처로 감점 |
| **타입 시스템** | 88/100 | 88/100 | map generic, nullable array 미비로 감점 |
| **섹션 처리** | 80/100 | 80/100 | 미인식 섹션 폴스루 문제로 감점 |
| **뷰/인터페이스** | 98/100 | 98/100 | 거의 완벽 |
| **검증(Validation)** | 95/100 | 95/100 | |
| **API 완성도** | 95/100 | 95/100 | |
| **바인딩 간 호환** | — | 97/100 | C#-TS AST 구조 동등성 |
| **종합** | **90/100** | **89/100** | C#은 model-level attr 누락으로 -1 |

**결론**: 양쪽 파서 모두 M3L 사양의 핵심 기능을 안정적으로 파싱하며, 실제 Code Generator 구축에 사용 가능한 수준이다. 발견된 7개 결함 중 D-001(`@override`)만이 실무 사용에 장애가 될 수 있으며, 나머지는 edge case이거나 향후 확장에서 자연스럽게 해결 가능하다.
