- [AC-1] After a task's checks pass and before it ticks, the reviewer runs on the task's staged diff, which includes files the task created.
  @ref crates/lemma/tests/build.rs#the_task_diff_includes_files_the_task_created
- [AC-2] The reviewer is also given the delta's diff so far, labelled read-only context, and it holds earlier work only.
  @ref crates/lemma/tests/build.rs#the_reviewer_sees_earlier_work_as_read_only_context
- [AC-3] [intent] The review prompt is given the task, the claims it settles, design, convention, dictums, spec intent and related docs.
- [AC-4] The reviewer's reply must carry `findings` and `stop`; each finding has `text` and `basis`.
  @ref crates/lemma/tests/build.rs#the_basis_enum_is_the_ids_in_the_review_context
- [AC-5] A finding's `basis` can only be `task`, `design`, `convention`, a claim id of the delta or a dictum rule id.
  @ref crates/lemma/tests/build.rs#the_basis_enum_is_the_ids_in_the_review_context
- [AC-6] A struck dictum rule is not a basis.
  @ref crates/lemma/tests/build.rs#a_struck_dictum_rule_is_not_a_basis
- [AC-7] [intent] Every `related` doc id is also an allowed basis.
- [AC-8] A finding whose basis is outside the allowed set is rejected with `unknown basis`, the task stays unticked and nothing commits.
  @ref crates/lemma/tests/build.rs#a_finding_with_a_basis_outside_the_enum_is_rejected
- [AC-9] No findings and an empty `stop` pass: the task ticks and commits.
  @ref crates/lemma/tests/build.rs#a_clean_review_commits_with_the_review_trailer
- [AC-10] [intent] The reviewer judges only the task's diff; the delta-so-far diff is there to catch a task undoing earlier work.
- [AC-11] [intent] Work the task did not ask for is reported as a finding with basis `task`, and the prompt says to back it out.
- [AC-12] [intent] A task that pursues a different design from `design.md` is a `stop`, not a finding.
- [AC-13] Findings with an empty `stop` run the builder again with a fix prompt listing each finding with its basis; the checks and the review gate then rerun, and a clean second review commits.
  @ref crates/lemma/tests/build.rs#findings_go_back_to_the_builder_and_a_clean_second_review_commits
- [AC-14] At most `review_rounds` fix rounds run per task, default 1; findings left after the last round stop the build and nothing commits.
  @ref crates/lemma/tests/build.rs#findings_left_after_the_last_round_stop_with_a_list_and_a_record_file
- [AC-15] Setting `review_rounds = 2` allows two fix rounds.
  @ref crates/lemma/tests/build.rs#review_rounds_two_allows_two_fix_rounds
- [AC-16] When the runner command has `{session}`, the fix round runs the `resume` command with the session id the build call ran in.
  @ref crates/lemma/tests/build.rs#the_resume_command_gets_the_session_the_build_call_ran_in
- [AC-17] [intent] The default `resume` command is `claude -p --resume {session} --output-format json --json-schema {schema}`.
- [AC-18] When the runner command has no `{session}`, the fix round is a fresh runner with the build prompt plus the fix section.
  @ref crates/lemma/tests/build.rs#without_a_session_the_fix_is_a_fresh_runner_with_the_build_prompt_and_the_fix_section
- [AC-19] The refs in the fix reply replace the refs the build reply wrote for the task.
  @ref crates/lemma/tests/build.rs#the_fix_reply_refs_replace_the_refs_the_build_reply_wrote
- [AC-20] A fix that breaks lint stops the build before the second review.
  @ref crates/lemma/tests/build.rs#a_fix_that_breaks_lint_stops_before_the_second_review
- [AC-21] When fix rounds are exhausted the build prints each finding as `  <basis>: <text>` and `stopped. T<n> left unticked after <k> fix rounds; working tree kept.`, then exits 2.
  @ref crates/lemma/tests/build.rs#findings_left_after_the_last_round_stop_with_a_list_and_a_record_file
- [AC-22] A reviewer `stop` prints `stopped. T<n> review: <reason>; working tree kept.` and exits 2.
  @ref crates/lemma/tests/build.rs#a_stop_ends_the_build_with_the_tree_kept_and_the_task_unticked
- [AC-23] On a stop the findings are written to `<git-dir>/lemma/reviews/<delta>-T<n>.json`, which is not committed, and nothing is written to `tasks.md` or `claims.md`.
  @ref crates/lemma/tests/build.rs#findings_left_after_the_last_round_stop_with_a_list_and_a_record_file
- [AC-24] A `stop` verdict writes the record file too.
  @ref crates/lemma/tests/build.rs#a_stop_verdict_writes_the_record_file_too
- [AC-25] The working tree is never reset on a stop; the edits stay and HEAD does not move.
  @ref crates/lemma/tests/build.rs#a_stop_ends_the_build_with_the_tree_kept_and_the_task_unticked
- [AC-26] A task that passes review with no findings commits with the trailer `Lemma-Review: clean`.
  @ref crates/lemma/tests/build.rs#a_clean_review_commits_with_the_review_trailer
- [AC-27] A task whose findings were fixed commits with the trailer `Lemma-Review: <n> findings fixed in <k> rounds`.
  @ref crates/lemma/tests/build.rs#review_rounds_two_allows_two_fix_rounds
- [AC-28] [intent] Each reviewer and fix call shows the spinner.
- [AC-29] Each fix call prints its own watch line when the command has `{session}`.
  @ref crates/lemma/tests/build.rs#the_resume_command_gets_the_session_the_build_call_ran_in
- [AC-30] A clean review prints `review: clean` under the task heading.
  @ref crates/lemma/tests/build.rs#a_clean_review_commits_with_the_review_trailer
- [AC-31] A review with findings prints `review: <n> findings, fix round <k>`.
  @ref crates/lemma/tests/build.rs#findings_go_back_to_the_builder_and_a_clean_second_review_commits
- [AC-32] `lemma review` on an in-progress delta errors with `review needs a draft delta; <id> is in-progress, and lemma build reviews each task`, with or without `--dry-run`.
  @ref crates/lemma/tests/prompts.rs#review_on_an_in_progress_delta_says_build_reviews_each_task
- [AC-33] [intent] The `review-diff` prompt, its schema and the follow-up writer no longer exist.
- [AC-34] A leftover `review-diff.md` prompt file is ignored.
  @ref crates/lemma/tests/prompts.rs#a_leftover_review_diff_prompt_file_is_ignored
- [AC-35] Human-written `## Follow-ups` tasks are still built after the main tasks.
  @ref crates/lemma/tests/build.rs#follow_ups_run_after_the_main_tasks
