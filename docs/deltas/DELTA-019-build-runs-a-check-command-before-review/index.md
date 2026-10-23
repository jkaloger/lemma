---
title: "build runs a check command before review"
type: delta
status: in-progress
author: "Jack Kaloger"
date: 2026-10-23
tags: []
related:
  - changes: SPEC-002
reviewed: 340e8297661358b550ace6a6d88e4c251f069c04
---

## Intent

`lemma build` asks the runner to leave the tests passing but never checks it. A task commits whether its tests are red or green (DECISION-016). Add an optional `check` command in `lemma.toml` that build runs after each task. A failure goes to a fix round the same way a lint or evidence failure does. DELTA-018 (`lemma vibe`) depends on this.

## Out of scope

- Parsing test output, or mapping a failure to a claim. Exit status only.
- Running the check outside build (lint, Stop hook, CI).
- A shell. Several steps go in a script.
- A default `check`. Unset keeps today's behaviour.
