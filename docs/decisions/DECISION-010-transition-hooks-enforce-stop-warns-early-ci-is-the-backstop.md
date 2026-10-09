---
title: Transition hooks enforce; Stop warns early; CI is the backstop
type: decision
status: accepted
author: Jack Kaloger
date: 2026-10-02
tags:
- rfc-001
related:
- constrains: SPEC-003
---

## Context

From RFC-001 §8.1, §8.3 and §8.4 (2026-10-02). A rule the framework states but does not check is the failure the design exists to avoid. Each check needs a surface that fires whatever agent, editor or git flow is in use.

## Decision

**Surfaces and precedence.** lazyspec pre-transition hooks are primary: the document transition is the truth, and the hook fires for any agent, editor or git flow because it runs inside lazyspec. Claude Code Stop is early warning for work that never reaches a transition. CI running the same binary is the backstop, so the design is not a Claude Code feature. No guardrail depends on a prompt being seen. No pre-push hook: it duplicates CI.

**Severity is fixed by the rule.** No configuration table.

**One binary.** Every check is a subcommand of `lemma`. lazyspec `[[hooks]]` entries and Claude Code hook commands both point at it.

## Consequences

- DECISION-001 adds a per-edit surface (`PostToolUse`, informational) and states the escalation: inform per edit, block at Stop, block at the gate and in CI.
- Waivers (RFC-001 §8.3) went in release 1.3 (DELTA-004). A claim that should not be machine-checked is `[intent]`.
- RFC-001 fixed severity by surface. Release 1.4 (DELTA-005) replaced that with severity by kind: invariants are errors, drift is a warning.
