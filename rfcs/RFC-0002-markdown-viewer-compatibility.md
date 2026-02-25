# M3L RFC-0002: Markdown Viewer Compatibility Improvements

> **RFC Status**: Accepted
> **Author**: UJ (iyulab)
> **Date**: 2026-02-25
> **M3L Version Target**: Next Minor
> **Affects Sections**: 1.1, 1.5(new), 2.2, 2.3, 2.5, 3.1, 4.2, 4.9, Appendix

---

## 1. Summary

본 제안은 M3L 문서가 Notion, GitHub, Obsidian, VS Code Preview 등 일반적인 마크다운 뷰어에서 열람될 때 **의미 있는 문서로서 자연스럽게 읽히는 것**을 보장하기 위한 스펙 개선안이다.

## 2. Changes

| # | 변경 | 우선순위 | 섹션 |
|---|------|----------|------|
| 3.1 | Core Principles Compatibility 강화 + 1.5 Markdown Rendering Principles 신규 | P0 | 1.1, 1.5 |
| 3.2 | 프레임워크 속성 백틱 래핑 `[Attr]` → `` `[Attr]` `` | P0 | 2.5.6 |
| 3.3 | 필드 줄 길이 ~80자 가이드라인 | P1 | 2.3.4 |
| 3.4 | 인라인 Enum `values:` 키 권장 | P1 | 3.1.7 |
| 3.5 | 주석 체계 우선순위 정립 | P1 | 4.2 |
| 3.6 | 모델 간 `---` 구분선 선택적 허용 | P2 | 2.2.5 |
| 3.7 | object 중첩 깊이 3단계 권장 | P2 | 4.9.4 |
| 3.8 | 마크다운 안전 문자 참조표 | P2 | 1.5.1 |

## 3. Backward Compatibility

모든 변경은 하위 호환을 유지한다. 기존 문법은 계속 유효하며, 새 권장 사항은 선택적이다.

## 4. Decision

- **Verdict**: ACCEPT (원안 그대로)
- **Triage Date**: 2026-02-25
