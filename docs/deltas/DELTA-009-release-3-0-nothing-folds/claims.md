- [AC-1] [intent] Nothing folds: `apply`, fold, `migrate`, check 4, the `readiness` surface, assumption history, `[assumed]`, `[observed]`, the `lemma-fold` hook and the plugin skills are gone.
- [AC-2] `lemma refs` still resolves a claim's refs and prints `ok` per resolving ref.
  @ref crates/lemma/tests/refs.rs#a_resolving_ref_is_ok_and_exits_zero
- [AC-3] [intent] `@per` parses as a citation line and is otherwise ignored.
- [AC-4] A delta that has intent, a claim per behaviour, a task per claim, and every ref resolving passes `lemma lint DELTA-n --gate in-progress` and `--gate complete` silently.
  @ref crates/lemma/tests/lint.rs#a_planned_and_built_delta_passes_both_gates
- [AC-5] A gate fails a delta with no `## Intent` text.
  @ref crates/lemma/tests/lint.rs#a_gate_needs_intent
- [AC-6] A gate fails a delta with no claims.
  @ref crates/lemma/tests/lint.rs#a_gate_needs_a_claim
- [AC-7] A claim bullet with no id is an error at a gate.
  @ref crates/lemma/tests/lint.rs#a_bullet_without_an_id_is_an_error_at_a_gate
- [AC-8] Every claim is settled by exactly one main task; two tasks settling one claim, or none, is an error.
  @ref crates/lemma/tests/lint.rs#every_claim_is_settled_by_exactly_one_main_task
- [AC-9] A follow-up task may settle an already-settled claim, and `--gate complete` requires follow-ups ticked.
  @ref crates/lemma/tests/lint.rs#follow_ups_may_settle_a_settled_claim_and_must_be_ticked_at_complete
- [AC-10] A task settling a claim `claims.md` does not have is always an error.
  @ref crates/lemma/tests/lint.rs#a_task_settling_an_unknown_claim_is_always_an_error
- [AC-11] A task line not shaped `- [ ] T<n> <text> (settles AC-<n>, ..)` or `(chore)` is always an error.
  @ref crates/lemma/tests/lint.rs#a_malformed_task_line_is_always_an_error
- [AC-12] Claim and task ids are unique within the delta, and a claim id that is not `AC-<n>` is an error.
  @ref crates/lemma/tests/lint.rs#duplicate_and_foreign_ids_are_always_errors
- [AC-13] An `@ref` line in `design.md` is a shape error.
  @ref crates/lemma/tests/lint.rs#a_ref_in_design_is_always_an_error
- [AC-14] An `@review` line under a claim or task, or a `> review:` blockquote in a prose part, is an unresolved review line that blocks a gate.
  @ref crates/lemma/tests/lint.rs#review_lines_under_a_claim_a_task_and_prose_block_a_gate
- [AC-15] A `[proposed]` claim blocks a gate, and `[proposed]` with `[intent]` on one claim is an error.
  @ref crates/lemma/tests/lint.rs#a_proposed_claim_blocks_a_gate_and_never_pairs_with_intent
- [AC-16] A claim with no `@ref` and no `[intent]` is an error only at `--gate complete`.
  @ref crates/lemma/tests/lint.rs#a_missing_ref_is_an_error_only_at_complete
- [AC-17] A written `@ref` that does not resolve is an error with or without a gate.
  @ref crates/lemma/tests/lint.rs#a_written_ref_must_resolve_with_or_without_a_gate
- [AC-18] `lemma lint` with no delta checks the working tree against base and leaves delta shape alone.
  @ref crates/lemma/tests/lint.rs#without_a_delta_lint_leaves_deltas_alone
- [AC-19] An unknown gate or unknown delta is refused with exit 2.
  @ref crates/lemma/tests/lint.rs#an_unknown_gate_or_delta_is_refused
- [AC-20] `lemma lint` prints `path:line: severity: message` and exits 2 only when an error remains.
  @ref crates/lemma/tests/lint.rs#text_output_is_path_line_severity_message
  @ref crates/lemma/tests/lint.rs#json_exits_zero_when_only_warnings_remain
- [AC-21] A delta written from the pack templates has no intent until the human fills it, and trips no shape rule until a gate is asked for.
  @ref crates/lemma/tests/pack.rs#a_delta_template_with_only_guidance_has_no_intent
  @ref crates/lemma/tests/pack.rs#delta_templates_trip_no_shape_rule_until_a_gate
