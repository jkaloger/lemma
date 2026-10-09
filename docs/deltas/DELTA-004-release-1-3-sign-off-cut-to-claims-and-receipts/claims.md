- [AC-1] [intent] A human moving a delta `in-progress` to `complete` approves it. There is no `approved-by` field; lazyspec's `reviewed: <sha>` stamp records where.
- [AC-2] Stop reports an error when a delta whose status at HEAD is not `complete` is `complete` in the working tree.
  @ref crates/lemma/tests/stop.rs#working_tree_complete_over_head_in_progress_errors
- [AC-3] Stop reports nothing for a delta already `complete` at HEAD.
  @ref crates/lemma/tests/stop.rs#a_delta_already_complete_at_head_passes
- [AC-4] [intent] A commit made inside the session escapes the working-tree `complete` check, as it escaped the `approved-by` check.
- [AC-5] [intent] Contract mutation no longer invalidates anything; it stays a validate finding.
- [AC-6] [intent] A `checked` line naming a name and sha under a claim is a receipt. It is fresh when nothing under the governing spec's `governs` changed since `<sha>`, and it satisfies the gate.
- [AC-7] [intent] `lemma receipt` does not write a `checked` line; humans do. `re-pin` leaves it alone; only a human re-dates it. It replaces `verified-by`.
- [AC-8] [intent] `waived:` is dropped. A claim that should not be machine-checked is `[intent]`.
- [AC-9] [intent] The delta lifecycle is `draft`, `in-progress`, `complete`. `archived` and the `complete`-to-`archived` dispatch are removed.
- [AC-10] [intent] The `in-progress` to `complete` gate checks receipts and, when clean, returns the fold updates in the same response.
- [AC-11] [intent] A gate error returns no fold updates. Moving to `complete` by hand is a no-op.
- [AC-12] [intent] The by-hand dry-run at `in-progress` reports gate findings plus the `would fold` preview and writes nothing.
