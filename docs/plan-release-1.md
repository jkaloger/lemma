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
