- [AC-1] [intent] The re-pin refusal compares the receipt's test symbol, not its file: a pin has drifted when the symbol's extracted text at the pin commit differs from its text at HEAD while the claim's text lines did not change.
- [AC-2] [intent] `.rs` and `.ts`/`.tsx` files use the lazyspec extractors; a file with no extractor compares whole-file. A symbol absent at the pin counts as changed.
- [AC-3] [intent] A sibling test added to the receipt's file, or a doc-comment edit to it, re-pins without `--force`; a changed test body under an unchanged claim refuses, so `--force` is only for that case.
- [AC-4] [intent] `re-pin --force` writes ` [forced]` after the sha on the receipt line.
- [AC-5] [intent] `lemma validate` warns on every forced receipt of an in-progress delta or a spec: `<claim>: receipt was force re-pinned; review the test against the claim, then remove [forced] or add a checked line`.
- [AC-6] [intent] The `in-progress` to `complete` gate and its dry-run report the same forced-receipt warning, and the fold carries the marker into the spec.
- [AC-7] [intent] `re-pin` and `lemma receipt` preserve the `[forced]` marker when they rewrite the line. Only a hand edit removes it.
- [AC-8] [intent] `lemma receipt` refuses a document that is not a delta: spec receipts arrive by fold and humans write the `checked` line.
- [AC-9] [intent] When the claim already has a receipt for the same `<path>#<symbol>`, `lemma receipt` refuses if the pin has drifted, naming `lemma re-pin <delta> <claim-id> --force`.
- [AC-10] [intent] `receipt`, `re-pin` and `base` add `"wrote": [{"id", "part", "line", "text"}]` to their JSON output, one entry per line written or rewritten, omitted when empty. Hook commands are unchanged.
- [AC-11] [intent] `lemma validate` reports `claim has no receipt` on an in-progress delta as a warning. The `complete` gate keeps it an error.
- [AC-12] [intent] The "two in-flight deltas touching one claim id" warning counts `draft` and `in-progress` deltas.
- [AC-13] [intent] The by-hand `in-progress` dry-run adds a warning per uncommitted file of the delta bundle: `<file> has uncommitted changes; commit the delta before asking for review`. `checked ... ready` still follows when there is no error.
- [AC-14] [intent] `draft` means the spec's Intent is proposed and no fold has been accepted; `active` means it describes real behaviour. A human moves it. The `in-progress` to `complete` gate and its dry-run warn for each draft target: `<spec> is draft; move it to active once this fold lands`.
- [AC-15] `lemma session-start` records HEAD's sha at `<git-dir>/lemma/sessions/<session_id>` unless the file exists, so a resume keeps the original base.
  @ref crates/lemma/tests/stop.rs#records_the_base_once
- [AC-16] `lemma stop` takes the files a session touched from the session base: commits made since the base, tracked working-tree changes and untracked files.
  @ref crates/lemma/tests/stop.rs#the_session_base_brings_commits_made_in_the_session_into_range
  @ref crates/lemma/tests/stop.rs#uncommitted_and_untracked_files_count
- [AC-17] With no base file for its session, `lemma stop` diffs against HEAD as before.
  @ref crates/lemma/tests/stop.rs#committed_changes_before_the_session_are_out_of_range
- [AC-18] [intent] The "complete in the working tree but not at HEAD" check keeps comparing against HEAD, so a human who completes and commits mid-session is not flagged on every later Stop.
- [AC-19] [intent] A delta that became `complete` since the session base counts as covering its specs in the cross-`governs` check.
- [AC-20] [intent] The plugin has a `SessionStart` hook running `lemma session-start`, which reads `session_id` and `cwd` from stdin.
- [AC-21] [intent] The pack ships `references/adopt.md` and dictum rule `CHG-10`: the first delta into a spec whose `behaviour` part is empty, over code that already has tests. Claims state what existing tests settle, receipts name existing symbols, a characterisation test is added only for a claim with none, and there is no production change.
- [AC-22] [intent] Crate and plugin carry the README's release number, `1.5.0`, and `lemma --version` prints it.
