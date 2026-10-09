---
title: "Checks"
type: spec
status: active
author: "Jack Kaloger"
date: 2026-10-09
tags: []
governs:
  - "crates/lemma/src/lint.rs"
  - "crates/lemma/src/shape.rs"
  - "crates/lemma/src/evidence.rs"
  - "crates/lemma/src/range.rs"
  - "crates/lemma/src/world.rs"
  - "crates/lemma/src/refs.rs"
  - "crates/lemma/src/claims.rs"
  - "crates/lemma/src/tasks.rs"
  - "crates/lemma/src/payload.rs"
  - "crates/lemma/src/project.rs"
  - "crates/lemma/src/git.rs"
  - "crates/lemma/src/testgit.rs"
  - "crates/lemma/src/main.rs"
tests:
  - "crates/lemma/tests/**"
related: []
---

## Intent

`lemma lint` and `lemma refs`: the mechanical checks that keep implementation changes and delta claims connected. lemma is one signal source in a stack (DECISION-001). It owns change coverage, document shape and the link between a claim and the test that settles it. It never judges whether a test proves its claim, and never runs tests.

The same checks back every surface: the lazyspec `validate` and `pre-transition` hooks, the session hooks (SPEC-003), the build loop (SPEC-002) and CI.

## Constraints

- lemma never runs tests. The test runner and CI own results; human review owns adequacy (DECISION-001).
- Refs are unpinned `@ref <path>#<symbol>`. No commit pins, hashes or freshness (DECISION-001, superseding DECISION-008).
- Specs hold no claims. A completed delta is the record of why its tests say what they say; nothing folds (DECISION-003).
- One `governs` area alone needs no delta, so chores stay free. Crossing areas needs an in-flight delta per touched spec (DECISION-009, CHG-3).
- Invariants are errors; drift is a warning. A changed test that a landed delta refs is a warning naming the delta; a changed test that a dictum rule refs is an error unless the dictum strikes the rule and adds a new id in the same range.
- Output is diagnostics: `path:line: severity: message`, silent when clean, exit 2 on error. Same shape as a compiler so agents treat lemma like the rest of the stack.
- lemma holds no lifecycle knowledge. Transitions and which gate runs on which are `.lazyspec.toml` config (DECISION-001).
- Base resolution must work on branches and on direct-to-main repos. This repository works on main.

## Non-goals

- Prose rules that need judgement. Trigger to add a reviewer-backed lint: three shape rules that need judgement (DECISION-003).
- Freshness, symbol drift, re-pinning (DECISION-001).
- Pending-test detection. Repo conventions own it.
- Test-report ingestion and claim truth (jev, DECISION-012). Each would be a new signal source with its own command, not a mode of `lint`.
- The ungoverned-file check. `[governs] unowned` is off in this repository.

## Open questions

- Check 2 and coverage need trunk reachable. CI is authoritative; local signals on a shallow clone approximate.
- Claims in completed deltas can contradict later deltas. Accepted; a derived report of live commitments is the amendment if anyone asks for the list (DECISION-003).
