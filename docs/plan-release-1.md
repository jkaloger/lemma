# Plan: release 1

Builds the thin slice in RFC-001 §13. README states behaviour; this file fixes the gaps README leaves open and lays out the work. Lazyspec contract notes: `.scratch/lazyspec-contract.md` (local).

## Scope

Pack: `spec`, `delta`, `decision` types and templates; `testing` and `code-change` dictums; `[[hooks]]` table.
Binary: `validate`, `pre-transition` (`draft`→`in-progress`, `in-progress`→`complete`, `complete`→`archived` fold), `stop`, `re-pin`.
Plugin: `entry`, `delta` skills; `Stop` hook entry.
CI: workflow building and testing the binary, and an adopter workflow template running `lazyspec validate`.

Out: everything README marks release 2+ (approved-by gate, verified-by, waivers, contract mutation, intent/waiver info count, idea/bug/convention/dictum types, `prompt` subcommand, other skills).

## Decisions filling README gaps

1. **Claim grammar.** Inside a `## Behaviour` section, a claim is a list item `- [ID] text`; ID matches `[A-Z][A-Z0-9]*(-[A-Z0-9]+)+`. Indented lines that follow belong to the claim (receipts, later `verified-by:`, `waived:`). `[intent]` anywhere after the id marks intent. `- ~~[ID]~~ ...` is a strike (remove at fold). `### <spec-id>` subheadings partition a multi-target delta.
2. **Fold target link.** Relationship `folds-into` (inverse `folded-from`) from delta to spec. Decision → spec: `affects`. Decision → decision: `supersedes`.
3. **Spec lifecycle.** `draft` → `active`. README lists `active` only, but §5.4 creates new specs at draft, so both states exist.
4. **Receipt.** `@ref <path>#<symbol>@<sha>` under the claim. It resolves when the sha is a commit, the file exists in the working tree, and the symbol is found. `.rs`/`.ts`/`.tsx` go through the lazyspec tree-sitter extractors; any other file uses a word-boundary text match.
5. **Freshness.** Stale when `git diff --name-only <sha> HEAD` names the test file or any file under the governing spec's `governs`. This counts committed history only: a receipt pinned at HEAD is fresh until the next commit touches its area. Uncommitted edits are a Stop concern, not a receipt concern. The governing spec for a delta claim is its partition's spec, or the single fold target. For a claim in a spec, it is that spec.
6. **One pre-transition hook.** A single `[[hooks]]` entry, `types=["delta"]`, `context_types=["spec","delta"]`. The binary dispatches on `transition.from/to`. Unknown transitions return no findings.
7. **Fold writes.** One update per target spec: the body with a `## Behaviour` section created if absent, claims appended/replaced/deleted by id, the hash echoed from the context document.
8. **Stop scope.** The session diff is `git diff --name-only HEAD` plus untracked files (`git ls-files --others --exclude-standard`). Areas: for each changed file, the set of spec/dictum ids that govern it (ungoverned files are ignored). The diff is bounded when one id is common to every governed file, or when the governed files all fall under the `governs` of in-progress deltas' fold targets. Pending tests: `it.todo(`, `test.todo(`, `describe.todo(`, a bare `#[ignore]` (not `#[ignore = "..."]`) in touched files. If `stop_hook_active` is true on stdin, exit 0 (the escape). An error goes to stderr with exit 2.
9. **re-pin.** `lemma re-pin <id>` loads the store, finds the doc, and rewrites every receipt sha to the short HEAD sha. It refuses (exit 2, nothing written) if any symbol no longer resolves. It flags with a warning, exit 0, when a receipt file changed between the old sha and HEAD while the doc's claim lines at the old sha (`git show <old>:<docpath>`) equal the current ones.
10. **Linking.** `lazyspec = "0.15"` from crates.io. Store/Config for `stop`/`re-pin`; symbols for resolution. `validate`/`pre-transition` read stdin, and load fold-target specs from the store when the payload lacks them (by-hand `lazyspec hook run` sends one document).
11. **Dictums.** `init --template` copies only templates and hooks. Dictums live at `.lazyspec/dictums/` in the pack; the `lemma:entry` setup step copies them from `.lazyspec/cache/config/.lazyspec/dictums/` when missing. Open lazyspec-boundary item.

## Work

