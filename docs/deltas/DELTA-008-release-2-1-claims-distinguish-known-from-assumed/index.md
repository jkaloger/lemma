---
title: 'Release 2.1: claims distinguish known from assumed'
type: delta
status: complete
author: Jack Kaloger
date: 2026-10-06
tags: []
related:
- changes: SPEC-001
- changes: SPEC-003
- changes: SPEC-004
reviewed: 87c3c31669f2ac169e76437be0f2b35b3e1c5889
---

## Intent

Implements DECISION-002: claims distinguish what is known from what is assumed. Starts from 2.0 (`2bf8a50`). Work is ordered so the items that break no format land first, the hydra acceptance (W48) runs on them, and the marker and rename work follows with one migration.

## Out of scope

- Evidence sources beyond test refs; topology and history signals; runtime signals; jev.
- A lazyspec-side distinction between human and agent transitions (ask in `.scratch/lazyspec-contract.md`).
- An `[observed]` ratio finding: back pocket, trigger is adopt mirroring a suite.
- Lemma on its own repository: trigger is W59 recorded; `lemma adopt` this repo on 2.1 and make 2.2 the first delta, which exercises adopt's `[observed]` prose on a real codebase.
- Not done: W55 rename (decision 68, `behaviour` to `claims`, `affects` to `constrains`) — stopped after W54; superseded by DECISION-003 and DELTA-009.
- Not done: W56 `migrate` for 2.0 to 2.1 (decision 69) — superseded by DECISION-003 and DELTA-009.
- Not done: W57 README, CI template, version 2.1.0 (decision 70) — superseded by DECISION-003 and DELTA-009.
- Not done: W60 W48 follow-ups, prose and migrate (F5, F6, F1, F2) — superseded by DECISION-003 and DELTA-009.
- Not done: W58 whole-diff review against DECISION-002 and the plan — superseded by DECISION-003 and DELTA-009.
- Not done: W59 acceptance (human), hydra on 2.1 after `lemma migrate`, deciding whether `[observed]` needs the ratio finding — superseded by DECISION-003 and DELTA-009.
- Open at the time: whether lazyspec can tell a human transition from an agent status edit; unanswered, the `lemma stop` completion rule stayed the guardrail. Whether `[observed]` gets used to mirror a suite; W59 never ran. When lazyspec RFC-076 lands; the entry skill's seed-copy step was a stopgap until then.
