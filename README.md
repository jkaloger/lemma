# lemma

A lazyspec workflow pack, a Claude Code plugin and a hooks binary for LLM-driven development. A delta is a set of claims; a claim is complete when a receipt names the test that settles it; a delta folds into its spec by claim id when it is archived.

Design rationale is recorded in [docs/RFC-001-lemma.md](docs/RFC-001-lemma.md). This document states behaviour.

## Synopsis

```
lazyspec init --template https://github.com/jkaloger/lemma
lazyspec hook trust
claude plugin install lemma@lemma
cargo install lemma

lemma validate
lemma pre-transition
lemma stop
lemma re-pin <id>
lemma receipt <delta> <claim-id> <path#symbol>
```

## Description

lemma has three parts, released together under one version number.

The **pack** is a `.lazyspec.toml`, a set of document templates, two dictums and a `[[hooks]]` table. `lazyspec init --template` copies it into a project. The pack defines seven document types in two families.

The **plugin** is a Claude Code plugin carrying six skills and the Claude Code hook entries. Each skill speaks claims, receipts, fold and approval directly. The lazyspec plugin is not a dependency.

The **binary** is a single Rust executable, `lemma`, with one subcommand per enforcement surface. lazyspec `[[hooks]]` entries and Claude Code hook commands both invoke it. It links the lazyspec engine as a crate. It is not invoked by hand in normal use.

lemma never runs tests. It checks that each claim names a test symbol that resolves and is fresh. The codebase's own test runner, pre-commit and CI own pass and fail.

## Document types

| Type         | Family        | Staleness | Statuses                                        | Written by                              |
| ------------ | ------------- | --------- | ----------------------------------------------- | --------------------------------------- |
| `spec`       | evergreen     | drift     | `active`                                        | the archive hook only                   |
| `convention` | evergreen     | drift     | `active`                                        | humans                                  |
| `dictum`     | evergreen     | drift     | `active`                                        | humans                                  |
| `delta`      | point-in-time | age       | `draft` → `in-progress` → `complete` → `archived` | agents and humans                       |
| `decision`   | point-in-time | age       | `proposed` → `accepted` → `superseded`          | agents and humans; never edited after acceptance |
| `bug`        | point-in-time | age       | `open` → `closed`                               | agents and humans                       |
| `idea`       | point-in-time | age       | `open` → `promoted` \| `dropped`                | agents and humans                       |

Evergreen types carry `governs` globs. A change under the globs moves the document to the `stale` band.

A **spec** is one capability. Its `Intent` section is required and non-empty. Its `Behaviour` section is optional and is the only section in which a tagged claim may appear. A spec must link at least one rationale artefact: a hydra tree, a promoted idea or a decision record. A spec with no `Behaviour` section is valid.

A **delta** is the unit of in-flight work and the only type a skill drives through its statuses. Its frontmatter links each spec it folds into. Its `Behaviour` section carries claims as they will appear in the spec. When the delta folds into more than one spec, `Behaviour` is partitioned by `### <spec-id>` subheadings. An optional `tasks` bundle part holds non-claim work; no hook reads it. A delta that targets a new capability creates that spec at `draft` with `Intent` only and links to it.

A **decision** records one consequential choice and links the spec it affects. A superseding decision links the one it replaces.

A **bug** carries one reproduction claim and links the spec claim id it violates. It closes with reason `fixed` when its own regression receipt resolves, or when a delta it spawned reaches `complete`, or with reason `wontfix` and a stated reason. A bug never writes a spec.

An **idea** carries intent only. A promoted idea links the delta or hydra tree it became. A dropped idea carries a reason.

A **convention** holds the project's principles and is injected as the prompt preamble. A **dictum** is a module-shaped rule set; each rule is a tagged claim whose receipt is the lint rule, clippy rule or architecture test that enforces it.

## Claims and receipts

A **claim** is a tagged list item with a stable id inside a `Behaviour` section:

```
- [AC-C17-1] A reopened head keeps its prior answer until it is cut again.
  @ref src/store.rs#reopen_keeps_prior@3f9c2e1
```

Ids are namespaced by the delta that introduced them. A claim carrying the marker `[intent]` is excluded from the receipt check. `lemma validate` reports the count per document as `<n> of <m> claims are [intent]`. lazyspec findings have no info severity, so the finding is emitted as a warning.

