---
title: 'Release 3.1: progress, advance at build, short tasks'
type: delta
status: complete
author: Jack Kaloger
date: 2026-10-08
tags: []
related:
- changes: SPEC-002
- changes: SPEC-004
reviewed: 87c3c31669f2ac169e76437be0f2b35b3e1c5889
---

## Intent

Dogfood fixes from the first full delta run in lemma-playground (DELTA-001, review to build). Runs were silent for minutes, status moves were typed by hand against a command that does not exist, and tasks read like prose diffs. This release adds a spinner and watch line to every runner call, a `{session}` runner placeholder, colour, an offer at `lemma build` to move a draft delta to in-progress, `next:` lines that name a real command, and short task titles from `plan` and diff review.

## Out of scope

- Live typed progress events. The schema reply is one `StructuredOutput` call at the end (verified 2026-10-08). A `lemma event` side channel is deferred, trigger: the watch line is not enough in two dogfood runs.
- Spec feedback from completed deltas (needs thought).
- Offering `complete` from `lemma review`.
