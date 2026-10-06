---
title: "Claims distinguish what is known from what is assumed"
type: decision
status: proposed
author: "Jack Kaloger"
date: 2026-10-06
tags: []
related: []
---

## Context

A review of 2.0 against the critique "specification may be the wrong paradigm" found the paradigm sound and one hole in the claim grammar. RFC-001 §2 and §5.1 already define a spec as the small set of things the memoryless agent cannot reconstruct from code, tests and git, not a complete description. The checks hold to that: lemma reads only `Intent` and claim location. The pressure toward premature, complete-looking specification comes from elsewhere:

- `[intent]` carries two meanings. RFC §6.5 defines it as "normative, no test can settle this" (UX quality, performance envelopes). In practice it is also the only place to park "not decided yet". A folded spec cannot tell a constraint from an open question, and the `<n> of <m> claims are [intent]` warning treats both as debt.
- `build.md` step 1 says "write the full delta before any code". The `draft → in-progress` gate requires only Intent, target wiring and one claim, so mechanically the bar is low; the skill prose is what the agent follows. Agent-facing prose is part of the architecture.
- `adopt` writes current tested behaviour into a spec as claims so that check 2 will protect the tests. RFC §5.1 says current tested behaviour is owned by the suite and not documented. Adopt breaks that rule to buy a check, which is modelling information for lemma's benefit.
- At `complete` the human sees claims that all have refs and cannot tell an agreed requirement from an acceptance criterion the agent invented to make the work specifiable. Approval is batch approval with no per-claim signal of where the claim came from.
- The pack's dictums are plain files under `.lazyspec/dictums/`. No `dictum` type is declared, so lazyspec never loads them, `lemma signal` cannot point at them, and their `@ref` lines are unchecked.

A worked example (payment retries: Adyen only, no card data in logs, asynchronous, idempotent, UI after the third failure undecided) showed the same constraint appearing three times: as spec prose, as a delta claim with a test, and as an `[intent]` line. Moving chosen constraints to decisions and imposed constraints to a dictum with `governs` left the spec's claims to the capability's own commitments.

The principle: do not reduce specification, reduce false certainty. A change begins highly constrained when reality is, and loosely specified when it is not. An agent may discover, infer or implement an assumption; it cannot manufacture the provenance that turns it into an agreed commitment.

## Decision

**Three markers, one grammar.** A claim is still `- [ID] text` in a claims part, immutable once folded, struck and replaced to change.

| Marker | Meaning | Check 3 | Folds | At `complete` |
| --- | --- | --- | --- | --- |
| none | normative, test-settled | ref required | yes | listed as a commitment with its provenance |
| `[intent]` | normative, no test can settle it | excluded | yes | listed as a commitment with its provenance |
| `[observed]` | descriptive: existing behaviour worth protecting from accidental change | ref required | yes | listed as observed |
| `[assumed]` | the work depends on something not yet established | excluded | never | error until resolved |

`[assumed]` exists only in an in-flight delta. It has four exits: it becomes a normative claim the human accepts; it becomes `[observed]` once a test shows the behaviour; its line is deleted (a strike is wrong: the spec never held it); or it moves to the spec's open questions as prose. `lemma check --require evidence` reports each remaining `[assumed]` claim as an error, so `complete` is the reconciliation step: no silent guesses are left in the ledger.

`[observed]` is rare. It is for a test an agent would break without realising, not for mirroring the suite into the spec. Check 2 reports a changed region under an observed claim with the wording `characterised behaviour changed; confirm the change is intended`, same severity.

**Provenance is optional and honest.** `@per <citation>` under a claim names where its authority comes from. Lemma resolves a citation mechanically only when it is the id of a decision that was `accepted` before the range's base; it displays any other citation as an unverified reference. Human acceptance of the delta remains a legitimate source of authority for a new commitment. The delta-namespaced id already records which change introduced a claim.

**The review surface shows where truth came from.** `lemma apply <delta>` preview and the `in-progress → complete` gate group the delta's claims as new commitments (with provenance, or `provenance: none; accepting this delta establishes this behaviour`), observed behaviour, resolved assumptions (what each became), discarded assumptions, and unresolved assumptions. Resolution needs no markup. The delta rarely exists at the range's base, so resolution is read from the range's commits: an id that was `[assumed]` in any committed version of the claims part within the range and is not `[assumed]` now was resolved (in place, `was assumed, now a commitment`, or by a new id when the id changed); one that is gone was discarded. In hook mode the surface is one finding per claim, warning severity, at the `complete` gate only, so the output stays diagnostics. The `[intent]` count is dropped from the output; the grouping replaces it.

**Workflow records what is known.** The `draft → in-progress` gate is `--require intent`: Intent present, fold targets resolve, each target has its claims part. No claim is required to start. `build.md` reads "record what is known before code: constraints, commitments, assumptions; preserve unknowns as `[assumed]` as you meet them". `adopt` reads "claim what would be expensive or surprising to rediscover, as `[observed]`; the suite owns the rest". CHG-9 and TEST-1 are amended to match.

**Dictums are documents.** The pack declares `convention` and `dictum` types (as lazyspec's own repository does) and ships `testing` and `code-change` as DICTUM documents. A dictum is a claim site with its own rules, because dictums are evergreen, human-edited and never folded (RFC §5.3). Check 3 applies as for a spec, with the dictum's `tests` key (same defaults) as the test scope. Check 2 on a dictum claim is an error unless the dictum itself changes that claim in the range, by hand, strike plus new id. Check 4's fold rule does not apply to dictums. A dictum's `governs` does not count as an area for check 1. Imposed constraints (policy, regulation, vendor limits) are dictum rules with `governs` and a provenance citation; chosen constraints are decisions.

**Names say what they are.** The spec part `behaviour` becomes `claims`; the delta part `behaviour.<spec-id>` becomes `claims.<spec-id>`. The decision-to-spec and dictum-to-spec relationship is `constrains` (inverse `constrained-by`), replacing `affects`. Templates carry the suggested headings (spec: Intent, Constraints, Non-goals, Open questions; delta: Intent, Approach, Out of scope) with guidance in HTML comments, which the parser already ignores.

Amends DECISION-001, which stays accepted: check 3 ("every live claim has evidence, unless marked `[intent]`") gains the `[observed]` and `[assumed]` cases; the pack hook `--require claims` becomes `--require intent`; "a claim that no test can settle is marked `[intent]`" is narrowed to its RFC §6.5 meaning. Everything else in DECISION-001 stands.

## Consequences

- Lemma 2.1 is a format break: part rename, `affects` to `constrains`. `lemma migrate` renames parts and relationships in place. One migration covers the rename and the new markers.
- An `[assumed]` claim that is never resolved blocks `complete`. The cost is that an agent may delete assumptions to clear the gate; the grouped preview lists discarded assumptions from the range's history, so the human sees it.
- `[observed]` can still be abused to mirror a suite. The guard is prose and review, not a check; if dogfooding shows abuse, a ratio finding is the back-pocket amendment.
- Provenance that lemma cannot resolve is displayed, not trusted. Whether lazyspec can distinguish a human transition from an agent status edit is an open question for `.scratch/lazyspec-contract.md`; until it can, the in-session completion rule in `lemma stop` is the guardrail, and it is a proxy for authority, not authority.
- The `[intent]` count disappears from `--json` and `--hook` output. The hydra W48 acceptance runs after the prose changes and before the marker work, so the two effects are measured apart.
- Evidence providers beyond test refs, topology and history signals, and runtime sources stay out, each with a trigger, as RFC §14 does.
