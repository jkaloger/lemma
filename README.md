# lemma

A lazyspec workflow pack, a Claude Code plugin and a hooks binary for LLM-driven development. A delta is a set of claims; a claim is complete when a receipt names the test that settles it; a delta folds into its spec by claim id when it completes.

Design rationale is recorded in [docs/RFC-001-lemma.md](docs/RFC-001-lemma.md). This document states behaviour.

## Synopsis

```
lazyspec init --template https://github.com/jkaloger/lemma   # or a directory: /path/to/lemma
lazyspec hook trust
claude plugin install lemma@lemma
cargo install lemma

lemma validate
lemma pre-transition
lemma stop
lemma session-start
lemma re-pin <id> [claim-id ...] [--force]
lemma receipt <delta> <claim-id> <path#symbol>
lemma base <delta> [claim-id ...]
```

## Description

lemma has three parts, released together under one version number.

The **pack** is a `.lazyspec.toml`, a set of document templates, two dictum files and a `[[hooks]]` table. `lazyspec init --template` copies it into a project, from a URL or from a directory checkout. The pack defines three document types in two families.

The **plugin** is a Claude Code plugin carrying two skills and the Claude Code hook entries. Each skill speaks claims, receipts, fold and approval directly. The lazyspec plugin is not a dependency.

The **binary** is a single Rust executable, `lemma`, with one subcommand per enforcement surface. lazyspec `[[hooks]]` entries and Claude Code hook commands both invoke it. It links the lazyspec engine as a crate. It is not invoked by hand in normal use.

lemma never runs tests. It checks that each claim names a test symbol that resolves and is fresh. The codebase's own test runner, pre-commit and CI own pass and fail.

## Document types

| Type         | Family        | Staleness | Statuses                                        | Written by                              |
| ------------ | ------------- | --------- | ----------------------------------------------- | --------------------------------------- |
| `spec`       | evergreen     | drift     | `draft` → `active`                              | humans and agents; claims by the fold only |
| `delta`      | point-in-time | age       | `draft` → `in-progress` → `complete`           | agents and humans                       |
| `decision`   | point-in-time | age       | `proposed` → `accepted` → `superseded`          | agents and humans; never edited after acceptance |

A spec is `draft` while its `Intent` is proposed and no fold has been accepted, and `active` once it describes real behaviour. A human moves it; nothing automatic does.

A spec carries `governs` globs. A change under the globs moves the document to the `stale` band. A spec also carries `tests`, a list of globs naming its test files. When `tests` is absent it defaults to `**/tests/**`, `**/*_test.*`, `**/*.test.*` and `**/*.spec.*`.

A **spec** is one capability, a lazyspec bundle. Its `index.md` carries frontmatter and the `Intent` section, which is required and non-empty. Its `behaviour` part (`behaviour.md`, empty at creation) is the only place a tagged claim may appear; a claim-shaped item in `index.md` is an error. A spec whose `behaviour` part is empty is valid.

A **delta** is the unit of in-flight work and the only type a skill drives through its statuses. It is a bundle. Its `index.md` carries frontmatter, which links each spec it folds into, and `Intent`. Claims, as they will appear in the spec, live in one part per fold target, named `behaviour.<spec-id>` (file `behaviour.<spec-id>.md`). Each fold target has its part, and each `behaviour.*` part names a fold target. An optional `tasks` part holds non-claim work; no hook reads it. A delta that targets a new capability creates that spec at `draft` with `Intent` only and links to it.

A **decision** records one consequential choice and links the spec it affects. A superseding decision links the one it replaces.

A bug is a draft delta whose first claim reproduces the defect. An idea is a draft spec with `Intent` only. Neither is a type.

## Claims and receipts

A **claim** is a tagged list item with a stable id inside a `behaviour` part. The part is the section; it has no `## Behaviour` heading.

```
- [AC-C17-1] A reopened head keeps its prior answer until it is cut again.
  @ref src/store.rs#reopen_keeps_prior@3f9c2e1
```

Ids are namespaced by the delta that introduced them. The spec governing a delta claim is the target its part names; for a spec claim it is the spec. A claim carrying the marker `[intent]` is excluded from the receipt check. `lemma validate` reports the count per document as `<n> of <m> claims are [intent]`. lazyspec findings have no info severity, so the finding is emitted as a warning.

