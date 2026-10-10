---
title: "nvim plugin for lemma"
type: delta
status: draft
author: "Jack Kaloger"
date: 2026-10-10
tags: []
related:
- changes: SPEC-004
---

## Intent

Lemma docs are plain markdown with lemma-specific syntax: claim refs (`path#test`), `@ref`/`@per`/`@review` directives, `[intent]` tags, `> review:` annotations, doc ids (`SPEC-002`, `DECISION-003`). In nvim they read as noise. Ref targets are opaque until you leave the doc and run `lemma refs`. Idea: an nvim plugin that highlights lemma syntax and expands refs in place (resolve status, jump to target, peek the test or doc). Ships as a fourth part of the pack beside binary, plugin and templates.

## Out of scope

- An LSP server.
- Editing or accepting annotations from the editor.
- Editors other than nvim.