A **receipt** is an `@ref` line under a claim naming a test symbol, pinned at a commit. The receipt is **fresh** when neither the test file nor the code under the spec's `governs` globs has changed since the pin. Otherwise it is **stale**.

`verified-by: <name> @ <sha>` under a claim records human sign-off as a receipt. It goes stale by the same rule.

`approved-by: <name> @ <sha>` on a delta pins its `Behaviour` section. A delta carrying a valid approval may be moved to `complete` by an agent. A claim added, replaced or removed after the pinned commit invalidates the approval.

A line beginning `waived:` under a claim, with a reason, suppresses one finding on that claim. Waivers are counted in the same finding from release 2.

## Fold

The transition `complete` → `archived` runs the fold. For each target spec, the hook appends a claim whose id is new, replaces the line of a claim whose id exists, and deletes a claim whose id is struck through in the delta. Receipts move with their claims. The hook rewrites the spec through the lazyspec `updates` contract and never creates a spec.

## Hooks binary

| Subcommand                   | Invoked by                                  | Severity              |
| ---------------------------- | ------------------------------------------- | --------------------- |
| `lemma validate`             | lazyspec `validate` event, CI               | warn                  |
| `lemma pre-transition`       | lazyspec `pre-transition` event             | error                 |
| `lemma stop`                 | Claude Code `Stop` hook                     | error, with the hook's escape |
| `lemma re-pin <id>`          | agents, after resolving drift               | refuses or flags      |
| `lemma receipt <delta> <claim-id> <path#symbol>` | agents, after a claim's test is committed | refuses |
| `lemma prompt`               | Claude Code `UserPromptSubmit` hook         | none; injects the convention preamble |

Severity is fixed by surface. There is no configuration table.

`lemma validate` reads documents as JSON on stdin and reports: a claim without a receipt on an in-progress delta; a receipt that does not resolve; a stale receipt; two in-progress deltas touching one claim id; a fold target that does not resolve; a stale `verified-by` or `approved-by`; an evergreen spec with no `governs`; a spec with no rationale link; a contract mutation, meaning a file named by a receipt changed while no in-progress delta replaces or removes that claim. The count of `[intent]` claims is one finding for each document with at least one `[intent]` claim, and is not emitted for archived deltas; the waiver count is release 2. Native lazyspec validation (required sections, relationships, staleness bands, source ownership) is relied on and not repeated.

`lemma pre-transition` applies per transition. `draft` → `in-progress`: `Intent` present, at least one claim, fold targets resolve. `in-progress` → `complete`: every claim not marked `[intent]` has a resolving, fresh receipt; an agent-invoked completion requires a valid `approved-by`; a contract mutation or a flagged re-pin invalidates `approved-by`. `complete` → `archived`: the fold.

`lemma stop` reports an error when the session's diff spans more than one `governs` area and no in-progress delta covers it; when a document touched in the session has an error finding; when a touched test file contains a pending test (`it.todo`, `#[ignore]` without a reason); when `approved-by` was written during the session; when a receipt file touched in the session shows a contract mutation.

`lemma re-pin <id>` updates the receipt commits on one document. It refuses when a referenced symbol no longer resolves. It flags the re-pin when the receipt file changed and no claim on the document changed with it.

`lemma receipt <delta> <claim-id> <path#symbol>` inserts an `@ref` line as the first indented line under the claim, pinned at the short HEAD sha. It refuses, writing nothing, when the claim id does not exist on the delta, when the symbol does not resolve in the working tree, or when the file has uncommitted changes. A test already committed at HEAD needs no further commit. Re-running for the same `<path>#<symbol>` replaces that receipt in place, re-pinning it to the new HEAD, instead of adding a second `@ref`.

The `reviewed: <sha>` frontmatter field is written by lazyspec on a status change. It records the commit the document was last moved at. It is not approval. Agents do not write it, as with `approved-by`.

lazyspec invokes `lemma pre-transition` with the same status for `from` and `to` when run by hand with `lazyspec hook run pre-transition <id> --dry-run`. lemma then evaluates the gate on the next edge from that status (`draft` → `in-progress`, `in-progress` → `complete`) and reports findings without updating anything. When there are no findings it emits the warning `checked <from> → <to>: ready`. At `complete` it previews the fold as warnings (`would fold <n> claims into <spec>`) and writes nothing.

## Skills

