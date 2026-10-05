# lemma

A lazyspec workflow pack, a Claude Code plugin and a hooks binary for LLM-driven development. A delta is a set of claims; a claim is complete when a receipt names the test that settles it; a delta folds into its spec by claim id when it completes.

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

The **pack** is a `.lazyspec.toml`, a set of document templates, two dictums and a `[[hooks]]` table. `lazyspec init --template` copies it into a project. The pack defines five document types in two families.

The **plugin** is a Claude Code plugin carrying six skills and the Claude Code hook entries. Each skill speaks claims, receipts, fold and approval directly. The lazyspec plugin is not a dependency.

The **binary** is a single Rust executable, `lemma`, with one subcommand per enforcement surface. lazyspec `[[hooks]]` entries and Claude Code hook commands both invoke it. It links the lazyspec engine as a crate. It is not invoked by hand in normal use.

lemma never runs tests. It checks that each claim names a test symbol that resolves and is fresh. The codebase's own test runner, pre-commit and CI own pass and fail.

## Document types

| Type         | Family        | Staleness | Statuses                                        | Written by                              |
| ------------ | ------------- | --------- | ----------------------------------------------- | --------------------------------------- |
| `spec`       | evergreen     | drift     | `active`                                        | the fold only                           |
| `convention` | evergreen     | drift     | `active`                                        | humans                                  |
| `dictum`     | evergreen     | drift     | `active`                                        | humans                                  |
| `delta`      | point-in-time | age       | `draft` → `in-progress` → `complete`           | agents and humans                       |
| `decision`   | point-in-time | age       | `proposed` → `accepted` → `superseded`          | agents and humans; never edited after acceptance |

Evergreen types carry `governs` globs. A change under the globs moves the document to the `stale` band.

A **spec** is one capability, a lazyspec bundle. Its `index.md` carries frontmatter and the `Intent` section, which is required and non-empty. Its `behaviour` part (`behaviour.md`, empty at creation) is the only place a tagged claim may appear; a claim-shaped item in `index.md` is a warning. A spec must link at least one rationale artefact: a hydra tree or a decision record. A spec whose `behaviour` part is empty is valid.

A **delta** is the unit of in-flight work and the only type a skill drives through its statuses. It is a bundle. Its `index.md` carries frontmatter, which links each spec it folds into, and `Intent`. Claims, as they will appear in the spec, live in one part per fold target, named `behaviour.<spec-id>` (file `behaviour.<spec-id>.md`). Each fold target has its part, and each `behaviour.*` part names a fold target. An optional `tasks` part holds non-claim work; no hook reads it. A delta that targets a new capability creates that spec at `draft` with `Intent` only and links to it.

A **decision** records one consequential choice and links the spec it affects. A superseding decision links the one it replaces.

A bug is a draft delta whose first claim reproduces the defect. An idea is a draft spec with `Intent` only. Neither is a type.

A **convention** holds the project's principles and is injected as the prompt preamble. A **dictum** is a module-shaped rule set; each rule is a tagged claim whose receipt is the lint rule, clippy rule or architecture test that enforces it.

## Claims and receipts

A **claim** is a tagged list item with a stable id inside a `behaviour` part. The part is the section; it has no `## Behaviour` heading.

```
- [AC-C17-1] A reopened head keeps its prior answer until it is cut again.
  @ref src/store.rs#reopen_keeps_prior@3f9c2e1
```

Ids are namespaced by the delta that introduced them. The spec governing a delta claim is the target its part names; for a spec claim it is the spec. A claim carrying the marker `[intent]` is excluded from the receipt check. `lemma validate` reports the count per document as `<n> of <m> claims are [intent]`. lazyspec findings have no info severity, so the finding is emitted as a warning.

A **receipt** is an `@ref` line under a claim naming a test symbol, pinned at a commit, or an `@checked <name>@<sha>` line recording that a human checked the claim at a commit. An `@ref` receipt is **fresh** when neither the test file nor the code under the spec's `governs` globs has changed since the pin; an `@checked` receipt is fresh when nothing under the `governs` globs has changed since `<sha>`. Otherwise a receipt is **stale**. `lemma receipt` never writes `@checked`; humans do.

A claim that no machine check can settle is marked `[intent]`.

A human moving a delta `in-progress` → `complete` approves it. lazyspec's `reviewed: <sha>` stamp records where.

## Fold

The transition `in-progress` → `complete` runs the fold, in the same response as the gate and only when the gate has no errors. For each target spec, the hook takes the claims in the delta's `behaviour.<spec-id>` part, appends a claim whose id is new, replaces the line of a claim whose id exists, and deletes a claim whose id is struck through. Receipts move with their claims. The hook writes the merged claim list to the spec's `behaviour` part through the lazyspec `updates` contract (`{id, part: "behaviour", hash, body}`) and never creates a spec or a part. A target spec with no `behaviour` part is an error finding and nothing is written.

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

`lemma validate` reads documents as JSON on stdin and reports: a claim in a spec or delta `index.md`; a fold target with no `behaviour.<spec-id>` part; a `behaviour.*` part that names no fold target; a claim without a receipt on an in-progress delta; a receipt that does not resolve; a stale receipt; two in-progress deltas touching one claim id; a fold target that does not resolve; an evergreen spec with no `governs`; a spec with no rationale link; a contract mutation, meaning a file named by a receipt changed while no in-progress delta replaces or removes that claim. The count of `[intent]` claims is one finding for each document with at least one `[intent]` claim. Findings on claims and receipts carry `part` and the line in the part file, which has no frontmatter. Findings on `index.md` carry the line in the file. Native lazyspec validation (required sections, relationships, staleness bands, source ownership) is relied on and not repeated.

