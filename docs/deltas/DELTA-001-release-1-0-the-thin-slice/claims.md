- [AC-1] [intent] A claim in a `## Behaviour` section is a list item `- [ID] text` where the ID matches `[A-Z][A-Z0-9]*(-[A-Z0-9]+)+`; indented lines after it belong to the claim.
- [AC-2] [intent] `[intent]` after the id marks an intent claim, `- ~~[ID]~~ ...` is a strike removed at fold, and `### <spec-id>` subheadings partition a multi-target delta.
- [AC-3] [intent] A delta links to the spec it folds into with `folds-into` (inverse `folded-from`); a decision links to a spec with `affects` and to a decision with `supersedes`.
- [AC-4] [intent] A spec is `draft` or `active`; new specs start at draft.
- [AC-5] [intent] A receipt line naming path, symbol and sha resolves when the sha is a commit, the file exists in the working tree and the symbol is found; `.rs`, `.ts` and `.tsx` use the lazyspec tree-sitter extractors and any other file a word-boundary text match.
- [AC-6] [intent] A receipt is stale when `git diff --name-only <sha> HEAD` names the test file or any file under the governing spec's `governs`; uncommitted edits are not a receipt concern.
- [AC-7] [intent] The governing spec of a delta claim is its partition's spec or the single fold target; for a spec claim it is that spec.
- [AC-8] [intent] One pre-transition hook entry dispatches on the transition; an unknown transition returns no findings.
- [AC-9] [intent] `draft` to `in-progress` and `in-progress` to `complete` are gated, and `complete` is blocked while a claim has no resolving, fresh receipt.
- [AC-10] [intent] Archiving a delta writes one update per target spec: a `## Behaviour` section created if absent, claims appended, replaced or deleted by id, the hash echoed from the context document.
- [AC-11] Stop counts uncommitted and untracked files when it works out which areas a session touched.
  @ref crates/lemma/tests/stop.rs#uncommitted_and_untracked_files_count
- [AC-12] A change across two governed areas with no delta covering both is an error at Stop, printed to stderr with exit 2.
  @ref crates/lemma/tests/stop.rs#committed_changes_across_two_areas_are_errors_in_check_format
- [AC-13] An in-flight delta whose fold targets cover both areas bounds the change, so Stop passes.
  @ref crates/lemma/tests/stop.rs#an_in_flight_delta_covering_both_specs_bounds_the_change
- [AC-14] [intent] A change is also bounded when one spec or dictum id governs every changed governed file; ungoverned files are ignored.
- [AC-15] [intent] Stop flags pending tests in touched files: `it.todo(`, `test.todo(`, `describe.todo(`, and a bare `#[ignore]`.
- [AC-16] Stop exits 0 without checking when `stop_hook_active` is true.
  @ref crates/lemma/tests/stop.rs#stop_hook_active_exits_zero_without_checking
- [AC-17] [intent] `lemma re-pin <id>` rewrites every receipt sha in the document to the short HEAD sha.
- [AC-18] [intent] `lemma re-pin` refuses with exit 2 and writes nothing when any receipt symbol no longer resolves.
- [AC-19] [intent] `lemma re-pin` warns with exit 0 when a receipt file changed between the old sha and HEAD while the claim lines at the old sha equal the current ones.
- [AC-20] [intent] `validate` and `pre-transition` read stdin and load fold-target specs from the store when the payload lacks them.
- [AC-21] [intent] The pack ships the `spec`, `delta` and `decision` types with templates, the `testing` and `code-change` dictums and a `[[hooks]]` table; `init --template` copies only templates and hooks, so the entry skill copies the dictums when missing.
- [AC-22] [intent] The plugin ships the `entry` and `delta` skills and a `Stop` hook entry.
- [AC-23] [intent] CI builds and tests the binary, and an adopter workflow template runs `lazyspec validate`.
- [AC-24] [intent] The five RFC-001 §13 acceptance steps run end to end against a scratch cargo repo standing in for hydra.
