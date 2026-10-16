<!-- How the change works, in prose. No `@ref` lines here; refs live under claims. -->

## Part edits are restored (SPEC-002)

`reject_part_edits` stays the detector: compare each delta part after a runner or fix call against the snapshot taken before it. On a difference, lemma writes the snapshot back, prints an info step `runner edited <path>; restored`, and carries on to `write_refs` and the checks. No finding, no fix round. The snapshot is what lemma or the human left before the call, so human edits made during a stop survive and lemma's own ref writes come back.

This supersedes DELTA-015's "a delta part edit becomes a fix-round finding". Its test `a_delta_part_edit_becomes_a_fix_round_finding` changes, so build.rs drifts against DELTA-015; expected.

SPEC-002 constraint "The runner never edits a delta part" becomes: the runner never edits a delta part; lemma restores any part a runner call changes.

## Trailing markers are lint errors (SPEC-001)

In `shape::always`, a live claim whose first line ends in whitespace then `[intent]` or `[proposed]` is an error on that line: `<id>: [intent] goes after the id, not at the end`. End of line only, outside backticks, so mid-text mentions (DELTA-010 AC-25 "each [proposed] claim") stay clean. Applies to every delta lint covers; completed deltas outside the range are untouched.

Detection lives in `claims.rs` beside `split_markers` (e.g. `trailing_marker(first_line) -> Option<&str>`), so DELTA-016 deletes it with the grammar.

## Open questions

- Restore vs. a better finding: the fix prompt could carry the expected part diff instead. Restore chosen: lemma owns the parts, so no runner round is needed to get them right.