- [AC-22] An in-flight delta covers the change to a spec through its `changes` edge, and only the spec it changes.
  @ref crates/lemma/tests/lint.rs#an_in_flight_delta_covers_only_the_spec_it_changes
- [AC-23] Editing a test that a landed delta's claim refs warns, naming the test, the delta and the claim: `test <path>::<sym> changed; DELTA-004/AC-2 claims it; confirm intended`.
  @ref crates/lemma/tests/lint.rs#editing_a_test_a_landed_delta_claims_warns_and_names_it
- [AC-24] At `--gate complete` on a transition, evidence is checked on the moved delta only, not on other deltas in flight.
  @ref crates/lemma/tests/lint.rs#the_complete_gate_checks_evidence_on_the_moved_delta_only
- [AC-25] Editing a test that a dictum claim refs is an error unless the dictum strikes the id and adds a new one.
  @ref crates/lemma/tests/dictum.rs#a_changed_region_a_dictum_claim_references_is_a_check_2_error
  @ref crates/lemma/tests/dictum.rs#the_dictum_striking_the_id_and_adding_a_new_one_excuses_the_change
- [AC-26] [intent] `lemma.toml` sets `runner`, `reviewer` (default the runner) and `timeout` (default 1800 seconds); the prompt goes on stdin with `LEMMA_DELTA` and `LEMMA_TASK` in the environment, and stdout is parsed as JSON with `result` as the text.
- [AC-27] A runner that exits non-zero or reports `is_error` is a failure, and nothing is written.
  @ref crates/lemma/tests/review.rs#a_failing_reviewer_is_exit_two_with_its_stderr_and_nothing_written
  @ref crates/lemma/tests/plan.rs#a_failed_planner_writes_nothing_not_even_the_tagged_ids
- [AC-28] [intent] A runner that outlives `timeout` is killed and is a failure.
- [AC-29] A malformed `lemma.toml` fails the command.
  @ref crates/lemma/tests/prompts.rs#a_bad_lemma_toml_fails_the_command
- [AC-30] A prompt file under `.lazyspec/prompts/` overrides the binary's default for that prompt.
  @ref crates/lemma/tests/prompts.rs#a_pack_prompt_overrides_the_default
- [AC-31] An unknown `{{key}}` in a prompt is an error naming the file, line and key, both on `--dry-run` and in `lemma lint`.
  @ref crates/lemma/tests/prompts.rs#dry_run_names_an_unknown_prompt_key_and_exits_two
  @ref crates/lemma/tests/prompts.rs#lint_reports_an_unknown_prompt_key_at_its_path_and_line
- [AC-32] `--dry-run` on review and build prints the assembled prompt and exits without calling the runner.
  @ref crates/lemma/tests/prompts.rs#review_dry_run_on_a_draft_renders_review_doc_with_claims_and_intent
  @ref crates/lemma/tests/prompts.rs#build_dry_run_prints_the_first_unticked_task_and_its_claim
- [AC-33] The prompt carries the dictums whose `governs` overlap a changed spec's `governs`, else all dictums.
  @ref crates/lemma/tests/prompts.rs#dictums_are_selected_by_governs_overlap_with_the_changed_specs
  @ref crates/lemma/tests/prompts.rs#every_dictum_is_included_when_none_is_selected
- [AC-34] [intent] The prompt carries the convention body, each changed spec's `index.md`, every `related:` doc body by id, and the delta parts.
- [AC-35] `lemma review` on a draft delta writes each finding under its anchor: as `@review` under a claim or task, as `> review:` at the end of a heading's section in a prose part.
  @ref crates/lemma/tests/review.rs#findings_land_under_the_right_anchor_in_each_part
- [AC-36] Running review twice with the same reply writes each review line once.
  @ref crates/lemma/tests/review.rs#running_twice_with_the_same_reply_changes_nothing_the_second_time
- [AC-37] A finding with an unknown anchor is an error naming it, and nothing is written.
  @ref crates/lemma/tests/review.rs#an_unknown_anchor_is_an_error_naming_it_and_nothing_is_written
- [AC-38] A review with zero findings reports clean and changes nothing.
  @ref crates/lemma/tests/review.rs#zero_findings_with_review_lines_left_reports_clean_and_changes_nothing
- [AC-39] [intent] A review with zero findings runs plan in the same invocation.
- [AC-40] Plan assigns `AC-<n>` to claims in document order, continuing from the highest id.
  @ref crates/lemma/tests/plan.rs#ids_are_assigned_in_document_order_continuing_from_the_highest
