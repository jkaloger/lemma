- [AC-1] [intent] A receipt is a ref line naming a path and a symbol, with no pin.
- [AC-2] [intent] A ref pinned to a sha or blob, a ` [forced]` ref or a base line is an error with the hint `pinned ref; run lemma migrate`.
- [AC-3] [intent] A claim no test settles is `[intent]`; the checked marker is no longer a receipt.
- [AC-4] [intent] A delta claim whose id exists in the target spec is an error unless struck: `claims are immutable; strike <id> and add a new id`.
- [AC-5] [intent] Fold appends new ids to the spec and deletes struck ids.
- [AC-6] [intent] A struck id the spec lacks is an error at `apply` and in check 4.
- [AC-7] [intent] Two in-flight deltas striking one id is a warning.
- [AC-8] A change inside one governed area needs no delta.
  @ref crates/lemma/tests/lint.rs#a_change_inside_one_area_needs_no_delta
- [AC-9] When the range's changed governed files span more than one spec's area, each touched spec with no covering delta is an error.
  @ref crates/lemma/tests/lint.rs#changes_across_two_areas_without_a_delta_error_for_each_spec
- [AC-10] A delta in flight (draft or in-progress) covers the spec it changes and no other.
  @ref crates/lemma/tests/lint.rs#an_in_flight_delta_covers_only_the_spec_it_changes
  @ref crates/lemma/tests/lint.rs#a_draft_delta_covers
- [AC-11] A delta that became complete in the range covers its specs.
  @ref crates/lemma/tests/lint.rs#a_delta_completed_within_the_range_covers_its_specs
- [AC-12] A delta that was already complete before the range does not cover it.
  @ref crates/lemma/tests/lint.rs#a_delta_completed_before_the_range_does_not_cover_it
- [AC-13] [intent] A delta became complete in the range when its `index.md` changed in the range and its status at the base was not `complete`, or it did not exist at the base.
- [AC-14] [intent] For a claim present in a spec's `behaviour` part at the base, each ref region whose extracted, normalised text differs between base and working tree is an error unless a delta strikes that claim.
- [AC-15] Editing a sibling test in the same file is not a change to a ref's region.
  @ref crates/lemma/tests/lint.rs#editing_a_sibling_test_is_clean
- [AC-16] A ref region that is present at the base and gone from the working tree counts as changed.
  @ref crates/lemma/tests/lint.rs#a_ref_deleted_from_the_working_tree_is_changed
- [AC-17] A ref region absent at the base is not a change.
  @ref crates/lemma/tests/lint.rs#a_symbol_absent_at_the_base_is_not_a_change
- [AC-18] [intent] A strike silences check 2 only from a delta that folds into the claim's spec, strikes in that spec's `behaviour.<spec-id>` part, and is in-progress or became complete in the range; a draft delta does not count.
- [AC-19] [intent] `.ts`, `.tsx`, `.js`, `.jsx`, `.mjs` and `.cjs` files use the TypeScript extractor and grammar.
- [AC-20] [intent] A file with no extractor is compared whole.
- [AC-21] [intent] Check 3: every live, non-`[intent]` claim has at least one ref that resolves to a symbol in a file matching the governing spec's `tests` globs.
- [AC-22] A ref that does not resolve to a symbol is always an error.
  @ref crates/lemma/tests/lint.rs#an_unresolved_ref_is_always_an_error
- [AC-23] A ref into a file outside the governing spec's `tests` globs is an error, and the changed specs' `tests` globs decide which files count.
  @ref crates/lemma/tests/lint.rs#a_ref_into_production_code_is_always_an_error
  @ref crates/lemma/tests/lint.rs#the_changed_specs_tests_globs_decide
- [AC-24] [intent] Check 4 keeps the 1.5 structural findings: fold targets resolve, parts name targets, no claim in `index.md`.
- [AC-25] An invalid `tests` glob is an error.
  @ref crates/lemma/tests/lint.rs#an_invalid_glob_is_an_error
- [AC-26] [intent] A spec whose `behaviour` part differs from the base must equal the base claims folded, in id order, with every delta that became complete in the range, compared as claim id and first-line text with `[intent]` ignored; otherwise the error is `spec behaviour edited outside a fold; change it through a delta`.
- [AC-27] [intent] A change in the range to a spec's `governs` or `tests` is a warning: `governs/tests changed in this range; review`.
- [AC-28] [intent] A spec absent at the base is not compared.
- [AC-29] [intent] Check 4 claim identity is the whole block minus ref, base and checked lines, pins and `[intent]`; a ref held at the base that is gone is an error, and added refs are allowed.
- [AC-30] `--base <rev>` wins over the resolved base.
  @ref crates/lemma/tests/lint.rs#the_base_comes_from_the_merge_base_on_a_branch
- [AC-31] On a branch, committed changes since the merge-base with trunk are in range.
  @ref crates/lemma/tests/lint.rs#on_a_branch_committed_changes_are_in_range
- [AC-32] The trunk is `main` if it exists, else `master`.
  @ref crates/lemma/tests/lint.rs#master_is_found_without_a_trunk_flag
