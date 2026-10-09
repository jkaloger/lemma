---
title: 'Release 1.4: the gate and fold check what they trust'
type: delta
status: complete
author: Jack Kaloger
date: 2026-10-05
tags: []
related:
- changes: SPEC-001
- changes: SPEC-004
reviewed: 87c3c31669f2ac169e76437be0f2b35b3e1c5889
---

## Intent

Closes the gaps from the 2026-10-05 review: the gate and fold trust state they never checked, and validate cannot fail CI. Starts after release 1.3 lands.

## Out of scope

- Test execution in lemma. Receipts stay symbol resolution; the trust model is stated, not changed.
- Retrospective reconciliation of human edits: hydra interview first, release 2.