`lemma pre-transition` applies per transition. `draft` → `in-progress`: `Intent` present, fold targets resolve, every fold target has its `behaviour.<spec-id>` part, every `behaviour.*` part names a fold target, at least one claim across the parts. `in-progress` → `complete`: fold targets resolve and each has a `behaviour` part, every claim not marked `[intent]` has a resolving, fresh receipt, and the fold runs. A fold target loaded from the store rather than the hook payload cannot be written and is an error.

`lemma stop` reports an error when the session's diff spans more than one `governs` area and no in-progress delta covers it; when a document touched in the session has an error finding; when a touched test file contains a pending test (`it.todo`, `#[ignore]` without a reason); when a delta is `complete` in the working tree and was not `complete` at HEAD, including a delta new in the working tree.

`lemma re-pin <id>` updates the `@ref` receipt commits in every `behaviour*` part of one document. It never touches `@checked` lines; a stale `@checked` stays stale until a human re-signs. It refuses, writing no part, when a referenced symbol no longer resolves. It flags the re-pin when the receipt file changed and no claim in that part changed with it; each part's claim lines are compared at the old sha.

`lemma receipt <delta> <claim-id> <path#symbol>` finds the claim id across the delta's `behaviour.*` parts and inserts an `@ref` line, in that part file, as the first indented line under the claim, pinned at the short HEAD sha. It refuses, writing nothing, when the claim id is live in no part of the delta, when it is live in more than one, when the symbol does not resolve in the working tree, or when the file has uncommitted changes. A test already committed at HEAD needs no further commit. Re-running for the same `<path>#<symbol>` replaces that receipt in place, re-pinning it to the new HEAD, instead of adding a second `@ref`.

The `reviewed: <sha>` frontmatter field is written by lazyspec on a status change. It records the commit the document was last moved at. Agents do not write it.

lazyspec invokes `lemma pre-transition` with the same status for `from` and `to` when run by hand with `lazyspec hook run pre-transition <id> --dry-run`. lemma then evaluates the gate on the next edge from that status (`draft` → `in-progress`, `in-progress` → `complete`) and reports findings without updating anything. At `complete` it does nothing. When there are no findings it emits the warning `checked <from> → <to>: ready`. At `in-progress` it also previews the fold as warnings (`would fold <n> claims into <spec>`) and writes nothing.

## Skills

| Skill            | Covers                                                                         |
| ---------------- | ------------------------------------------------------------------------------ |
| `lemma:entry`    | Dispatch. If an in-progress delta exists, resumes it under `delta`. Otherwise routes by the prompt. Does not classify fix, chore or refactor. |
| `lemma:shape`    | ideate, design (hands to hydra), specify (router over hydra tree or prompt), plan, decide, triage |
| `lemma:delta`    | build, fix, chore, refactor, spike                                             |
| `lemma:verify`   | test, conform, audit                                                           |
| `lemma:maintain` | reconcile, housekeep, configure                                                |
| `lemma:operate`  | ship, orchestrate, resume                                                      |

Each workflow is a reference file inside its group skill, loaded on demand. Orientation prompts (explain, orient, investigate, review) have no skill.

The human decision points are the shaping track and the transition `in-progress` → `complete`. The agent drives `draft` → `in-progress` and works until every claim has a receipt. Readiness for review is derived: `in-progress`, every claim receipted, validate clean. The fold runs with the transition to `complete`.

## Files

| Path                                | Contents                                              |
| ----------------------------------- | ----------------------------------------------------- |
| `.lazyspec.toml`                    | type definitions, `[governs]`, `[[hooks]]` entries pointing at `lemma` |
| `.lazyspec/templates/`              | one template per type; `spec/` (`index.md`, `behaviour.md`) and `delta/` (`index.md`) are directory templates |
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

Release 1.3 cuts the sign-off vocabulary to claims, receipts and `[intent]`, and the types to five: approval is the human's `in-progress` → `complete` transition, `@checked` is the human receipt, and `complete` runs the fold. Release 1.2 makes specs and deltas bundles and moves claims out of `## Behaviour` sections into `behaviour` parts; flat spec and delta files are not supported. Release 1.1 follows hydra adoption (steps 1 to 3 of the acceptance): finding lines are file lines, the `[intent]` count, by-hand dry-run, `lemma receipt` and the lazyspec version check. Release 1 is built and has passed the five-step acceptance against a clone of the hydra repository; adopting it in hydra itself is outstanding. Release 1 ships the `spec`, `delta` and `decision` types; the `testing` and `code-change` dictums; `lemma validate`, the `in-progress` → `complete` gate, the fold, `lemma stop` with the cross-governs bound and pending-test read, and `lemma re-pin`; the `entry` and `delta` skills; a CI job running `lemma validate`. The release criterion is the five-step acceptance run in the hydra repository described in the RFC. Everything else in this document is release 2 or later.

## See also

- [docs/RFC-001-lemma.md](docs/RFC-001-lemma.md), the founding design
- [lazyspec](https://github.com/jkaloger/lazyspec), the engine: document types, `governs`, `@ref`, `[[hooks]]`
- [hydra](https://github.com/jkaloger/hydra), the interview tool whose trees are the long-form rationale record
