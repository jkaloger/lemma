---
title: "Receipts are commit-pinned test refs checked for freshness"
type: decision
status: superseded
author: "Jack Kaloger"
date: 2026-10-02
tags: [rfc-001]
related: []
---

## Context

From RFC-001 §6 and §8.2 (2026-10-02). Verification belongs in the graph, not in a runner. A spec tool that re-runs the suite at Stop is neither fast nor lightweight and duplicates an existing gate.

## Decision

lemma never runs tests. It checks that receipts exist and are fresh; the codebase runs the tests and CI owns pass and fail.

A claim is a tagged list item with a stable id. A receipt is an `@ref path#symbol@sha` line under it. Tests carry nothing: no annotations, no naming convention. A claim with no receipt warns while the delta is in progress and errors at `complete`.

A receipt is stale when the test file or code under the spec's `governs` changed since the pin. `re-pin` updates receipt commits after drift is resolved; it refuses when the symbol no longer resolves and flags a re-pin that follows a test change with no matching claim change, the silencing move.

An intentional change is a claim replaced or removed by id in an in-flight delta. A regression is a governed-code change with no delta touching the affected claims.

Unverifiable claims ran two trials: an `[intent]` marker excluding a claim from the receipt check, and `verified-by: <name> @ <sha>` sign-off.

## Consequences

Superseded by DECISION-001: refs are unpinned, freshness and `re-pin` are gone, and a changed evidence region is reported for a human to judge. Releases 1.0 to 1.5 (DELTA-001 to DELTA-006) built and hardened receipts, `@checked`, `@base` and `[forced]`; release 2.0 (DELTA-007) removed them. `[intent]` survives. `verified-by` went in release 1.3.
