<!-- How the change works, in prose. No `@ref` lines here; refs live under claims. -->

Two halves. Lemma grows a machine-readable listing; the plugin is a thin view over it plus lexical highlighting.

## `lemma refs --json <file>`

`lemma refs` today takes a claim id and prints plain text. Add a file form: given a doc path, emit JSON listing every ref and doc id in it. Each entry: kind (`ref`, `per`, `doc`), line and column span in the doc, raw text, status (`ok` or the same diagnostic lint gives), and target `{path, line}` when it resolves.

Status comes from `evidence::resolve`, unchanged, so DELTA-014's test-symbol rule and the governing spec's `tests` glob apply as in lint. Target line: the lazyspec extractor returns the symbol's source text, not a span; locate that text in the file and count lines. A ref with no symbol targets line 1.

**Doc ids.** A doc id is a configured type prefix from `.lazyspec.toml`, then `-`, then digits, not inside `[...]`. Pack prefixes: `SPEC`, `DELTA`, `DECISION`, `CONVENTION`, `DICTUM`. Rule ids (`TEST-1`) and claim ids (`[AC-n]`) are not doc ids. A doc id targets that doc's `index.md`, line 1. A configured prefix with no such doc is status `unknown`.

**Exit.** `--json` exits 0 whenever it prints the listing; entry status carries resolution, so unresolved refs do not change the exit code. Exit 2 only on bad input: file missing, or not under the lazyspec root.

**SPEC-001 amendment.** SPEC-001's constraint "Output is diagnostics" fits the checks, not `refs`, which already prints a listing. This delta scopes that constraint to checks and states `refs` prints a listing, plain or `--json`.

The claim-id form stays as is.

## Plugin (`nvim/`)

Layout: `nvim/plugin/lemma.lua` (setup, commands), `nvim/lua/lemma/{highlight,jump,version}.lua`, `nvim/tests/`. Install: lazy.nvim spec pointing at the lemma repo, appending `nvim/` to `rtp`. README gives the spec.

**Highlighting.** Applies to markdown buffers under a lazyspec root (nearest `.lazyspec.toml` upward). Lua match patterns via extmarks, recomputed on change, for: claim ids `[AC-n]`, struck ids `~~[AC-n]~~`, `@ref`/`@per`/`@review` keywords and their argument, `[intent]`/`[proposed]` markers, `> review:` lines, doc ids. Doc ids use the five pack prefixes, hardcoded: SPEC-004 fixes the pack at five types, and reading `.lazyspec.toml` from Lua is not worth a TOML parser. Each group (`LemmaClaimId`, `LemmaDirective`, `LemmaRefTarget`, `LemmaMarker`, `LemmaReview`, `LemmaDocId`, `LemmaStruck`) is defined `default` and links to a standard group, so colour schemes and user overrides win. No binary needed.

**Jump.** `<Plug>(lemma-jump)`, no default mapping; README suggests `gd` in lemma buffers. The plugin decides lexically, with the highlighter's patterns, whether the cursor is on a ref or doc id. If not, it runs builtin `normal! gd`, binary or no binary, so mapping `gd` costs nothing elsewhere and spawns no process. If so, it runs `lemma refs --json` on the buffer's file via `vim.system` (async), picks the entry under the cursor, opens target path at line. Unresolved: `vim.notify` with lemma's diagnostic. Saved file only; unsaved buffer warns "save first", since lemma reads disk.

**Version.** First jump on a ref or doc id checks `lemma --version` once per session. Missing binary: notify once; later jumps on refs do nothing, other `gd` still falls through; highlighting unaffected. Version differs from the plugin's: warn once, carry on.

## Pack (SPEC-004)

Pack becomes four parts: binary, Claude Code plugin, templates, nvim plugin. One version across all four, held by hand at release; no coherence check (SPEC-004 non-goal, DECISION-005). `nvim/lua/lemma/version.lua` holds the plugin's copy; SPEC-004 governs that file beside `version.rs` and `plugin.json` (chore).

## Tests

mini.test under `nvim --headless`, files in `nvim/tests/`. mini.test fetched into a gitignored dir by `nvim/tests/run.sh`. Highlighting tests set buffer text and assert extmark groups by range. Jump tests stub the `lemma` call with canned JSON and assert cursor/buffer; one end-to-end test runs the real binary against a fixture repo when on `PATH`. CI runs the script on a job that installs nvim.

Refs to Lua tests: lemma has no Lua extractor, so per DELTA-014 a ref passes on the `tests` glob alone. Refs name the mini.test case anyway, e.g. `nvim/tests/test_jump.lua#jumps_to_ref_target`, for the reader.

## Open questions

- Highlight via extmarks (works in any markdown, cost on huge files) or a treesitter injection query. Extmarks first; revisit if slow.
- Whether `lemma refs --json` should take buffer text on stdin so unsaved buffers jump. Not first cut.
