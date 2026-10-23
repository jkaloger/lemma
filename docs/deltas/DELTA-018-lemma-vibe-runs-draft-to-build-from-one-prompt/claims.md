<!-- One bullet per behaviour, in plain words: `- An empty account has balance zero.` `lemma review` assigns ids (`- [AC-1] ...`). Mark a behaviour no test can settle `[intent]` after its id. -->

- [AC-1] `lemma vibe -m <prompt>` on a dirty tree exits 2 without calling a runner.
  @ref crates/lemma/tests/vibe.rs#a_dirty_tree_exits_2_without_calling_a_runner
- [AC-2] The draft stage creates a new draft delta whose parts hold the runner reply's intent, out of scope, design and claims, linked `changes` to each spec in the reply.
  @ref crates/lemma/tests/vibe.rs#the_draft_creates_a_delta_holding_the_reply_linked_to_its_specs
- [AC-3] A draft reply with no `changes` spec, or an unknown one, stops the run before any delta is created.
  @ref crates/lemma/tests/vibe.rs#a_draft_reply_with_no_or_an_unknown_spec_stops_before_any_delta_is_created
- [AC-4] A draft stage that fails after the delta is created deletes it before exiting.
  @ref crates/lemma/tests/vibe.rs#a_draft_that_fails_after_create_deletes_the_delta
- [AC-5] Every review call that writes findings, except the last, is followed by a revise call whose replacement parts lemma writes, keeping the index frontmatter as it was.
  @ref crates/lemma/tests/vibe.rs#each_review_with_findings_but_the_last_is_revised_and_lemma_writes_the_parts
- [AC-6] Review stops early on the first clean round.
  @ref crates/lemma/tests/vibe.rs#review_stops_at_the_first_clean_round
- [AC-7] When the last review call writes findings, lemma does not revise; it removes every review line, prints each as a warning, and runs plan.
  @ref crates/lemma/tests/vibe.rs#findings_from_the_last_review_are_not_revised_but_stripped_warned_and_planned
- [AC-8] `--rounds` defaults to 3 review calls.
  @ref crates/lemma/tests/vibe.rs#rounds_defaults_to_three_review_calls
- [AC-9] After plan, no claim in `claims.md` carries `[proposed]`, and the count accepted is printed.
  @ref crates/lemma/tests/vibe.rs#after_plan_no_claim_is_proposed_and_the_count_accepted_is_printed
- [AC-10] Vibe moves the delta to in-progress and commits it without asking, refusing if the gate lint has errors.
  @ref crates/lemma/tests/vibe.rs#vibe_moves_the_delta_to_in_progress_and_commits_it_without_asking
  @ref crates/lemma/tests/vibe.rs#vibe_refuses_the_move_when_the_gate_lint_has_errors
- [AC-11] A build that stops a task exits 2 and prints `lemma build <id> --resume`; a build that fails without a stopped task prints `lemma build <id>`.
  @ref crates/lemma/tests/vibe.rs#a_build_that_stops_a_task_exits_2_and_names_the_resume
  @ref crates/lemma/tests/vibe.rs#a_build_that_fails_without_a_stopped_task_names_the_plain_build
- [AC-12] Vibe never moves the delta to complete; a successful run ends with `next: lazyspec update <id> --status complete`.
  @ref crates/lemma/tests/vibe.rs#vibe_leaves_the_delta_in_progress_and_ends_naming_the_complete_step
- [AC-13] A stage that fails exits 2 and prints the `next:` line its row in the design table gives.
  @ref crates/lemma/tests/vibe.rs#a_failing_stage_exits_2_and_prints_its_next_line
- [AC-14] In plain mode vibe output carries no ANSI colour codes.
- [AC-15] [intent] In live mode each stage opens with a coloured banner, and the run ends with a summary of per-stage time, rounds, accepted claims and committed tasks.
