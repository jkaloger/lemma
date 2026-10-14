---
title: "Claims are structured data"
type: delta
status: draft
author: "Jack Kaloger"
date: 2026-10-13
tags: []
related:
  - changes: SPEC-001
  - changes: SPEC-002
---

## Intent

`claims.md` is structured data wearing markdown. Line grammar (`- [AC-n] text`, `[intent]`, `[proposed]`, `~~[id]~~`, indented `@ref path#symbol`, `@per`) parsed by hand in `claims.rs`, used across ~10 modules. Grammar is loose: `claims::parse` silently skips any line not matching item pattern, so `- [AC1] ...` or a mis-indented `@ref` vanish without error. Annotations are inline text markers; accepting one means deleting exact characters, easy to half-delete.

Refs are where this hurts most. Each is a cross-file pointer to a test symbol, highest-value and most error-prone part of a claim. Structured claims make a bad ref a parse error, and give editors something to complete and jump through.

Move delta claims to a structured file with a strict parser: unknown or malformed input is an error, annotations are whole fields, a new claim stays one short entry with no id.

## Out of scope

- Dicta. Their claims aren't mechanically enforced; they keep markdown.
- `tasks.md`. Mostly lemma-written; may follow later.
- An LSP server and editor plugin (DELTA-013). This delta fixes the format they would read.
- Specs. They hold no claims (DECISION-003).
