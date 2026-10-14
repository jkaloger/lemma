---
title: "nvim plugin for lemma"
type: delta
status: draft
author: "Jack Kaloger"
date: 2026-10-10
tags: []
related:
- changes: SPEC-005
- changes: SPEC-004
- changes: SPEC-001
---

## Intent

Lemma docs are plain markdown with lemma-specific syntax: claim refs (`path#test`), `@ref`/`@per`/`@review` directives, `[intent]` tags, `> review:` annotations, doc ids (`SPEC-002`, `DECISION-003`). In nvim they read as noise. Ref targets are opaque until you leave the doc and run `lemma refs`.

This delta ships an nvim plugin that highlights lemma syntax and jumps from a ref or doc id to its target. Highlighting is pure Lua. Jumping asks a new `lemma refs --json <file>`, so the editor resolves refs exactly as `lemma lint` does. The plugin ships as a fourth part of the pack, beside binary, Claude Code plugin and templates, on the same version.

## Out of scope

- An LSP server.
- Editing or accepting annotations from the editor.
- Editors other than nvim.
- Peek floats and inline ref status. SPEC-005 open question.
- A structured claims format. DELTA-016, cut down to a strict markdown parser, is independent: the plugin never parses claims.