- [AC-41] Plan keeps the human's bullet text byte for byte.
  @ref crates/lemma/tests/plan.rs#human_bullet_text_is_preserved_byte_for_byte
- [AC-42] Plan appends proposed claims with `[proposed]`.
  @ref crates/lemma/tests/plan.rs#proposed_claims_are_appended_with_the_marker
- [AC-43] Plan writes tasks after any hand-written tasks.
  @ref crates/lemma/tests/plan.rs#tasks_are_appended_after_hand_written_ones_and_before_follow_ups
- [AC-44] Plan refuses when any task is ticked, without calling the planner.
  @ref crates/lemma/tests/plan.rs#a_ticked_task_refuses_and_the_planner_is_not_called
- [AC-45] Plan puts frontmatter keys in canonical order and normalises headings.
  @ref crates/lemma/tests/plan.rs#frontmatter_keys_are_put_in_canonical_order
  @ref crates/lemma/tests/plan.rs#headings_are_normalised_outside_fences
- [AC-46] Re-running plan on a planned delta changes nothing.
  @ref crates/lemma/tests/plan.rs#rerunning_on_a_planned_delta_changes_nothing
- [AC-47] [intent] `lemma review` on an in-progress delta writes each finding as an unticked task under `## Follow-ups` with its `settles`, writes nothing to `claims.md`, and reports the count.
- [AC-48] `lemma build` runs each unticked task in order, ticks it and commits `DELTA-004 T2: <text>`.
  @ref crates/lemma/tests/build.rs#each_task_is_ticked_and_committed_in_order
- [AC-49] Build stops with the tree kept and the task unticked when lint fails after the runner.
  @ref crates/lemma/tests/build.rs#a_lint_error_stops_with_the_tree_kept
- [AC-50] Build stops when a settled claim has no resolving ref.
  @ref crates/lemma/tests/build.rs#a_settled_claim_without_a_ref_stops
- [AC-51] A re-run of build resumes at the first unticked task.
  @ref crates/lemma/tests/build.rs#a_rerun_resumes_at_the_first_unticked_task
- [AC-52] `--task T<n>` builds only that task.
  @ref crates/lemma/tests/build.rs#task_runs_only_that_task
- [AC-53] A chore task skips the ref check.
  @ref crates/lemma/tests/build.rs#a_chore_skips_the_ref_check
- [AC-54] Build refuses when the tree is dirty at start.
  @ref crates/lemma/tests/build.rs#a_dirty_tree_is_refused
- [AC-55] Build refuses when the delta is not `in-progress`.
  @ref crates/lemma/tests/build.rs#a_delta_that_is_not_in_progress_is_refused
- [AC-56] Follow-up tasks run after the main tasks.
  @ref crates/lemma/tests/build.rs#follow_ups_run_after_the_main_tasks
- [AC-57] The pack's hooks run `lemma lint --hook` at validate, `--gate in-progress` for the intent gate and `--gate complete` for the check gate; `lemma-fold` is gone.
  @ref crates/lemma/tests/pack.rs#the_hook_table_runs_lint_at_validate_and_each_gate
- [AC-58] The plugin carries hooks only and no skills directory.
  @ref crates/lemma/tests/pack.rs#the_plugin_ships_hooks_only
- [AC-59] A repo adopted from the pack lints clean.
  @ref crates/lemma/tests/pack.rs#a_repo_adopted_from_the_pack_checks_clean
- [AC-60] [intent] The CI template runs `lemma lint`.
- [AC-61] [intent] The convention says humans write documents, agents annotate, and a human deletes an annotation to accept it; the build rule says the human writes intent, design and behaviour, the plan step names tasks and the build loop produces evidence; CHG-10 is struck.
- [AC-62] Signal's one-line pointer for a touched file names an in-flight delta's parts and its `related:` documents.
  @ref crates/lemma/tests/signal.rs#pointers_name_an_in_flight_deltas_parts_and_related_documents
- [AC-63] Stop blocks a session that leaves a touched delta's claim ref unresolved.
  @ref crates/lemma/tests/stop.rs#a_broken_ref_in_a_touched_delta_is_an_error
- [AC-64] Stop blocks a session that completes a delta, since completion is the human's step after `lemma review`.
  @ref crates/lemma/tests/stop.rs#working_tree_complete_over_head_in_progress_errors
- [AC-65] [intent] The crate, plugin and README are version 3.0.0.
