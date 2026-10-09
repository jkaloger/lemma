---
title: 'Release 1.1: hydra adoption fixes'
type: delta
status: complete
author: Jack Kaloger
date: 2026-10-05
tags: []
related:
- changes: SPEC-001
- changes: SPEC-003
reviewed: 87c3c31669f2ac169e76437be0f2b35b3e1c5889
---

## Intent

Fixes from hydra adoption (RFC-001 §13 steps 1-3, 2026-10-04/05). Friction numbers refer to that log. Lazyspec-side asks go to `.scratch/lazyspec-contract.md`.

## Out of scope

- Friction 1 (convention/dictum types into release 1) and 2 (rationale link to non-lazyspec doc): pending decision. Revisit after acceptance steps 4-5 run in hydra. Open at the time: both; friction 2 was settled by decision 46 (DELTA-006, rationale link is provenance); friction 1 went to release 2 types.
- Lazyspec-side: 1 (init copying pack documents), 3 (`{author}`), 7 (suppress `reviewed:` per type), 8 (`hook run --to`).
- Deferred: 4 (report scope globs matching nothing).
- Not done: W13 acceptance, re-run hydra steps 1-3 against 1.1, then steps 4-5 (needs a second spec, e.g. `src/store.rs`), feeding back into friction 1 and 2. Superseded by the release 1.5 hydra run (DELTA-006).
