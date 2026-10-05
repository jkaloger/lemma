# Plan: release 2.0

Implements DECISION-001: lemma is one signal source in a stack. It checks that implementation changes and spec changes stay mechanically connected, and nothing else. Starts from 1.5 (`6a43299`).

## Scope

In: unpinned refs, immutable claims, checks 1–4, `check`, `apply`, `refs`, `stop`, `signal` (PostToolUse), context pointers, once-per-session dedupe, `migrate`, base resolution for branches and trunk-only repos, pack hooks as composition, skills, README, CI template.
Out: extractor fixes (upstream to lazyspec, `.scratch/lazyspec-contract.md`; lemma uses the 0.15 extractors as they are). jev and test-report ingestion (future signal sources). Pending-test detection (moves to repo conventions).

## Decisions

48. **Refs are unpinned.** A receipt is `@ref <path>#<symbol>`. A pinned form (`@<sha>`, `@{blob:…}`) or ` [forced]` is an error: `pinned ref; run lemma migrate`. `@base` lines are an error with the same hint.
49. **`@checked` goes.** A claim no test settles is `[intent]`. Human judgement of adequacy happens at `complete`, not in a line. `migrate` turns a claim whose only receipts are `@checked` into `[intent]`.
50. **Claims are immutable.** A delta claim whose id exists in the target spec is an error unless struck: `claims are immutable; strike <id> and add a new id`. Fold appends new ids and deletes struck ids. A struck id the spec lacks is an error at `apply` and in `check` (check 4). Two in-flight deltas striking one id is a warning.
51. **The four checks.**
    1. Coverage: when the range's changed governed files span more than one spec's `governs`, each touched spec that no in-flight delta (draft or in-progress) folds into is an error. One area alone needs no delta (chores stay free). Amends DECISION-001 check 1.
    2. Evidence: for each claim present in a spec's `behaviour` part at the base, each `@ref` region whose extracted, normalised text differs between base and working tree is an error unless an in-flight delta strikes that claim. A region absent at base counts as changed. Files with no extractor compare whole.
    3. Claims have evidence: every live, non-`[intent]` claim has at least one `@ref` that resolves to a symbol in a file matching the governing spec's `tests` globs.
    4. Integrity: decision 50, plus the 1.5 structural findings kept (fold targets resolve, parts name targets, no claim in `index.md`, invalid globs).
52. **Base resolution.** `--base <rev>` wins. Else, with trunk `main` (`--trunk <name>`), if `merge-base HEAD main` is not HEAD, use it. Else HEAD is on trunk (direct-to-main): Stop and `signal` use the session base; the gate uses the parent of the commit that first added the delta's `index.md`, or HEAD when the delta is uncommitted. Else HEAD. CI passes `--base` explicitly: the PR base ref, or the push's `before` sha.
53. **`lemma check`.** Runs checks 1–4. Severity: invariants (2, 4, refs that do not resolve, pinned refs) are errors; a claim without evidence (3) is a warning unless required. `--require <claims|evidence|clean>` (repeatable) turns readiness into errors: `claims` = Intent present, every fold target has its part, at least one claim; `evidence` = check 3 errors; `clean` = no uncommitted changes in ref files or under a target's `governs`. Output is `path:line: severity: message` lines, silent when clean, exit 2 on error. `--json` prints `{"findings": […]}`. `--hook` reads a lazyspec payload on stdin, prints findings JSON, exits 0, scoped to the payload documents.
54. **Transitions are config.** The pack's `[[hooks]]`: `check --hook` on validate; `check --hook --require claims` on draft→in-progress; `check --hook --require evidence --require clean` then `apply` on in-progress→complete. Lazyspec runs them in order and stops at the first error, so `apply` never writes after a failed check. `lemma pre-transition` and `lemma validate` go.
55. **`lemma apply`.** With no argument it is the pre-transition hook: reads the payload, folds each target, returns `updates`; errors on check 4 and on a target with no `behaviour` part; warns for a draft target. `lemma apply <delta>` previews: prints per target the ids it would add and delete, writes nothing.
56. **`lemma refs <claim>`.** Prints `<claim> <path>#<symbol> ok|unresolved|not-a-test` per ref, across specs and in-flight deltas.
57. **`lemma stop`.** Claude Code Stop adapter: checks 1–3 against the session base (decision 52), plus the 1.5 rule that a delta reaching `complete` in the working tree but not at HEAD is an error. Exit 2 with diagnostics on stderr on error. `stop_hook_active` exits 0. Pending-test detection goes.
58. **`lemma signal`.** Claude Code PostToolUse adapter for Edit, Write and MultiEdit. Scoped to the touched file: context pointers on the first touch of a governed area in the session (one line: `<path> → SPEC-003 store · DECISION-004 …`, from `governing()` and `affects` links, no content); check 2 for refs into the file; check 1 if the file adds an uncovered area; check 3 and 4 when the file is a spec or delta part. Output goes back to the agent as `additionalContext`; never blocks. Each message is said once per session: hashes kept in `<git-dir>/lemma/sessions/<id>.said`.
59. **`lemma migrate`.** Rewrites every spec and delta part in place: strips the pin and ` [forced]` from `@ref` lines, deletes `@base` lines, applies decision 49. Prints the files it wrote. Idempotent.
60. **Removed.** `re-pin`, `receipt`, `base`, `validate`, `pre-transition`; receipt freshness, symbol drift, `[forced]`, `@checked`, `@base`, pending-test detection. `session-start` stays.
61. **Version 2.0.0** in crate, plugin and README.

## Work

- [ ] W38 `claims`/`fold`: decisions 48–50. Parser keeps `@ref path#symbol`; pinned refs, `[forced]`, `@base`, `@checked` parse as findings. Fold: append new, delete struck, error on unstruck existing id and on struck absent id. Tests: each.
- [ ] W39 `evidence` (from `receipt`): resolve a ref against `tests` globs; extract the normalised region at a rev (`git show <rev>:<path>`) and in the working tree; whole-file fallback; `changed(base)`. Tests: sibling test added is unchanged; body edit is changed; absent at base is changed; non-extractor file compares whole.
- [ ] W40 `range`: decision 52, including the trunk-only paths. Tests: branch uses merge-base; on trunk with a session base uses it; gate on trunk uses the delta's first commit parent; uncommitted delta uses HEAD; `--base` wins.
- [ ] W41 `check`: decisions 51, 53. Output modes, `--require`. Tests: one per check and per requirement; silent when clean; exit codes.
- [ ] W42 `apply` and `refs`: decisions 55, 56. Tests: hook updates echo the content hash; preview writes nothing; refs statuses.
- [ ] W43 `stop` and `signal`: decisions 57, 58. Tests: Stop on a branch and on trunk; signal context once per session; signal silent on a clean edit; evidence signal names the claim.
- [ ] W44 `migrate`: decision 59. Tests: pinned, forced, base, checked-only fixtures; idempotent.
- [ ] W45 removals and `main.rs`: decision 60. Delete `repin.rs`, `attach.rs`, `transition.rs`, `validate.rs`, the old `base` command and their tests; keep shared helpers.
- [ ] W46 pack and plugin: `.lazyspec.toml` hooks (decision 54), templates, dictums, `plugin/hooks/hooks.json` (PostToolUse), delta and entry skills and every reference (write `@ref`, run `lemma check`, strike to change a claim, no re-pin), CI template with `--base`, README rewrite, version 2.0.0.
- [ ] W47 review: whole-diff review against DECISION-001 and this plan.
- [ ] W48 acceptance (human): hydra on 2.0 after `lemma migrate`; compare refusals and token cost against the 1.5 W37 baseline.
