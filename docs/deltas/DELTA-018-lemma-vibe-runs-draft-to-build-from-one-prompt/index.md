---
title: "lemma vibe runs draft to build from one prompt"
type: delta
status: draft
author: "Jack Kaloger"
date: 2026-10-23
tags: []
related:
  - changes: SPEC-002
---

## Intent

Today one change takes four human steps, each one waiting on the last: write the delta, `lemma review` until clean, `lemma plan` then accept or reject `[proposed]` claims, `lemma build`. For small or throwaway changes that wait costs more than the review is worth. `lemma vibe -m "<prompt>"` runs the whole loop unattended from one sentence: an agent drafts the delta, review and revise run for up to N rounds, plan runs, `[proposed]` claims are accepted, and build runs to the last task. Output is colourful so a long run is watchable.

Vibe is an opt-in exception to principle 3 (humans write documents; a human deletes an annotation to accept it) and to the README's "not a vibe coding tool". The other gates stay: every task still passes the review gate and commits only when clean, and completion is still the human's step (DECISION-009).

## Out of scope

- Marking the delta complete. Vibe ends at the build and prints `next: lazyspec update <id> --status complete`.
- Writing or accepting DECISION docs. "Decision acceptance" here means accepting `[proposed]` claims.
- Picking up an existing delta. Vibe always starts a new one; `lemma review|plan|build` cover existing ones.
- Parallel deltas, or more than one agent per delta (DECISION-013).
- Interactive pauses between stages. A run that needs a human stops and prints the command to resume by hand.
