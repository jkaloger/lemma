---
title: "Review gates each task in build"
type: decision
status: accepted
author: "Jack Kaloger"
date: 2026-10-09
tags: []
related: []
reviewed: 159b9633f0e830653b007fb2553124a9a4352018
---

## Context

DECISION-003 has `lemma review` on an in-progress delta read the diff and write follow-up tasks. Review then makes work instead of gating it. Findings land as tasks after the work already committed, the human prunes them, build runs the rest. Nothing judges a task against its task text, design, decisions or dictums before it commits. Principle 1, no unverified completion, holds only as far as lint and refs reach.

## Decision

**Review gates each task in `build`.** After lint and refs pass and before the tick, the reviewer judges the task's staged diff against written sources: the task, its claims, the design, the convention, governing dictums, related documents. The delta's diff so far is read-only context.

**Every finding cites a written basis.** The reply schema limits `basis` to ids in context. A finding with no basis cannot be expressed.

**Findings go back to the builder, bounded.** A fix round resumes the builder's session where the runner supports it. `review_rounds` caps the rounds, default 1. Findings left after the cap stop the build, tree kept. Nothing commits around a finding. Findings are never written to delta parts.

**In-progress `review` goes.** `lemma review` is for draft deltas only. The `review-diff` prompt goes. `## Follow-ups` stays as a human-written section.

Supersedes DECISION-003's clause "On an in-progress delta `review` reads the diff and writes follow-up tasks" and `review-diff` in its prompt list. The rest of DECISION-003 stands.

## Consequences

- Each task costs one reviewer call, and one runner plus one reviewer call per fix round.
- Cross-task drift is seen only as context, not judged. A whole-delta pass is deferred. Trigger: cross-task drift reaches `complete` twice in dogfooding.
- A gate cannot fence paths. Scope creep is a finding against `task`. Trigger for a path fence: an out-of-scope edit passes the gate.
- A commit's `Lemma-Review:` trailer records that the gate ran. No delta part does.
