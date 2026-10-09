---
title: 'Release 3.0: nothing folds'
type: delta
status: complete
author: Jack Kaloger
date: 2026-10-07
tags: []
related:
- changes: SPEC-001
- changes: SPEC-002
- changes: SPEC-003
- changes: SPEC-004
reviewed: 87c3c31669f2ac169e76437be0f2b35b3e1c5889
---

## Intent

Implements DECISION-003: nothing folds; specs hold intent; deltas are hand-written, agent-annotated, binary-built. Starts from 2.1 at W54 (the uncommitted dictum work lands first). W55 to W60 of release 2.1 (DELTA-008) are superseded.

## Out of scope

- Lint via reviewer for prose rules. Trigger in DECISION-003.
- Parallel tasks. Trigger in DECISION-003.
- Runner adapters beyond a `result` JSON field. Trigger in DECISION-003.
- Derived commitments report. Trigger in DECISION-003.
- `migrate` from 2.x. Trigger in DECISION-003.
- Not done: W75 dogfood (human) — `lazyspec init --template . --force` on this repo; hand-write one delta for a 3.1 item; run review, plan, advance, build, review, advance end to end; log to `.scratch/dogfood-3.0.md`; decide each DECISION-003 trigger. Left to the human; 3.1 (DELTA-010) and 3.2 (DELTA-011) followed from its findings.
- Not done: W55–W60 of release 2.1 — superseded by this release (DELTA-008).
- Open at the time: shape and stability of `claude -p --output-format json` across versions. W66 pins what is read (`result`, `is_error`) and ignores the rest.
- Open at the time: whether runner sessions need the plugin hooks once build runs lint itself. W72 decided: kept. Build runs lint after the runner returns; `signal` gives the runner context and findings while it works, and `stop` blocks a runner that leaves a touched delta's refs broken. Revisit in W75 if runner sessions show noise.
- Open at the time: what `lemma stop` guards when transitions are human-typed. W72 decided: build never completes a delta, but an agent can still run `lazyspec advance`, so the completion rule stays, reworded to name completion as the human's step after `lemma review`.
