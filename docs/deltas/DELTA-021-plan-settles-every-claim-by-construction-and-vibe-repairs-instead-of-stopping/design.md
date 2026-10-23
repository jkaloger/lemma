<!-- How the change works, in prose. No `@ref` lines here; refs live under claims. -->

## Plan reply shape

Today a task carries `settles: [...]`, and nothing requires every claim to appear. The reply becomes:

```json
{"tasks": [{"text": "..."}],
 "settled_by": {"AC-1": 2, "AC-3": 2},
 "claims": [{"text": "...", "task": 3}]}
```

`schema::plan` takes the ids that are still unsettled (live claims no kept task settles) and makes each one a required key of `settled_by`, with an integer value of at least 1. A key holds one value, so each claim is settled exactly once, by construction. Kept tasks keep their numbers. A value refers to a new task by its position in `tasks`. A new claim names its task in `task`, so it can't be left unsettled either. A task no claim points at is a chore.

The schema can't bound a position by the length of `tasks`. `plan::draft` checks the range. On an out-of-range position, plan asks again once with the error in the prompt, then fails.

After drafting, plan runs the gate's claim-coverage rule (`shape`) on what it is about to write and refuses to write a draft that breaks it. Under the new schema this can't happen; the check guards against regression.

`plan.md` drops "Every claim is settled by exactly one task" and describes `settled_by`, saying that an `[intent]` claim is settled by the task that builds the behaviour.

## vibe: findings go back, not out

`vibe::stages` stops only on cancel or an environment error. Each stage gets a repair path:

- draft: a reply `fill` rejects (no `changes`, unknown spec) is asked again with the message.
- plan: a plan error is asked again with the message.
- gate (`in_progress`): findings are written as review lines and go to `revise`, then the gate runs again.
- build: `Outcome::Stopped` gains the stopped task's id and its `Ending`. Environment failures move out of `Stopped`. A runner or check command that can't start (today `Ending::Unticked`, build.rs:426, 713) and a failed git commit (today `Ending::Ticked`, build.rs:650) return `Err` from `build::run`, in plain build too, and still leave the stop record, so `--resume` works as before. Every `Stopped` left is a task stop (`Unticked`, `Checked` with rounds spent, `Reviewed`), and vibe re-plans it (next section) and calls `build::run` again. vibe tells the two apart by type.

The cap is `vibe_repairs` in `lemma.toml`, default 3. For draft, plan and the gate, one attempt is one re-ask: every agent call after a stage's first counts, including the out-of-range re-ask inside `plan::draft`. For build, one attempt is one re-plan of a task. Tasks from the first plan are at depth 0. A replacement task carries its parent's depth plus one, and vibe stops when a task at depth `vibe_repairs` stops for any reason. Depth is per task, so chores are bounded too. vibe keeps depths in memory for the run, keyed by task id. They are not in the build record or `tasks.md`, so clearing the stop state leaves them alone. Hitting the cap is the only finding-driven stop (DECISION-017).

## Re-planning a stopped task

`builds::Stopped` records the task's base. vibe resets tracked files to that base and removes the untracked files the task created: those untracked now that were not untracked when the task started. vibe lists untracked files before each `build::run` call. Untracked files that were already there (a human's, or another delta's) are kept. vibe then clears the stop state for the task. It then calls a new plan mode with the stopped task's text, the claims it settles, and the findings from its last round. The reply has `tasks` and `settled_by` but no `claims`: a re-plan proposes no new claims. Its schema requires at least one task, so a re-plan never drops a task, chores included. `settled_by` requires exactly the stopped task's claims. The new tasks replace the stopped line in `tasks.md`, numbered after the highest existing id, so ids are never reused. Ticked tasks are not touched.

The reset qualifies SPEC-002 "A stop never resets the working tree" under vibe (DECISION-017), as DECISION-015 qualifies principle 3. The task diff is discarded so the re-planned task starts clean. It also qualifies "plan runs only on a draft delta with no review lines left": re-plan runs on an in-progress delta, scoped to the stopped task. Plain build and plain plan keep both constraints.

An invalid re-plan reply (a position out of range, or `settled_by` keys that differ from the stopped task's claims) is re-asked with the error, up to `vibe_repairs` re-asks per re-plan of one task. Once they run out, vibe stops at build.
