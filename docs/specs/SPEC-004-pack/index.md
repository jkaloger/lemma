---
title: "Pack"
type: spec
status: active
author: "Jack Kaloger"
date: 2026-10-09
tags: []
governs:
  - ".lazyspec.toml"
  - ".lazyspec/templates/**"
  - "docs/convention/**"
  - "docs/ci/**"
  - ".github/**"
  - "Cargo.toml"
  - "crates/lemma/Cargo.toml"
  - "crates/lemma/src/version.rs"
  - "plugin/.claude-plugin/plugin.json"
  - "nvim/lua/lemma/version.lua"
tests:
  - "crates/lemma/tests/**"
related: []
---

## Intent

What an adopting project receives, and how the four parts ship. `lazyspec init --template` copies the pack: `.lazyspec.toml` with five types, their relationships and the `[[hooks]]` table that composes lemma commands; document templates; prompt files; and the seed convention and dictums. The Claude Code plugin installs from this repository's marketplace, the nvim plugin with lazy.nvim from this repository's `nvim/` (SPEC-005), and the binary from crates.io. All four carry one version number, which the README states.

## Constraints

- Mechanism in lazyspec, policy in lemma. If a game-development pack could want it, it belongs in lazyspec (DECISION-005).
- One version across pack, Claude Code plugin, nvim plugin and binary, released together (DECISION-005). `crates/lemma/Cargo.toml`, `plugin.json` and `nvim/lua/lemma/version.lua` are held in step by hand at release.
- Five types: spec, delta, decision, convention, dictum (DECISION-006, as cut by DELTA-004).
- The delta lifecycle is `draft` → `in-progress` → `complete`. The `changes` edge from delta to spec is required (DECISION-003).
- Dictums are human-edited. A dictum rule whose referenced test must change is struck and replaced with a new id in the same change (CONVENTION).
- The CI template runs `lemma lint` with an explicit base, then `lazyspec validate`.

## Non-goals

- `migrate` from 2.x. A 2.x repository re-adopts with `lazyspec init --template --force` (DECISION-003).
- A version-coherence check across the four parts (DECISION-005).
- Stack-specific dictums such as `testing-web`.

## Open questions

- lazyspec hook trust hashes in-repo `run` files. A `cargo install`ed binary sits outside the docs root, so trust covers the `[[hooks]]` table only. Whether lazyspec needs trust-by-path is open.
- Seed documents are copied by hand until lazyspec RFC-076 copies pack documents on `init`.
