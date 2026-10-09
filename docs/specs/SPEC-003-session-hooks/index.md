---
title: "Session hooks"
type: spec
status: active
author: "Jack Kaloger"
date: 2026-10-09
tags: []
governs:
  - "crates/lemma/src/session.rs"
  - "crates/lemma/src/signal.rs"
  - "crates/lemma/src/stop.rs"
  - "crates/lemma/src/payload.rs"
  - "crates/lemma/src/project.rs"
  - "crates/lemma/src/git.rs"
  - "crates/lemma/src/main.rs"
  - "plugin/**"
  - ".claude-plugin/**"
tests:
  - "crates/lemma/tests/**"
related: []
---

## Intent

`lemma session-start`, `lemma signal` and `lemma stop`, and the Claude Code plugin that registers them. They give an agent scoped context and early warning inside a session. Prompting cannot scope context to the code being touched; these hooks can. `signal` points the agent at the specs, decisions, dictums and in-flight deltas governing the file it just touched. `stop` refuses to end a turn that leaves error findings behind.

## Constraints

- Signals escalate: per edit they inform, at Stop they block the turn, at the gate and in CI they block the change (DECISION-001, DECISION-010).
- Context is pointers to documents, not their content.
- Each message is said once per session, keyed on a stable identity rather than rendered text.
- `signal` never blocks and exits 0 on any failure. Every hook is a no-op when `lemma` is not on `PATH`.
- The session base is recorded once at session start and kept on resume.
- Completing a delta is the human's step. `stop` reports a delta completed during the session (DECISION-009).
- The plugin carries hook entries only. No skills (DECISION-003, superseding DECISION-011).

## Non-goals

- A PreToolUse edit gate bounded to a delta's `governs`. Trigger: after-the-fact enforcement at Stop proves too late (DECISION-009).
- Enforcing through a prompt. No guardrail depends on a prompt being seen (DECISION-010).
- Reading runner transcripts.

## Open questions

- Whether lazyspec can tell a human transition from an agent status edit. Asked of lazyspec; until it answers, the in-session completion rule in `stop` is the guardrail.
