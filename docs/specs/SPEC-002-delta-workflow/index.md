---
title: "Delta workflow"
type: spec
status: active
author: "Jack Kaloger"
date: 2026-10-09
tags: []
governs:
  - "crates/lemma/src/review.rs"
  - "crates/lemma/src/reviews.rs"
  - "crates/lemma/src/plan.rs"
  - "crates/lemma/src/build.rs"
  - "crates/lemma/src/gate.rs"
  - "crates/lemma/src/advance.rs"
  - "crates/lemma/src/annotate.rs"
  - "crates/lemma/src/runner.rs"
  - "crates/lemma/src/prompts.rs"
  - "crates/lemma/src/context.rs"
  - "crates/lemma/src/schema.rs"
  - "crates/lemma/src/claims.rs"
  - "crates/lemma/src/tasks.rs"
  - "crates/lemma/src/payload.rs"
  - "crates/lemma/src/project.rs"
  - "crates/lemma/src/git.rs"
  - "crates/lemma/src/testgit.rs"
  - "crates/lemma/src/main.rs"
  - ".lazyspec/prompts/**"
  - "crates/lemma/src/activity.rs"
tests:
  - "crates/lemma/tests/**"
  - "crates/lemma/src/activity.rs"
  - "crates/lemma/src/runner.rs"
  - "crates/lemma/src/output.rs"
related: []
---

## Intent

`lemma review`, `lemma plan` and `lemma build`: the loop that turns a human-written delta into committed, evidenced work. A human writes intent, design and behaviour. An agent reviews the draft and the human resolves each finding by deleting it. The planner assigns claim ids and names tasks. The build loop runs each task in a fresh runner, reviews the task's diff and commits it once its claims have refs and the review is clean.

The human types four commands: `lemma review`, `lemma plan`, `lemma build`, and `lazyspec update --status complete`.

## Constraints

- Humans write documents; agents annotate; a human deletes an annotation to accept it (CONVENTION principle 3, DECISION-003).
- Review never plans. Plan runs only on a draft delta with no review lines left.
- The runner never edits a delta part. lemma writes ids, tasks and refs from the runner's JSON reply.
- Every review-gate finding cites a written basis: the task, the design, the convention, a claim, a dictum rule or a related document. Fix rounds are bounded by `review_rounds`, and nothing commits around a finding (DECISION-004).
- Build never completes a delta. Completion is the human's step (DECISION-009).
- A stop never resets the working tree. A re-run resumes at the first unticked task.
- No shell layer. Orchestration is child processes plus JSON. The runner is a `lemma.toml` command string, and every reply is checked against a JSON Schema generated from the delta.
- Prompts are pack files a human can edit. The binary carries the same defaults.
- One agent per delta (DECISION-013).

## Non-goals

- A whole-delta review pass after the last task. Trigger: cross-task drift reaches `complete` twice in dogfooding (DECISION-004).
- A path fence on what a task may touch. Trigger: an out-of-scope edit passes the gate (DECISION-004).
- `--no-review`. A stub reviewer command covers testing.
- Parallel tasks within a delta. Trigger: a build whose ordering is false for more than a handful of tasks (DECISION-003).
- Runner adapters beyond the JSON reply contract (DECISION-003).
- Live typed progress events. The watch line covers it. Trigger: the watch line is not enough in two dogfood runs.
- Offering `complete` from `lemma review`.

## Open questions

- Whether completed deltas should feed back into spec intent, and how. Raised during 3.1 dogfooding; not designed.
- Whether a plan done in the same breath as a lenient review surprises the human. `plan` is now its own command; the trigger for more is a plan surprising the human twice (DECISION-003).
