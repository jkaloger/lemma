---
name: delta
description: Use when changing code in a lemma project - build, fix, chore, refactor or spike - or when resuming an in-progress delta. Drives the delta document from draft to review-ready with claims and test refs.
---

# lemma delta

No unbounded code changes; no unverified completion. The delta is the unit of in-flight work: a set of claims, each settled by a test it names with `@ref`.

Pick the workflow by judgement per `.lazyspec/dictums/code-change.md`, then load one reference: `references/build.md`, `fix.md`, `chore.md`, `refactor.md`, `spike.md` or `adopt.md`. Nothing classifies for you; misjudgement is caught by `lemma check`, the project's own checks and the Stop hook.

## Vocabulary

- **Claim**: a list item `- [AC-<DELTA-ID>-<n>] text` in a `behaviour.<spec-id>` part of the delta, one part per fold target (file `behaviour.<spec-id>.md` beside the delta's `index.md`). The part is the section; it has no `## Behaviour` heading, and a claim in `index.md` is an error. Ids are namespaced by the delta (`AC-DELTA-017-1`). At most one marker after the id:
  - none: normative, settled by a test.
  - `[intent]`: normative, but no test can settle it (UX quality, a performance envelope). Not for "not decided yet".
  - `[observed]`: existing behaviour worth protecting from accidental change. Needs a ref. Rare: for a test an agent would break without realising, not a mirror of the suite.
  - `[assumed]`: the work depends on something not yet established. Delta only; never folds.
- **Provenance**: an optional `@per <citation>` line indented under a claim, naming where its authority comes from: an accepted decision id, a ticket, a policy. Never invent one; a claim with none is established by the human accepting the delta.
- **Ref**: an `@ref <path>#<symbol>` line indented under the claim, naming the test. No commit, no hash. You write it by hand. It must name a test file, per the spec's `tests` globs (default `**/tests/**`, `**/*_test.*`, `**/*.test.*`, `**/*.spec.*`). Check the ref with `lemma refs <claim-id>`.
- **Immutable claims**: a claim's text never changes once it is in a spec. To change or fix wording, strike the old id (`- ~~[ID]~~`) and add a new id, carrying its `@ref` lines. Reusing an existing id is an error.
- **Fold**: when a human moves the delta to `complete`, a hook appends new claim ids to the spec and deletes struck ones. You never write a spec's `behaviour` part. Preview it with `lemma apply <delta-id>`; it writes nothing.
- **Approval**: a human moving the delta to `complete`. Never do it, and never write `reviewed:`.

## Lifecycle

1. Create: `lazyspec create delta "<title>"`.
2. Link each target spec: `lazyspec link <delta> folds-into <spec>`. For a new capability, first create the spec at `draft` with `Intent` only (`lazyspec create spec`; its empty `behaviour` part comes with the template), then link it.
3. Write `## Intent` in `index.md`. Write the claims into one part per fold target: `lazyspec update <delta> --part behaviour.<spec-id> --body-file -` creates it. A part name that matches no fold target is an error, as is a fold target without its part. Record what is known; mark what is not as `[assumed]` rather than inventing a commitment. Keep claims observable and falsifiable. To change a spec claim, strike its id and add a new one; do not restate the id.
4. Advance `draft` to `in-progress` yourself: `lazyspec update <delta> --status in-progress`. Requires `Intent`, at least one claim across the parts, resolving fold targets, and a `behaviour.<spec-id>` part for each. You may do this without asking.
5. For each claim: write the failing test, make it pass, then add `@ref <path>#<symbol>` under the claim in the part file. Run `lemma check`: diagnostics are `path:line: severity: message`, and silence means clean. `lemma refs <claim-id>` lists each ref as `ok`, `unresolved` or `not-a-test`. A production symbol is not a ref. An existing test needs no commit to be referenced.
6. `lemma signal` nudges arrive after edits as one-line context: spec pointers on the first touch of a governed area, and findings for the file you just edited. Act on them. Each is said once per session.
7. Readiness for review: delta `in-progress`, `lemma check --require evidence --require clean` clean, and `lemma apply <delta-id>` previews the fold you expect. `clean` needs governed code and ref files committed. The `<n> of <m> claims are [intent]` count is a warning in `--json` and `--hook` output (text output stays silent) and is not a fault: never remove `[intent]` or invent refs to silence it.
   Every `[assumed]` claim must be resolved before review-ready, by one of four exits: it becomes a normative claim (drop the marker, add a test and ref, `@per` if authority exists); it becomes `[observed]` once a test shows the behaviour; its line is deleted (never struck: the spec never held it); or it moves to the spec's open questions as prose, which is a spec edit for the human. Never remove `[assumed]` to clear a gate without one of these exits.
8. Stop at review-ready and report. Never move the delta to `complete`; that is the human's approval, and it runs the fold. If the Stop hook reports a delta `complete` in the working tree but not at HEAD, a human moved it: leave the status alone and ask them to commit; never change it back. Do not hand-edit specs to imitate the fold.

## Evidence changed

Check 2 flags a changed test region that a landed spec claim references, unless an in-flight delta strikes that claim. If the test must change, put the claim in a delta: strike it, add a new id with the `@ref` lines. If two in-flight deltas strike one id, the second `apply` fails; reconcile first.

## Bound

A direct change stays inside one `governs` area. Crossing areas needs an in-flight delta (draft or in-progress) for each touched spec; `lemma check` flags it. Pending tests (`it.todo`, `#[ignore]` without a reason) are a repo convention, not a lemma check.
