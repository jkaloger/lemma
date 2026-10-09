- [AC-1] A runner command containing `{session}` has it replaced by a fresh UUID on each runner call.
  @ref crates/lemma/tests/build.rs#the_resume_command_gets_the_session_the_build_call_ran_in
- [AC-2] [intent] The default runner command is `claude -p --session-id {session} --output-format json --json-schema {schema}`.
- [AC-3] [intent] A runner or reviewer command without `{session}` runs unchanged.
- [AC-4] [intent] Every runner call (review, plan, build) shows a spinner on stderr with a label and elapsed time, only when stderr is a terminal, and the spinner clears before the summary line prints.
- [AC-5] When the runner command has `{session}`, one stdout line `   watch: tail -n +1 -F "$HOME/.claude/projects/<encoded root>/<uuid>.jsonl" | jq -r '<filter>'` prints before the runner call, with the uuid of the call's session.
  @ref crates/lemma/tests/build.rs#a_runner_with_session_prints_the_watch_line_before_the_report
- [AC-6] When the runner command has no `{session}`, no watch line prints.
  @ref crates/lemma/tests/build.rs#a_runner_without_session_prints_no_watch_line
- [AC-7] [intent] The encoded root in the watch line is the canonical repo root with every non-alphanumeric character replaced by `-`, and the line uses no glob.
- [AC-8] [intent] The watch filter prints one line per tool call, `<name> <input, first 100 chars>`, and lemma does not read the transcript itself.
- [AC-9] [intent] Task headings print bold, `commit` and `lint: clean` green, `stopped` and failures red, and the `next:` label dim.
- [AC-10] [intent] Colour is off when output is not a terminal and under `NO_COLOR`.
- [AC-11] [intent] `lemma build` on a draft delta at a terminal asks `<id> is draft; move it to in-progress? [y/N]`, after the clean-tree check.
- [AC-12] [intent] Answering yes runs `lazyspec update <id> --status in-progress` with inherited stdio; a non-zero exit stops the build with `<id> stayed draft; fix the findings above and run lemma build <id> again`.
- [AC-13] `lemma build` on a draft delta without a terminal is refused, naming `lazyspec update <id> --status in-progress`.
  @ref crates/lemma/tests/build.rs#a_delta_that_is_not_in_progress_is_refused
- [AC-14] [intent] After a successful move, build commits what the move wrote with message `<id>: in-progress` before the first task.
- [AC-15] Every `next:` line names `lazyspec update <id> --status <status>`, not `lazyspec advance`.
  @ref crates/lemma/tests/prompts.rs#build_dry_run_with_every_task_ticked_says_all_complete
- [AC-16] The default `plan` prompt asks for each task as a short imperative title of at most 10 words, and no longer asks to name the files.
  @ref crates/lemma/tests/prompts.rs#the_default_plan_prompt_asks_for_short_task_titles
- [AC-17] [intent] The `review-diff` prompt asks for the same short titles.
- [AC-18] [intent] The plan and review-diff schemas cap task `text` at `maxLength: 80`.
- [AC-19] The commit subject for a task is `<id> T<n>: <text>`.
  @ref crates/lemma/tests/build.rs#the_report_follows_the_documented_shape
- [AC-20] [intent] Off a terminal, the build offer refuses before the clean-tree check; on a terminal, dirty paths are allowed only under the delta's own directory.
- [AC-21] [intent] Before `lazyspec update`, the delta is linted in-process at the in-progress gate; error findings print and stop with `<id> stayed draft; ...` and nothing moves, even when lazyspec skips untrusted hooks.
- [AC-22] [intent] After a zero exit from `lazyspec update`, the status is re-read; anything but in-progress stops the build the same way.
- [AC-23] [intent] The advance commit stages the delta directory only; a failed commit says `<id> moved to in-progress but not committed; commit it and run lemma build <id>`.
- [AC-24] [intent] A `--dry-run` on a delta that is not in-progress also names `lazyspec update <id> --status in-progress`.
- [AC-25] After planning, the `next:` line reads `read the diff, accept or reject each [proposed] claim, then lemma build <id>`.
  @ref crates/lemma/tests/plan.rs#ids_are_assigned_in_document_order_continuing_from_the_highest
