Scope in: hydra adoption friction 5, 6, 9, 10, 11, 12, and the lemma side of 7 and 8 (RFC-001 §13 steps 1-3, run 2026-10-04/05; friction numbers refer to that log). Lazyspec-side asks go to `.scratch/lazyspec-contract.md`.

**12. Finding lines are file lines.** Findings report the line in the file, not the body. Claim and receipt lines are offset by the frontmatter line count (closing `---` plus one). Body-relative lines stay internal to `claims`.

**13. Intent count.** `validate` emits one info-level finding per doc with `[intent]` claims: `<n> of <m> claims are [intent]`. Lazyspec findings have no `info` severity, so it goes out as `warning` until one exists. Struck claims excluded; archived deltas skipped. Agents treat it as informational, never "fix" it. Amends README: intent count moves from release 2 to 1.1 (waiver count stays release 2).

**14. By-hand pre-transition.** `lazyspec hook run pre-transition` sends `from == to`. lemma then evaluates the gate on the edge leaving `from` (`draft`→`in-progress`, `in-progress`→`complete`) and reports findings only, never updates. With nothing to report it emits a warning `checked <from> → <to>: ready`, so a clean run is distinguishable from no dispatch. `complete` by hand: archive gate findings plus one `would fold <n> claims into <spec>` warning per target, no write. Amends decision 6 (DELTA-001).

**15. `reviewed:` stamp.** README states that lazyspec writes `reviewed: <sha>` on status change, that it records the commit the doc was last moved at, and that it is not approval. Agents leave it alone like `approved-by`.

**16. `lemma receipt <delta> <claim-id> <path#symbol>`.** Resolves the symbol at the working tree, pins short HEAD, inserts `@ref` as the claim's first indented line. An existing receipt for the same `path#symbol` is rewritten in place instead. Refuses (exit 2, nothing written) when the claim id is missing, the symbol doesn't resolve, or the file has uncommitted changes (pin would lie). Shares resolve/write code with `repin`.

**17. Version check.** `stop`, `re-pin` and `receipt` check `lazyspec --version` ≥ 0.15 and exit 2 with the install hint otherwise. `stop` doesn't need the CLI, so there it prints the hint to stderr and exits 0. Tests put a fake `lazyspec` on PATH; no skip switch. Entry skill setup runs the same check first.

Later: DELTA-004 removed waivers and `verified-by` and merged the archive dry-run into `complete` (decision 27, amending 14, DELTA-004). DELTA-006 made `lemma receipt` delta-only and drift-aware (decision 37, DELTA-006), added written-lines output (38) and the dry-run uncommitted-file warning (41), and put the rationale link on provenance (46, settling friction 2). DELTA-007 removed `receipt`, `re-pin`, `validate` and `pre-transition`, so the by-hand dry-run, the intent count in `validate` and receipt pinning no longer exist; `lemma check` and `lemma apply` replaced them.
