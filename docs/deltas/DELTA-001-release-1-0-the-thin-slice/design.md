Scope in: pack with `spec`, `delta`, `decision` types and templates, `testing` and `code-change` dictums, a `[[hooks]]` table. Binary: `validate`, `pre-transition` (`draft`→`in-progress`, `in-progress`→`complete`, `complete`→`archived` fold), `stop`, `re-pin`. Plugin: `entry`, `delta` skills and a `Stop` hook entry. CI: a workflow building and testing the binary, and an adopter workflow template running `lazyspec validate`. Lazyspec contract notes live in `.scratch/lazyspec-contract.md` (local). README states behaviour; the decisions below fix the gaps it leaves open.

**1. Claim grammar.** Inside a `## Behaviour` section, a claim is a list item `- [ID] text`; ID matches `[A-Z][A-Z0-9]*(-[A-Z0-9]+)+`. Indented lines that follow belong to the claim (receipts, later `verified-by:`, `waived:`). `[intent]` anywhere after the id marks intent. `- ~~[ID]~~ ...` is a strike (remove at fold). `### <spec-id>` subheadings partition a multi-target delta.

**2. Fold target link.** Relationship `folds-into` (inverse `folded-from`) from delta to spec. Decision → spec: `affects`. Decision → decision: `supersedes`.

**3. Spec lifecycle.** `draft` → `active`. README lists `active` only, but DECISION-007 (RFC-001 §5.4) creates new specs at draft, so both states exist.

**4. Receipt.** A receipt is an indented `@ref` line under the claim, naming `<path>#<symbol>@<sha>`. It resolves when the sha is a commit, the file exists in the working tree, and the symbol is found. `.rs`/`.ts`/`.tsx` go through the lazyspec tree-sitter extractors; any other file uses a word-boundary text match.

**5. Freshness.** Stale when `git diff --name-only <sha> HEAD` names the test file or any file under the governing spec's `governs`. This counts committed history only: a receipt pinned at HEAD is fresh until the next commit touches its area. Uncommitted edits are a Stop concern, not a receipt concern. The governing spec for a delta claim is its partition's spec, or the single fold target. For a claim in a spec, it is that spec.

**6. One pre-transition hook.** A single `[[hooks]]` entry, `types=["delta"]`, `context_types=["spec","delta"]`. The binary dispatches on `transition.from/to`. Unknown transitions return no findings.

**7. Fold writes.** One update per target spec: the body with a `## Behaviour` section created if absent, claims appended/replaced/deleted by id, the hash echoed from the context document.

**8. Stop scope.** The session diff is `git diff --name-only HEAD` plus untracked files (`git ls-files --others --exclude-standard`). Areas: for each changed file, the set of spec/dictum ids that govern it (ungoverned files are ignored). The diff is bounded when one id is common to every governed file, or when the governed files all fall under the `governs` of in-progress deltas' fold targets. Pending tests: `it.todo(`, `test.todo(`, `describe.todo(`, a bare `#[ignore]` (not `#[ignore = "..."]`) in touched files. If `stop_hook_active` is true on stdin, exit 0 (the escape). An error goes to stderr with exit 2.

**9. re-pin.** `lemma re-pin <id>` loads the store, finds the doc, and rewrites every receipt sha to the short HEAD sha. It refuses (exit 2, nothing written) if any symbol no longer resolves. It flags with a warning, exit 0, when a receipt file changed between the old sha and HEAD while the doc's claim lines at the old sha (`git show <old>:<docpath>`) equal the current ones.

**10. Linking.** `lazyspec = "0.15"` from crates.io. Store/Config for `stop`/`re-pin`; symbols for resolution. `validate`/`pre-transition` read stdin, and load fold-target specs from the store when the payload lacks them (by-hand `lazyspec hook run` sends one document).

**11. Dictums.** `init --template` copies only templates and hooks. Dictums live at `.lazyspec/dictums/` in the pack; the `lemma:entry` setup step copies them from `.lazyspec/cache/config/.lazyspec/dictums/` when missing. Open lazyspec-boundary item.

**RFC-001 §13 release-1 acceptance.** Run in the hydra repository (W5 ran them against a scratch cargo repo standing in for hydra):

1. `lazyspec init --template` from this repository.
2. One evergreen spec for the reopen cascade, `Intent` plus `governs`.
3. A delta with two claims, one `[intent]`, built, with `@ref` receipts to real cargo test symbols.
4. `complete` blocked while a receipt is missing; passes the human gate once added; archive writes the spec lines by id.
5. A governed-code edit with no delta open shows the receipt stale and `re-pin` clears it; an edit across two `governs` areas is flagged at Stop.

Release 1 as RFC-001 §13 lists it: pack `spec`, `delta`, `decision` types and templates, `testing` and `code-change` dictums; binary validate, the `in-progress` → `complete` gate (receipt resolves and is fresh), the fold at `archived`, Stop with the cross-governs bound and the static pending-test read, `re-pin`, a CI job running validate; plugin `entry` and `delta`.

Later: DELTA-004 replaced the approval and receipt vocabulary (decisions 24-28, DELTA-004) and merged `archived` into `complete`. DELTA-005 added the clean-tree and fold-base checks. DELTA-007 removed `re-pin`, `validate`, `pre-transition`, receipt freshness and pending-test detection (decision 60, DELTA-007), and DELTA-009 removed the fold. DELTA-003 moved claims from `## Behaviour` sections to parts.
