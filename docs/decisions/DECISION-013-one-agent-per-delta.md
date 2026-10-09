---
title: One agent per delta
type: decision
status: accepted
author: Jack Kaloger
date: 2026-10-02
tags:
- rfc-001
related:
- constrains: SPEC-002
---

## Context

From RFC-001 §11 (2026-10-02). lazyspec has no lease mechanism, and nothing in the experiments was blocked by sequential work.

## Decision

One agent per delta. Parallelism across deltas is separate agents in separate worktrees on separate deltas. Git is the collision guard.

## Consequences

- Parallelism within a delta waits until dogfooding shows a need. The fallback design is a claim owner written into the delta, checked by lint and Stop.
- `lemma build` runs tasks in order, one runner at a time.
