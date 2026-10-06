# lemma

A lazyspec workflow pack, a Claude Code plugin and a hooks binary for LLM-driven development. lemma verifies that implementation changes and specification changes stay mechanically connected. A delta is a set of claims; a claim names the test that settles it with an `@ref` line; a delta folds into its spec by claim id when it completes.

Design rationale is recorded in [docs/decisions/DECISION-001-lemma-is-a-signal-source-not-a-verifier.md](docs/decisions/DECISION-001-lemma-is-a-signal-source-not-a-verifier.md) and [docs/RFC-001-lemma.md](docs/RFC-001-lemma.md). This document states behaviour.

## Synopsis

```
lazyspec init --template https://github.com/jkaloger/lemma   # or a directory: /path/to/lemma
lazyspec hook trust
claude plugin install lemma@lemma
cargo install lemma

lemma check [--base <rev>] [--trunk <name>] [--require claims|evidence|clean]... [--json|--hook]
lemma apply [<delta>]
lemma refs <claim>
lemma stop
lemma session-start
lemma signal
lemma migrate
```

## Description

lemma has three parts, released together under one version number.

The **pack** is a `.lazyspec.toml`, a set of document templates, seed convention documents and a `[[hooks]]` table. `lazyspec init --template` copies it into a project, from a URL or from a directory checkout. The pack defines three document types in two families.

The **plugin** is a Claude Code plugin carrying two skills and the Claude Code hook entries. The lazyspec plugin is not a dependency.

The **binary** is a single Rust executable, `lemma`, with one subcommand per surface. lazyspec `[[hooks]]` entries and Claude Code hook commands compose its commands. It links the lazyspec engine as a crate. lemma holds no lifecycle knowledge: transitions are configured in `.lazyspec.toml`.

lemma is one signal source in a stack. It owns change coverage and spec integrity. The test runner and CI own test results, the compiler, linter and formatter own code conventions, repo hooks own project-specific rules (pending tests included), and human review owns semantic adequacy. lemma never runs tests and never judges whether a test still proves its claim.

## Document types

| Type       | Family        | Staleness | Statuses                               | Written by                                         |
| ---------- | ------------- | --------- | -------------------------------------- | -------------------------------------------------- |
| `spec`     | evergreen     | drift     | `draft` → `active`                     | humans and agents; claims by the fold only         |
| `delta`    | point-in-time | age       | `draft` → `in-progress` → `complete`   | agents and humans                                  |
| `decision` | point-in-time | age       | `proposed` → `accepted` → `superseded` | agents and humans; never edited after acceptance   |

A spec is `draft` while its `Intent` is proposed and no fold has been accepted, and `active` once it describes real behaviour. A human moves it; nothing automatic does.

A spec carries `governs` globs. A change under the globs moves the document to the `stale` band. A spec also carries `tests`, a list of globs naming its test files. When `tests` is absent it defaults to `**/tests/**`, `**/*_test.*`, `**/*.test.*` and `**/*.spec.*`.

A **spec** is one capability, a lazyspec bundle. Its `index.md` carries frontmatter and the `Intent` section, which is required and non-empty. Its `behaviour` part (`behaviour.md`, empty at creation) is the only place a tagged claim may appear; a claim-shaped item in `index.md` is an error. A spec whose `behaviour` part is empty is valid.

A **delta** is the unit of in-flight work and the only type a skill drives through its statuses. It is a bundle. Its `index.md` carries frontmatter, which links each spec it folds into, and `Intent`. Claims live in one part per fold target, named `behaviour.<spec-id>` (file `behaviour.<spec-id>.md`). Each fold target has its part, and each `behaviour.*` part names a fold target. An optional `tasks` part holds non-claim work; no hook reads it. A delta that targets a new capability creates that spec at `draft` with `Intent` only and links to it.

A **decision** records one consequential choice and links the spec it affects. A superseding decision links the one it replaces.

A bug is a draft delta whose first claim reproduces the defect. An idea is a draft spec with `Intent` only. Neither is a type.

A delta is **in flight** while it is `draft` or `in-progress`. In-flight deltas against a spec are found with `lazyspec list delta`.

## Claims and refs

A **claim** is a tagged list item with a stable id inside a `behaviour` part. The part is the section; it has no `## Behaviour` heading.

```
- [AC-C17-1] A reopened head keeps its prior answer until it is cut again.
  @ref src/store.rs#reopen_keeps_prior
```

Ids are namespaced by the delta that introduced them. The spec governing a delta claim is the target its part names; for a spec claim it is the spec. A claim carrying the marker `[intent]` is excluded from check 3. lemma reports the count per document as `<n> of <m> claims are [intent]`, in `--json` and `--hook` output only; text mode is silent when clean.

