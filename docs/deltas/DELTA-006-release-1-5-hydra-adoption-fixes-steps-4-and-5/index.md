---
title: 'Release 1.5: hydra adoption fixes, steps 4 and 5'
type: delta
status: complete
author: Jack Kaloger
date: 2026-10-05
tags: []
related:
- changes: SPEC-001
- changes: SPEC-003
- changes: SPEC-004
reviewed: 87c3c31669f2ac169e76437be0f2b35b3e1c5889
---

## Intent

Fixes from hydra adoption, acceptance steps 4 and 5 (2026-10-05 log; numbers in the scope lines are its entries). Covers entries 1, 2, 4, 5, 7, 9, 11, 12, 13, 14, 15, 16, 20, 21, 23, 24, 25, 26, and the skill side of 3, 6 and 18. Lazyspec-side asks go to `.scratch/lazyspec-contract.md`.

## Out of scope

- Lazyspec-side, hydra entries 3 (`init` copies dictums), 6 (list attribute kind), 8 (inverse relations on `show`), 18 (cache dir ignored or under `.git/`), 27 (bundle delete leaves parts).
- Symbol-granular freshness. A receipt stays stale when its file or governed code changed since the pin (a governed change should be re-verified); only the re-pin refusal moves to the symbol (decision 35).
- Not done: W36 `.scratch/lazyspec-contract.md` for asks 3, 6, 8, 18, 27. Left unticked in the plan; the asks stayed lazyspec-side.
- Not done: W37 acceptance, re-run hydra steps 4-5 against 1.5 with the `lemma-adoption` branch. It never ran; the 2.1 hydra run (DELTA-008 W48) became the baseline instead.
