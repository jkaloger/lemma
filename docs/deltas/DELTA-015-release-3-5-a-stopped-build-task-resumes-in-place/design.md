`lemma build <delta> --resume` continues the one stopped task in place. DECISION-004 stays whole: nothing commits around a finding, and a resumed task goes through the same gate as a fresh one.

**Stop record.** `<git-dir>/lemma/builds/<delta>.json`, one per delta. Build is sequential, so a delta has at most one stopped task (one agent per delta, DECISION-013). Not committed; per clone. It replaces build's use of `reviews/<delta>-T<n>.json`. `lemma review`'s `reviews/<delta>.json` (resolved draft-review findings, `reviews.rs`) is a different file and stays.

```json
{
  "task": "T2",
  "ending": "unticked|ticked|reviewed",
  "base": "<HEAD at task start>",
  "head": "<HEAD at stop>",
  "session": "<uuid|null>",
  "findings": [{ "text": "...", "basis": "..." }],
  "tree": { "<path>": "<blob sha>" },
  "at": "<rfc3339>"
}
```

- `ending`: `unticked` stopped before the tick; `ticked` ticked but the commit failed; `reviewed` stopped by the reviewer or by findings left after `review_rounds`.
- `base`: HEAD when the task started. `head`: HEAD when it stopped.
- `session`: the runner session id, or null when the commands have no `{session}`.
- `findings`: what the stop reported, as text plus basis.
- `tree`: blob sha of each dirty path at stop. Compared against the tree at resume to list what the human changed.

Build writes the record on every stop and deletes it when the task commits.

**Entry rules.** Evaluated before `require_clean`.

| tree  | record                                                      | `--resume` | result                                                                                         |
| ----- | ----------------------------------------------------------- | ---------- | ---------------------------------------------------------------------------------------------- |
| dirty | none                                                        | no         | refused as today                                                                               |
| dirty | present                                                     | no         | refused: `T2 stopped <at>: <reason>; rerun with --resume, or clean the tree to start T2 fresh` |
| any   | none                                                        | yes        | error: nothing to resume                                                                       |
| any   | for a task other than the next unticked one (or `--task`'s) | yes        | error, names both tasks                                                                        |
| clean | present                                                     | no         | work was discarded; delete the record, start the task fresh                                    |
| any   | valid                                                       | yes        | resume                                                                                         |

`ticked` records name the ticked task, which is no longer the first unticked one; the task check for `--resume` compares against the record's task for that ending.

**HEAD moved.** Resume is allowed when the record's `head` is an ancestor of current HEAD, so commits made during the stop are fine. Otherwise it is refused with the two shas and why: history changed under the task. Resume never resets or rewrites anything.

**Task diff.** The review gate's `task_diff` on a resume is `base` to the working tree, not `git diff --cached HEAD`. Commits the human made during the stop are therefore reviewed with the rest. The task commit holds only what is uncommitted; history is not rewritten, so the task's work may span earlier human commits and the task commit. A fresh run's `task_diff` is unchanged.

**Resume flow.**

1. `ending: ticked`: retry the commit only, then continue to the next task. No runner, no review; both already passed.
2. Otherwise print `resuming T2 (stopped <at>: <reason>)`, then the paths changed since the stop: working tree against the `tree` snapshot, plus paths in commits since `head`.
3. Run `check_work` on the tree as it is.
   - Pass: the review gate, then tick, then commit.
   - Fail: a fix round with the failures as findings, through the recorded session's `resume` command. When the runner commands lack `{session}`, a fresh runner is used (existing rule). Then the review gate, tick and commit as on pass.

   A resume starts one fresh `review_rounds` budget, shared by check-failure fix rounds and reviewer fix rounds alike, as on a fresh run. The stopped run's budget is not carried: a stop after the cap left none, and the human's edits are new work to judge.

4. Human edits made during the stop, including paths outside the task's original diff, are part of the task diff and are reviewed.
5. The record is deleted on commit. Build continues to the next task.

If the resumed task stops again, a new record replaces the old one.

**Check failures become findings.** Applies to every run, not only resume. After a runner or fix reply, `check_work` failures are turned into findings and fed to a fix round inside the same `review_rounds` bound, instead of stopping at once. Covers lint errors, unresolved or not-a-test refs, a settled claim without a ref, edits to delta parts, and a reply without JSON.

Each failure becomes a finding whose text is the diagnostic. The reply schema limits `basis` to written sources: the task, the design, claim ids, dictum ids, related documents. A lint diagnostic that names a claim gets that claim id as basis; every other check failure gets `task`. No new `check` basis is added: a check failure is not a source the builder could have read, and the schema stays closed. If that proves too coarse, a `check` basis is a later change to the reply schema, not this delta.

A reply with `blocked` still stops: that is the runner asking for a human. The stops that remain: reviewer `stop`, findings left after the cap, `blocked`, commit failure. DECISION-004 already bounds reviewer findings by `review_rounds`; this extends the same bound to check findings, one budget for both.

**Stop hook (SPEC-003).** While a valid build stop record exists for a delta, `lemma stop` reports that delta's findings as warnings, not errors, and names `lemma build <delta> --resume`. The record is valid when its task is the next unticked one (or the ticked one for `ending: ticked`) and `head` is an ancestor of HEAD. Other findings are unchanged: still errors. This follows DECISION-010: `stop` warns early, and a stopped build is waiting on the human, not blocking a session that cannot clear it.

This changes SPEC-003's escalation constraint, which says signals block at Stop. The constraint gains one exception: findings of a delta with a valid build stop record warn at Stop. Why: the session that hits them is not the one that can clear them; clearing is the human running `--resume` or cleaning the tree. Nothing commits around them: the resume goes through the review gate, and the gate and CI still block the change. The exception is narrow because validity is checked, not assumed: a stale or mismatched record keeps the findings as errors.

**Docs.** README's sections on build stops and rerun (about lines 135-182) are rewritten: stop record, `--resume`, check failures getting a fix round, the stop hook warning. SPEC-002's "A re-run resumes at the first unticked task" gains the resume rule; SPEC-003's escalation constraint gains the stop-record exception, and its `stop` section gains the warning rule.

**Tests.** `a_rerun_resumes_at_the_first_unticked_task` encodes commit-then-rerun and changes to the `--resume` path: dirty tree refused with the record named; `--resume` completes the task. New cases: no record, wrong task, clean tree discards the record, HEAD moved forward and sideways, `ticked` retry, check failure fix round, human edit outside the original diff reviewed.