- [AC-33] `--trunk <name>` names the trunk.
  @ref crates/lemma/tests/lint.rs#trunk_is_configurable
- [AC-34] The default trunk also considers `origin/main`, `origin/master` and the `origin/HEAD` target; when several exist the merge-base nearer to HEAD wins, so merging an advanced `origin/main` into a branch stays clean.
  @ref crates/lemma/tests/lint.rs#merging_an_advanced_remote_trunk_into_a_branch_is_clean
- [AC-35] On trunk (direct-to-main) with no session base, committed changes are out of range unless `--base` says so.
  @ref crates/lemma/tests/lint.rs#on_trunk_committed_changes_are_out_of_range_unless_the_base_says_so
- [AC-36] On trunk, Stop uses the session base, so commits made in the session are in range.
  @ref crates/lemma/tests/stop.rs#the_session_base_brings_commits_made_in_the_session_into_range
- [AC-37] On trunk, the gate for a payload with a transition ranges from the parent of the commit that first added the moved delta's `index.md`.
  @ref crates/lemma/tests/lint.rs#on_trunk_the_gate_uses_the_parent_of_the_deltas_first_commit
- [AC-38] [intent] On trunk, the gate uses HEAD as the base when the moved delta is uncommitted.
- [AC-39] [intent] CI passes `--base` explicitly: the PR base ref, or the push's `before` sha (fetched, tolerating failure, falling back to `--trunk origin/<default-branch>` after a force-push); a new-branch push passes `--trunk origin/<default-branch>`. A push to the default branch uses `before` only when it is an ancestor of HEAD, else `HEAD^`, else no check.
- [AC-40] [intent] `lemma check` runs checks 1 to 4; invariants (check 2, check 4, refs that do not resolve, pinned refs) are errors and a claim without evidence (check 3) is a warning unless required.
- [AC-41] [intent] `--require <claims|evidence|clean>` (repeatable) turns readiness into errors: `claims` is Intent present, every fold target has its part, at least one claim; `evidence` makes check 3 errors; `clean` is no uncommitted changes in ref files or under a target's `governs`.
- [AC-42] Output is `path:line: severity: message` lines and is silent when clean.
  @ref crates/lemma/tests/lint.rs#text_output_is_path_line_severity_message
  @ref crates/lemma/tests/lint.rs#a_clean_project_is_silent_and_exits_zero
- [AC-43] Exit status is 2 on an error finding, and `--json` prints `{"findings": [...]}`.
  @ref crates/lemma/tests/lint.rs#json_prints_findings_and_exits_two_on_error
- [AC-44] `--hook` reads a lazyspec payload on stdin, prints findings JSON and exits 0, even with errors.
  @ref crates/lemma/tests/lint.rs#it_exits_zero_and_prints_findings_json_even_with_errors
- [AC-45] In `--hook` mode, delta checks are scoped to the payload documents.
  @ref crates/lemma/tests/lint.rs#delta_checks_are_scoped_to_the_payload_documents
- [AC-46] The `validate` event, which carries no transition, skips the range checks.
  @ref crates/lemma/tests/lint.rs#the_validate_event_skips_the_range_checks
- [AC-47] A pre-transition gate limits coverage findings to the specs the moved delta changes.
  @ref crates/lemma/tests/lint.rs#a_transition_limits_coverage_to_the_moved_deltas_specs_and_lets_history_through
- [AC-48] [intent] A pre-transition gate limits check 2 findings to the moved delta's target specs, and the moved delta's strikes count for check 2 at the draft to in-progress gate.
- [AC-49] [intent] The pack's hooks run `check --hook` on validate, `check --hook --require claims` on draft to in-progress, and `check --hook --require evidence --require clean` then `apply` on in-progress to complete; lazyspec stops at the first error, so `apply` never writes after a failed check.
- [AC-50] [intent] `lemma pre-transition` and `lemma validate` are gone.
- [AC-51] [intent] `lemma apply` with no argument is the pre-transition hook: it reads the payload, folds each target and returns `updates`; it errors on check 4 and on a target with no `behaviour` part, and warns for a draft target.
- [AC-52] [intent] `lemma apply <delta>` previews, printing per target the ids it would add and delete, and writes nothing.
- [AC-53] `lemma refs <claim>` prints `<claim> <path>#<symbol> ok|unresolved|not-a-test` per ref and exits 2 on any failure.
  @ref crates/lemma/tests/refs.rs#each_ref_gets_its_own_status_and_any_failure_exits_two
- [AC-54] [intent] `lemma refs` looks across specs and in-flight deltas.
- [AC-55] `lemma stop` on an error exits 2 with check-format diagnostics on stderr.
  @ref crates/lemma/tests/stop.rs#committed_changes_across_two_areas_are_errors_in_check_format
- [AC-56] Stop scopes check 3 to session-touched documents: a broken ref in an untouched delta is silent, in a touched delta it is an error, and in a delta changing the spec of a touched file it is an error.
  @ref crates/lemma/tests/stop.rs#a_broken_ref_in_an_untouched_delta_is_silent
  @ref crates/lemma/tests/stop.rs#a_broken_ref_in_a_touched_delta_is_an_error
  @ref crates/lemma/tests/stop.rs#a_broken_ref_in_a_delta_changing_the_spec_of_a_touched_file_is_an_error
