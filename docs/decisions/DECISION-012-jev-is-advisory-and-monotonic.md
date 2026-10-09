---
title: jev is advisory and monotonic
type: decision
status: accepted
author: Jack Kaloger
date: 2026-10-02
tags:
- rfc-001
related:
- affects: SPEC-001
---

## Context

From RFC-001 §10 (2026-10-02). jev grades claims with a model. A model in the enforcement path could turn a failure into a pass.

## Decision

jev lands inside lemma after the deterministic core is dogfooded. It is advisory and monotonic. It may add a finding, ask for human sign-off, or flag a claim or ref it thinks is missing. It may never remove a finding, turn a failure into a pass, waive evidence or complete a document. Output is structured (verdict, confidence, abstain), labelled advisory, never counted as evidence. It is a separate subcommand, never invoked by validate, pre-transition hooks or CI. Without an API key it does nothing and says so.

## Consequences

- Not built. DECISION-001 places it as its own signal source with its own command and diagnostics.
- The build-loop review gate (DECISION-004) is a model in the loop, but it can only stop a commit, never pass one that lint or refs fail.
