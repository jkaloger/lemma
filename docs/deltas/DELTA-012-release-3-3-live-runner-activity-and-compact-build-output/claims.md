- [AC-1] When stdout or stderr is not a terminal, `review`, `plan` and `build` print exactly what they print today, watch lines included.
  @ref crates/lemma/tests/plain.rs#piped_review_plan_and_build_print_the_plain_report_with_watch_lines
- [AC-2] In live mode, while a runner call with `{session}` waits, the spinner shows the latest tool call in its transcript as `<tool> <target>`.
  @ref crates/lemma/src/runner.rs#the_spinner_shows_the_latest_tool_call_in_the_transcript
- [AC-3] A file tool's target is its path relative to the repo root; `Bash` shows the first line of its command; `Grep` and `Glob` show the pattern; other tools show the name only.
  @ref crates/lemma/src/activity.rs#each_tool_shows_its_target
- [AC-4] The activity line is cut to terminal width.
  @ref crates/lemma/src/activity.rs#the_activity_line_is_cut_to_terminal_width
- [AC-5] A missing transcript, or a transcript line that is not JSON, leaves the spinner showing label and elapsed only, and the run's outcome is unchanged.
  @ref crates/lemma/src/runner.rs#a_missing_or_garbled_transcript_leaves_label_and_elapsed_and_the_outcome_unchanged
- [AC-6] A runner command without `{session}` shows no activity.
  @ref crates/lemma/src/runner.rs#a_runner_command_without_session_shows_no_activity
- [AC-7] Live mode prints no `watch:` line.
  @ref crates/lemma/src/runner.rs#live_mode_prints_no_watch_line
- [AC-8] A stopped task in live mode ends its block with `transcript: <path>` for the call that failed.
  @ref crates/lemma/src/runner.rs#a_stopped_live_task_ends_with_the_transcript_of_the_failed_call
- [AC-9] In live mode each build step line carries `✓` on pass, `✗` on failure, `·` for information.
  @ref crates/lemma/src/output.rs#live_step_lines_carry_a_glyph_per_outcome
- [AC-10] When a task commits in live mode, its block is replaced by one line: `✓`, task id and title, elapsed, review outcome and short sha.
  @ref crates/lemma/src/output.rs#a_committed_live_block_is_replaced_by_one_summary_line
- [AC-11] A stopped task's block is not collapsed.
  @ref crates/lemma/src/output.rs#a_stopped_task_is_never_collapsed
- [AC-12] `NO_COLOR` removes colour and keeps glyphs.
  @ref crates/lemma/src/output.rs#no_color_removes_colour_and_keeps_glyphs
- [AC-13] [intent] lemma reads the transcript for display only; no reply, check, decision or exit status depends on it.
  @ref crates/lemma/src/runner.rs#the_transcript_never_decides_the_reply_or_the_exit_status
- [AC-14] A transcript line split across two polls produces its activity once, after the rest of the line arrives.
  @ref crates/lemma/src/activity.rs#a_line_split_across_two_polls_produces_its_activity_once
- [AC-15] The collapsed task line's review outcome reads `review clean` or `<n> finding(s) fixed`, matching the commit trailer counts.
  @ref crates/lemma/src/output.rs#the_collapsed_review_outcome_matches_the_commit_trailer_counts