A **ref** is an `@ref <path>#<symbol>` line indented under a claim. It carries no commit and no hash. The path must match the `tests` globs of the governing spec, and the symbol must resolve in that file. A symbol in production code is not a ref. Code carries no spec ids. A ref is not evidence that the test ran or passed.

The **evidence region** of a ref is the referenced symbol, attributes included. A `.rs` file uses the Rust extractor; `.ts`, `.tsx`, `.js`, `.jsx`, `.mjs` and `.cjs` files use the TypeScript extractor and grammar. A file with no extractor is compared whole.

A **claim is immutable**: its text never changes in a spec. A delta changes behaviour by striking an id, written `- ~~[ID]~~`, and adding a new id. A new claim that replaces a struck one carries its `@ref` lines. A delta claim whose id already exists in the target spec and is not struck is an error: `claims are immutable; strike <id> and add a new id`.

A claim that no test can settle is marked `[intent]`. Adequacy of a test against its claim is judged by the human at `complete`.

A pinned ref (`@<sha>`, `@{blob:…}`), a ` [forced]` marker, an `@base` line and an `@checked` line are 1.x syntax. All four are errors. A pinned ref or ` [forced]` marker prints `pinned ref; run lemma migrate`, an `@base` line prints `` `@base` line; run lemma migrate``, and an `@checked` line prints `` `@checked` line; run lemma migrate``.

A human moving a delta `in-progress` → `complete` approves it. lazyspec writes a `reviewed: <sha>` stamp on every status change. Agents do not write it, and it does not record approval.

## Fold

The transition `in-progress` → `complete` runs `lemma apply` after the check gate. For each target spec, the fold appends the delta's claims whose ids are new and deletes the claims whose ids are struck. A struck id that the spec lacks is an error. A target spec with no `behaviour` part is an error and nothing is written. A target spec that is still `draft` produces a warning. Refs move with their claims.

The fold writes the merged claim list to the spec's `behaviour` part through the lazyspec `updates` contract (`{id, part: "behaviour", hash, body}`). It never creates a spec or a part.

## Checks

`lemma check` runs four checks.

| Check        | Reports                                                                                                                                                                                                   |
| ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1. Coverage  | The changed governed files in the range are not all within one area, and a touched spec is not covered. A spec is covered when an in-flight (`draft` or `in-progress`) delta folds into it, or a delta folding into it became `complete` within the range (its `index.md` changed in the range and was not `complete` at the base). A change is within one area when some spec governs every changed governed file, so overlapping globs do not count as two areas; a change inside one area needs no delta.                           |
| 2. Evidence  | For a claim present in a spec's `behaviour` part at the base, a ref whose evidence region was present at the base and is now absent or differs, unless a delta strikes that claim. Only a delta that folds into the claim's spec, in its `behaviour.<spec-id>` part, and is `in-progress` or became `complete` in the range, counts; a `draft` delta does not. At a pre-transition gate the moved delta counts as if already at its target status. A region absent at the base is not a change, so a ref that was already broken on trunk is left to check 3. Formatting and comments are normalised away. |
| 3. Claims have evidence | A live claim, not marked `[intent]`, with no ref that resolves to a symbol in a file matching the governing spec's `tests` globs.                                                              |
| 4. Integrity | A delta claim reusing a spec claim id without a strike; a struck id the spec lacks; a fold target that does not resolve; a fold target without its part; a part naming no fold target; a claim-shaped item in `index.md`; an invalid glob; a pinned ref; a spec's `behaviour` part edited outside a fold (below). |

Check 4 also compares each spec's `behaviour` part with the base. The claims in the working tree must equal the base claims folded with every delta that became `complete` in the range (folded in id order). A delta counts only when its `index.md` changed in the range and it was not `complete` at the base, so the transition's status change and spec fold must be committed together; a fold committed apart from its completion is reported as an edit outside a fold. Claims are compared as id and the whole claim block, ignoring `@ref`, `@base` and `@checked` lines, pins and `[intent]` markers. Any difference is an error on the spec part: `spec behaviour edited outside a fold; change it through a delta`. Each ref a claim should hold (pins ignored) must still be present; adding refs is allowed. A removed or repointed ref is an error: `<claim>: ref <path>#<symbol> removed from the spec outside a fold; change it through a delta`. A change in the range to a spec's `governs` or `tests` frontmatter is a warning: `governs/tests changed in this range; review`. A spec that did not exist at the base is not compared.

