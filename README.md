# lemma

A lazyspec workflow pack, a Claude Code plugin and a hooks binary for LLM-driven development. A human writes a delta: intent, design and behaviour. lemma reviews it with an agent, plans it into tasks, runs each task in a fresh agent and commits it once its claims are proved by tests. Specs hold intent only. Nothing folds: a completed delta is the record of why its tests say what they say.

Design rationale is recorded in [DECISION-001](docs/decisions/DECISION-001-lemma-is-a-signal-source-not-a-verifier.md), [DECISION-003](docs/decisions/DECISION-003-nothing-folds-deltas-are-the-record-specs-hold-intent.md) and [RFC-001](docs/RFC-001-lemma.md). This document states behaviour. [docs/examples/tally](docs/examples/tally/README.md) shows the workflow on a small project.

## Synopsis

```
lazyspec init --template https://github.com/jkaloger/lemma   # or a directory: /path/to/lemma
lazyspec hook trust
claude plugin install lemma@lemma
cargo install lemma

lemma lint [<delta>] [--gate in-progress|complete] [--base <rev>] [--trunk <name>] [--json|--hook]
lemma review <delta> [--dry-run]
lemma plan <delta> [--dry-run]
lemma build <delta> [--task T<n>] [--dry-run]
lemma refs <claim>
lemma session-start | lemma signal | lemma stop
```

## Description

lemma has three parts, released together under one version number. This document describes 3.0.0.

The **pack** is a `.lazyspec.toml`, document templates, prompt files, seed convention documents and a `[[hooks]]` table. `lazyspec init --template` copies it into a project.

The **plugin** is a Claude Code plugin carrying hook entries only.

The **binary** is a single Rust executable, `lemma`. It links the lazyspec engine as a crate. Lifecycle transitions are configured in `.lazyspec.toml`; lemma holds no lifecycle knowledge. Orchestration runs agents as child processes and reads their JSON output; there is no shell layer.

lemma is one signal source in a stack. It owns change coverage, document shape and the connection between claims and tests. The test runner and CI own test results. Human review owns whether a test proves its claim. lemma never runs tests.

## Document types

| Type         | Statuses                               | Written by                                 |
| ------------ | -------------------------------------- | ------------------------------------------ |
| `spec`       | `draft` → `active`                     | humans                                     |
| `delta`      | `draft` → `in-progress` → `complete`   | humans; agents annotate, plan and build    |
| `decision`   | `proposed` → `accepted` → `superseded` | humans and agents; not edited once accepted |
| `convention` | `draft` → `accepted`                   | humans                                     |
| `dictum`     | `draft` → `accepted`                   | humans                                     |

A **spec** is an `index.md` with `## Intent`, `## Constraints`, `## Non-goals` and `## Open questions`, a `governs` list of globs and an optional `tests` list of globs. It holds no claims. A change under its `governs` makes it drift-stale. When `tests` is absent it defaults to `**/tests/**`, `**/*_test.*`, `**/*.test.*` and `**/*.spec.*`.

A **delta** is the unit of work. It links each spec it changes with the relationship `changes` (`related: [- changes: SPEC-002]`); the edge `delta-changes-spec` is required. A delta is **in flight** while `draft` or `in-progress`. A `complete` delta is immutable.

A **decision** records one choice and links what it affects or constrains. A superseding decision links the one it replaces.

The **convention** and **dictums** state the rules a project holds every change to. A dictum's rules are claims, edited by hand. When a test a rule refs must change, the rule is struck (`- ~~[ID]~~`) and a new id added in the same change.

## Delta parts

A delta is a bundle of four parts.

| Part        | Contents                                                                                        |
| ----------- | ----------------------------------------------------------------------------------------------- |
| `index.md`  | Frontmatter (`changes`, `related`), `## Intent`, `## Out of scope`                              |
| `design.md` | Prose. An `@ref` line here is a shape error                                                     |
| `claims.md` | Claims. The part is the section; it has no heading                                              |
| `tasks.md`  | An ordered checklist, then an optional `## Follow-ups` section with the same grammar             |

A **claim** is a list item with an id `AC-<n>`, unique within the delta:

```
- [AC-1] The balance of an account is the sum of its entries in minor units.
  @ref tests/ledger.rs#balance_sums_entries
- [AC-4] [intent] A balance read of one account costs one query regardless of entry count.
```

