---
title: A ref is a test by its symbol, not its path
type: decision
status: proposed
author: Jack Kaloger
date: 2026-10-12
tags: []
related:
- constrains: SPEC-001
---

## Context

DELTA-012 T2: the build runner wrote Rust unit tests in `#[cfg(test)] mod tests` inside `crates/lemma/src/activity.rs` and cited them as `crates/lemma/src/activity.rs#tool_targets_follow_the_tool_kind`. Check 3 of DECISION-001 rejected the refs: not in the governing spec's `tests` globs. SPEC-002 `tests` was only `crates/lemma/tests/**`.

The crate has no `lib.rs`, so integration tests cannot call its pure functions. DELTA-012's design asks for pure seams tested without a terminal, which in Rust means colocated unit tests. The build prompt never shows the runner the `tests` globs (`context.rs` sends the spec body, frontmatter dropped), so the runner had no way to know. A stopgap lists `activity.rs` in SPEC-002 `tests` and `governs`.

Check 3 asks "is this ref a test?", a property of a symbol, and answers with a file-path glob. A glob admitting `activity.rs` also admits the implementation fn `activity`, so a claim could cite the code it describes. Tight per-file globs narrow the hole and need upkeep on every spec.

## Decision

Refines check 3 of DECISION-001. Check 3 decides test-ness per symbol where an extractor exists:

- **Rust:** the symbol carries a test attribute (`#[test]`, `#[tokio::test]`, any `#[...::test]`), or sits inside a `#[cfg(test)]` module.
- **TypeScript/JavaScript:** the symbol is an `it` or `test` block, already how those refs are written.
- **No extractor:** fall back to the governing spec's `tests` globs, as today.

`tests` stays as an explicit override. A ref passes if the symbol is a test, or the file matches `tests`. Symbol test-ness is sufficient on its own. The override covers what syntax cannot see (`rstest`, `proptest!`, custom macros) and languages without an extractor.

**Replies are checked before they are written.** `lemma build` resolves each ref in a runner reply with the same resolver lint uses, before writing `claims.md`. A failing ref is a fix round, or blocked once rounds run out. It is never written. The runner cannot produce a ref lint will reject.

Why not path globs: a path cannot tell a test from the code beside it.

**Voiding condition.** A supported language where test-ness cannot be read from syntax. That language gets no extractor rule and stays on `tests` globs.

## Consequences

- A ref to an implementation fn fails even in a file that also holds tests, unless that file is listed in `tests`. The override keeps that hole open only where a human chose it.
- Colocated Rust tests need no spec edit. `tests` entries for source files become removable.
- Check 3 depends on the extractors reading attributes and enclosing modules. DECISION-001 already requires attributes in the evidence region.
- Check 2 is unchanged: it diffs evidence regions, whatever made them evidence.
- The build and fix prompts must say where tests may live, so the runner writes valid refs first time.
