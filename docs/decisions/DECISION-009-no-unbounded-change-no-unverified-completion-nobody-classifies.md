---
title: No unbounded change, no unverified completion, nobody classifies
type: decision
status: accepted
author: Jack Kaloger
date: 2026-10-02
tags:
- rfc-001
related:
- constrains: SPEC-001
- constrains: SPEC-002
- constrains: SPEC-003
---

## Context

From RFC-001 §7.1, §7.3 and §7.4 (2026-10-02). "No code without a plan" over-binds a scoped change and under-binds a planned one: a plan without verification proves nothing, while verification without a plan still bounds the change.

## Decision

**Governing principle.** No unbounded code changes; no unverified completion. An agent may make a scoped change directly when the outcome is clear, permissions are appropriate and verification is available. A reviewed approach is required when uncertainty, coordination or risk calls for one.

**Nobody classifies.** Fix, chore, refactor and spike are distinctions held in a dictum (DICTUM-002) and in the agent's judgement. lemma reads only structural signals: is there an in-flight delta, and does the diff cross `governs` areas. Misjudgement is caught by the codebase's own checks and by the cross-area check, not by a label. Sensitive-path lists and diff-size ceilings were rejected.

**Bound.** A direct change is bounded to one `governs` area. Crossing areas needs an in-flight delta, checked after the fact at Stop, at the gate and in CI. A PreToolUse edit gate is held back until enforcement at Stop proves too late.

**Exit gate is human.** Completion is the human's step, at `in-progress` → `complete`.

## Consequences

- The per-workflow rules are DICTUM-002 CHG-1 to CHG-8 and CHG-11.
- RFC-001 let a human pre-approve completion with `approved-by: <name> @ <sha>`. Release 1.3 (DELTA-004) removed it: the transition itself is the approval.
- "Ready for review" is derived, not a status.
