---
title: "build runs the project's check command"
type: decision
status: proposed
author: "Jack Kaloger"
date: 2026-10-23
tags: []
related:
  - constrains: SPEC-002
---

## Context

DECISION-001 leaves "do the tests pass" to CI and the test runner. In `lemma build`, the runner is told to "leave the tests passing", but nothing checks it. The task review reads the diff and does not run it. A task can commit with red tests, and nothing notices until CI, after several more tasks have stacked on top. Under `lemma vibe` (DECISION-015) no human looks before then.

## Decision

A project may set `check` in `lemma.toml`, a command such as `cargo test`. `lemma build` runs it after each task's lint and evidence checks, before the task review. A non-zero exit is a check finding carrying the tail of the output. It goes to a fix round and shares the `review_rounds` budget. Unset means no check, same as today.

This narrows DECISION-001's "Test result: CI / test runner" layer and does not reverse it. Lemma runs the project's command and reads only its exit status. It still does not judge whether a test proves a claim, and CI is still authoritative.

## Consequences

- Each task costs one check run, and each failed fix round costs another. A slow suite makes builds slow; point `check` at a faster subset.
- The command runs without a shell (SPEC-002, "No shell layer"). Several steps need a script.
- A flaky check burns fix rounds. Revisit if a flake stops a build twice.