Two in-flight deltas striking one id produce a warning. Checks 2 and 4 and refs that do not resolve are errors. Check 3 is a warning unless required.

`--require` is repeatable and turns readiness into errors:

| Value      | Requires                                                                                          |
| ---------- | ------------------------------------------------------------------------------------------------- |
| `claims`   | `Intent` present, every fold target has its part, at least one claim across the parts             |
| `evidence` | Check 3 findings are errors                                                                       |
| `clean`    | No uncommitted changes in ref files or under a target's `governs` globs                           |

Output is `path:line: severity: message` lines, one per finding, with the next command where one applies. Output is empty when clean. `--json` prints `{"findings": [...]}`. `--hook` reads a lazyspec payload on stdin, prints findings JSON, exits 0 and is scoped to the payload's documents.

Hook mode and the range. The `validate` event has no transition and no range, so checks 1 and 2 are skipped; CI runs `lemma check --base` for them. A pre-transition gate (a payload with a transition) runs checks 1, 2 and the spec-edit rule of check 4 against the range for the moved delta, and limits their findings to the specs that delta folds into.

### Base resolution

The base is the revision that check 2 diffs against and that bounds the range for check 1.

1. `--base <rev>` is used when given.
2. Otherwise, with the trunk (`--trunk <name>`; by default the remote-tracking `origin/main`, else `origin/master`, else the target of `origin/HEAD`, together with local `main`, else `master`), if `git merge-base HEAD <trunk>` is not HEAD, the merge-base is used. When both a remote-tracking and a local trunk exist, the merge-base nearer to HEAD wins, so a stale local `main` or a merge of `origin/main` into a branch does not widen the range.
3. Otherwise HEAD is on trunk (direct-to-main). `lemma stop` and `lemma signal` use the session base recorded by `lemma session-start`. The transition gate (a payload with a transition) uses the parent of the commit that first added the moved delta's `index.md`, or HEAD when the delta is uncommitted. The `validate` event uses no base.
4. Otherwise HEAD.

CI passes `--base` explicitly: the pull request base ref, or on a push to the default branch the push's `before` sha when it is an ancestor of HEAD, else the parent of HEAD (no check when HEAD has none). A push to any other branch passes `--trunk origin/<default-branch>`. Check 2 needs the base reachable, so CI fetches full history. Local signals resolve against the local trunk.

## Commands

| Command                      | Invoked by                                         | Behaviour                                                              |
| ---------------------------- | -------------------------------------------------- | ---------------------------------------------------------------------- |
| `lemma check`                | agents, CI                                         | Runs checks 1 to 4. Exits 2 on an error finding.                       |
| `lemma check --hook`         | lazyspec `validate` and `pre-transition` events    | Reads a payload, prints findings JSON, exits 0.                        |
| `lemma apply`                | lazyspec `pre-transition` event, `in-progress` → `complete` | Folds each target and returns `updates`. Errors on check 4 and on a target with no `behaviour` part. |
| `lemma apply <delta>`        | agents                                             | Prints, per target, the ids it would add and delete. Writes nothing.   |
| `lemma refs <claim>`         | agents                                             | Prints `<claim> <path>#<symbol> ok\|unresolved\|not-a-test` per ref, across specs and in-flight deltas. |
| `lemma stop`                 | Claude Code `Stop` hook                            | Runs checks 1 to 3 against the session base, check 3 scoped to session-touched documents. Reports errors only. Exits 2 on error, diagnostics on stderr. |
| `lemma session-start`        | Claude Code `SessionStart` hook                    | Records the session base. Always exits 0.                              |
| `lemma signal`               | Claude Code `PostToolUse` hook                     | Returns per-edit context and findings. Never blocks.                   |
| `lemma migrate`              | humans, once                                       | Rewrites 1.x documents to 2.0 syntax.                                  |

### stop

`lemma stop` reads the base for its `session_id` as described under base resolution. Without a base file, or when the base is not an ancestor of HEAD, it diffs against HEAD. Its diff includes commits made in the session, tracked working-tree changes and untracked files. It reports errors only: checks 1 and 2 as in `lemma check`, check 3 scoped to the documents touched in the range (spec and delta files changed since the base, untracked included) and to specs whose `governs` cover a touched file, so a pre-existing unresolved ref in an untouched spec does not block a session. It also reports an error when a delta is `complete` in the working tree and was not `complete` at the session base (HEAD when there is no session base), including a delta new since then and one completed and committed in the session. The completion error is said once per session, recorded in the session's `.said` file, so a human who completes mid-session blocks only the first Stop after it. When `stop_hook_active` is set, it exits 0.

### session-start

