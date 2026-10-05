---
title: "Lemma is a signal source, not a verifier"
type: decision
status: proposed
author: "Jack Kaloger"
date: 2026-10-05
tags: []
related: []
---

## Context

Releases 1.1–1.5 grew lemma toward a universal verifier: commit-pinned receipts, freshness, symbol drift, `re-pin`, `--force`, `[forced]`, `@base`, transition-specific gates. Each answered a real hydra finding, but together they make lemma heavy, token-costly and hard to inspect. They also stretch it into questions it cannot settle mechanically: whether a test still proves a claim is semantic, and a re-pin that succeeds because the test body is unchanged verifies nothing.

Lemma is one signal source in a stack, not the whole stack:

| Layer | Owner | Question |
| --- | --- | --- |
| Change coverage | git + lemma | what changed, which specs it affects, does a delta cover it |
| Spec integrity | lemma | claims have ids, evidence exists, edits are based on the version they replace, apply cannot silently overwrite |
| Test result | CI / test runner | do the tests pass |
| Code conventions | compiler, linter, formatter, static analysis | is the code well-formed |
| Repo conventions | repo hooks, CI checks | project-specific rules, pending tests included |
| Claim truth (later) | jev and similar | is the claim true of the system |
| Semantic adequacy | human review | does the test still prove the claim, is the claim right, does the delta capture the change |

Supersedes RFC-001 §1's thesis ("hooks are how 'no unverified completion' becomes a true sentence") and §6 (receipts, freshness, re-pin).

## Decision

**Scope.** Lemma verifies that implementation changes and specification changes stay mechanically connected during agentic development. Nothing else.

**Connected** means four checks, all mechanical:

1. Every changed governed file has an in-flight delta covering its spec.
2. Every changed piece of evidence for a landed claim has a delta that changes that claim.
3. Every live claim has evidence, unless marked `[intent]`.
4. Every delta edit is based on the claim version it replaces; `apply` fails rather than overwrite.

**Claims are immutable.** A claim's text never changes in a spec. A delta changes behaviour by striking an id and adding a new one. Two deltas striking one id: the second `apply` fails, because the id is gone. Check 4 needs no stored hash; `@base` goes.

**Evidence lives in the docs, unpinned.** A claim names its test with `@ref <path>#<symbol>`, no commit and no hash. Code stays free of spec ids. The evidence region is the referenced symbol, attributes included; an `it`/`test` block is referenced by its title string. Languages without an extractor fall back to the whole file. Check 3: the ref resolves to a symbol in a file matching the governing spec's `tests` globs. Check 2 diffs evidence regions against the merge-base with `main`; renaming or deleting the symbol fails check 3. Commit pins, `re-pin`, `receipt`, `base`, `[forced]` and receipt freshness go.

**No freshness.** Lemma never asks whether evidence is still adequate. A changed evidence region is reported as uncovered (check 2), and the human judges it at `complete`.

**Output is diagnostics.** `file:line: message` plus a next command, exit 2 on error, silent when clean. Same shape as compiler and linter output so agents treat lemma like the rest of the stack.

**Signals escalate.** The same checks run per edit (Claude Code `PostToolUse`, scoped to the touched file, said once per session), at Stop (warning), and at the gate and in CI (block).

**Context is a signal too.** First touch of a governed area in a session emits one line of pointers from lazyspec's graph (spec, dictums, decisions), not their content.

**Transitions live in config.** Lazyspec `[[hooks]]` entries compose lemma commands; lemma holds no lifecycle knowledge.

**Commands.** `lemma check [range]` runs checks 1–4 (and is the per-edit signal); `lemma apply <delta>` folds; `lemma refs <claim>` lists each ref and whether it resolves.

**Extensible.** A new signal (jev claim truth, test-report ingestion) is a new source in the stack with its own command and diagnostics, not a new mode of `check`.

## Consequences

- Lemma 2.0: format break. Migration strips the commit from each `@ref` line and removes `@base` and `[forced]` from deltas and specs.
- Removed: `repin.rs`, `base.rs`, most of `receipt.rs` and `transition.rs`, the `[forced]` and freshness paths in `validate.rs`. Pending-test detection moves out of lemma to repo conventions.
- Fixing a claim's wording means strike plus new id, with its `@ref` lines carried to the new id in the same delta. Id churn is the main cost.
- Refactoring inside a referenced test needs a delta naming the claim. Chores touching tests get heavier.
- Check 2 needs `main` reachable. CI is authoritative; local signals are best effort against local `main`.
- The evidence-region extractor is new work: lazyspec's symbol extractors omit Rust attributes (`#[should_panic]`, `#[ignore]`) and cannot find `it`/`test` blocks by title. Both fixes go upstream to lazyspec (`.scratch/lazyspec-contract.md`); `lazyspec pin` has the same blind spot.
- Spoofing an evidence change requires editing a referenced region, which check 2 reports against protected `main`. Repointing a ref is a doc edit inside a delta, visible at `complete`.
- Release 1.5 W37 (hydra acceptance) still runs, as the baseline 2.0 must beat on tokens and refusals.
