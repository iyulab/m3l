# M3L 확장성 제안서 — Parser 소비자를 위한 생태계 설계

**문서 버전**: 1.0
**작성일**: 2026-02-25
**대상**: M3L Specification
**범위**: 파서 출력 계약, 속성 확장 체계, Code Generator 진입장벽 최소화
**상태**: Triaged

---

## Triage 결과 요약

| # | 제안 | Verdict | 시기 |
|---|---|---|---|
| 1 | AST Schema 공식화 | **ADAPT** — 현재 types.ts 기반 점진적 확장 | Phase 1 (즉시) |
| 2 | Attribute Registry | **DEFER** — v0.x에서는 시기상조 | v1.0 설계 시 |
| 3 | Parser Extension Interface | **DEFER** — 실제 소비자 피드백 후 | Registry 도입 후 |
| 4 | Standard Attribute Catalog | **ACCEPT** — 사양 10절에 추가 | Phase 1 (즉시) |

### 즉시 실행 항목 (Phase 1)

1. Standard Attribute Catalog를 specification.md Section 10.8로 추가
2. types.ts에 `M3LType` 구조화, `parserVersion`/`astVersion` 등 핵심 필드 반영
3. Conformance Test Set 초안 설계

### DEFER 항목의 재검토 조건

- Attribute Registry: 2개 이상의 외부 code generator가 M3L 사용 시
- Parser Extension Interface: Attribute Registry 도입 시점 또는 3개 이상 code generator에서 공통 패턴 발견 시

---

## 원본 제안서

(아래는 원본 제안서 전문입니다)

---

## 1. 배경 및 문제 정의

### 1.1 M3L의 역할 경계

M3L은 데이터 모델링 **사양(Spec)**과 **파서(Parser)**를 제공하는 프로젝트이다. Code Generator는 M3L의 범위 밖이며, 각 도입사가 자체 구축한다.

### 1.2 현재 확장성 현황

| 메커니즘 | 위치 | 확장 가능? | 파서 검증? | Code Generator 활용? |
|---|---|---|---|---|
| `@` 네이티브 속성 | 2.5절 | ✕ (고정 세트) | ✕ (세트 미정의) | △ |
| `` `[FrameworkAttr]` `` 커스텀 속성 | 2.5.6절 | ◎ (자유 형식) | ✕ (문자열 통과) | △ |
| `### Metadata` 섹션 | 3.5절 | ◎ (자유 형식) | ✕ (구조 제약 없음) | △ |
| `"description"` 문자열 | 4.2.1절 | ◎ | N/A | ◎ |
| `# inline comment` | 4.2.4절 | ◎ | N/A | △ |

### 1.3 핵심 문제 (5가지 장벽)

| # | 문제 | 영향 |
|---|---|---|
| G1 | AST 스키마가 사양에 없다 | 파서 출력 형태를 추측해야 함 |
| G2 | `@` 속성의 표준/확장 구분이 없다 | `@unique`와 `@my_attr`을 동등 취급 |
| G3 | 커스텀 속성에 타입/검증을 선언할 수 없다 | 속성 값 유효성 보장 불가 |
| G4 | `[FrameworkAttr]`의 파서 출력이 미정의다 | 소비자가 다시 파싱해야 함 |
| G5 | 파서 확장 인터페이스가 없다 | 파서 fork 필요 |

---

## 2-10. (원본 제안서 전문 — 별도 참조)

상세 내용은 원본 제안서 문서를 참조하십시오. 제안서는 다음 4가지 핵심 제안을 포함합니다:

1. **AST Schema 공식화** (30+ TypeScript 인터페이스 정의)
2. **Attribute Registry** (`::attribute` 타입 인디케이터, 3-tier 분류)
3. **Parser Extension Interface** (Visitor 패턴, Custom Attribute Parser)
4. **Standard Attribute Catalog** (표준 `@` 속성 전체 목록)

부록으로 TypeScript 및 Python의 완전한 타입 정의를 포함합니다.
