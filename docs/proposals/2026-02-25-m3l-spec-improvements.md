# M3L (Meta Model Markup Language) Improvement Proposal

**Document Version**: 1.0
**Date**: 2026-02-25
**Target**: M3L Specification (docs/specification.md)
**Status**: Accepted (ADAPT)

---

## Overview

This proposal identifies areas for improving the M3L specification to evolve it from an example-driven document to an implementable specification. Changes are prioritized by severity and maintain backward compatibility where possible (v0.x allows breaking changes when justified).

## Priority Definitions

| Level | Meaning | Criteria |
|---|---|---|
| **P0** | Immediate | Parser implementation blocked or core feature non-functional |
| **P1** | Core Fix | Causes confusion in practice or internal specification contradiction |
| **P2** | Practical Extension | Directly impacts production adoption |
| **P3** | Ecosystem Maturity | Needed for code generators and tooling ecosystem |

## Triage Decisions

| # | Item | Verdict | Notes |
|---|---|---|---|
| 3.1 | Formal Grammar (PEG) | ACCEPT | Added as Appendix A, token order aligned with parser: `type -> default -> attrs -> desc` |
| 3.2 | Type Catalog | ACCEPT | Added to Section 2.4 |
| 3.3 | Namespace Resolution | ACCEPT | Added to Section 2.1 |
| 4.1 | Cascade 2-tier | ADAPT | Simplified to Symbol + Extended, deprecated standalone/parameter forms |
| 4.2 | Relations Single Source | ACCEPT | Role separation clarified in Section 3.2 |
| 4.3 | Computed Expressions | ADAPT | Keep opaque string (current), add `@computed_raw` for explicit platform-specific |
| 4.4 | Enum Collision Rules | ACCEPT | Added to Section 3.1 |
| 5.1 | stdlib.m3l | DEFER | Future work — standard interfaces as separate file |
| 5.2 | Import Resolution | ACCEPT | Added to Section 5.1 |
| 5.3 | Document Defects D1-D10 | ACCEPT | All fixed |
| 6.1 | API Metadata | DEFER | P3 — future extension |
| 6.2 | Error Catalog | ACCEPT | Error codes renamed to M3L-Exxx format |
| 6.3 | Access Control | DEFER | P3 — future extension |

## Document Defects Fixed

| ID | Location | Issue | Fix |
|---|---|---|---|
| D1 | 3.3 | Section number jump 3.3.3 → 3.3.5 | Renumbered to 3.3.4 |
| D2 | 8 | Title "M3L Simple Extensions" vs TOC "Best Practices" | Split into Section 8 (Extensions) and Section 9 (Best Practices) |
| D3 | 4.4 | Korean comments mixed in English spec | Translated to English |
| D4 | 3.2.1.1 | Unclosed code fence | Added closing ``` |
| D5 | 7.1 | `Deletable` interface undefined | Added definition in example |
| D6 | 3.4.4 | `ContentBase`, `Commentable`, `Shareable` undefined | Marked as illustrative |
| D7 | 7.1-7.2 | `BaseModel` definition inconsistency | Unified with Timestampable inheritance |
| D8 | Various | `Category` referenced but undefined | Added Category definition in examples |
| D9 | 8.1.2 | `datetime` type not in official type list | Changed to `timestamp` |
| D10 | 2.3.1.1 | `money` precision mismatch | Unified to `decimal(19,4)` |

## Changes Applied

See the specification changelog in `docs/specification.md` for detailed changes.
