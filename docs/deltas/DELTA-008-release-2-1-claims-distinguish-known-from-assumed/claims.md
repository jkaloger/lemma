- [AC-1] [intent] `[assumed]` is allowed in an in-flight delta only and is excluded from check 3.
- [AC-2] [intent] An `[assumed]` claim is never folded; `apply` errors if one remains.
- [AC-3] [intent] Under `--require evidence` each remaining `[assumed]` claim is an error: `assumption unresolved; accept it as a claim, mark it [observed] with a test, delete it, or move it to the spec's open questions`.
- [AC-4] [intent] A spec claims part containing `[assumed]` is a check 4 error.
- [AC-5] [intent] An `[observed]` claim requires a ref and folds.
- [AC-6] [intent] A changed region under an `[observed]` claim is reported as `characterised behaviour changed; confirm the change is intended`.
- [AC-7] [intent] The markers `[intent]`, `[observed]` and `[assumed]` are mutually exclusive on one claim; two is an error.
- [AC-8] [intent] Check 4 claim identity ignores `[observed]` and `[assumed]`, as it ignores `[intent]`.
- [AC-9] [intent] A citation line under a claim, any number of them, is parsed like a ref line.
- [AC-10] [intent] Check 4 claim identity ignores citation lines.
- [AC-11] [intent] A citation matching a decision id resolves when that decision is `accepted` at the range's base; the preview shows `provenance: DECISION-003 (accepted)`, `(not accepted at base)` for a decision not accepted at the base, and `(unverified)` for any other text.
- [AC-12] [intent] A normative claim with no citation shows `provenance: none; accepting this delta establishes this behaviour`; a citation is never required.
- [AC-13] [intent] The `lemma apply <delta>` preview and the `in-progress → complete` gate findings group per target: `New commitments`, `Observed behaviour`, `Resolved assumptions`, `Discarded assumptions`, `Unresolved assumptions`.
- [AC-14] [intent] Resolution is derived from the range's commits: ids that were `[assumed]` in any committed version of the claims part within the range and are not now are resolved; one that is gone is discarded.
- [AC-15] [intent] A resolved id kept in place prints `AC-17: was assumed, now a commitment`; one replaced prints `A-1 → AC-17`, recognised only when exactly one assumed id vanished and exactly one new id appeared in one step with overlapping text (Jaccard of lowercase words at least 0.5); otherwise the assumption is discarded.
- [AC-16] [intent] In `--hook` mode the surface is one warning per claim (`AC-17: new commitment; provenance: none; accepting this delta establishes this behaviour`), emitted at the `in-progress → complete` gate only.
- [AC-17] [intent] The `<n> of <m> claims are [intent]` finding is removed from all output modes.
- [AC-18] [intent] `--require intent` replaces `--require claims`: `Intent` present and non-empty, every fold target resolves, every target has its claims part (empty allowed).
- [AC-19] [intent] The `readiness` "no claims" branch is deleted and the pack hook for `draft → in-progress` runs `lemma check --hook --require intent`.
- [AC-20] [intent] `--require claims` is accepted as an alias for one release and prints a deprecation hint.
- [AC-21] [intent] The build step says to record what is known before code (constraints, commitments with citations, assumptions as `[assumed]`) and not to invent an acceptance criterion; the adopt step says to claim what would be expensive or surprising to rediscover as `[observed]`.
- [AC-22] [intent] CHG-9 and TEST-1 are reworded for the markers: build records what is known going in, and an `[assumed]` claim is not yet normative.
- [AC-23] [intent] The delta skill's vocabulary and lifecycle cover the three markers and citation lines, and never remove `[assumed]` to clear a gate without one of its four exits.
- [AC-24] [intent] `.lazyspec.toml` declares `convention` and `dictum` types, the pack ships `CONVENTION.md`, `DICTUM-001-testing.md` and `DICTUM-002-code-change.md` as seed documents, and `.lazyspec/dictums/` is gone.
- [AC-25] [intent] The entry skill copies the pack's seed documents if `docs/convention/` is empty after adopt, until lazyspec RFC-076 lands.
- [AC-26] A repo adopted from the pack checks clean.
  @ref crates/lemma/tests/pack.rs#a_repo_adopted_from_the_pack_checks_clean
- [AC-27] A delta template with only guidance comments has no Intent.
  @ref crates/lemma/tests/pack.rs#a_delta_template_with_only_guidance_has_no_intent
- [AC-28] A delta template with Intent filled in has an Intent.
  @ref crates/lemma/tests/pack.rs#a_filled_delta_template_has_intent
- [AC-29] [intent] The spec template carries `## Intent`, `## Constraints`, `## Non-goals` and `## Open questions` and the delta template `## Intent`, `## Approach` and `## Out of scope`, each with one line of guidance in an HTML comment.
- [AC-30] [intent] The world loads `dictum` documents as claim sites with the dictum as its own governing document; test scope is the dictum's `tests` key with the spec defaults.
- [AC-31] An unresolved ref in a dictum claim is a check 3 error naming the dictum.
  @ref crates/lemma/tests/dictum.rs#an_unresolved_dictum_ref_is_a_check_3_error
- [AC-32] A dictum claim without a ref is flagged unless it is `[intent]`.
  @ref crates/lemma/tests/dictum.rs#a_dictum_claim_without_a_ref_is_flagged_unless_intent
- [AC-33] A dictum claim with a resolving ref is clean.
  @ref crates/lemma/tests/dictum.rs#a_dictum_claim_with_a_resolving_ref_is_clean
- [AC-34] A changed region that a dictum claim references is a check 2 error telling the author to strike the claim in the dictum.
  @ref crates/lemma/tests/dictum.rs#a_changed_region_a_dictum_claim_references_is_a_check_2_error
- [AC-35] The dictum striking the id and adding a new one in the range excuses the change; striking without adding a new id does not.
  @ref crates/lemma/tests/dictum.rs#the_dictum_striking_the_id_and_adding_a_new_one_excuses_the_change
  @ref crates/lemma/tests/dictum.rs#striking_without_adding_a_new_id_does_not_excuse_the_change
- [AC-36] A dictum is not an area for check 1, so a dictum governing a path alone needs no delta.
  @ref crates/lemma/tests/dictum.rs#a_dictum_is_not_an_area_for_check_1
  @ref crates/lemma/tests/dictum.rs#a_dictum_governing_a_path_alone_needs_no_delta
- [AC-37] A hand edit to a dictum claim is not a check 4 error, because dictums are human-edited and never folded.
  @ref crates/lemma/tests/dictum.rs#a_hand_edit_to_a_dictum_claim_is_not_a_check_4_error
- [AC-38] An invalid dictum `governs` glob is flagged.
  @ref crates/lemma/tests/dictum.rs#an_invalid_dictum_governs_glob_is_flagged
- [AC-39] [intent] The validate hook's `types` list includes `dictum`.
- [AC-40] On the first touch of a path a dictum governs, the signal pointer includes the dictum, and is silent after.
  @ref crates/lemma/tests/dictum.rs#signal_points_at_the_dictum_governing_the_touched_path_on_first_touch