A claim is written first as a plain bullet without an id; `lemma plan` assigns ids. The markers `[intent]` (no test can settle the claim) and `[proposed]` (added by the planner, awaiting the human) follow the id and are mutually exclusive. A **ref** is an `@ref <path>#<symbol>` line indented under a claim, naming a test file that matches a changed spec's `tests` globs and a symbol in it. An `@per <citation>` line is read as a citation and otherwise ignored. A claim is cited outside its delta as `DELTA-004/AC-2`; a dictum rule by its id.

A **task** settles claims or is a chore:

```
- [ ] T1 add `Balance` type and the `SUM` query (settles AC-1)
- [ ] T3 wire route into the router (chore)

## Follow-ups
- [ ] T4 page the read in `balance()` (settles AC-2)
```

Ids `T<n>` are unique within the delta. A checkbox line that does not match this grammar is a shape error.

A **review line** is an annotation an agent writes and a human deletes to resolve: `@review <text>` indented under a claim or task, or `> review: <text>` as a blockquote at the end of a section.

## Workflow

1. A human creates the delta, writes `Intent`, `Out of scope`, design prose and claims as plain bullets, and links the specs it changes.
2. `lemma review <delta>` on a `draft` delta runs the reviewer. Findings are written as review lines. The human edits the delta, deletes each review line it resolves and runs `lemma review` again. Review never plans; the human decides when review is done.
3. `lemma plan <delta>` plans a `draft` delta with no review lines left: bullets receive ids, missing behaviour is appended as `[proposed]` claims and tasks are appended to `tasks.md`. The human reads the diff, deletes or accepts each `[proposed]` tag, and moves the delta on with `lazyspec update <id> --status in-progress`, or lets `lemma build` offer the move.
4. `lemma build <delta>` runs each unticked task in a fresh runner and commits it.
5. `lemma review <delta>` on an `in-progress` delta reviews the diff and writes follow-up tasks. The human deletes those it rejects; `lemma build` runs the rest.
6. `lazyspec update <id> --status complete` moves the delta to `complete`.

```
$ lemma review DELTA-004
DELTA-004 is draft: reviewing the document
3 review lines written
next: resolve each review line (edit, then delete the line) and run lemma review DELTA-004 again, or lemma plan DELTA-004 once none remain

$ lemma review DELTA-004
DELTA-004 is draft: reviewing the document
review clean
next: lemma plan DELTA-004

$ lemma plan DELTA-004
review clean, planning
claims.md: 4 claims tagged AC-1..AC-4, 1 [proposed]
tasks.md: 3 tasks written
index.md: frontmatter and headings tidied
next: read the diff, accept or reject each [proposed] claim, then lazyspec update DELTA-004 --status in-progress
```

The human types four commands: `lemma review`, `lemma plan`, `lemma build` and `lazyspec update`. Every lemma command that does work ends with a `next:` line naming the next step; errors, `--dry-run` prompts, `lint`, `refs` and the hooks print none.

## Commands

### lint

`lemma lint` runs the coverage, history and dictum checks on the working tree against the base, and check 3 on dictums. Given a delta, it also runs the evidence and shape checks for that delta. `--gate <status>` adds the rules of the transition into `in-progress` or `complete`, and requires a delta outside hook mode. Output is `path:line: severity: message`, one line per finding, empty when clean. `--json` prints `{"findings": [...]}`. Exit status is 2 on an error finding, else 0. An unknown delta or gate exits 2.

`--hook` reads a lazyspec payload on stdin, prints findings JSON, exits 0 and is scoped to the payload's documents. The `validate` event has no range, so coverage, history and dictum checks are skipped. A pre-transition payload ranges from the moved delta's base; coverage and dictum findings are limited to the specs the moved delta changes, and history findings pass through. A finding about a document outside the payload carries its path in the message instead of an id.

### review

`lemma review <delta>` on a `draft` delta renders the `review-doc` prompt and runs the reviewer. The reply is `{"findings":[{"part","anchor","text"}]}`, and its schema is built from the delta: `part` is `index`, `design`, `claims` or `tasks`, and `anchor` is limited to what that part accepts, a claim id, the exact text of a claim with no id, a task id, a heading's text, or the part name for the whole part. Every anchor is resolved before anything is written; an unknown anchor is an error naming it and nothing is written. An identical line under the same anchor is not written twice. Review never plans. With no findings it prints `review clean`, or `review clean; <n> review lines remain` when review lines are left.

