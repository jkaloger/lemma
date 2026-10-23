<!-- One bullet per behaviour, in plain words: `- An empty account has balance zero.` `lemma review` assigns ids (`- [AC-1] ...`). Mark a behaviour no test can settle `[intent]` after its id. -->

- [AC-1] With no `check` in `lemma.toml`, build runs no check command and behaves as before.
  @ref crates/lemma/tests/build.rs#without_a_check_build_runs_no_check_command
- [AC-2] With `check` set, build runs it in the repo root after each task, only when the task's lint and evidence checks pass, and before the task review.
  @ref crates/lemma/tests/build.rs#the_check_runs_in_the_repo_root_after_lint_and_evidence_pass_and_before_review
- [AC-3] A check that exits non-zero is one fix-round finding with basis `task`, whose text holds `check: <command> exited <code>` and the last 40 lines of output; the reviewer is not called that round.
  @ref crates/lemma/tests/build.rs#a_failing_check_is_one_task_finding_with_the_exit_code_and_output_tail
- [AC-4] A claim id in the check output does not pin the finding to that claim.
  @ref crates/lemma/tests/build.rs#a_claim_id_in_the_check_output_does_not_pin_the_finding_to_that_claim
- [AC-5] A check failure counts against `review_rounds`; when the budget runs out the task stops unticked.
  @ref crates/lemma/tests/build.rs#a_check_failure_spends_review_rounds_and_stops_unticked_when_they_run_out
- [AC-6] A check that times out is one fix-round finding whose text holds `check: <command> timed out after <n>s` and the last 40 lines of output.
  @ref crates/lemma/tests/build.rs#a_timed_out_check_is_one_task_finding_with_the_timeout_and_output_tail
- [AC-7] A check command that cannot start stops the build with that message and no fix round.
  @ref crates/lemma/tests/build.rs#a_check_that_cannot_start_stops_the_build_with_that_message_and_no_fix_round
- [AC-8] A resumed task runs the check before its review.
- [AC-9] A check that exits 0 prints the step `check: pass` and the task proceeds to review.
  @ref crates/lemma/tests/build.rs#a_passing_check_prints_check_pass_and_proceeds_to_review
