---
title: "Planned behaviour folds from deltas into specs"
type: decision
status: superseded
author: "Jack Kaloger"
date: 2026-10-02
tags: [rfc-001]
related: []
---

## Context

From RFC-001 §3, §5.4, §5.6, §7.2 and §7.5 (2026-10-02). Three positions existed on where planned behaviour lives. Heavy: tagged claims in delta documents, folded into capability specs, with `@ref` receipts. Middle: tagged claims in evergreen specs with targeted drift reconciliation. Light: pending tests are the plan, intent lives in the README, no claims or receipts. A scenario quiz on 2026-09-30 sent every revealed preference to the structured side while the stated preference was "lightweight".

## Decision

Heavy, with "lightweight" defined as friction per interaction (chores free, human edits never blocking), not as artefact count.

The delta is the unit of in-flight work and replaces the rfc, story and iteration chain. It carries claims as tagged list items exactly as they will appear in the spec. A fresh id adds, an existing id replaces, a struck id removes. At the end of the delta lifecycle a hook folds the claims into the spec by id and moves the receipts. Only that hook writes spec claims. Fold targets are frontmatter links. A delta for a new capability creates the spec at draft with `Intent` only, so the fold always appends to an existing node.

An optional `tasks` part holds non-claim work as a scratchpad no hook reads. Claims with receipts are the only gating progress.

Two tracks join at the spec: shaping (human-paced) produces specs and decisions; delivery (agent-paced, hook-enforced) produces claims and receipts that fold into specs.

Rejected: editing the spec in the same change as the code (parallel deltas conflict on the spec file); OpenSpec's ADDED/MODIFIED/REMOVED format (stable ids make the delta implicit); separate iteration documents, GitHub issues or the PR as the work unit.

## Consequences

Superseded by DECISION-003: nothing folds, specs hold intent only, and a completed delta is the record. Releases 1.0 to 2.1 (DELTA-001 to DELTA-008) implemented and refined the fold before it was cut in release 3.0 (DELTA-009).
