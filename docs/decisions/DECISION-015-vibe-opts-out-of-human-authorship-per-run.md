---
title: "vibe opts out of human authorship per run"
type: decision
status: proposed
author: "Jack Kaloger"
date: 2026-10-23
tags: []
related:
  - constrains: SPEC-002
---

## Context

Principle 3: humans write documents, agents annotate, a human deletes an annotation to accept it. SPEC-002 builds the delta loop on it, along with two rules that follow from it: plan runs only with no review lines left, and nothing commits around a finding (DECISION-004). For small or throwaway changes the human waits between `review`, `plan` and `build` cost more than they catch. DELTA-018 adds `lemma vibe`, which runs the loop from one prompt.

## Decision

`lemma vibe` is a per-run opt-out of principle 3 for the delta it creates. Under vibe:

- An agent writes the delta, and lemma writes the parts from the agent's reply.
- An agent resolves review lines. When review rounds run out, the remaining lines are removed with a warning and plan runs anyway.
- `[proposed]` claims are accepted without a human.

Everything else holds. Each task passes the task review gate and the check command (DECISION-016) before it commits. Completion stays the human's step (DECISION-009), and that is where the human reads what vibe wrote. Principle 3 is unchanged for every other command.

## Consequences

- A vibe delta can reach in-progress with design gaps a human would have caught. The complete step is the only human gate left, so completing a vibe delta means reading the whole delta, not just ticking it.
- The README's "not a vibe coding tool" warning gets a qualifier pointing at `lemma vibe`.
- Revisit trigger: two vibe deltas reach complete with a defect that review-while-draft would have flagged. Then drop proceed-anyway, or make vibe stop at the round limit.
