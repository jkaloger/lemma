---
title: 'Release 3.4: refs are tests by symbol'
type: delta
status: draft
author: Jack Kaloger
date: 2026-10-12
tags: []
related:
- changes: SPEC-001
- changes: SPEC-002
---

## Intent

DELTA-012 T2 failed lint on valid work. The runner wrote colocated Rust unit tests and cited them; check 3 rejected the refs because the file was not in SPEC-002 `tests`. The crate has no `lib.rs`, so unit tests are the only way to test pure functions, and a glob admitting the file also admits the implementation fn beside the tests. DECISION-014 moves test-ness from the file path to the symbol. This delta makes check 3 read it from the symbol where an extractor exists (Rust test attributes and `#[cfg(test)]` modules, TS/JS `it`/`test` blocks), keeping `tests` globs as fallback and explicit override. It also makes `lemma build` resolve reply refs with the same resolver before writing `claims.md`, so a bad ref is a fix round, not a lint failure later. The build and fix prompts say where tests may live.

The stopgap entry for `crates/lemma/src/activity.rs` in SPEC-002 `tests` can go once this lands. That is a human spec edit, a chore task in this delta, not delta behaviour.

## Out of scope

- Test-macro detection beyond the attribute rule. `rstest`, `proptest!` and custom macros use the `tests` override.
- Changing check 2.
- Extractor support for new languages.
