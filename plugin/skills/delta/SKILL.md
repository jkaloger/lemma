---
name: delta
description: Use when changing code in a lemma project - build, fix, chore, refactor or spike - or when resuming an in-progress delta. Drives the delta document from draft to review-ready with claims and test receipts.
---

# lemma delta

No unbounded code changes; no unverified completion. The delta is the unit of in-flight work: a set of claims, each settled by a test the receipt names.

Pick the workflow by judgement per `.lazyspec/dictums/code-change.md`, then load one reference: `references/build.md`, `fix.md`, `chore.md`, `refactor.md` or `spike.md`. Nothing classifies for you; misjudgement is caught by the project's own checks and the Stop hook's cross-`governs` flag.

## Vocabulary

- **Claim**: a list item in `## Behaviour`, `- [AC-<DELTA-ID>-<n>] text`. Ids are namespaced by the delta (`AC-DELTA-017-1`). `[intent]` after the id marks a claim no test can settle.
- **Receipt**: an `@ref <path>#<symbol>@<sha>` line indented under the claim, naming the test. It goes stale when the test file or code under the spec's `governs` changes after `<sha>`.
- **Fold**: at `archived`, a hook appends new claim ids to the spec, replaces existing ids, deletes struck ones (`- ~~[ID]~~ text`). You never write a spec's `Behaviour`.
- **Approval**: `approved-by:` on a delta is human-only. Never write it.

## Lifecycle

1. Create: `lazyspec create delta "<title>"`.
2. Link each target spec: `lazyspec link <delta> folds-into <spec>`. For a new capability, first create the spec at `draft` with `Intent` only (`lazyspec create spec`), then link it. Folding into several specs: partition `## Behaviour` under `### <spec-id>` subheadings.
3. Write `## Intent` and the claims. Keep claims observable and falsifiable. Restating a spec claim's id replaces it.
4. Advance `draft` to `in-progress` yourself: `lazyspec update <delta> --status in-progress`. Requires `Intent`, at least one claim, resolving fold targets. You may do this without asking.
5. For each claim: write the failing test, make it pass, commit test and implementation together, then `lemma receipt <delta-id> <claim-id> <path>#<symbol>`. It pins short HEAD and inserts the `@ref` line; never hand-edit `@ref` lines. It refuses on an unknown claim, an unresolved symbol or uncommitted changes in the file. If the test already exists and is committed at HEAD, run `lemma receipt` with no commit. Any later commit touching the test file or governed code stales the receipt; receipt after the last such commit (re-running `lemma receipt` for the same `<path>#<symbol>` updates the existing receipt in place rather than adding another), or `lemma re-pin <delta-id>` once your own later changes are done.
6. Run `lazyspec validate --warnings` (lemma findings are warnings, hidden without the flag); keep it clean. A receipt that does not resolve or has gone stale is yours to fix. To confirm review-readiness, `lazyspec hook run pre-transition <delta-id> --dry-run` checks the gate on the next edge and prints `checked in-progress → complete: ready` when clean. The `<n> of <m> claims are [intent]` and `checked … ready` warnings are informational: never remove `[intent]` or invent receipts to silence them.
7. Stop at review-ready: `in-progress`, every non-`[intent]` claim receipted, validate clean. Report that. Do not move to `complete`; that is the human gate. Pre-approval (`approved-by`) is release 2; never write one.
8. `complete` to `archived` is the mechanical fold. Do not hand-edit specs to imitate it.

## Drift

When governed code or a receipt's test changed outside your work, resolve the drift relevant to your claims first, then `lemma re-pin <delta-id>`. Never re-pin to silence a finding. `re-pin` refuses when a symbol no longer resolves and flags when a receipt file changed with no claim change. Receipts pin branch commits; after a squash merge or rebase they no longer resolve on the main history, so re-pin the spec after merging.

## Bound

A direct change stays inside one `governs` area. Crossing areas needs an in-progress delta covering them. Do not leave pending tests (`it.todo`, `#[ignore]` without a reason).
