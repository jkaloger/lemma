---
title: 'Release 1.2: specs and deltas become bundles'
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

Specs and deltas become lazyspec bundles. Claims move out of `## Behaviour` sections into `behaviour` parts.

## Out of scope

- Back-compat with flat spec and delta files. The only adopter is the hydra acceptance clone; it is re-created at W13 (DELTA-002).
- RFC-001 is untouched.