`lemma session-start` reads `session_id` and `cwd` from stdin and writes HEAD's sha to `<git-dir>/lemma/sessions/<session_id>`. It keeps an existing file, so a resumed session keeps its original base. It prints nothing and exits 0 on any failure.

### signal

`lemma signal` reads the Claude Code `PostToolUse` payload for the Edit, Write and MultiEdit tools. It is scoped to the touched file and returns its output to the agent as `additionalContext`.

| Condition                                               | Output                                                                                   |
| ------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| First touch of a governed area in the session           | One line of pointers from the lazyspec graph: `<path> → SPEC-001 <title> · DECISION-001 <title>`. Ids and titles, no content. Deduped once per governed area per session. Dictums appear only when lazyspec `governing()` returns them (dictums with `governs`). |
| The file is referenced by a ref                         | Check 2 finding, naming the claim.                                                       |
| The file adds an uncovered area                         | Check 1 finding.                                                                         |
| The file is a spec or delta part                        | Check 3 and 4 findings. For a spec file, also the spec-edit rule and the `governs`/`tests` warning of check 4. |

Each message is said once per session, keyed on a stable identity rather than its text: `context:<ids of the governing documents>`, `check1:<spec-id>`, `check2:<claim>:<path>#<symbol>`, and `check4:<document>:<path>:<message>`. A later edit that changes the finding's wording (more files in the range, a shifted line number) does not say it again. Hashes of the keys are kept in `<git-dir>/lemma/sessions/<session_id>.said`. A clean edit produces no output. The base is resolved only when the file is governed, referenced by a ref, or a spec or delta file. `lemma signal` exits 0 silently on any failure.

### migrate

`lemma migrate` rewrites every spec and delta part in place. It strips the pin and ` [forced]` from `@ref` lines, deletes `@base` lines, rewrites a claim with an `@base` line in a `draft` or `in-progress` delta whose id exists in its target spec (a replacement; a `complete` delta only loses the line) as a strike of the old id plus the claim under `<old-id>-R` (`-R2`, `-R3` if taken, including ids assigned earlier in the same run), and turns a claim whose only receipts are `@checked` lines into an `[intent]` claim. It prints each file it wrote, followed by one indented line per replacement rewrite. It is idempotent.

## Hooks

The pack's `[[hooks]]` entries compose lemma commands. lazyspec runs the entries for one event in declaration order and stops at the first error, so `lemma apply` never writes after a failed check.

| Event            | Transition                   | Runs                                                                |
| ---------------- | ---------------------------- | ------------------------------------------------------------------- |
| `validate`       |                              | `lemma check --hook`                                                |
| `pre-transition` | `draft` → `in-progress`      | `lemma check --hook --require claims`                               |
| `pre-transition` | `in-progress` → `complete`   | `lemma check --hook --require evidence --require clean`, then `lemma apply` |

The `validate` entry applies to `spec`, `delta` and `decision`. The `pre-transition` entries apply to `delta` and load `spec` and `delta` as context. `lazyspec validate` exits 2 when any finding has error severity.

The plugin's `hooks.json` registers three Claude Code hooks, each a no-op when `lemma` is not on `PATH`.

| Hook           | Matcher                  | Runs                  |
| -------------- | ------------------------ | --------------------- |
| `SessionStart` |                          | `lemma session-start` |
| `PostToolUse`  | `Edit\|Write\|MultiEdit` | `lemma signal`        |
| `Stop`         |                          | `lemma stop`          |

Signals escalate: the same checks run per edit (said once, never blocking), at Stop (error, scoped to the session), and at the transition gate and in CI (error).

## Skills

| Skill         | Covers                                                                                                                                  |
| ------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| `lemma:entry` | Dispatch. If an in-progress delta exists, resumes it under `delta`. Otherwise routes by the prompt. Does not classify fix, chore or refactor. |
| `lemma:delta` | build, fix, chore, refactor, spike, adopt                                                                                               |

Each `delta` workflow is a reference file loaded on demand. Orientation prompts (explain, orient, investigate, review) have no skill.

The `adopt` workflow is the first delta into a spec whose `behaviour` part is empty, over code that already has tests. Its claims state what the existing tests settle, its refs name existing test symbols, a characterisation test is added only for a claim with none, and production code does not change.

A chore stays inside one `governs` area and does not change a test referenced by a landed spec claim; check 2 reports it. A change to such a test is a delta that strikes the claim and adds a new id.

The human decision point is the transition `in-progress` → `complete`. The agent drives `draft` → `in-progress` and works until every claim has a ref. Readiness for review is derived: the delta is `in-progress` and `lemma check --require evidence --require clean` is clean. The fold runs with the transition to `complete`.