- [x] W1 `crates/lemma`: `claims` (parse), `receipt` (resolve, fresh), `fold`, `validate`, `transition`, `stop`, `repin`, `main` (clap). Unit tests per module. Integration tests use a temp git repo.
- [x] W2 pack: `.lazyspec.toml`, `.lazyspec/templates/{spec,delta,decision}.md`, `.lazyspec/dictums/{testing,code-change}.md`.
- [x] W3 plugin: `.claude-plugin/marketplace.json`, `plugin/.claude-plugin/plugin.json`, `plugin/hooks/hooks.json`, `plugin/skills/{entry,delta}/`.
- [x] W4 CI: `.github/workflows/ci.yml`.
- [x] W5 acceptance: RFC §13 five steps run against a scratch cargo repo standing in for hydra.

# Plan: release 1.1

Fixes from hydra adoption (RFC §13 steps 1–3, 2026-10-04/05). Friction numbers below refer to that log. Lazyspec-side asks go to `.scratch/lazyspec-contract.md`.

## Scope

In: friction 5, 6, 9, 10, 11, 12, lemma side of 7 and 8.
Out, pending decision: 1 (convention/dictum types into release 1), 2 (rationale link to non-lazyspec doc). Revisit after acceptance steps 4–5 run in hydra.
Out, lazyspec: 1 (init copying pack documents), 3 (`{author}`), 7 (suppress `reviewed:` per type), 8 (`hook run --to`).
Deferred: 4 (report scope globs matching nothing).

## Decisions

12. **Finding lines are file lines.** Findings report the line in the file, not the body. Claim and receipt lines are offset by the frontmatter line count (closing `---` plus one). Body-relative lines stay internal to `claims`.
13. **Intent count.** `validate` emits one info-level finding per doc with `[intent]` claims: `<n> of <m> claims are [intent]`. Lazyspec findings have no `info` severity, so it goes out as `warning` until one exists. Struck claims excluded; archived deltas skipped. Agents treat it as informational, never "fix" it. Amends README: intent count moves from release 2 to 1.1 (waiver count stays release 2).
14. **By-hand pre-transition.** `lazyspec hook run pre-transition` sends `from == to`. lemma then evaluates the gate on the edge leaving `from` (`draft`→`in-progress`, `in-progress`→`complete`) and reports findings only, never updates. With nothing to report it emits a warning `checked <from> → <to>: ready`, so a clean run is distinguishable from no dispatch. `complete` by hand: archive gate findings plus one `would fold <n> claims into <spec>` warning per target, no write. Amends decision 6.
15. **`reviewed:` stamp.** README states that lazyspec writes `reviewed: <sha>` on status change, that it records the commit the doc was last moved at, and that it is not approval. Agents leave it alone like `approved-by`.
16. **`lemma receipt <delta> <claim-id> <path#symbol>`.** Resolves the symbol at the working tree, pins short HEAD, inserts `@ref` as the claim's first indented line. An existing receipt for the same `path#symbol` is rewritten in place instead. Refuses (exit 2, nothing written) when the claim id is missing, the symbol doesn't resolve, or the file has uncommitted changes (pin would lie). Shares resolve/write code with `repin`.
17. **Version check.** `stop`, `re-pin` and `receipt` check `lazyspec --version` ≥ 0.15 and exit 2 with the install hint otherwise. `stop` doesn't need the CLI, so there it prints the hint to stderr and exits 0. Tests put a fake `lazyspec` on PATH; no skip switch. Entry skill setup runs the same check first.

## Work

- [x] W6 `claims`/`validate`/`transition`: file-relative lines (decision 12). Test: finding line equals claim's line in the file.
- [x] W7 `validate`: intent count (decision 13). Test: doc with one intent claim of two gets one warning; doc with none gets none.
- [x] W8 `transition`: by-hand dispatch (decision 14). Tests: `in-progress`→`in-progress` with missing receipt errors; clean emits ready warning; `complete`→`complete` emits no updates.
- [x] W9 `receipt` subcommand (decision 16). Tests: inserts at HEAD; refuses unresolved symbol, unknown claim, dirty file.
- [x] W10 version check (decision 17), in `main` before store load.
- [x] W11 docs: README install (`claude plugin install lemma@lemma`, restart before delta work), `reviewed:` (decision 15), intent count move, `receipt` and by-hand dry-run usage.
- [x] W12 skills: delta step 5 adds "test already at HEAD: `lemma receipt …`, no commit"; step 5 uses `lemma receipt` over hand edits; entry setup runs version check and ends with restart note when plugin just installed.
- [ ] W13 acceptance: re-run hydra steps 1–3 against 1.1, then steps 4–5 (needs second spec, e.g. `src/store.rs`). Feed back into decisions on friction 1 and 2.