A delta claim that replaces a spec claim, or strikes one, carries an indented `@base <hash>` line. The hash is the first 12 hex characters of the SHA-256 of the spec claim's block as it stood when the delta claim was written. The block is the claim's lines, each right-trimmed, with the id line left-trimmed and any `@base` line excluded, joined by newline. `lemma base` writes the line.

A **receipt** is an `@ref` line under a claim naming a test symbol, pinned at a commit, or an `@checked <name>@<sha>` line recording that a human checked the claim at a commit. An `@ref` receipt is **fresh** when neither the test file nor the code under the spec's `governs` globs has changed since the pin; an `@checked` receipt is fresh when nothing under the `governs` globs has changed since `<sha>`. Otherwise a receipt is **stale**. `lemma receipt` never writes `@checked`; humans do.

An `@ref` line may end with ` [forced]` after the sha. `lemma re-pin --force` writes it when it re-pins a receipt whose test changed under an unchanged claim. `re-pin` and `lemma receipt` keep it when they rewrite the line, the fold carries it into the spec, and only a hand edit removes it. A forced receipt is still a receipt; `lemma validate` and the `in-progress` → `complete` gate warn on it until a human reviews the test against the claim and removes the marker or adds `@checked`.

An `@ref` path must match the `tests` globs of the governing spec. A symbol in production code is not a receipt. `@checked` has no path and is exempt. A receipt is a resolving symbol in a test file at a commit. It is not evidence that the test ran or passed; the project's test runner and the human reviewer own that.

A claim that no machine check can settle is marked `[intent]`.

A human moving a delta `in-progress` → `complete` approves it. lazyspec writes a `reviewed: <sha>` stamp on every status change, which records the commit the document was last moved at; agents do not write it, and it does not record approval.

In-flight deltas against a spec are found with `lazyspec list delta`, where both `draft` and `in-progress` deltas are in flight; the spec's `related` does not list them.

## Fold

The transition `in-progress` → `complete` runs the fold, in the same response as the gate and only when the gate has no errors. For each target spec, the hook takes the claims in the delta's `behaviour.<spec-id>` part, appends a claim whose id is new, replaces the claim whose id exists, and deletes a claim whose id is struck through. A struck claim needs no receipt. The fold warns when a target spec is still `draft`. Receipts move with their claims. `@base` lines are removed before a claim is written into the spec. The fold errors when the `@base` hash of a replacing or striking claim differs from the hash of the spec's current claim block, and the finding names both hashes. It errors when a claim's id already exists in the spec and the claim has no `@base`, so an id collision cannot overwrite a spec claim. It errors when a claim carries `@base` and its id is absent from the spec. The hook writes the merged claim list to the spec's `behaviour` part through the lazyspec `updates` contract (`{id, part: "behaviour", hash, body}`) and never creates a spec or a part. A target spec with no `behaviour` part is an error finding and nothing is written.

## Hooks binary

| Subcommand                   | Invoked by                                  | Severity              |
| ---------------------------- | ------------------------------------------- | --------------------- |
| `lemma validate`             | lazyspec `validate` event, CI               | error for invariants, warning for drift |
| `lemma pre-transition`       | lazyspec `pre-transition` event             | error                 |
| `lemma stop`                 | Claude Code `Stop` hook                     | error, with the hook's escape |
| `lemma session-start`        | Claude Code `SessionStart` hook             | none, always exits 0  |
| `lemma re-pin <id> [claim-id ...] [--force]` | agents, after resolving drift | refuses, or warns under `--force` |
| `lemma receipt <delta> <claim-id> <path#symbol>` | agents, after a claim's test is committed | refuses |
| `lemma base <delta> [claim-id ...]` | agents, after writing a replacing or striking claim | refuses |

Severity is fixed by surface. There is no configuration table.

`lemma validate` reads documents as JSON on stdin. The errors are invariants: a receipt that does not resolve; a fold target that does not resolve; a fold target without its `behaviour.<spec-id>` part, which is a warning while the delta is `draft`; a `behaviour.*` part that names no fold target; a claim-shaped item in an `index.md`; a replacing or striking claim without `@base`; a striking claim whose id the spec lacks; an `@ref` path outside the governing spec's `tests` globs. The warnings are drift: a claim without a receipt on an in-progress delta (the `complete` gate keeps it an error, so a feature-branch push is not red); a forced receipt on an in-progress delta or a spec; a stale receipt; an `@base` that does not match the spec claim's current block, or an `@base` on an id the spec lacks, on a draft or in-progress delta; two in-flight deltas, `draft` or `in-progress`, touching one claim id; the count of `[intent]` claims, one finding for each document with at least one `[intent]` claim. Findings on claims and receipts carry `part` and the line in the part file, which has no frontmatter. Findings on `index.md` carry the line in the file. Native lazyspec validation (required sections, relationships, staleness bands, source ownership) is relied on and not repeated. `lazyspec validate` exits 2 when any finding has error severity. Warnings are shown only with `--warnings`.