## Files

| Path                    | Contents                                                                                           |
| ----------------------- | -------------------------------------------------------------------------------------------------- |
| `.lazyspec.toml`        | type definitions, `[governs]`, `[[hooks]]` entries pointing at `lemma`                             |
| `.lazyspec/templates/`  | one template per type, with the spec `tests` key; `spec/` (`index.md`, `behaviour.md`) and `delta/` (`index.md`) are directory templates |
| `docs/convention/`      | seed `CONVENTION` and dictums `DICTUM-001-testing`, `DICTUM-002-code-change`                       |
| `plugin/`               | Claude Code plugin: skills (the delta skill's `references/` hold one file per workflow) and hook entries |
| `crates/lemma/`         | the binary                                                                                         |
| `docs/decisions/`       | DECISION-001, the scope of 2.0                                                                     |
| `docs/RFC-001-lemma.md` | founding design and rationale                                                                      |
| `docs/ci/lemma-validate.yml` | CI job template                                                                               |

## Exit status

`lemma check` exits 2 when any finding has error severity, and 0 otherwise. With `--hook` it exits 0 and lazyspec applies severity. `lemma apply` without an argument prints `updates` and findings as JSON and exits 0; with an argument it exits 2 on error. `lemma refs` exits 2 when a ref is `unresolved` or `not-a-test`. `lemma stop` exits 2 on an error finding and writes diagnostics to stderr. `lemma session-start`, `lemma signal` and `lemma migrate` exit 0 on success. `lemma stop` prints a hint to stderr and exits 0 when `lazyspec --version` is missing or older than 0.15; `lemma signal` does not check the version and exits 0 silently on any failure; the other commands exit 2.

## Examples

Adopt the pack in a repository and trust its hooks. A directory replaces the URL for a local checkout:

```
lazyspec init --template https://github.com/jkaloger/lemma
lazyspec init --template /path/to/lemma
lazyspec hook trust
```

Check the working tree against the merge-base with `main`:

```
lemma check
```

Check readiness for review:

```
lemma check --require evidence --require clean
```

Check a pull request in CI:

```
lemma check --base origin/main
```

List the refs of a claim:

```
lemma refs AC-C17-1
```

Preview the fold of a delta:

```
lemma apply C17
```

Migrate a 1.x repository:

```
lemma migrate
```

## Migration from 1.x

2.0 is a format break. `lemma migrate` rewrites documents in place. The `validate`, `pre-transition`, `re-pin`, `receipt` and `base` subcommands are removed, along with receipt freshness, symbol drift, `[forced]`, `@checked`, `@base` and pending-test detection. The pack's `[[hooks]]` entries change to those under Hooks; an adopting repository replaces its `.lazyspec.toml` hooks and `hooks.json` with the 2.0 pack and plugin.

## Install

Each part uses its native channel.

```
lazyspec init --template https://github.com/jkaloger/lemma   # pack; or a directory: /path/to/lemma
claude plugin install lemma@lemma                          # skills and Claude Code hooks
cargo install lemma                                          # binary
```

Add `.lazyspec/cache/` to `.gitignore`. Hook trust is per machine: each clone runs `lazyspec hook trust`, and CI runs it before `lazyspec validate`, as `docs/ci/lemma-validate.yml` does.

The skills and the SessionStart, PostToolUse and Stop hooks load only after the Claude Code session is restarted.

lemma requires lazyspec 0.15 or later.

The binary is built with `opt-level = "z"`, fat LTO, one codegen unit, `panic = "abort"`, stripped.

## Status

Release 2.0 implements DECISION-001. Refs are unpinned and claims are immutable. `lemma check` replaces `lemma validate` and `lemma pre-transition`, and `lemma apply` and `lemma refs` join it. `lemma signal` adds per-edit signals and context pointers. `lemma migrate` converts 1.x documents. Extractor fixes (Rust attributes, `it`/`test` blocks by title) belong to lazyspec; lemma uses the 0.15 extractors as they are.

## See also

- [docs/decisions/DECISION-001-lemma-is-a-signal-source-not-a-verifier.md](docs/decisions/DECISION-001-lemma-is-a-signal-source-not-a-verifier.md), the scope of 2.0
- [docs/RFC-001-lemma.md](docs/RFC-001-lemma.md), the founding design
- [lazyspec](https://github.com/jkaloger/lazyspec), the engine: document types, `governs`, `@ref`, `[[hooks]]`
- [hydra](https://github.com/jkaloger/hydra), the interview tool whose trees are the long-form rationale record
