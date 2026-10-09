---
title: 'Release 2.0: lemma is a signal source'
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

Implements DECISION-001: lemma is one signal source in a stack. It checks that implementation changes and spec changes stay mechanically connected, and nothing else. Starts from 1.5 (`6a43299`).

## Out of scope

- Extractor fixes: upstream to lazyspec (`.scratch/lazyspec-contract.md`); lemma uses the 0.15 extractors as they are.
- jev and test-report ingestion: future signal sources.
- Pending-test detection: moves to repo conventions.
- Not done: W47 review, the whole-diff review against DECISION-001 and the plan. Left unticked in the plan.
- Not done: W48 acceptance (human), hydra on 2.0 after `lemma migrate`, comparing refusals and token cost against the 1.5 W37 baseline. DELTA-008 ran the first acceptance (its W48) on the 2.1 work instead.
