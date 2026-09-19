# M3L RFC-0003: 숫자 폭 사다리 채우기 — `byte`·`short`·`double`, 그리고 `float` 의 폭

> **RFC Status**: Accepted
> **Author**: UJ (iyulab)
> **Date**: 2026-09-19
> **M3L Version Target**: Next Minor
> **Affects Sections**: 10.3.3 (`BaseType`), 10.4.1

---

## 1. Summary

타입 카탈로그(§10.4.1)에 `byte`(8-bit 부호 없는 정수)·`short`(16-bit 정수)·`double`(64-bit 부동소수)을
추가하고, 지금까지 폭이 적혀 있지 않던 `float` 을 32-bit 로 명시한다.

## 2. Motivation

### 2.1 카탈로그는 이미 폭을 말한다 — 사다리에 구멍이 있을 뿐이다

§10.4.1 은 `integer` 를 «32-bit», `long` 을 «64-bit» 로 정의한다. 즉 M3L 의 정수 타입은 **폭을 명시하는
타입**이다. 그런데 그 사다리는 32 에서 시작한다 — 8·16-bit 정수를 표현할 방법이 없다.

부동소수 쪽은 더 크다. `float` 은 «Floating-point number» 라고만 적혀 있어 폭이 없고, 그 외의 부동소수
타입이 없다. 결과적으로 **M3L 로는 64-bit 부동소수를 명시적으로 표현할 수 없다.** `float` 을 64-bit 로
읽는 소비자와 32-bit 로 읽는 소비자가 둘 다 스펙을 지키고 있는 상태다.

### 2.2 미등재 타입은 이미 오류다

검증기는 카탈로그에 없고 모델·enum·인터페이스로도 정의되지 않은 타입 이름을 `M3L-E009`(undefined
type)로 보고한다. 타입은 이미 **닫힌 어휘**다. 그러니 폭이 다른 수치 타입이 필요한 모델에는 지금 두
길밖에 없다 — 더 넓은 타입으로 뭉개거나(`short` 자리에 `integer`), 오류를 감수하고 카탈로그 밖 이름을
쓰는 것. 앞의 것은 모델의 정보를 버리고, 뒤의 것은 언어가 금지한 것이다.

이 RFC 는 카탈로그의 성격을 바꾸지 않는다 — 이미 정한 설계(폭을 명시하는 닫힌 어휘)를 **채운다.**

### 2.3 선행 사례

| | 8-bit | 16-bit | 64-bit 부동소수 |
|---|---|---|---|
| SQL (SQL Server) | `TINYINT` | `SMALLINT` | `FLOAT` |
| SQL (PostgreSQL) | — | `SMALLINT` | `DOUBLE PRECISION` |
| .NET CLR | `byte` | `short` | `double` |
| Java | `byte` | `short` | `double` |

64-bit 부동소수는 널리 쓰이는 스키마·타입 언어가 공통으로 갖는다. 8·16-bit 정수는 모든 곳에 있지는
않다(예: JSON Schema·OpenAPI 는 정수 폭을 `format` 으로만 구분한다) — 이 RFC 가 기대는 근거는 «다들
있다»가 아니라 §2.1 의 **내적 일관성**이다: 폭을 명시하는 사다리가 중간에서 시작하면 안 된다.

## 3. Specification changes

### 3.1 §10.4.1 Primitive Types

| Type | Parameters | Description | Example |
|---|---|---|---|
| `byte` | — | 8-bit unsigned integer (0–255) — a number, not a byte array (see `binary`) | `byte` |
| `short` | — | 16-bit integer | `short` |
| `float` | — | 32-bit floating-point number (IEEE 754 binary32) *(변경: 폭 명시)* | `float` |
| `double` | — | 64-bit floating-point number (IEEE 754 binary64) | `double` |

표 아래에 «Numeric widths» 문단을 둔다: 정수 사다리 `byte`(8, unsigned) → `short`(16) → `integer`(32)
→ `long`(64), 부동소수 사다리 `float`(32) → `double`(64), 정확한 소수가 필요하면 `decimal`.

### 3.2 §10.3.3 `BaseType`

`BaseType` 대안 목록에 `'short'` · `'byte'` · `'double'` 을 추가한다.

## 4. Design decisions

### 4.1 `byte` 는 부호 없음(0–255)

8-bit 정수의 부호는 생태계마다 갈린다 — SQL Server `TINYINT` 와 .NET `byte` 는 부호 없음, Java `byte`
는 부호 있음. 이 RFC 는 **부호 없음**을 택한다. 데이터 모델링에서 8-bit 필드가 쓰이는 자리(수준·코드·
플래그 값)는 거의 음수가 아니고, 가장 흔한 저장 대상(`TINYINT`)과 폭·범위가 일치한다. 부호 있는
8-bit 값이 필요하면 `short` 가 그 범위를 담는다.

`byte` 는 **수**다. 바이트 배열은 `binary` 이며, `byte[]` 는 «`byte` 값의 배열»(§10.4.4)이지
`binary` 의 별칭이 아니다 — 이 구분을 표에 명시한다.

### 4.2 `float` 을 32-bit 로 명시하는 것은 좁히는 변경이다

지금까지 `float` 의 폭은 지정되지 않았다. 32-bit 로 못박는 것은 **해석의 폭을 좁히는** 변경이며,
`float` 을 64-bit 로 읽어 온 소비자에게는 의미 변경이다. 그래도 32-bit 를 택하는 이유:

- 새로 `double` 이 생기므로 64-bit 가 필요한 모델은 그것을 말할 수 있게 된다 — 좁힘이 표현력을
  잃게 하지 않는다.
- `float`(32)·`double`(64) 은 C 계열 언어 전반과 IEEE 754 명명 관례에서 가장 널리 공유되는 짝이다.

파서의 출력은 바뀌지 않는다 — AST 는 여전히 `"type": "float"` 을 싣는다. 바뀌는 것은 그 이름이
뜻하는 폭에 대한 스펙의 진술이다.

## 5. Backward compatibility

- **파서 출력**: 기존 입력의 AST 는 바뀌지 않는다. 세 이름은 이미 식별자로 파싱돼 `type` 에 그대로
  실려 있었다.
- **검증**: 세 이름을 쓰던 모델은 지금까지 `M3L-E009` 를 받았고, 이제 받지 않는다(오류 감소만 있음).
  이 세 이름과 같은 이름의 모델·enum 을 정의한 문서는 카탈로그 타입과 이름이 겹치게 되는데, 검증기는
  지금도 이런 겹침(예: `text` 라는 모델)에 진단을 내지 않으므로 새 진단은 생기지 않는다.
- **`float`**: §4.2 — 스펙상의 의미 변경. 64-bit 를 뜻하던 모델은 `double` 로 옮긴다.

## 6. Conformance

`spec/conformance/inputs/numeric-widths.m3l.md` 가 두 사다리의 모든 폭을 한 모델에 싣고, 적합성
테스트가 `M3L-E009` 0건과 필드별 타입·수식자를 단언한다.

## 7. Decision

- **Verdict**: ACCEPT
- **Decision Date**: 2026-09-19