`lemma pre-transition` applies per transition. `draft` → `in-progress`: `Intent` present, fold targets resolve, every fold target has its `behaviour.<spec-id>` part, every `behaviour.*` part names a fold target, at least one claim across the parts, every replacing or striking claim has `@base`, and no striking claim names an id the spec lacks. `in-progress` → `complete`: fold targets resolve and each has a `behaviour` part, every claim not marked `[intent]` has a resolving, fresh receipt whose `@ref` path matches the governing spec's `tests` globs, no receipt file and no file under a target's `governs` has uncommitted changes (untracked files included), and the fold runs. Uncommitted changes in files outside those paths do not block the gate. The gate warns, without blocking, for each forced receipt (`<claim>: receipt was force re-pinned; review the test against the claim, then remove [forced] or add @checked`) and for each target spec that is `draft` (`<spec> is draft; move it to active once this fold lands`); the warnings come back with the fold's updates. A fold target loaded from the store rather than the hook payload cannot be written and is an error.

`lemma session-start` reads `session_id` and `cwd` from stdin and writes HEAD's sha to `<git-dir>/lemma/sessions/<session_id>`, the session's base. It keeps an existing file, so a resumed session keeps its original base, and it prints nothing and exits 0 on any failure. `lemma stop` reads the base for its `session_id`; the session's diff is `git diff --name-only <base> HEAD` together with tracked working-tree changes and untracked files, so a commit made inside the session still counts. Without a base file, or when the base is not an ancestor of HEAD, it diffs against HEAD.

`lemma stop` reports an error when the session's diff spans more than one `governs` area and no in-progress delta covers it; when a touched test file contains a pending test (`it.todo`, `#[ignore]` without a reason); when a delta is `complete` in the working tree and was not `complete` at HEAD, including a delta new since then. The diff bound runs against the session base; the completion rule runs against HEAD, so a human who completes and commits mid-session is not flagged on later Stops. A delta that became `complete` since the base covers its specs in the `governs` check.

`lemma re-pin <id> [claim-id ...] [--force]` updates the `@ref` receipt commits in every `behaviour*` part of one document. When claim ids are named, only those claims are re-pinned. When none are named, every claim is. An unknown claim id is refused. It never touches `@checked` lines; a stale `@checked` stays stale until a human re-signs. It refuses, writing nothing, when a referenced symbol no longer resolves. It refuses, writing nothing, when a receipt's file has uncommitted changes, whether or not `--force` is given. A receipt has drifted when the text of its symbol at the pinned commit differs from its text at HEAD, or the symbol is absent at the pinned commit, while the claim's text lines (the block minus `@ref`, `@checked` and `@base` lines) did not change in a commit that also touched the receipt file and are not edited in the working tree. The symbol is extracted with the lazyspec extractors for `.rs`, `.ts` and `.tsx`; a file with no extractor is compared whole. A sibling test added to the file, or a doc-comment edit, is not drift, so `lemma re-pin` at review-ready is routine. When a receipt has drifted, `lemma re-pin` refuses, writing nothing; with `--force` it re-pins, writes ` [forced]` after the sha, and emits a warning. `--force` is for a changed test under an unchanged claim. An existing ` [forced]` marker is kept.

`lemma receipt <delta> <claim-id> <path#symbol>` finds the claim id across the delta's `behaviour.*` parts and inserts an `@ref` line, in that part file, as the first indented line under the claim, pinned at the short HEAD sha. It refuses, writing nothing, when the claim id is live in no part of the delta, when it is live in more than one, when the symbol does not resolve in the working tree, when the file has uncommitted changes, or when the path does not match the governing spec's `tests` globs. It refuses a document that is not a delta: spec receipts arrive by the fold and humans write `@checked`. When the claim's id exists in the target spec and the claim has no `@base`, it also inserts `@base`. A test already committed at HEAD needs no further commit. Re-running for the same `<path>#<symbol>` replaces that receipt in place, re-pinning it to the new HEAD, instead of adding a second `@ref`, unless that receipt has drifted (as for `re-pin`), when it refuses and names `lemma re-pin <delta> <claim-id> --force`. A forced marker on the replaced line is kept.

