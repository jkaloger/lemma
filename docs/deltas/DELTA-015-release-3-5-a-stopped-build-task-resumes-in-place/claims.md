<!-- One bullet per behaviour, in plain words: `- An empty account has balance zero.` `lemma review` assigns ids (`- [AC-1] ...`). Mark a behaviour no test can settle `[intent]` after its id. -->

- [AC-1] Every build stop writes a stop record at `<git-dir>/lemma/builds/<delta>.json` holding the task, ending, base, head, session, findings, a blob sha per dirty path and the time; it is not committed.
  @ref crates/lemma/tests/build.rs#a_stop_writes_an_uncommitted_record_of_the_task_base_head_session_findings_tree_and_time
  @ref crates/lemma/tests/build.rs#a_runner_call_that_fails_still_records_its_session
- [AC-2] A stop's ending is `unticked` when it stopped before the tick, `ticked` when the tick landed but the commit failed, and `reviewed` when the reviewer stopped it or findings were left after `review_rounds`.
  @ref crates/lemma/tests/build.rs#a_stop_before_the_tick_records_the_ending_unticked
  @ref crates/lemma/tests/build.rs#a_failed_commit_after_the_tick_records_the_ending_ticked
  @ref crates/lemma/tests/build.rs#findings_left_after_review_rounds_record_the_ending_reviewed
  @ref crates/lemma/tests/build.rs#a_reviewer_stop_records_the_ending_reviewed
- [AC-3] A stopped task that stops again replaces the record; the record is deleted when the task commits.
  @ref crates/lemma/tests/build.rs#a_second_stop_of_the_task_replaces_the_record
  @ref crates/lemma/tests/build.rs#the_record_is_deleted_when_the_task_commits
- [AC-4] Build no longer writes `reviews/<delta>-T<n>.json`; `lemma review`'s `reviews/<delta>.json` is unchanged.
  @ref crates/lemma/tests/build.rs#build_writes_no_per_task_review_file_and_leaves_the_review_record_alone
- [AC-5] Without `--resume`, a dirty tree with a stop record is refused with `T<n> stopped <at>: <reason>; rerun with --resume, or clean the tree to start T<n> fresh`.
- [AC-6] Without `--resume`, a dirty tree with no stop record is refused as before.
- [AC-7] Without `--resume`, a clean tree with a stop record deletes the record and starts the task fresh.
- [AC-8] `--resume` with no stop record errors: nothing to resume.
- [AC-9] `--resume` errors naming both tasks when the record's task is not the next unticked task, or not the `--task` given.
- [AC-10] `--resume` is refused with both shas when the record's `head` is not an ancestor of HEAD; nothing is reset or rewritten.
- [AC-11] `--resume` is allowed when commits were made on top of the record's `head` during the stop.
- [AC-12] Resuming a `ticked` stop retries only the commit, runs no runner and no review, then continues to the next task.
- [AC-13] A resume prints `resuming T<n> (stopped <at>: <reason>)` and the paths changed since the stop: the working tree against the recorded blob shas, plus paths in commits since `head`.
- [AC-14] A resume runs the checks on the tree as it is; on pass it runs the review gate, then ticks and commits.
- [AC-15] A resume starts a fresh `review_rounds` budget, shared by its check-failure and reviewer fix rounds; the stopped run's budget is not carried.
- [AC-16] A resume whose checks fail runs a fix round with the failures as findings, through the recorded session's `resume` command.
- [AC-17] A resume whose record has no session, or whose runner commands lack `{session}`, runs the fix round as a fresh runner.
- [AC-18] On a resume, the reviewer's task diff runs from the record's `base` to the working tree, so commits the human made during the stop are reviewed.
- [AC-19] Human edits made during the stop, including paths outside the task's original diff, are in the resumed task's diff and are reviewed.
- [AC-20] The resumed task's commit holds only uncommitted work; earlier human commits stay as they are.
- [AC-21] A fresh run's task diff is unchanged.
- [AC-22] After a runner or fix reply, a check failure becomes a finding and goes to a fix round within the same `review_rounds` budget as reviewer findings, instead of stopping at once.
  @ref crates/lemma/tests/build.rs#a_check_failure_goes_to_a_fix_round_and_a_clean_review_commits
  @ref crates/lemma/tests/build.rs#a_check_fix_round_spends_the_budget_reviewer_findings_would_use
- [AC-23] A reply without JSON, a lint error, an unresolved or not-a-test ref, a settled claim without a ref, and an edit to a delta part each become a fix-round finding.
  @ref crates/lemma/tests/build.rs#a_reply_without_json_becomes_a_fix_round_finding
  @ref crates/lemma/tests/build.rs#a_lint_error_becomes_a_fix_round_finding_on_the_claim_it_names
  @ref crates/lemma/tests/build.rs#a_not_a_test_ref_becomes_a_fix_round_finding
  @ref crates/lemma/tests/build.rs#a_settled_claim_without_a_ref_becomes_a_fix_round_finding_on_the_claim
  @ref crates/lemma/tests/build.rs#a_delta_part_edit_becomes_a_fix_round_finding
- [AC-24] A check failure finding's text is the diagnostic; its basis is the claim id when the diagnostic names a claim, otherwise `task`.
  @ref crates/lemma/tests/build.rs#a_settled_claim_without_a_ref_becomes_a_fix_round_finding_on_the_claim
  @ref crates/lemma/tests/build.rs#a_lint_error_becomes_a_fix_round_finding_on_the_claim_it_names
  @ref crates/lemma/tests/build.rs#a_reply_without_json_becomes_a_fix_round_finding
  @ref crates/lemma/tests/build.rs#a_delta_part_edit_becomes_a_fix_round_finding
- [AC-25] Check failures left after the last fix round stop the build with an `unticked` record.
  @ref crates/lemma/tests/build.rs#a_lint_error_left_after_the_last_fix_round_stops_with_the_tree_kept
  @ref crates/lemma/tests/build.rs#a_fix_that_breaks_lint_after_the_last_round_stops_before_the_second_review
- [AC-26] A reply with `blocked` still stops at once.
  @ref crates/lemma/tests/build.rs#a_blocked_reply_stops_at_once_without_a_fix_round
- [AC-27] While a valid stop record exists for a delta, `lemma stop` reports that delta's findings as warnings, not errors, and names `lemma build <delta> --resume`.
- [AC-28] `lemma stop` treats a stop record as invalid, and keeps that delta's findings as errors, when its task is not the next unticked one (or the ticked one for `ticked`) or its `head` is not an ancestor of HEAD.
- [AC-29] Findings unrelated to a delta with a valid stop record are still errors in `lemma stop`.
- [AC-30] `--resume` on a `ticked` stop record is accepted when the record names the ticked task, even though that task is no longer the first unticked one.
