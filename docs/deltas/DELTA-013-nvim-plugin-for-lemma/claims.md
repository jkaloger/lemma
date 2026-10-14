<!-- One bullet per behaviour, in plain words: `- An empty account has balance zero.` `lemma review` assigns ids (`- [AC-1] ...`). Mark a behaviour no test can settle `[intent]` after its id. -->

- [AC-1] `lemma refs --json <file>` lists every `@ref`, `@per` and doc id in the doc with kind, line and column span, raw text and status.
  @ref crates/lemma/tests/refs.rs#json_lists_every_ref_and_per_with_span_text_and_status
- [AC-2] A ref's status in `lemma refs --json` is the diagnostic `evidence::resolve` gives, the same text lint reports for that ref when the doc is in lint's base range.
  @ref crates/lemma/tests/refs.rs#json_ref_status_is_the_diagnostic_lint_reports
- [AC-3] A resolving ref with a symbol targets the symbol's first line; without a symbol, line 1.
  @ref crates/lemma/tests/refs.rs#json_target_is_the_symbols_first_line_or_line_one_without_a_symbol
- [AC-4] A doc id is a configured type prefix, `-` and digits, outside `[...]`; rule ids and claim ids are not listed as doc ids.
  @ref crates/lemma/tests/refs.rs#json_lists_doc_ids_with_a_configured_prefix_outside_brackets_only
  @ref crates/lemma/tests/refs.rs#json_doc_id_span_is_in_bytes_after_a_multibyte_character
- [AC-5] A doc id targets that doc's `index.md` line 1; a configured prefix with no such doc has status `unknown`.
  @ref crates/lemma/tests/refs.rs#json_doc_id_targets_its_index_line_one_or_is_unknown
- [AC-6] `lemma refs --json` exits 0 when it prints the listing, whatever the entries' status, and exits 2 on a missing file or a file outside the lazyspec root.
  @ref crates/lemma/tests/refs.rs#json_exits_zero_whatever_the_status_and_two_on_bad_input
- [AC-7] `lemma refs <claim>` output is unchanged.
  @ref crates/lemma/tests/refs.rs#each_ref_gets_its_own_status_and_any_failure_exits_two
- [AC-8] In a markdown buffer under a lazyspec root, claim ids, struck ids, directives and their targets, `[intent]`/`[proposed]` markers, `> review:` lines and doc ids with a pack prefix get their `Lemma*` highlight group.
- [AC-9] Markdown buffers outside a lazyspec root get no lemma highlights.
- [AC-10] Highlighting works with no `lemma` binary on `PATH`.
- [AC-11] Every `Lemma*` group is a default link to a standard group, so a user's highlight overrides it.
- [AC-12] Highlights follow edits to the buffer.
- [AC-13] `<Plug>(lemma-jump)` on a resolving ref opens the target file at the target line.
- [AC-14] `<Plug>(lemma-jump)` on a doc id opens that doc.
- [AC-15] `<Plug>(lemma-jump)` on an unresolved ref notifies lemma's diagnostic and does not move.
- [AC-16] `<Plug>(lemma-jump)` with no ref or doc id under the cursor runs builtin `gd` and does not call `lemma`, with or without the binary on `PATH`.
- [AC-17] `<Plug>(lemma-jump)` on a ref in a modified buffer warns to save first and does not move.
- [AC-18] With no `lemma` on `PATH`, the first jump on a ref or doc id notifies once and later jumps on refs or doc ids do nothing.
- [AC-19] When `lemma --version` differs from the plugin version, the first jump warns once and still jumps.
- [AC-20] The plugin installs with lazy.nvim from this repository and the README shows the spec. [intent]
- [AC-21] The pack's four parts carry one version. [intent]
- [AC-22] `<Plug>(lemma-jump)` on a doc id whose status is `unknown` notifies lemma's diagnostic and does not move.