When a draft review writes review lines, each written finding `{part, anchor, text}` is appended to `<git-dir>/lemma/reviews/<delta-id>.json`, without duplicates; the file is not committed. At the start of each draft review, a recorded finding whose text no longer appears in its part is resolved: the human deleted it. Resolved findings fill the `{{resolved}}` prompt value, and a reviewer finding with the same part, anchor and normalised text as a resolved one is dropped.

On an `in-progress` delta, `lemma review` renders the `review-diff` prompt with the diff since the delta's base. The reply is `{"findings":[{"text","settles":[..]}]}`, with `settles` limited to live claim ids. Each finding is appended under `## Follow-ups` as an unticked task, created if missing, settling the ids given or a chore. A finding whose text matches an existing task is skipped. Nothing is written to `claims.md`. Any other status is an error.

### plan

`lemma plan <delta>` requires a `draft` delta, else it is an error (`plan needs a draft delta; <id> is <status>`). It refuses while any part has review lines (`<n> review lines remain; resolve them before planning`) and does not run the reviewer. `--dry-run` prints the assembled `plan` prompt and exits.

Planning first assigns `AC-<n>` to every bullet without an id, in document order, its text unchanged, in memory, and renders the `plan` prompt from that, so the planner sees the ids and does not echo claims. The reply is `{"claims":[{"text"}],"tasks":[{"text","settles":[..]}]}`. `claims` holds only new behaviour, each appended as `- [AC-<n>] [proposed] <text>`; one whose text equals an existing claim maps to that claim's id instead. A task's `settles` entries are an existing claim id or the position, from 1, of a claim in the reply's `claims`; an unresolvable entry is an error. Nothing is written, including the new ids, unless the reply is accepted. Tasks are numbered after the highest existing `T<n>` and appended after the main tasks, before `## Follow-ups`. Frontmatter keys are reordered to `title, type, status, author, date, tags, related`, then the rest; `Intent` and `Out of scope` headings are normalised to `##`. Planning refuses when any task is ticked. A delta whose bullets all have ids and whose claims are all settled is only tidied and prints `already planned`.

`--dry-run` prints the assembled prompt and exits; on `plan` it prints the `plan` prompt.

Every command that does work ends with a final stdout line `next: <step>`, using the delta id: after `review`, `plan` and `build`. Errors, `--dry-run` prompts, `lint`, `refs` and the hooks print none. `build` prints it when it ends complete, and `build --dry-run` when every task is ticked.

### build

`lemma build <delta>` refuses a delta that is not `in-progress` and a working tree with uncommitted or untracked changes. For each unticked task in order, main tasks before follow-ups:

1. The `build` prompt is rendered with the task and the claims it settles, and the runner runs with `LEMMA_DELTA` and `LEMMA_TASK` set. The reply is `{"outcome":"done"|"blocked","reason","refs":[{"claim","path","symbol"}]}`; `claim` is limited to the claims the task settles and a chore has no refs.
2. If the runner changed `index.md`, `design.md`, `claims.md` or `tasks.md`, the task fails with `runner edited <path>; lemma writes delta parts`. If `outcome` is `blocked`, it fails with `runner blocked: <reason>`. A reply with no JSON is a failure.
3. Each ref is written as an `@ref <path>#<symbol>` line under its claim, after the claim's existing lines; a ref already there is skipped. A ref to a claim the task does not settle is a failure.
4. `lemma lint <delta>` runs; any error stops the loop.
5. Unless the task is a chore, each claim it settles is `[intent]`, or has at least one ref and every ref resolves.
6. The task is ticked, then `git add -A` and `git commit -m "<delta> T<n>: <text>"`.

Any failure prints the reason and `stopped. T<n> left unticked; working tree kept.`, and exits 2. A failed commit leaves the task ticked and prints `stopped. T<n> ticked but not committed; working tree kept.` The tree is never reset. A re-run resumes at the first unticked task. `--task T<n>` runs one task. When no task is left the output is `all tasks complete`.

