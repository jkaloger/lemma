---
title: 'Nothing folds: deltas are the record, specs hold intent'
type: decision
status: accepted
author: Jack Kaloger
date: 2026-10-07
tags: []
related:
- supersedes: DECISION-002
- supersedes: DECISION-007
- supersedes: DECISION-011
- constrains: SPEC-001
- constrains: SPEC-002
- constrains: SPEC-003
- constrains: SPEC-004
supersedes:
- DECISION-002
reviewed: c7d86bf644690595c6470975c162eaded4a9548a
---

## Context

RFC-001 §5.1 sets the test for a document: it earns its place only when the memoryless agent cannot reconstruct it from code, tests and git. A folded claim is a line that refs a test. It is reconstructible from that test. The fold fails the RFC's own test.

Who reads the folded list? The build agent needs the delta's claims, the spec's intent, the dictums. The review agent needs the same plus the diff. A human reads tests. No reader needs the accumulated list, so check 4, `apply`, base-version detection and the 2.1 assumption-resolution machinery maintain an artefact nobody consumes. Dictums show the opposite: evergreen claims with refs work there because a human edits them in place. The fold tried to make agents own evergreen claims.

Second pressure: the entry and delta skills exist to coax an agent into authoring a delta. The human prefers to write the delta and have agents review and refine it. Skills that prompt authorship are then dead weight, and the "repeat myself to the agent" cost moves into prompt files the repo owns.

Third: 2.1 added `[assumed]`, `[observed]`, `@per` resolution and a grouped review surface. Real distinctions, wrong enforcement cost. The hydra W48 run already flagged `[observed]` mirroring the suite.

## Decision

**Nothing folds.** A delta is point-in-time and immutable once `complete`. Its claims stay in the delta with their refs. Cite a claim as `DELTA-004/AC-2`. If two deltas disagree, the later one and its test win, as with decisions. A derived "current commitments for SPEC-002" is a read over completed deltas, never a write.

**Specs hold intent only.** `index.md` with Intent, Constraints, Non-goals, Open questions, `governs`. No claims part. Drift-stale under `governs`, as now. Human-written.

**Deltas have four parts.** `index.md` (Intent, Out of scope, `changes: [SPEC-..]`, `related:`), `design.md` (prose; no `@ref` to code), `claims.md` (one part, not one per target), `tasks.md` (ordered checklist; each task settles claims or is `(chore)`). Ids are `AC-<n>` within the delta.

**Humans write, agents annotate, humans delete to agree.** `lemma review` on a draft writes `@review` lines under claims and `> review:` blockquotes under prose. When review is clean the same run plans: ids on claims, missing behaviours added as `[proposed]`, `tasks.md` written (append-only, refused once any task is ticked), formatting tidied. On an in-progress delta `review` reads the diff and writes follow-up tasks. Review lines and `[proposed]` block transitions; the human resolves by deleting.

**Build is a loop the binary runs.** `lemma build` runs each unticked task in a fresh runner (`lemma.toml` `runner`, default `claude -p`) with the task, design, its claims, matching dictums, spec intent and `related:` docs. After each: lint, settled claims' refs resolve, tick, commit. Stop on first failure, tree intact, resume from first unticked.

**Prompts are files in the pack.** `.lazyspec/prompts/{review-doc,plan,review-diff,build}.md`, human-editable. Binary ships defaults.

**Markers.** `[intent]` (no test can settle) and `[proposed]` (agent-added, awaiting human). `[assumed]`, `[observed]`, `@per` resolution and assumption history go. `@per` remains a citation line the reviewer reads; lemma does not resolve it.

**Checks.** Coverage (check 1, via `changes`) and the dictum variant of check 2 are unchanged. Check 3 runs on the delta at `complete`. Check 2 on a completed delta's claim becomes a warning naming the delta. Check 4 is deleted. New: shape (intent present, every claim settled by exactly one task, every task settles a claim or is a chore, no `@ref` in design, no review or `[proposed]` lines at a gate).

**Rust everything.** Orchestration is `Command` plus JSON; no shell layer. The plugin ships hooks only; skills are deleted.

Amends DECISION-001: "connected" is three checks plus shape; the fold rule is gone. Supersedes DECISION-002 (never accepted). Supersedes RFC-001 §5.4 (delta folds), §5.6 (tasks as unread scratchpad), §7.5 (claims arrive only by fold), §8 fold entries.

## Consequences

- 3.0 is a format break. No `migrate`: no production repo is on 2.x; a 2.x repo re-adopts. The 2.1 `migrate` goes with the rest.
- Claims in old deltas can contradict later ones. Accepted; the test is truth, the delta is history. A derived report is the amendment if anyone asks for the list.
- `lemma lint` with a reviewer call for prose rules is deferred. Trigger: three shape rules that need judgement.
- Parallel tasks (`after T2`) deferred. Trigger: a build whose ordering is false for more than a handful of tasks.
- Review-then-plan in one command may plan on a lenient review. Mitigation: plan output is a doc diff the human reads before `advance`. Trigger to split the verb: a plan surprises the human twice in dogfooding.
- Codex and other headless runners are a `lemma.toml` string. The JSON contract is the runner's `result` field; anything else needs an adapter, deferred.
- W54 (dictums as claim sites) lands as is; W55 to W60 are superseded.
