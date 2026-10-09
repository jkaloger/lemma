---
title: 'Release 1.3: sign-off cut to claims and receipts'
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

Cuts the sign-off vocabulary to claims, receipts and `[intent]`. Approval becomes the `in-progress` to `complete` transition, receipts become human-checked, `complete` folds, and the type list shrinks to five. Starts after release 1.2 lands.

## Out of scope

- Removing `waived:` and `verified-by` needs no code (both were release 2); README only.
