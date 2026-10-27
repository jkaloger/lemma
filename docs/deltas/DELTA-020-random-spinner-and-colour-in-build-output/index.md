---
title: Random spinner and colour in build output
type: delta
status: in-progress
author: unknown
date: 2026-10-23
tags: []
related:
- changes: SPEC-002
reviewed: 0bc279ca2ae3fc347c9ecc9dab7e2829e61e88f8
---

## Intent

Build output is hard to scan: every task looks the same and the status of a task is easy to miss among the surrounding lines. A spinner picked at random per task gives each task a distinct visual identity, so a long run is easier to follow and a task change is noticeable. More colour, applied by meaning (task id, status, refs), makes the output easier to read at a glance. It is a readability change only. Output that is not live (piped, CI) is untouched, and `NO_COLOR` removes colour only, so the watch line stays as SPEC-002's progress surface.

## Out of scope

- User-configurable spinner sets, colour themes or a `lemma.toml` option.
- Colouring `lemma lint`, `lemma refs`, `lemma review` or `lemma plan` output.
- Changing diagnostic format (`path:line: severity: message`).
- Typed progress events or any structured progress stream.
- Animation outside live mode.
