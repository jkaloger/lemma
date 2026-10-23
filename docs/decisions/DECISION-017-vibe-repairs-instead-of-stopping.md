---
title: "vibe repairs instead of stopping"
type: decision
status: accepted
author: "Jack Kaloger"
date: 2026-10-23
tags: []
related:
  - constrains: SPEC-002
---

## Context

DECISION-015 lets `lemma vibe` run the delta loop from one prompt, but any stage error still stops the run. DELTA-020 stopped at build because the planner left an `[intent]` claim unsettled: plan wrote a `tasks.md` the draft → in-progress gate rejects. Plan holds the same coverage rule (`is_planned`) but never applies it to its own output. The human is left to repair a delta they asked an agent to write. A vibe run that ends with a hand-fix to do costs the human more than the waits vibe removed.

## Decision

A `lemma vibe` run ends with the delta built, unless the human cancels it or the environment can't run it (no runner, no check command, no git). Inside the run, every finding a stage produces goes back to an agent as input, and the stage runs again. No finding is a reason to stop.

- An invalid reply (draft, revise, plan) is re-asked with the error.
- Plan output can't break the shape rules the gate checks. Where a rule can be a property of the reply schema, it is.
- Gate findings at draft → in-progress go to revise, then the gate runs again.
- A build task that runs out of `review_rounds` is re-planned. Its diff is discarded, and the planner replaces the task with one or more tasks that settle the same claims, given the findings that stopped it. Build continues from the first replacement.

Everything else in DECISION-015 holds. Each task passes the check command (DECISION-016) and the task review gate before it commits, and completion stays the human's step (DECISION-009).

## Consequences

- A vibe delta can arrive at complete with tasks that differ from the first plan. The human reads `tasks.md` and git history at complete.
- Re-planning can recurse when a replacement task also stops. The cap wins over "always ends built" (principle 1): `vibe_repairs` (default 3) bounds re-asks per stage and re-plan depth per task (a replacement is its parent's depth plus one). Hitting the cap is the one finding-driven stop.
- Re-planning discards the stopped task's diff. This qualifies SPEC-002's "a stop never resets the working tree" under vibe, and "plan runs only on a draft delta" for the re-plan of a stopped task.
- Plain `lemma build` keeps stopping on an exhausted task. Re-plan is vibe-only, because outside vibe a human owns the plan.
- Revisit trigger: a vibe delta reaches complete with a claim settled by a task that doesn't prove it, because a re-plan weakened the task to get past review.
