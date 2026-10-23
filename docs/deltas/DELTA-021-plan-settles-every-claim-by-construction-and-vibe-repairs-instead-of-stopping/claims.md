<!-- One bullet per behaviour, in plain words: `- An empty account has balance zero.` `lemma review` assigns ids (`- [AC-1] ...`). Mark a behaviour no test can settle `[intent]` after its id. -->
- [AC-1] The plan reply schema requires a `settled_by` entry for every live claim no kept task settles, including `[intent]` claims.
  @ref crates/lemma/tests/plan.rs#the_plan_schema_requires_settled_by_for_every_claim_no_kept_task_settles
- [AC-2] A planner reply that settles a claim with a task position outside the reply's tasks is asked again once with the error, then plan fails without writing.
  @ref crates/lemma/tests/plan.rs#an_out_of_range_position_is_asked_again_once_with_the_error
  @ref crates/lemma/tests/plan.rs#a_second_out_of_range_reply_fails_without_writing
- [AC-3] Plan never writes a `tasks.md` that the draft → in-progress gate's claim-coverage rule rejects.
  @ref crates/lemma/tests/plan.rs#a_draft_leaving_a_claim_unsettled_is_refused_without_writing
  @ref crates/lemma/tests/plan.rs#a_draft_settling_a_claim_twice_is_refused_without_writing
- [AC-4] A planned new claim is written with its settling task, and no new claim is left unsettled.
  @ref crates/lemma/tests/plan.rs#a_new_claim_is_written_with_its_settling_task
- [AC-5] Under vibe, a draft reply that `fill` rejects is asked again with the rejection message.
- [AC-6] Under vibe, draft → in-progress gate findings go to a revise call and the gate runs again.
- [AC-7] Under vibe, a build task that stops for any reason other than an environment error (rounds run out, reviewer stop verdict, left unticked) has its tracked changes reset and the untracked files it created removed, keeping untracked files that were there before it started, and is replaced in `tasks.md` by re-planned tasks that settle the same claims. Build then continues.
- [AC-8] Re-planned tasks get ids after the highest existing task id, and ticked tasks are unchanged.
- [AC-9] Plain `lemma build` still stops on a task that runs out of review rounds.
- [AC-10] A vibe draft, plan or gate stage stops after `vibe_repairs` re-asks (default 3).
- [AC-11] vibe build stops when a task at re-plan depth `vibe_repairs` stops, whatever its ending. Tasks from the first plan are at depth 0, so with the default a task line is re-planned at most 3 times. A replacement task has its parent's depth plus one, chores included.
- [AC-12] A re-plan proposes no new claims; its tasks settle exactly the stopped task's claims.
- [AC-13] vibe stops on an environment error (runner or check command can't start) without a repair attempt.
- [AC-14] An invalid re-plan reply is re-asked with the error, up to `vibe_repairs` times per re-plan of one task, and then vibe stops at build.
- [AC-15] A re-plan reply with no tasks is invalid, so a stopped task, chore or not, is never dropped from `tasks.md`.
- [AC-16] A runner or check command that can't start, or a failed git commit, makes `lemma build` return an error rather than a task stop, and the stop record still lets `--resume` pick it up.
  @ref crates/lemma/tests/build.rs#a_runner_that_cannot_start_is_an_error_that_leaves_a_resumable_record
  @ref crates/lemma/tests/build.rs#a_check_that_cannot_start_is_an_error_that_leaves_a_resumable_record
  @ref crates/lemma/tests/build.rs#a_failed_commit_is_an_error_and_resume_retries_the_commit
- [AC-17] Under vibe, a plan stage error is asked again with the error message.
- [AC-18] A new task that no `settled_by` value or new claim points at is written to `tasks.md` as a chore.
  @ref crates/lemma/tests/plan.rs#a_task_nothing_points_at_is_written_as_a_chore
