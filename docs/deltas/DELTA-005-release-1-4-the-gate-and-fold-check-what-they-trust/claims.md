- [AC-1] [intent] The `in-progress` to `complete` gate errors when any receipt file, or any file under a target's `governs`, has uncommitted changes. A dirty ungoverned file passes; Stop still owns ungoverned drift.
- [AC-2] [intent] A delta claim that replaces or strikes a spec claim carries a base-hash line, the hash of the spec claim's block (text and receipts, not the id line's indentation) when the delta claim was written. `draft` to `in-progress` errors on a replace or strike without one.
- [AC-3] [intent] The fold errors when the spec's current block hash differs from the base hash, naming both hashes. With a matching base the fold replaces the claim.
- [AC-4] [intent] A delta claim whose id already exists in the spec and has no base-hash line is an error, so an accidental id collision cannot overwrite.
- [AC-5] [intent] Two in-flight deltas touching one claim id stay a warning.
- [AC-6] A ref that does not resolve is an error.
  @ref crates/lemma/tests/lint.rs#an_unresolved_ref_is_always_an_error
- [AC-7] [intent] Validate reports these as errors: claim without receipt on an in-progress delta; fold target does not resolve; part/target mismatch; claim in `index.md`; replace without a base-hash line.
- [AC-8] [intent] Validate reports these as warnings: receipt stale; base-hash mismatch on a draft; intent count; two in-flight deltas on one id.
- [AC-9] [intent] The CI template keeps `lazyspec validate --warnings` and fails on the error set, for example an unreceipted in-progress claim.
- [AC-10] [intent] `lemma re-pin <id> [claim-id ...]` re-pins the named claims, or every claim when none are named.
- [AC-11] [intent] `re-pin` refuses (exit 2, nothing written) a receipt whose file changed between the old sha and HEAD while that claim's block is unchanged, unless `--force` is passed, which writes and emits the warning.
- [AC-12] [intent] The re-pin comparison is per claim: a receipt file change counts as reviewed when the claim's text lines (block minus receipt, `checked` and base-hash lines) changed in a commit that also touched the receipt file, or are edited in the working tree.
- [AC-13] [intent] `re-pin` refuses a receipt file with uncommitted changes outright, `--force` or not.
- [AC-14] A ref into production code, a path outside the governing spec's `tests` globs, is an error and not a receipt.
  @ref crates/lemma/tests/lint.rs#a_ref_into_production_code_is_always_an_error
- [AC-15] A spec's own `tests` globs decide which ref paths count as tests, in place of the default list.
  @ref crates/lemma/tests/lint.rs#the_changed_specs_tests_globs_decide
- [AC-16] [intent] `tests` is a spec frontmatter key beside `governs` defaulting to `**/tests/**`, `**/*_test.*`, `**/*.test.*`, `**/*.spec.*`. `lemma receipt` refuses a path outside it.
- [AC-17] [intent] `lemma base <delta> [claim-id ...]` writes a base-hash line as an indented line under each named claim, or every claim whose id exists in its target spec when none are named, from the spec's current `behaviour` block, replacing an existing base-hash line.
- [AC-18] [intent] `lemma base` refuses (exit 2, nothing written) on an unknown claim id, a claim whose id the target spec lacks, or an unresolved target.
- [AC-19] [intent] The base hash is the first 12 hex chars of SHA-256 over the spec claim's block: each line right-trimmed, the id line left-trimmed, base-hash lines excluded, joined by `\n`.
- [AC-20] [intent] `lemma receipt` inserts a base-hash line only when the claim's id exists in the spec and none is present.
- [AC-21] [intent] The fold strips base-hash lines from a claim before writing it into the spec.
- [AC-22] [intent] A struck claim whose id the spec lacks is an error at `draft` to `in-progress` and in validate.
- [AC-23] [intent] A claim carrying a base-hash line whose id the spec lacks is drift: a validate warning and a fold error.
- [AC-24] An invalid `governs` or `tests` glob is an error finding.
  @ref crates/lemma/tests/lint.rs#an_invalid_glob_is_an_error
