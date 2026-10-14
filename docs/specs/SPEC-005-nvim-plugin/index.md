---
title: "nvim plugin"
type: spec
status: draft
author: "Jack Kaloger"
date: 2026-10-14
tags: []
governs:
  - "nvim/**"
tests:
  - "nvim/tests/**"
related: []
---

## Intent

Lemma docs carry lemma syntax: claim ids, `@ref`/`@per`/`@review` directives, `[intent]` markers, `> review:` annotations, doc ids. In nvim they read as noise, and a ref target is opaque until you leave the doc and run `lemma refs`. The plugin highlights that syntax and jumps from a ref or doc id to its target. Fourth part of the pack, beside binary, Claude Code plugin and templates.

## Constraints

- Highlighting is lexical and pure Lua. It works with no `lemma` binary on `PATH`.
- Resolution is lemma's. The plugin never decides whether a ref resolves or where it points; it asks `lemma refs --json`. One resolver, so the editor and `lemma lint` cannot disagree.
- Ships from this repository as `nvim/`, on the pack's one version (SPEC-004).
- Tests run under `nvim --headless` with mini.test.

## Non-goals

- An LSP server.
- Editing or accepting annotations from the editor.
- Editors other than nvim.
- Parsing claim grammar in Lua.

## Open questions

- Peek (float with the test body or doc intent) and inline ref status (virtual text or diagnostics). Left out of the first cut.
