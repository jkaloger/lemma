---
title: "Runner part edits are restored, trailing markers are lint errors"
type: delta
status: draft
author: "Jack Kaloger"
date: 2026-10-16
tags: []
related:
  - changes: SPEC-002
  - changes: SPEC-001
---

## Intent

Two faults stopped DELTA-013 T8 three times.

**A runner part edit is a finding the runner cannot fix.** A stop keeps the tree, including refs lemma itself wrote into `claims.md` from the reply. On `--resume` the runner saw `claims.md` dirty, took the lines for a stray edit, and ran `git checkout` on it. `reject_part_edits` compared against the snapshot taken before the call and flagged the edit. The fix round got "runner edited claims.md; lemma writes delta parts", checked `git diff` (clean against HEAD), changed nothing, and was flagged again: the snapshot it must match is invisible to it. SPEC-002 says lemma writes delta parts, so lemma already holds the right content and should put it back.

**A trailing `[intent]` is claim text.** `split_markers` reads markers only straight after `[AC-n]`. `- [AC-21] The pack's four parts carry one version. [intent]` parses as a non-intent claim with `[intent]` in its text. Lint says clean; build then asks the task for a ref the design forbids, and review rejects the test the fix round wrote for it. The template states the rule ("after its id") but nothing enforces it.

## Out of scope

- A structured claims format. DELTA-016 replaces the line grammar; this lint rule is a stopgap for the markdown one.
- Accepting a trailing marker as a marker. One placement keeps the grammar strict.
- Telling the runner about lemma-written lines in the kept tree. Restore makes it moot.
- Why `--resume` runs the runner before the checks. Separate fault, not looked at here.
