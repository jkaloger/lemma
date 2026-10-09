---
title: lemma is a pack, a Rust binary and a plugin under one version
type: decision
status: accepted
author: Jack Kaloger
date: 2026-10-02
tags:
- rfc-001
related:
- constrains: SPEC-004
---

## Context

From RFC-001 §1, §4 and §12 (2026-10-02). The rationale record is the hydra tree `metaframework` in the lazyspec repository at `.hydra/metaframework.json` (commit `f345349`).

Three pieces existed and none of them bound anything. lazyspec gives a typed document graph with `governs`, `@ref`, staleness and hooks. The lazyspec plugin gives generic verbs over any DAG. Per-repo configuration gives a starter. Each is advisory: an agent can skip the plan, declare completion without evidence, and leave the spec restating the code. The capability exists; the workflow it implies is scattered, so an agent can walk around it.

Prompting cannot scope context by code locality, cannot know when context has decayed, cannot prove that intent was met, and does not keep rationale in the repository.

## Decision

lemma layers the pieces: a pack for repeatability, hooks for enforcement, and an opinion about what a spec is for that the pack encodes. It is a lazyspec pack, a hooks binary and a Claude Code plugin, in one repository, under one version.

**Binary, in Rust.** Enforcement beyond required sections needs a real language with tests: one JSON contract, one file under the trust hash. Rust because per-tool-call hooks need fast start and a small binary, because the compiler holds agent-written code in check, and because only Rust can link the lazyspec engine as a crate instead of spawning the CLI. lazyspec stays the only interface agents learn.

**Boundary.** Mechanism in lazyspec, policy in lemma. The test: if a game-development pack could want it, it belongs in lazyspec.

**Distribution.** Three native channels and no new tooling: `lazyspec init --template <repo>` for the pack, `claude plugin install` from this repository's marketplace for the plugin, `cargo install` for the binary. One version number across all three, released together.

**Name.** lemma: a claim in a delta is a small result proven on the way to a larger one. `delta` was the fallback; git-delta owns that binary name on most Rust developers' `PATH`.

Rejected: a wrapper CLI fronting lazyspec (a second interface to learn); a pack with shell hooks only (untestable logic, many files under trust).

## Consequences

- No version-mismatch check between the three parts.
- Hook trust covers the `[[hooks]]` table only, because a `cargo install`ed binary sits outside the docs root. Open in SPEC-004.
- Name availability on crates.io and GitHub was not checked at decision time.
