---
title: 'Release 3.2: review gates each task in build'
type: delta
status: complete
author: Jack Kaloger
date: 2026-10-09
tags: []
related:
- changes: SPEC-002
- changes: SPEC-004
reviewed: 87c3c31669f2ac169e76437be0f2b35b3e1c5889
---

## Intent

`lemma build` commits a task once lint is clean and its refs resolve. Nothing judges the work against the task, design, decisions or dictums before it lands. In-progress review sits outside the loop: it turns findings into follow-up tasks, so review makes more work rather than gating work already done. Release 3.2 moves review into build, one gate per task. Findings go back to the builder, bounded. In-progress `review` goes. Starting point: 3.1 (DELTA-010).

## Out of scope

- A whole-delta review pass after the last task. Trigger: cross-task drift reaches `complete` twice in dogfooding.
- A path fence on what a task may touch. Trigger: an out-of-scope edit passes the gate.
- `--no-review`. A reviewer command set to a stub covers testing.
- Open at the time: version. Removing `review` on in-progress deltas drops behaviour; is that 3.2.0 or 4.0.0? Default was 3.2.0 because the pack is pre-adoption outside dogfood. Resolved: shipped as 3.2.0.