`lemma base <delta> [claim-id ...]` writes `@base <hash>` as an indented line under each named claim, computed from the current `behaviour` block of the claim's target spec. An existing `@base` line is replaced. When no claim ids are named, it bases every claim of the delta whose id exists in its target spec. It refuses, writing nothing, on an unknown claim id, on an id the target spec lacks, on a target that does not resolve, and on a target with no `behaviour` part.

`lemma re-pin`, `lemma receipt` and `lemma base` add `"wrote": [{"id", "part", "line", "text"}]` to their JSON output, one entry for each line they wrote or rewrote. `line` is the 1-based line in the part file and `text` is the written line. The key is omitted when nothing was written, including on a refusal.


lazyspec invokes `lemma pre-transition` with the same status for `from` and `to` when run by hand with `lazyspec hook run pre-transition <id> --dry-run`. lemma then evaluates the gate on the next edge from that status (`draft` → `in-progress`, `in-progress` → `complete`, including the clean-tree check) and reports findings without updating anything. At `complete` it does nothing. When there are no findings it emits the warning `checked <from> → <to>: ready`. At `in-progress` it also previews the fold as warnings (`would fold <n> claims into <spec>`) and writes nothing, and warns for each uncommitted file of the delta bundle (`<file> has uncommitted changes; commit the delta before asking for review`). That is a warning, not an error, because the real transition itself edits `index.md`; `checked … ready` still follows when there is no error.

## Skills

| Skill            | Covers                                                                         |
| ---------------- | ------------------------------------------------------------------------------ |
| `lemma:entry`    | Dispatch. If an in-progress delta exists, resumes it under `delta`. Otherwise routes by the prompt. Does not classify fix, chore or refactor. |
| `lemma:delta`    | build, fix, chore, refactor, spike, adopt                                      |

Each `delta` workflow (build, fix, chore, refactor, spike, adopt) is a reference file loaded on demand. Orientation prompts (explain, orient, investigate, review) have no skill.

The `adopt` workflow is the first delta into a spec whose `behaviour` part is empty, over code that already has tests. Its claims state what the existing tests settle, its receipts name existing test symbols, a characterisation test is added only for a claim with none, and production code does not change.

The human decision point is the transition `in-progress` → `complete`. The agent drives `draft` → `in-progress` and works until every claim has a receipt. Readiness for review is derived: `in-progress`, every claim receipted, governed code and receipt files committed, validate clean. The fold runs with the transition to `complete`.

## Files

