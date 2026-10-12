---
title: "Release 3.5: a stopped build task resumes in place"
type: delta
status: draft
author: Jack Kaloger
date: 2026-10-12
tags: []
related:
  - changes: SPEC-002
  - changes: SPEC-003
---

## Intent

SPEC-002 says "A stop never resets the working tree. A re-run resumes at the first unticked task." It cannot. `lemma build` calls `require_clean` first (`build.rs`, whole tree, no exception), so after any stop that touched code the rerun fails with "build needs a clean working tree; commit or stash first". The only exits are bad. Commit the partial work as a non-task commit, which is what `tests/build.rs` `a_rerun_resumes_at_the_first_unticked_task` does with `repo.commit("human fix")`: it bypasses the review gate, against DECISION-004 "nothing commits around a finding". Or stash it, which discards the work. A stop that ticked the task but did not commit is worse: the rerun skips the ticked task and the dirty tree blocks the rest.

Build writes a stop record at `<git-dir>/lemma/reviews/<delta>-T<n>.json` (`record_review`) and never reads it. A rerun starts a fresh runner blind to why the last one stopped.

Two related faults. Check failures after the first runner call (lint, settled-claim refs, bad JSON, part edits) stop at once in `build_task` with no fix round, while reviewer findings get a bounded one. DECISION-004 does not ask for that asymmetry. And the `lemma stop` hook blocks every turn of every session while a build task is stopped, which a human cannot clear except by resuming or discarding.

Trigger: DELTA-012 T2 stopped on a lint error and could not resume.

This delta adds `lemma build <delta> --resume`, which continues a stopped task in place: same tree, same session where possible, human edits included, still through the review gate. It turns check failures into findings for the fix round, and makes `lemma stop` warn rather than block on a resumable stop. That last is an exception to SPEC-003's constraint that signals block at Stop; this delta amends the constraint (see design).

## Out of scope

- Showing the `tests` setting in the build and fix prompts, and checking reply refs before writing `claims.md`: DELTA-014 owns it.
- Multiple stopped tasks per delta. Build is sequential, so there is at most one.
- Resuming across clones. The record is per clone.
- Any tree reset or history rewrite. Cleaning the tree is the discard; there is no discard command.
- Changing what the reviewer judges. Only the range `task_diff` covers changes, and only on resume.
