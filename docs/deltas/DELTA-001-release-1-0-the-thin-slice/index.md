---
title: 'Release 1.0: the thin slice'
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

Build the thin slice in RFC-001 §13: spec, delta and decision types, the `complete` gate, the fold, Stop, `re-pin` and the two skills, plus CI. README states behaviour; this release fixes the gaps README leaves open and does the work.

## Out of scope

- Everything README marks release 2+: approved-by gate, verified-by, waivers, contract mutation, intent/waiver info count, idea/bug/convention/dictum types, `prompt` subcommand, other skills.
- Dictums in `init --template`: lazyspec copies only templates and hooks, so the entry skill copies dictums. Open lazyspec-boundary item.
