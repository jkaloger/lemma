---
name: delta
description: Use when changing code in a lemma project - build, fix, chore, refactor or spike - or when resuming an in-progress delta. Drives the delta document from draft to review-ready with claims and test receipts.
---

# lemma delta

No unbounded code changes; no unverified completion. The delta is the unit of in-flight work: a set of claims, each settled by a test the receipt names.

Pick the workflow by judgement per `.lazyspec/dictums/code-change.md`, then load one reference: `references/build.md`, `fix.md`, `chore.md`, `refactor.md` or `spike.md`. Nothing classifies for you; misjudgement is caught by the project's own checks and the Stop hook's cross-`governs` flag.

## Vocabulary

- **Claim**: a list item `- [AC-<DELTA-ID>-<n>] text` in a `behaviour.<spec-id>` part of the delta, one part per fold target (file `behaviour.<spec-id>.md` beside the delta's `index.md`). The part is the section; it has no `## Behaviour` heading, and a claim in `index.md` is an error. Ids are namespaced by the delta (`AC-DELTA-017-1`). `[intent]` after the id marks a claim no test can settle.
- **Receipt**: an `@ref <path>#<symbol>@<sha>` line indented under the claim, naming the test. It must name a test file, per the spec's `tests` globs (default `**/tests/**`, `**/*_test.*`, `**/*.test.*`, `**/*.spec.*`). It goes stale when the test file or code under the spec's `governs` changes after `<sha>`.
- **Base**: an `@base <hash>` line indented under a claim that replaces or strikes a spec claim. It pins the spec claim's block the claim was written against; the fold errors if the spec moved. Written only by `lemma base` and `lemma receipt`.
- **Fold**: when a human moves the delta to `complete`, a hook appends new claim ids to the spec, replaces existing ids, deletes struck ones (`- ~~[ID]~~ text`). You never write a spec's `behaviour` part.
- **Human receipt**: `@checked <name>@<sha>` under a claim. Human-only; never write it.
- **Approval**: a human moving the delta to `complete`. Never do it, and never write `reviewed:`.

## Lifecycle

1. Create: `lazyspec create delta "<title>"`.
2. Link each target spec: `lazyspec link <delta> folds-into <spec>`. For a new capability, first create the spec at `draft` with `Intent` only (`lazyspec create spec`; its empty `behaviour` part comes with the template), then link it.
3. Write `## Intent` in `index.md`. Write the claims into one part per fold target: `lazyspec update <delta> --part behaviour.<spec-id> --body-file -` creates it. A part name that matches no fold target is an error, as is a fold target without its part. Keep claims observable and falsifiable. Restating a spec claim's id replaces it; striking needs `- ~~[ID]~~`. After writing a replacing or striking claim, run `lemma base <delta-id> <claim-id>`; a replace or strike without `@base` blocks step 4. Run it again if the spec changed after you read it.
4. Advance `draft` to `in-progress` yourself: `lazyspec update <delta> --status in-progress`. Requires `Intent`, at least one claim across the parts, resolving fold targets, and a `behaviour.<spec-id>` part for each. You may do this without asking.
5. For each claim: write the failing test, make it pass, commit test and implementation together, then `lemma receipt <delta-id> <claim-id> <path>#<symbol>`. It finds the claim in whichever `behaviour.*` part holds it, pins short HEAD and inserts the `@ref` line; never hand-edit `@ref` lines. It refuses on an unknown claim, an unresolved symbol, uncommitted changes in the file, or a path outside the spec's `tests` globs (a production symbol is not a receipt), and inserts `@base` for a replacing claim. Before asking for review, commit all governed code and receipt test files: the `complete` gate refuses a dirty tree. If the test already exists and is committed at HEAD, run `lemma receipt` with no commit. Any later commit touching the test file or governed code stales the receipt; receipt after the last such commit (re-running `lemma receipt` for the same `<path>#<symbol>` updates the existing receipt in place rather than adding another), or `lemma re-pin <delta-id>` (every `behaviour*` part) once your own later changes are done.
6. Run `lazyspec validate --warnings` (errors always show and fail the run; only drift warnings need the flag); keep it clean. A receipt that does not resolve or has gone stale is yours to fix. To confirm review-readiness, `lazyspec hook run pre-transition <delta-id> --dry-run` checks the gate on the next edge and prints `checked in-progress → complete: ready` when clean. The `<n> of <m> claims are [intent]` and `checked … ready` warnings are informational: never remove `[intent]` or invent receipts to silence them.
7. Stop at review-ready: `in-progress`, every non-`[intent]` claim receipted, validate clean. Report that. Never move the delta to `complete`; that is the human's approval, and it runs the fold. If the Stop hook reports a delta `complete` in the working tree but not at HEAD, a human moved it: leave the status alone and ask them to commit; never change it back. Do not hand-edit specs to imitate the fold.

## Drift

When governed code or a receipt's test changed outside your work, resolve the drift relevant to your claims first, then `lemma re-pin <delta-id> [claim-id ...]`, naming only the claims you resolved. `re-pin` refuses (exit 2, nothing written) when a symbol no longer resolves, when a receipt's test file has uncommitted changes (commit it first; `--force` does not override this), or when a receipt's file changed while that claim's text did not. Pass `--force` only after confirming the claim still holds against the changed test; never to silence a finding. A `@base` mismatch means the spec moved under you: re-read the spec claim, reconcile your claim text, then run `lemma base <delta-id> <claim-id>` again. Receipts pin branch commits; after a squash merge or rebase they no longer resolve on the main history, so re-pin the spec after merging.

## Bound

A direct change stays inside one `governs` area. Crossing areas needs an in-progress delta covering them. Do not leave pending tests (`it.todo`, `#[ignore]` without a reason).
