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

# Plan: release 1.2

Specs and deltas become lazyspec bundles. Claims move out of `## Behaviour` sections into `behaviour` parts.

## Scope

In: spec and delta templates as directory templates; claim parsing, validate, transition, fold, receipt and re-pin read and write parts; README and skills.
Out: back-compat with flat spec/delta files. The only adopter is the hydra acceptance clone; it is re-created at W13.

## Decisions

18. **Spec bundle.** `.lazyspec/templates/spec/` holds `index.md` (frontmatter, `## Intent`) and `behaviour.md` (empty). Spec claims live only in part `behaviour`. The part is the section: no `## Behaviour` heading inside it. A claim-shaped item in a spec's `index.md` is a warning. The empty part ships with the template because lazyspec refuses hook updates to a part a document lacks.
19. **Delta bundle.** `.lazyspec/templates/delta/` holds `index.md` (frontmatter, `## Intent`). Claims go in one part per fold target, named `behaviour.<target-id>` (file `behaviour.<target-id>.md`). `tasks` stays an optional, unread part. `### <spec-id>` partitions are gone. Gate `draft`→`in-progress`: every fold target has its part, every `behaviour.*` part names a fold target, at least one claim across them. Validate reports the same mismatches: a missing part is a warning at `draft` and an error from `in-progress` on; an orphan part is an error.
20. **Fold writes a part.** One update per target: `{id, part: "behaviour", hash, body}`. The body is the merged claim list. A target with no `behaviour` part is an error finding and nothing is written.
21. **Finding locations.** Findings on claims carry `part` and the line in the part file. Part files have no frontmatter, so the offset is zero. Findings on `index.md` keep decision 12.
22. **Governing spec.** For a delta claim it is the target named by its part. For a spec claim it is the spec. Replaces the partition rule in decision 5.
23. **receipt and re-pin edit part files.** `lemma receipt` finds the claim id across the delta's `behaviour.*` parts and edits that file. `re-pin` rewrites receipts in every `behaviour*` part of the document. The re-pin flag compares each part's claim lines at the old sha.

## Work

- [x] W14 `payload`: `Doc.parts` (`name`, `body`); `Finding.part`; `Update.part`. Behaviour comes from parts, not body.
- [x] W15 `claims`/`validate`/`transition`: part-scoped parsing, decisions 18, 19, 21, 22. Tests: claim in spec index errors; target without part errors; orphan part errors; finding carries part and part-file line.
- [x] W16 `fold`: decision 20. Tests: update targets part `behaviour`; missing part errors with no updates; append, replace and strike per target from separate parts.
- [x] W17 `receipt`/`repin`: decision 23. Tests: receipt lands in the right part file; refusal when the claim is in no part; re-pin flag per part.
- [x] W18 pack: directory templates for spec and delta; remove flat `spec.md` and `delta.md`.
- [x] W19 docs and skills: README (Document types, Claims and receipts, Fold, Files), RFC-001 untouched, entry and delta skills say where claims go.
- [x] Review fixes: `receipt` matches only live claims and refuses ambiguity; store-loaded fold targets error instead of writing; index claim-shaped items warn; `missing_parts` only over resolved targets, warning at draft; `claim_end`/`Section`/`BEHAVIOUR_PART` tidy.

# Plan: release 1.3

Cuts the sign-off vocabulary to claims, receipts and `[intent]`. Starts after 1.2 lands.

## Scope

In: approval by transition, human-checked receipts, merged `complete`/`archived`, README type list.
Out: removing `waived:` and `verified-by` needs no code (both were release 2); README only.

## Decisions

24. **Approval is the transition.** `approved-by` is removed. A human moving a delta `in-progress`→`complete` approves it; lazyspec's `reviewed: <sha>` stamp records where. Stop reports an error when a delta whose status at HEAD is not `complete` is `complete` in the working tree. A commit made inside the session escapes this, as it escaped the `approved-by` check. Contract mutation no longer invalidates anything; it stays a validate finding.
25. **Human-checked receipt.** `@checked <name>@<sha>` under a claim is a receipt. Fresh when nothing under the governing spec's `governs` changed since `<sha>`. `lemma receipt` does not write it; humans do. `re-pin` leaves it alone; only a human re-dates it. Replaces `verified-by`.
26. **No waivers.** `waived:` is dropped. A claim that should not be machine-checked is `[intent]`.
27. **`complete` folds.** Delta lifecycle is `draft`→`in-progress`→`complete`. The `in-progress`→`complete` gate checks receipts and, when clean, returns the fold updates in the same response. `archived` and the `complete`→`archived` dispatch are removed. By-hand dry-run at `in-progress` reports gate findings plus the `would fold` preview, no write. Amends decisions 6, 7, 14.
28. **Five types.** README drops `bug` and `idea`. A bug is a draft delta whose first claim reproduces the defect. An idea is a draft spec with `Intent` only. Release 2 types are `convention` and `dictum`.
## Work

- [x] W20 `.lazyspec.toml`: delta lifecycle without `archived`. `transition`: decision 27. Tests: clean `in-progress`→`complete` returns fold updates; gate error returns none; `complete` by hand is a no-op.
- [x] W21 `claims`/`receipt`: `@checked` (decision 25). Tests: parsed as receipt; stale on governs change; satisfies the gate; `re-pin` rewrites its sha.
- [x] W22 `stop`: decision 24. Test: working-tree `complete` over HEAD `in-progress` errors; HEAD already `complete` passes.
- [x] W23 docs and skills: README (approval, receipts, statuses, types, Hooks binary, Status), delta skill (agent stops at review-ready, never moves to `complete`).
# Plan: release 1.4