```
$ lemma build DELTA-004
T1 add `Balance` type and the `SUM` query (settles AC-1)
   runner: 2m04s, 6 files
   lint: clean
   AC-1 @ref tests/ledger.rs#balance_sums_entries resolves
   commit 3f1c9a2 DELTA-004 T1: add `Balance` type and the `SUM` query
```

`--dry-run` prints the prompt for the first unticked task, or the named one, and exits.

### refs

`lemma refs <claim>` prints `<citation> <path>#<symbol> ok|unresolved|not-a-test` for each ref of the claim. `DELTA-004/AC-2` names one delta's claim; a bare id matches it in every delta, complete ones included, and every dictum. Exit status is 2 when a ref does not resolve or the claim is unknown.

### session-start, signal, stop

These are Claude Code hooks and are not typed.

`lemma session-start` writes HEAD's sha to `<git-dir>/lemma/sessions/<session_id>`. An existing file is kept, so a resumed session keeps its base. It prints nothing and exits 0.

`lemma signal` reads the `PostToolUse` payload for Edit, Write and MultiEdit and returns `additionalContext` scoped to the touched file:

| Condition                                           | Output                                                                                                    |
| --------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| First touch of a governed area in the session       | One line of pointers: governing specs, decisions that affect or constrain them, dictums, each in-flight delta that changes a governing spec with its bundle directory and parts, and that delta's `related` documents |
| The file is referenced by a landed delta or dictum  | History or dictum finding                                                                                 |
| The file adds an uncovered area                     | Coverage finding                                                                                          |
| The file is a spec, dictum or in-flight delta part  | Findings for that document                                                                                |

Each message is said once per session, keyed on a stable identity and recorded in `<git-dir>/lemma/sessions/<session_id>.said`. `lemma signal` exits 0 on any failure.

`lemma stop` reports error findings only, on stderr, and exits 2 when there are any. It runs coverage and dictum checks against the session base, and the evidence check on the documents touched in the session: specs, dictums and in-flight deltas whose files changed or whose specs govern a changed file. It also reports a delta that is `complete` and was not at the session base, once per session: completing a delta is the human's step. When `stop_hook_active` is set it exits 0.

## Checks

| Check    | Rule                                                                                                          | Severity |
| -------- | ------------------------------------------------------------------------------------------------------------- | -------- |
| coverage | The changed governed files are not all within one spec's `governs`, and a touched spec has no in-flight delta that `changes` it and no delta that `changes` it completed within the range | error    |
| history  | A test changed whose region a landed delta's claim refs: `test <path>::<symbol> changed; DELTA-004/AC-2 claims it; confirm intended` | warning  |
| dictum   | A test changed that a dictum rule refs, and the dictum did not strike the rule and add a new id in the range   | error    |
| evidence | A written ref that does not resolve or is not a test. At `--gate complete`, a claim with no ref that is not `[intent]` | error    |
| shape    | Below                                                                                                         | error    |

A **landed** delta is `complete` and was not completed within the range. A test region is the referenced symbol, attributes included; `.rs` files use the Rust extractor, `.ts`, `.tsx`, `.js`, `.jsx`, `.mjs` and `.cjs` the TypeScript extractor, and other files compare whole. Formatting and comments are normalised away. A region absent at the base is not a change.

Shape rules applied to a named delta:

| When                   | Rule                                                                                                                        |
| ---------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| always                 | claim ids are `AC-<n>` and unique; task ids unique; task lines match the grammar; a task settles only claims that exist; no `@ref` in `design.md`; `[proposed]` and `[intent]` not together |
| `--gate in-progress`   | the always rules; `Intent` present and non-empty; at least one claim; every bullet has an id; every claim settled by exactly one main task; no review lines in any part; no `[proposed]` claims |
| `--gate complete`      | the in-progress rules; every task ticked, follow-ups included; every claim has a ref that resolves, or is `[intent]`        |

An invalid `governs` or `tests` glob is an error. An unknown `{{key}}` in a pack prompt file is an error.

### Base resolution

The base is the revision the history and dictum checks diff against and that bounds the range for coverage.

