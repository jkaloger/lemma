---
title: 'Release 3.3: live runner activity and compact build output'
type: delta
status: in-progress
author: Jack Kaloger
date: 2026-10-09
tags: []
related:
- changes: SPEC-002
reviewed: cfd12a91b6fbb5a133655b3fc8cf85cad315b4b1
---

## Intent

Build runs are hard to follow on a terminal. Each runner call prints a ~150-char `watch:` line the human copies into a second terminal; one task with a fix round prints four. Step lines all look alike, so a long build scrolls into a wall. DELTA-010 deferred live progress, trigger "the watch line is not enough in two dogfood runs". This delta shows the run's latest tool call beside the spinner, read from the transcript lemma already names, and prints each task as a compact block that collapses to one line when it commits. Terminal only: piped and CI output stays as it is.

## Out of scope

- A `lemma event` side channel or typed progress from the runner.
- Reading the transcript for anything but display. No decision, reply or exit status depends on it.
- What `lint`, `refs` and the hooks print.
- A full-screen or multi-pane TUI.
