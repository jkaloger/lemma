---
title: "plan settles every claim by construction and vibe repairs instead of stopping"
type: delta
status: in-progress
author: "Jack Kaloger"
date: 2026-10-23
tags: []
related:
  - changes: SPEC-002
reviewed: 000ad137abe1f106226032788bd45f108b37b389
---

## Intent

DELTA-020's vibe run stopped at build with "AC-3 is settled by no task". The planner skipped an `[intent]` claim, plan wrote the tasks unchecked, and the gate only caught it two stages later. Plan should not be able to produce a task list the gate rejects. Make claim coverage a property of the planner's reply schema. Then carry out DECISION-017: vibe feeds every finding back to an agent and runs the stage again, re-planning a build task that runs out of review rounds, so a run ends with the delta built. A repair cap bounds each stage (DECISION-017); hitting it is the one finding-driven stop.

## Out of scope

- Re-planning in plain `lemma build`. It still stops on an exhausted task.
- Retrying environment failures (runner or check command can't start, git errors). These still stop vibe.
- Changing the shape rules themselves. `[intent]` claims still need a settling task.
- Repairing DELTA-020. Rerun it once this lands.