1. `--base <rev>` is used when given.
2. Otherwise, with the trunk (`--trunk <name>`; by default the remote-tracking `origin/main`, else `origin/master`, else the target of `origin/HEAD`, together with local `main`, else `master`), if `git merge-base HEAD <trunk>` is not HEAD, the merge-base is used. When both a remote-tracking and a local trunk exist, the merge-base nearer to HEAD is used.
3. Otherwise HEAD is on trunk. `lemma stop` and `lemma signal` use the session base recorded by `lemma session-start`. A transition gate and `lemma review` on an in-progress delta use the parent of the commit that first added the delta's `index.md`, or HEAD when the delta is uncommitted. The `validate` event uses no base.
4. Otherwise HEAD.

## Files

### lemma.toml

```toml
runner   = "claude -p --output-format json --json-schema {schema} --permission-mode acceptEdits"
reviewer = "claude -p --output-format json --json-schema {schema} --model opus"
timeout  = 1800
```

| Key        | Default                          | Effect                                    |
| ---------- | -------------------------------- | ----------------------------------------- |
| `runner`   | `claude -p --output-format json --json-schema {schema}` | Command `lemma build` runs per task |
| `reviewer` | the `runner` value               | Command `lemma review` and `lemma plan` run |
| `timeout`  | `1800`                           | Seconds before a run is killed             |

An unknown key is an error. A command is split into words on whitespace, with single and double quotes grouping; no shell is involved. The prompt is written to stdin and the working directory is the repository root. Every run passes a JSON Schema for its reply (draft-07, `additionalProperties: false`, all fields required), generated from the delta. In each word of the command, `{schema}` is replaced by the schema as compact JSON and `{schema_file}` by the path of a temporary file holding it, which exists for the run; a command naming either is an error when no schema is given. Stdout is parsed as JSON. A non-null `structured_output` is the reply, and `result` is then optional. Otherwise the reply is the first JSON object found in `result`, so a runner that does not validate against a schema still works. `"is_error": true`, a `subtype` other than `success`, or a non-zero exit is a failure; the message is `result`, else the `subtype`. Other fields are ignored.

### Prompts

`.lazyspec/prompts/review-doc.md`, `plan.md`, `review-diff.md` and `build.md` are rendered for each run. The binary carries the same defaults and uses a pack file when present. `{{key}}` is replaced by:

| Key           | Value                                                                                                     |
| ------------- | --------------------------------------------------------------------------------------------------------- |
| `convention`  | The convention's body                                                                                     |
| `dictums`     | Dictums with no `governs`, and dictums whose `governs` overlap a changed spec's `governs`; all dictums if none qualify |
| `spec_intent` | Each changed spec's `index.md` body                                                                       |
| `related`     | The body of each document in the delta's `related` other than `changes`                                   |
| `delta`, `design`, `claims`, `tasks` | The delta's parts                                                                  |
| `task`        | The task line and the claims it settles (build)                                                           |
| `resolved`    | Findings the human deleted, one `- [<part> / <anchor>] <text>` bullet each, or `(none)` (review-doc)       |
| `diff`        | `git diff <base>` limited to the changed specs' `governs` and test globs (review-diff)                    |

In the `plan` prompt `claims` already carries the ids lemma assigned. The `build` prompt tells the runner not to edit any delta part; lemma writes the refs from its reply.

Two glob lists overlap when they share a glob, when one glob matches the other's text, or when some repository file matches a glob from each. An unknown key refuses `review`, `plan` and `build` at startup. A pack prompt without `{{resolved}}` still works; the reviewer is not told the resolved findings, but the exact-match drop still applies.

### Pack hooks

| Event            | Transition                 | Runs                                     |
| ---------------- | -------------------------- | ---------------------------------------- |
| `validate`       |                            | `lemma lint --hook`                      |
| `pre-transition` | `draft` → `in-progress`    | `lemma lint --hook --gate in-progress`   |
| `pre-transition` | `in-progress` → `complete` | `lemma lint --hook --gate complete`      |

The plugin's `hooks.json` registers `SessionStart` → `lemma session-start`, `PostToolUse` (`Edit|Write|MultiEdit`) → `lemma signal` and `Stop` → `lemma stop`, each a no-op when `lemma` is not on `PATH`.

### CI

`docs/ci/lemma-validate.yml` runs `lemma lint` with `--base` set to the pull request base ref, or on a push to the default branch to the push's `before` sha when it is an ancestor of HEAD, else the parent of HEAD. A push to another branch passes `--trunk origin/<default-branch>`. The job fetches full history, then runs `lazyspec validate`.