| Skill            | Covers                                                                         |
| ---------------- | ------------------------------------------------------------------------------ |
| `lemma:entry`    | Dispatch. If an in-progress delta exists, resumes it under `delta`. Otherwise routes by the prompt. Does not classify fix, chore or refactor. |
| `lemma:shape`    | ideate, design (hands to hydra), specify (router over idea, hydra tree or prompt), plan, decide, triage |
| `lemma:delta`    | build, fix, chore, refactor, spike                                             |
| `lemma:verify`   | test, conform, audit                                                           |
| `lemma:maintain` | reconcile, housekeep, configure                                                |
| `lemma:operate`  | ship, orchestrate, resume                                                      |

Each workflow is a reference file inside its group skill, loaded on demand. Orientation prompts (explain, orient, investigate, review) have no skill.

The human decision points are the shaping track and the transition `in-progress` → `complete`. The agent drives `draft` → `in-progress` and works until every claim has a receipt. Readiness for review is derived: `in-progress`, every claim receipted, validate clean. The transition to `archived` is mechanical.

## Files

| Path                                | Contents                                              |
| ----------------------------------- | ----------------------------------------------------- |
| `.lazyspec.toml`                    | type definitions, `[governs]`, `[[hooks]]` entries pointing at `lemma` |
| `.lazyspec/templates/`              | one template per type                                 |
| `.lazyspec/dictums/`                | `testing`, `code-change`                              |
| `.lazyspec/dictums/optional/`       | stack dictums (`testing-web`); pruned at setup        |
| `plugin/`                           | Claude Code plugin: skills and hook entries           |
| `crates/lemma/`                     | the hooks binary                                      |
| `docs/RFC-001-lemma.md`             | founding design and rationale                         |

## Exit status

`lemma validate` and `lemma pre-transition` print `{"findings": [...]}` and exit 0; lazyspec applies severity. `lemma stop` exits 2 on an error finding. `lemma re-pin` exits 2 when it refuses and 0 with a flag finding otherwise. `lemma receipt` exits 2 when it refuses. `lemma re-pin` and `lemma receipt` exit 2 when `lazyspec --version` is missing or older than 0.15; `lemma stop` prints the hint to stderr and exits 0.

## Examples

Adopt the pack in a repository and trust its hooks:

```
lazyspec init --template https://github.com/jkaloger/lemma
lazyspec hook trust
```

Check the gate on a delta's next transition without changing its status. A clean run prints `checked in-progress → complete: ready`:

```
lazyspec hook run pre-transition C17 --dry-run
```

Re-pin a delta's receipts after resolving drift:

```
lemma re-pin C17
```

Pin a receipt for a committed test:

```
lemma receipt C17 AC-C17-1 src/store.rs#reopen_keeps_prior
```

## Install

Each part uses its native channel.

```
lazyspec init --template https://github.com/jkaloger/lemma   # pack
claude plugin install lemma@lemma                          # skills and Claude Code hooks
cargo install lemma                                          # hooks binary
```

The skills and the Stop hook load only after the Claude Code session is restarted. Restart before any delta work.

lemma requires lazyspec 0.15 or later. `lemma stop`, `lemma re-pin` and `lemma receipt` check `lazyspec --version`. When lazyspec is missing or older than 0.15, `re-pin` and `receipt` exit 2; `stop` only warns on stderr and does not block the turn.

The binary is built with `opt-level = "z"`, fat LTO, one codegen unit, `panic = "abort"`, stripped.

## Status

Release 1.1 follows hydra adoption (steps 1 to 3 of the acceptance): finding lines are file lines, the `[intent]` count, by-hand dry-run, `lemma receipt` and the lazyspec version check. Release 1 is built and has passed the five-step acceptance against a clone of the hydra repository; adopting it in hydra itself is outstanding. Release 1 ships the `spec`, `delta` and `decision` types; the `testing` and `code-change` dictums; `lemma validate`, the `in-progress` → `complete` gate, the fold, `lemma stop` with the cross-governs bound and pending-test read, and `lemma re-pin`; the `entry` and `delta` skills; a CI job running `lemma validate`. The release criterion is the five-step acceptance run in the hydra repository described in the RFC. Everything else in this document is release 2 or later.

## See also

- [docs/RFC-001-lemma.md](docs/RFC-001-lemma.md), the founding design
- [lazyspec](https://github.com/jkaloger/lazyspec), the engine: document types, `governs`, `@ref`, `[[hooks]]`
- [hydra](https://github.com/jkaloger/hydra), the interview tool whose trees are the long-form rationale record