- [AC-57] A delta `complete` in the working tree that was not `complete` at the session base is an error at Stop, including a new delta already complete.
  @ref crates/lemma/tests/stop.rs#working_tree_complete_over_head_in_progress_errors
  @ref crates/lemma/tests/stop.rs#a_new_delta_already_complete_in_the_working_tree_errors
- [AC-58] A delta already complete at the session base does not trip the completion rule.
  @ref crates/lemma/tests/stop.rs#a_delta_already_complete_at_the_session_base_passes
- [AC-59] The completion rule is said once per session, so only the first Stop after a human completes and commits a delta is blocked.
  @ref crates/lemma/tests/stop.rs#a_delta_completed_and_committed_in_the_session_errors_once
- [AC-60] Another session hears the completion again.
  @ref crates/lemma/tests/stop.rs#another_session_hears_the_completion_again
- [AC-61] `stop_hook_active` makes Stop exit 0 without checking.
  @ref crates/lemma/tests/stop.rs#stop_hook_active_exits_zero_without_checking
- [AC-62] Stop no longer detects pending tests.
  @ref crates/lemma/tests/stop.rs#pending_tests_are_no_longer_detected
- [AC-63] `lemma signal` acts only on Edit, Write and MultiEdit.
  @ref crates/lemma/tests/signal.rs#only_edit_write_and_multiedit_are_acted_on
- [AC-64] On the first touch of a governed area the signal gives one context line of the governing ids and titles, no content.
  @ref crates/lemma/tests/signal.rs#the_context_pointer_is_one_line_with_ids_and_titles
- [AC-65] The context pointer is said once per area per session, with hashes kept in `<git-dir>/lemma/sessions/<id>.said`, and another session hears it again.
  @ref crates/lemma/tests/signal.rs#the_context_is_said_once_per_area_then_silent
  @ref crates/lemma/tests/signal.rs#another_session_hears_it_again
- [AC-66] Another governed area gets its own pointer.
  @ref crates/lemma/tests/signal.rs#another_area_gets_its_own_pointer
- [AC-67] Editing a test that a claim refs and that changed names the claim and is said once.
  @ref crates/lemma/tests/signal.rs#a_changed_test_names_the_landed_delta_and_is_said_once
- [AC-68] Editing a sibling test is silent.
  @ref crates/lemma/tests/signal.rs#editing_a_sibling_test_is_silent
- [AC-69] The file that makes the change span two areas is flagged as a coverage finding.
  @ref crates/lemma/tests/signal.rs#the_file_that_makes_the_change_span_two_areas_is_flagged
- [AC-70] A covering delta silences the coverage finding.
  @ref crates/lemma/tests/signal.rs#a_covering_delta_silences_the_coverage_finding
- [AC-71] When the touched file is a delta part or a spec, the signal reports its claim and glob findings.
  @ref crates/lemma/tests/signal.rs#editing_a_delta_part_reports_its_claims
  @ref crates/lemma/tests/signal.rs#editing_a_spec_reports_its_invalid_globs
- [AC-72] A clean edit to an ungoverned file is silent.
  @ref crates/lemma/tests/signal.rs#a_clean_edit_to_an_ungoverned_file_is_silent
- [AC-73] The signal never blocks: bad or empty input is silent.
  @ref crates/lemma/tests/signal.rs#it_never_blocks_on_bad_input
- [AC-74] A coverage finding is said once per spec however many more files change.
  @ref crates/lemma/tests/signal.rs#coverage_is_said_once_per_spec_even_when_more_files_change
- [AC-75] [intent] Signal messages are deduped on stable keys (`context:<ids>`, `check1:<spec-id>`, `check2:<claim>:<path>#<symbol>`, `check4:<doc>:<path>:<message>`), not rendered text.
- [AC-76] [intent] The signal resolves the base only when the file is governed, referenced or a spec or delta file, and spawns no `lazyspec --version`.
- [AC-77] [intent] For a spec file the signal includes the check 4 spec-edit rule and the `governs`/`tests` warning.
- [AC-78] [intent] `lemma migrate` rewrites every spec and delta part in place: it strips the pin and ` [forced]` from ref lines, deletes base lines and turns a claim whose only receipts are checked lines into `[intent]`, printing the files it wrote.
- [AC-79] [intent] `lemma migrate` is idempotent.
- [AC-80] [intent] A delta claim with a base line whose id exists in its target spec becomes a strike of the old id plus the claim under `<old-id>-R` (`-R2`, `-R3` if taken), printing an indented line per rewrite; only `draft` and `in-progress` deltas are rewritten.
- [AC-81] [intent] `re-pin`, `receipt`, `base`, `validate` and `pre-transition` are removed, along with receipt freshness, symbol drift, `[forced]`, checked lines, base lines and pending-test detection.
- [AC-82] `session-start` stays: it records the session base once per session.
  @ref crates/lemma/tests/stop.rs#records_the_base_once
- [AC-83] [intent] The crate, plugin and README carry version 2.0.0.