Closes the gaps from the 2026-10-05 review: the gate and fold trust state they never checked, and validate cannot fail CI. Starts after 1.3 lands.

## Scope

In: clean tree at `complete`, fold base check, validate error severities, per-claim re-pin, test-path receipts, README ordering and trust model.
Out: test execution in lemma (receipts stay symbol resolution; the trust model is stated, not changed). Retrospective reconciliation of human edits: hydra interview first, release 2.

## Decisions

29. **Clean tree at `complete`.** The `in-progress`→`complete` gate errors when any receipt file or any file under a target's `governs` has uncommitted changes (`git status --porcelain` on those paths). Same rule `lemma receipt` applies in decision 16: the working tree the symbols resolved against must be the commit the pin names. Amends decision 5's "Stop concern" split: Stop still owns ungoverned drift.
30. **Fold checks the base.** A delta claim that replaces or strikes a spec claim carries `@base <hash>`, the hash of the spec claim's block (text and receipts, not the id line's indentation) at the time the delta claim was written. `lemma receipt` and the delta skill write it; `draft`→`in-progress` errors on a replace or strike without one. The fold errors when the spec's current block hash differs from `@base`, naming both. A delta claim whose id already exists in the spec and has no `@base` is an error, so an accidental id collision cannot overwrite. Replaces the "two in-flight deltas touching one claim id" warning, which stays as a warning. Amends decision 7 and 20.
31. **Validate severities.** Invariants are errors; drift is a warning. Errors: claim without receipt on an in-progress delta; receipt does not resolve; fold target does not resolve; part/target mismatch (decision 19); claim in `index.md` (decision 18); replace without `@base`. Warnings: receipt stale; `@base` mismatch on a draft; intent count; two in-flight deltas on one id. The CI template keeps `lazyspec validate --warnings` and now fails on the error set. Amends decision 13's note: drift is advisory per RFC §7.7, invariants are not.
32. **re-pin per claim.** `lemma re-pin <id> [claim-id ...]` re-pins the named claims, or every claim when none are named. Decision 9's flag becomes a refusal: a receipt whose file changed between the old sha and HEAD while that claim's block is unchanged is not re-pinned (exit 2, nothing written) unless `--force` is passed, which writes and emits the warning. The comparison is per claim: a receipt file change counts as reviewed when the claim's text lines (block minus `@ref`, `@checked` and `@base` lines) changed in a commit that also touched the receipt file, or are edited in the working tree. A receipt file with uncommitted changes is refused outright, `--force` or not. Amends decision 9 and 23.
33. **Receipts name tests.** `@ref` paths must match the governing spec's `tests` glob list, a new frontmatter key beside `governs` (default: `**/tests/**`, `**/*_test.*`, `**/*.test.*`, `**/*.spec.*`). A production symbol is not a receipt. `lemma receipt` refuses, validate and the gate error. README states the trust model in one paragraph: a receipt is a resolving symbol in a test file at a commit, not evidence the test ran or passed; the project's runner and the human reviewer own those.

34. **`lemma base <delta> [claim-id ...]`.** Decision 30 has the skill write `@base`, but an agent cannot compute a hash by hand, so a subcommand does it. For each named claim, or every claim whose id exists in its target spec when none are named, it writes `@base <hash>` as an indented line under the claim from the spec's current `behaviour` block, replacing an existing `@base`. It refuses (exit 2, nothing written) on an unknown claim id, a claim whose id the target spec lacks, or an unresolved target. `lemma receipt` inserts `@base` only when the claim's id exists in the spec and no `@base` is present. The hash is the first 12 hex chars of SHA-256 over the spec claim's block: each line right-trimmed, the id line left-trimmed, `@base` lines excluded, joined by `\n`. The fold strips `@base` lines from a claim before writing it into the spec. A struck claim whose id the spec lacks is an error at `draft`→`in-progress` and in validate. A claim carrying `@base` whose id the spec lacks is drift: a validate warning, a fold error.

## Work

- [x] W24 `transition`: decision 29. Tests: dirty governed file blocks `complete`; dirty ungoverned file passes; dirty receipt file blocks.
- [x] W25 `claims`/`fold`/`transition`/`receipt`: decision 30. Tests: `@base` parsed; replace without `@base` fails `draft`→`in-progress`; fold with matching base replaces; fold with changed base errors naming both hashes; new id colliding with existing spec id errors; strike with stale base errors.
- [x] W26 `validate` and `docs/ci/lemma-validate.yml`: decision 31. Tests: each error-set finding is severity error; stale receipt stays warning; CI job exits non-zero on an unreceipted in-progress claim.
- [x] W27 `repin`: decision 32. Tests: named claims only are rewritten; changed-file-unchanged-claim refuses with nothing written; `--force` writes and warns; per-claim comparison when a sibling claim changed.
- [x] W28 `receipt`/`validate`/`transition` and README: decision 33. Tests: production path refused by `lemma receipt`; default globs accept `tests/foo.rs` and `foo.test.ts`; spec-level `tests` overrides default. README: shipped behaviour first, release 2+ fenced in Status only, trust-model paragraph under Receipts.
- [x] Review fixes: one annotation-line predicate; NUL-separated dirty paths shared with `stop`; `git show <rev>:./<path>`; re-pin treats a working-tree claim edit as a change and refuses a dirty receipt file outright; struck unknown id wins over orphan base; `tests: []` in the spec template; `lemma receipt` applies a spec's own `tests` globs; invalid `governs`/`tests` globs are error findings; malformed `tests` attribute refuses; CI installs lazyspec so the real-CLI validate test runs.