| Path                                | Contents                                              |
| ----------------------------------- | ----------------------------------------------------- |
| `.lazyspec.toml`                    | type definitions, `[governs]`, `[[hooks]]` entries pointing at `lemma` |
| `.lazyspec/templates/`              | one template per type, with the spec `tests` key; `spec/` (`index.md`, `behaviour.md`) and `delta/` (`index.md`) are directory templates |
| `.lazyspec/dictums/`                | `testing`, `code-change`                              |
| `plugin/`                           | Claude Code plugin: skills (the delta skill's `references/` hold one file per workflow, including `adopt.md`) and hook entries |
| `crates/lemma/`                     | the hooks binary                                      |
| `docs/RFC-001-lemma.md`             | founding design and rationale                         |

## Exit status

`lemma validate` and `lemma pre-transition` print `{"findings": [...]}` and exit 0; lazyspec applies severity. `lemma stop` exits 2 on an error finding. `lemma re-pin` exits 2 when it refuses, including for a receipt file with uncommitted changes under `--force`, and 0 otherwise, with warnings only under `--force`. `lemma receipt` and `lemma base` exit 2 when they refuse; `lemma receipt` refuses a document that is not a delta and a drifted receipt. `lazyspec validate` exits 2 when any finding from `lemma validate` has error severity. `lemma re-pin`, `lemma receipt` and `lemma base` exit 2 when `lazyspec --version` is missing or older than 0.15; `lemma stop` prints the hint to stderr and exits 0.

## Examples

Adopt the pack in a repository and trust its hooks. Pass a directory in place of the URL for a local checkout:

```
lazyspec init --template https://github.com/jkaloger/lemma
lazyspec init --template /path/to/lemma
lazyspec hook trust
```

Check the gate on a delta's next transition without changing its status. A clean run prints `checked in-progress → complete: ready`:

```
lazyspec hook run pre-transition C17 --dry-run
```

Re-pin a delta's receipts after resolving drift, or only the named claims, or past a refusal:

```
lemma re-pin C17
lemma re-pin C17 AC-C17-1
lemma re-pin C17 AC-C17-1 --force
```

Pin a receipt for a committed test:

```
lemma receipt C17 AC-C17-1 src/store.rs#reopen_keeps_prior
```

Write `@base` for every claim of a delta whose id exists in its target spec, or for one claim:

```
lemma base C17
lemma base C17 AC-C17-1
```

## Install

Each part uses its native channel.

```
lazyspec init --template https://github.com/jkaloger/lemma   # pack; or a directory: /path/to/lemma
claude plugin install lemma@lemma                          # skills and Claude Code hooks
cargo install lemma                                          # hooks binary
```

Add `.lazyspec/cache/` to `.gitignore`. Hook trust is per machine: each clone runs `lazyspec hook trust`, and CI runs it before `lazyspec validate`, as `docs/ci/lemma-validate.yml` does.

The skills and the SessionStart and Stop hooks load only after the Claude Code session is restarted. Restart before any delta work.

lemma requires lazyspec 0.15 or later. `lemma stop`, `lemma re-pin`, `lemma receipt` and `lemma base` check `lazyspec --version`. When lazyspec is missing or older than 0.15, `re-pin`, `receipt` and `base` exit 2; `stop` only warns on stderr and does not block the turn.

The binary is built with `opt-level = "z"`, fat LTO, one codegen unit, `panic = "abort"`, stripped.

## Status

Release 1.5 follows the second half of hydra adoption. Drift is measured on the test symbol, so re-pinning after a sibling test or a doc-comment edit in the receipt file needs no `--force`; `--force` is for a changed test under an unchanged claim. A `--force` re-pin writes a `[forced]` marker that validate and the gate report. `lemma receipt` is delta-only. `receipt`, `re-pin` and `base` report what they wrote. A missing receipt on an in-progress delta is a warning. Overlap counts drafts. The dry-run names uncommitted delta files. Stop diffs against the session base recorded by a `SessionStart` hook. A fold into a draft spec reminds the human to activate it. The `adopt` workflow joins the delta skill. Crate and plugin versions track the README. Release 1.4 closes the gaps found in review. The `in-progress` → `complete` gate refuses a dirty tree under receipt files and `governs`. Replacing and striking claims carry `@base`, which the fold checks and `lemma base` writes. `lemma validate` reports invariants as errors, so the CI job fails on them. `lemma re-pin` works per claim and refused a receipt whose file changed under an unchanged claim (1.5 narrows this to the test symbol) unless `--force` is given, and refuses a receipt file with uncommitted changes outright. Receipt paths must match the spec's `tests` globs. Release 1.3 cuts the sign-off vocabulary to claims, receipts and `[intent]`: approval is the human's `in-progress` → `complete` transition, `@checked` is the human receipt, and `complete` runs the fold. Release 1.2 makes specs and deltas bundles and moves claims out of `## Behaviour` sections into `behaviour` parts; flat spec and delta files are not supported. Release 1.1 follows hydra adoption (steps 1 to 3 of the acceptance): finding lines are file lines, the `[intent]` count, by-hand dry-run, `lemma receipt` and the lazyspec version check. Release 1 is built and has passed the five-step acceptance against a clone of the hydra repository; adopting it in hydra itself is outstanding. Release 1 ships the `spec`, `delta` and `decision` types; the `testing` and `code-change` dictum files; `lemma validate`, the `in-progress` → `complete` gate, the fold, `lemma stop` with the cross-governs bound and pending-test read, and `lemma re-pin`; the `entry` and `delta` skills; a CI job running `lemma validate`. The release criterion is the five-step acceptance run in the hydra repository described in the RFC.

Release 2 and later add: the `convention` and `dictum` document types, with the convention injected as the prompt preamble by `lemma prompt` and each dictum rule a tagged claim receipted by a lint or architecture test; the `shape`, `verify`, `maintain` and `operate` skills; a validate error for a spec with no rationale link; a validate finding for an evergreen spec with no `governs`; and a validate finding for contract mutation, meaning a file named by a receipt changed while no in-progress delta replaces or removes that claim. None of these is built.

## See also

- [docs/RFC-001-lemma.md](docs/RFC-001-lemma.md), the founding design
- [lazyspec](https://github.com/jkaloger/lazyspec), the engine: document types, `governs`, `@ref`, `[[hooks]]`
- [hydra](https://github.com/jkaloger/hydra), the interview tool whose trees are the long-form rationale record
