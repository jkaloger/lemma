---
title: A spec carries what code cannot
type: decision
status: accepted
author: Jack Kaloger
date: 2026-10-02
tags:
- rfc-001
related:
- affects: SPEC-001
- affects: SPEC-002
- affects: SPEC-003
- affects: SPEC-004
---

## Context

From RFC-001 §2, §5.1, §5.3 and §5.5 (2026-10-02).

A spec that restates the code is worse than none: it drifts, and the agent reading it cannot tell which source to trust. The reader is a memoryless agent at build time and at change time.

## Decision

A spec carries intent, constraints, non-goals, trade-offs and rationale. Anything that can be stated precisely in code is code, not prose. Brevity and rationale are the bar, not completeness.

Source of truth by kind of fact:

| Fact | Owner |
| --- | --- |
| intent, non-goals, constraints, rationale | the spec |
| tested behaviour | the test suite |
| interfaces | code, by `@ref`, never prose |
| structure | dictums |
| decisions | decision records |
| change | git |

A document earns its place only when the memoryless agent cannot reconstruct the information from code, tests and git.

**Two lifespans.** Evergreen documents (`spec`, `convention`, `dictum`) describe what the system is now and go stale when code under their `governs` changes. Point-in-time documents (`delta`, `decision`) describe a change or a choice, go stale by age, and are superseded rather than edited.

**One spec per capability.** Its `governs` lists every code area that implements it. Overlapping globs are expected. Rejected: one spec per module (intent gets several homes) and one per user-facing feature (does not map onto `governs`).

**Rationale lives in decisions.** One decision per consequential choice, linked to what it affects, superseded rather than edited. Hydra trees are long-form rationale, referenced and never restated. Commit messages carry nothing durable.

Dropped from earlier notes: deterministic multi-agent reconciliation of spec changes. No repository showed evidence for it, and it risks a manifest format that becomes a second source of truth.

## Consequences

- RFC-001 also had planned behaviour live in the spec as a promissory note until a test settled it. DECISION-003 removes that: specs hold intent only, and claims live in deltas.
- RFC-001 named seven types, including `idea` and `bug`. Release 1.3 (DELTA-004) cut them to five: a bug is a draft delta whose first claim reproduces the defect, and an idea is a draft spec with `Intent` only.
