<h1 align="center">
  Λ
  <br>λῆμμα
</h1>

Stepping stones for LLM-assisted development.

> [!NOTE]
> In mathematics and other fields, a **lemma** is a generally minor proven proposition used to prove a larger statement. It is also known as a “helping theorem” or an “auxiliary theorem”. [Source: Wikipedia, “Lemma (mathematics)”](<https://en.wikipedia.org/wiki/Lemma_(mathematics)>).

Lemma is my opinionated approach to using LLMs in software engineering. It prioritises small, verifiable changes over larger orchestration/software factory patterns.

```sh
lemma [command] [options]

# usage
lemma lint [<delta>] [--gate in-progress|complete] [--base <rev>] [--trunk <name>] [--json|--hook]
lemma review <delta> [--dry-run]
lemma plan <delta> [--dry-run]
lemma build <delta> [--task T<n>] [--dry-run]
lemma refs <claim>
lemma session-start | lemma signal | lemma stop
```

## Description

Lemma has three parts:

- The **pack** contains `.lazyspec.toml`, document templates, prompt files, seed convention documents and a `[[hooks]]` table. `lazyspec init --template` copies the pack into a project.

- The **plugin** is a Claude Code plugin that registers hooks.

- The **binary** is a Rust executable named `lemma` that links the lazyspec engine as a crate. Lifecycle transitions are configured in `.lazyspec.toml`; Lemma does not define lifecycle rules. For a task, the binary invokes a configured runner or reviewer as a child process and reads its JSON response directly, without a shell layer.

Lemma checks change coverage, document shape and the links between claims and tests. It does not run tests or decide whether a test proves a claim. The test runner and CI report test results; human review judges whether the evidence supports the claim. Automation can enforce our workflow's boundaries, but people remain responsible for intent and acceptance.

> [!WARNING]
> `lemma` is not a vibe coding tool, and as such requires careful consideration, planning and time to use.
> The result is much more laborious than spec driven/software factory/orchestration/vibe coding patterns, but I think results in better quality engineering outcomes.

## Document types

| Type         | Statuses                               | Written by                                                    |
| ------------ | -------------------------------------- | ------------------------------------------------------------- |
| `spec`       | `draft` → `active`                     | humans                                                        |
| `delta`      | `draft` → `in-progress` → `complete`   | humans write; reviewers annotate; planners and runners assist |
| `decision`   | `proposed` → `accepted` → `superseded` | humans and agents; accepted decisions are immutable           |
| `convention` | `draft` → `accepted`                   | humans                                                        |
| `dictum`     | `draft` → `accepted`                   | humans                                                        |

A **spec** is an `index.md` with `## Intent`, `## Constraints`, `## Non-goals` and `## Open questions`, a `governs` list of globs and an optional `tests` list of globs. Specs hold no claims. A change under a spec's `governs` globs makes the spec drift-stale. When `tests` is absent, it defaults to `**/tests/**`, `**/*_test.*`, `**/*.test.*` and `**/*.spec.*`.

A **delta** is the unit of change. It links each changed spec using the `changes` relationship, for example `related: [- changes: SPEC-002]`. The `delta-changes-spec` edge is required. A delta is in flight while `draft` or `in-progress`. A `complete` delta is immutable.

A **decision** records one choice and links the documents it affects or constrains. A superseding decision links the decision it replaces.

A **convention** and a **dictum** state rules that apply across changes. Dictum rules are claims edited by hand. When a referenced test must change, the rule is struck (`- ~~[ID]~~`) and a new id is added in the same change.

## Delta parts

A delta contains four parts:

| Part        | Contents                                                                        |
| ----------- | ------------------------------------------------------------------------------- |
| `index.md`  | Frontmatter (`changes`, `related`), `## Intent` and `## Out of scope`           |
| `design.md` | Prose; an `@ref` line is a shape error                                          |
| `claims.md` | Claims; the file has no heading                                                 |
| `tasks.md`  | Ordered checklist, followed optionally by `## Follow-ups` with the same grammar |

A **claim** is a list item whose id is unique within its delta:

```md
- [AC-1] The balance of an account is the sum of its entries in minor units.
  @ref tests/ledger.rs#balance_sums_entries
- [AC-4] [intent] A balance read of one account costs one query regardless of entry count.
```

A claim starts as a plain bullet without an id. `lemma plan` assigns ids. The `[intent]` marker means no test can settle the claim. `[proposed]` marks behaviour added by the planner for human review. The markers are mutually exclusive.

A **ref** is an indented `@ref <path>#<symbol>` line under a claim. It names a test file matching a changed spec's `tests` globs and a symbol in that file. An `@per <citation>` line is treated as a citation and otherwise ignored. A claim is cited outside its delta as `DELTA-004/AC-2`; a dictum rule is cited by its id.

A **task** either settles one or more claims or records a chore:

```md
- [ ] T1 add `Balance` type and the `SUM` query (settles AC-1)
- [ ] T3 wire route into the router (chore)

## Follow-ups

- [ ] T4 page the read in `balance()` (settles AC-2)
```

Task ids (`T<n>`) are unique within a delta. A checkbox line that does not match this grammar is a shape error.

A **review line** is an agent annotation for a human to resolve and delete. It is written as `@review <text>` under a claim or task, or as `> review: <text>` at the end of a section.

## Workflow

1. A human creates a delta, records its intent and scope, describes the design, lists claims as plain bullets, and links the specs it changes.
2. `lemma review <delta>` reviews a `draft` delta and records findings as review lines. The human resolves the findings, removes the corresponding lines and runs review again. Review does not plan work.
3. `lemma plan <delta>` plans a `draft` delta with no outstanding review lines. It assigns claim ids, proposes any missing behaviour as `[proposed]` claims and adds tasks. The human reviews the diff and accepts or removes each `[proposed]` marker before using `lemma build` to move the delta to `in-progress`.
4. `lemma build <delta>` handles one task at a time. A fresh runner attempts each unticked task; Lemma checks the result, obtains a task review and commits the accepted change.
5. `lazyspec update <id> --status complete` marks the delta complete.

The human remains responsible for running `lemma review`, `lemma plan`, `lemma build` and `lazyspec update`. The final command is used only to complete a delta. Review applies to `draft` deltas; build reviews each task in an `in-progress` delta. Each work-performing Lemma command ends with a `next:` line that names the next step. Errors, `--dry-run` prompts, `lint`, `refs` and hooks do not print a `next:` line.

## Commands

### lint

`lemma lint` runs coverage, history and dictum checks on the working tree against the base, and dictum check 3. With a delta argument, it also runs that delta's evidence and shape checks. `--gate <status>` applies the rules for transition to `in-progress` or `complete` and requires a delta outside hook mode. Output uses `path:line: severity: message`, one finding per line; clean output is empty. `--json` prints `{"findings": [...]}`. Exit status is 2 when an error finding exists and 0 otherwise. An unknown delta or gate exits 2.

`--hook` reads a lazyspec payload from stdin, prints findings as JSON and exits 0. Checks are scoped to payload documents. The `validate` event has no range, so coverage, history and dictum checks are skipped. For a pre-transition payload, the range starts at the moved delta's base. Coverage and dictum findings are limited to specs changed by that delta; history findings pass through. A finding about a document outside the payload includes its path in the message instead of an id.

### review

`lemma review <delta>` reviews a `draft` delta using the rendered `review-doc` prompt. The reply has the form `{"findings":[{"part","anchor","text"}]}`. The schema is built from the delta. `part` is `index`, `design`, `claims` or `tasks`. `anchor` must be accepted for that part: a claim id, the exact text of an unnumbered claim, a task id, a heading's text, or the part name for the whole part. All anchors are resolved before writing; an unknown anchor is an error and writes nothing. An identical review line under the same anchor is not written twice. Review does not plan. With no new findings, it prints `review clean`, or `review clean; <n> review lines remain` when review lines remain.

Draft findings are appended without duplicates to `<git-dir>/lemma/reviews/<delta-id>.json`; the file is not committed. At the start of each draft review, a recorded finding whose text no longer appears in its part is treated as resolved. Resolved findings populate `{{resolved}}`, and an identical finding from the reviewer is dropped.

Reviewing an `in-progress` delta fails with `review needs a draft delta; <id> is in-progress, and lemma build reviews each task`. Other statuses also fail.

### plan

`lemma plan <delta>` requires a `draft` delta. Otherwise it reports `plan needs a draft delta; <id> is <status>`. It refuses while review lines remain and does not run the reviewer. `--dry-run` prints the assembled `plan` prompt.

Planning assigns `AC-<n>` to each unnumbered claim in document order, without changing its text, in memory before rendering the prompt. The planner reply has the form `{"claims":[{"text"}],"tasks":[{"text","settles":[..]}]}`. New behaviour is appended as `- [AC-<n>] [proposed] <text>`. A reply claim matching existing claim text maps to that claim's id. A task's `settles` entries name an existing claim id or a one-based position in the reply's `claims`; an unresolved entry is an error. Nothing is written unless the reply is accepted. Tasks are numbered after the highest existing `T<n>` and inserted after main tasks, before `## Follow-ups`.

Planning reorders frontmatter keys to `title, type, status, author, date, tags, related`, followed by other keys, and normalises `Intent` and `Out of scope` headings to `##`. It refuses if any task is ticked. If every claim has an id and is settled, planning only tidies the delta and prints `already planned`.

### build

`lemma build <delta>` requires an `in-progress` or `draft` delta. `--dry-run` refuses a `draft` delta. Each refusal names `lazyspec update <id> --status in-progress`. For a draft delta, build offers the transition in a terminal with `<id> is draft; move it to in-progress? [y/N]`. Off a terminal, or after a negative answer, build refuses without moving it.

Before the prompt, build requires a clean working tree outside the delta directory. Uncommitted or untracked paths elsewhere produce `build needs a clean working tree; commit or stash first`. After confirmation, build lints with `--gate in-progress`. Error findings stop the command; lazyspec is not run. Otherwise build runs `lazyspec update <id> --status in-progress` with inherited stdio and verifies the resulting status. A failed update or wrong status leaves the delta draft. Build then commits only the delta directory as `<id>: in-progress`. A failed commit leaves the delta in progress and reports that it must be committed before build can continue.

An `in-progress` delta requires a clean working tree. Build processes unticked main tasks in order, then follow-ups. For each task:

1. The `build` prompt includes the task and its claims. The runner receives `LEMMA_DELTA` and `LEMMA_TASK`. Its reply is `{"outcome":"done"|"blocked","reason","refs":[{"claim","path","symbol"}]}`. `claim` must be one of the claims settled by the task. A chore has no refs.
2. A runner that changes `index.md`, `design.md`, `claims.md` or `tasks.md` fails with `runner edited <path>; Lemma writes delta parts`. A blocked outcome fails with its reason. A reply without JSON fails.
3. Refs are written under their claims after existing lines. Existing refs are skipped. A ref for a claim the task does not settle fails.
4. `lemma lint <delta>` runs. Any error stops the task.
5. Unless the task is a chore, every settled claim must be `[intent]`, or have at least one ref and all refs must resolve.
6. The review gate runs.
7. The task is ticked, then `git add -A` and `git commit` record it with message `<delta> T<n>: <text>` and a review trailer.

A failure before the review gate leaves the task unticked and preserves the working tree. A failed commit leaves the task ticked and preserves the working tree. Build never resets the tree. A rerun resumes at the first unticked task. `--task T<n>` runs one task. When no tasks remain, output is `all tasks complete`.

#### Output

Output has two modes, picked once per command. **Live** mode applies when stdout and stderr are both terminals. **Plain** mode applies otherwise. A task with one fix round prints in plain mode:

```
T<n> <text> (settles AC-1)
   watch: <command>
   runner: <elapsed>, <n> files
   lint: clean
   AC-1 @ref <path>#<symbol> resolves
   watch: <command>
   review: 1 finding, fix round 1
   watch: <command>
   fix: <elapsed>, <n> files
   lint: clean
   AC-1 @ref <path>#<symbol> resolves
   watch: <command>
   review: clean
   commit <sha> <delta> T<n>: <text>
```

In live mode no `watch:` line is printed, and each step line carries a status glyph: `✓` on pass, `✗` on failure, `·` for information. The review line and the fix-round heading are separate lines:

```
T<n> <text> (settles AC-1)
   ✓ runner: <elapsed>, <n> files
   ✓ lint: clean
   ✓ AC-1 @ref <path>#<symbol> resolves
   ✗ review: 1 finding
   · fix round 1
   ✓ fix: <elapsed>, <n> files
   ✓ lint: clean
   ✓ AC-1 @ref <path>#<symbol> resolves
   ✓ review: clean
   ✓ commit <sha> <delta> T<n>: <text>
```

When the task commits, the block is replaced by one line. The review outcome reads `review clean` or `<n> finding(s) fixed` and matches the commit trailer:

```
✓ T<n> <text>  <elapsed> · 1 finding fixed · <sha>
```

If the block's line count is unknown, the summary line is printed under the block. A stopped task's block is not collapsed. In live mode the stop block ends with `   transcript: <path>` for the call that failed.

#### Review gate

After the runner, lint and settled-claim checks pass, Lemma runs `git add -A` and invokes the `reviewer` command with the `review-task` prompt. The prompt includes the task, its claims, conventions, dictums, spec intent, related documents, design, claims, `task_diff` and `delta_diff`.

`task_diff` is `git diff --cached HEAD`, containing the staged work of this task, including untracked files. `delta_diff` is `git diff <base> HEAD`, containing committed work for the delta so far as read-only context. The prompt directs the reviewer to judge only the task diff and use the delta diff to detect a task undoing earlier work.

The reviewer reply is `{"stop":"<reason or empty>","findings":[{"text","basis"}]}`. `basis` must be `task`, `design`, a live claim id, a live dictum rule id or a related document id other than `changes`. A reply with another basis fails with `review: reviewer reply: unknown basis` and the invalid basis.

| Reply                     | Effect                                                   |
| ------------------------- | -------------------------------------------------------- |
| Empty `stop`, no findings | Prints `review: clean`; the task is ticked and committed |
| Empty `stop`, findings    | Runs a fix round, up to `review_rounds` times            |
| Non-empty `stop`          | Stops the build without a fix round                      |

The prompt directs the reviewer to report work outside the task as a finding with basis `task`, and to set `stop` when the task conflicts with `design.md`.

#### Fix rounds

A fix round prints `review: <n> findings, fix round <k>`, then runs the builder with the `fix` prompt. The prompt lists findings as `- [<basis>] <text>`. Its reply uses the build schema. The fix reply replaces the task's refs; refs are written onto claim lines as they stood before the task ran. The build reply checks apply again, followed by lint, settled-claim checks and another review gate.

| Runner configuration                           | Fix round command                                                                                    |
| ---------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| `runner` and `resume` both contain `{session}` | The `resume` command, with `{session}` set to the build call's session id and the `fix` prompt alone |
| Either command lacks `{session}`               | The `runner` command in a fresh run, with the `build` prompt followed by the `fix` prompt            |

`review_rounds` is the maximum number of fix rounds per task and defaults to 1. If findings remain after the final round, build stops without committing.

| Stop cause                            | Output                                                                                                     |
| ------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| Findings remain after `review_rounds` | Finding count and basis lines, then `stopped. T<n> left unticked after <k> fix rounds; working tree kept.` |
| Reviewer returns `stop`               | `stopped. T<n> review: <reason>; working tree kept.`                                                       |
| A fix round fails a check             | Check failure lines, then `stopped. T<n> left unticked; working tree kept.`                                |

Each stop exits 2 and preserves the working tree, including staged work. A rerun resumes at the first unticked task. When a stop leaves findings open, they are written to `<git-dir>/lemma/reviews/<delta>-T<n>.json` as `{"stop","findings":[{"text","basis"}]}`. This includes reviewer stops and fix-round failures caused by a blocked reply, part edit, bad ref, lint or settled-claim check. The file is not committed and is overwritten by the next stop for that task. `tasks.md` and `claims.md` are not modified by the stop record.

Each task commit has a `Lemma-Review` trailer. It is `Lemma-Review: clean` when the first review has no findings. Otherwise it is `Lemma-Review: <n> findings fixed in <k> rounds`, with singular forms `1 finding` and `1 round`. Delta parts do not record that the gate ran.

`lemma build --dry-run` prints the prompt for the first unticked task, or the task named with `--task`. If every task is ticked, it prints the completion prompt and exits.

### refs

`lemma refs <claim>` prints `<citation> <path>#<symbol> ok|unresolved|not-a-test` for each claim ref. `DELTA-004/AC-2` selects a claim in one delta. A bare id searches every delta, including complete deltas, and every dictum. An unresolved or non-test ref, or an unknown claim, exits 2.

### session-start, signal and stop

These commands are Claude Code hooks; they are not typed as part of the workflow.

`lemma session-start` writes HEAD's SHA to `<git-dir>/lemma/sessions/<session_id>`. An existing file is preserved so resumed sessions keep their base. The command prints nothing and exits 0.

`lemma signal` reads the `PostToolUse` payload for `Edit`, `Write` and `MultiEdit`, then returns `additionalContext` scoped to the touched file:

| Condition                                      | Output                                                                                                                                                                                               |
| ---------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| First touch of a governed area in a session    | Pointers to governing specs, affecting or constraining decisions, dictums, in-flight deltas that change a governing spec (including bundle directory and parts), and those deltas' related documents |
| File referenced by a landed delta or dictum    | History or dictum finding                                                                                                                                                                            |
| File adds an uncovered area                    | Coverage finding                                                                                                                                                                                     |
| File is a spec, dictum or in-flight delta part | Findings for that document                                                                                                                                                                           |

Each message is emitted once per session using a stable identity and recorded in `<git-dir>/lemma/sessions/<session_id>.said`. `lemma signal` exits 0 on failure.

`lemma stop` reports error findings on stderr and exits 2 when any exist. It checks coverage and dictums against the session base, and evidence on documents touched in the session: changed specs and dictums, plus in-flight deltas whose files changed or whose specs govern a changed file. It also reports a delta moved to `complete` after the session base, once per session. When `stop_hook_active` is set, it exits 0.

## Checks

| Check    | Rule                                                                                                                                                                  | Severity |
| -------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| coverage | Changed governed files are not all within one spec's `governs`; a touched spec has no in-flight delta changing it and no delta changing it completed within the range | error    |
| history  | A test changed whose region is claimed by a landed delta: `test <path>::<symbol> changed; DELTA-004/AC-2 claims it; confirm intended`                                 | warning  |
| dictum   | A test changed that a dictum rule references, and the range does not strike the rule and add a new id                                                                 | error    |
| evidence | A ref does not resolve or is not a test. At `--gate complete`, a non-`[intent]` claim has no ref                                                                      | error    |
| shape    | Document and transition rules below are violated                                                                                                                      | error    |

A **landed** delta is `complete` and was not completed within the checked range. A test region is the referenced symbol, including attributes. Rust files use the Rust extractor. TypeScript and JavaScript files with extensions `.ts`, `.tsx`, `.js`, `.jsx`, `.mjs` and `.cjs` use the TypeScript extractor. Other files are compared as a whole. Formatting and comments are normalised away. A region absent at the base is not considered changed.

Shape rules for a named delta:

| When                 | Rules                                                                                                                                                                                                |
| -------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Always               | Claim ids use `AC-<n>` and are unique; task ids are unique; task lines match the grammar; tasks settle only existing claims; `design.md` has no `@ref`; `[proposed]` and `[intent]` are not combined |
| `--gate in-progress` | Always rules; non-empty `Intent`; at least one claim; every claim has an id; each claim is settled by exactly one main task; no review lines in any part; no `[proposed]` claims                     |
| `--gate complete`    | In-progress rules; every task, including follow-ups, is ticked; every claim has a resolving ref or is `[intent]`                                                                                     |

Invalid `governs` or `tests` globs are errors. An unknown `{{key}}` in a pack prompt is an error.

## Base resolution

The base is the revision used by history and dictum checks and the start of the coverage range.

1. `--base <rev>` takes precedence when specified.
2. Otherwise, Lemma resolves a trunk using `--trunk <name>` or, by default, `origin/main`, `origin/master`, the target of `origin/HEAD`, local `main`, then local `master`. If `git merge-base HEAD <trunk>` is not `HEAD`, Lemma uses the merge base. When both remote-tracking and local trunks exist, Lemma uses the merge base nearer to `HEAD`.
3. Otherwise, when `HEAD` is on trunk, `lemma stop` and `lemma signal` use the session base. A transition gate and the build review gate's `delta_diff` use the parent of the commit that first added the delta's `index.md`, or `HEAD` if the delta is uncommitted. The `validate` event has no base.
4. In other cases, Lemma uses `HEAD`.

## Files

### lemma.toml

```toml
runner   = "claude -p --session-id {session} --output-format json --json-schema {schema} --permission-mode acceptEdits"
reviewer = "claude -p --session-id {session} --output-format json --json-schema {schema} --model opus"
resume   = "claude -p --resume {session} --output-format json --json-schema {schema}"
review_rounds = 1
timeout  = 1800
```

| Key             | Default                                                                        | Effect                                                                      |
| --------------- | ------------------------------------------------------------------------------ | --------------------------------------------------------------------------- |
| `runner`        | `claude -p --session-id {session} --output-format json --json-schema {schema}` | Command run for each build task                                             |
| `reviewer`      | Value of `runner`                                                              | Command run by `lemma review`, `lemma plan` and the build review gate       |
| `resume`        | `claude -p --resume {session} --output-format json --json-schema {schema}`     | Command for a fix round when both `runner` and `resume` contain `{session}` |
| `review_rounds` | `1`                                                                            | Maximum fix rounds per task                                                 |
| `timeout`       | `1800`                                                                         | Run timeout in seconds                                                      |

Unknown keys are errors. Commands are split into words on whitespace; single and double quotes group words. No shell is involved. Prompts are written to stdin, and the working directory is the repository root. Each run receives a JSON Schema for its reply. The schema uses draft-07, `additionalProperties: false` and requires all fields.

In each command word, `{schema}` is replaced with compact JSON and `{schema_file}` with the path to a temporary schema file that exists for the run. A command containing either placeholder fails when no schema is available. Stdout is parsed as JSON. A non-null `structured_output` is the reply; `result` is optional in that case. Otherwise, the first JSON object in `result` is used. `is_error: true`, a `subtype` other than `success` or a non-zero exit is a failure. The message is `result`, or `subtype` if `result` is absent. Other fields are ignored.

`{session}` in a runner or reviewer command is replaced with a fresh v4 UUID for each call. In a resume command it is replaced with the build call's UUID. Commands without the placeholder run without a session or watch line. The default command passes it as `--session-id`; Claude Code writes the transcript to `~/.claude/projects/<encoded root>/<uuid>.jsonl` while it runs. In live mode Lemma reads the transcript to show activity, and for nothing else. No reply, check, decision or exit status depends on it.

In plain mode, Lemma prints a watch command for the transcript before each agent call. The command prints the call's tool calls. While a call waits, a spinner with its label and elapsed time appears on stderr when stderr is a terminal. In live mode, while a call with `{session}` waits, the spinner also shows the latest tool call in the transcript as `<tool> <target>`. A file tool shows its path relative to the repository root. `Bash` shows the first line of its command. `Grep` and `Glob` show the pattern. Other tools show the name only. The line is cut to terminal width. A missing transcript or a line that is not JSON leaves label and elapsed only. Terminal output uses colour unless `NO_COLOR` is set. `NO_COLOR` keeps the status glyphs. task headings are bold, `commit` and `lint: clean` are green, `stopped` and errors are red, and the `next:` label is dim.

### Prompts

`.lazyspec/prompts/review-doc.md`, `plan.md`, `build.md`, `review-task.md` and `fix.md` are rendered for each run. The binary includes the same defaults and uses pack prompts when present.

| Key                                  | Value                                                                                                                    |
| ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------ |
| `convention`                         | Convention body                                                                                                          |
| `dictums`                            | Dictums without `governs`, and dictums whose `governs` overlap a changed spec's `governs`; all dictums when none qualify |
| `spec_intent`                        | Each changed spec's `index.md` body                                                                                      |
| `related`                            | Body of each related document except those linked by `changes`                                                           |
| `delta`, `design`, `claims`, `tasks` | Delta parts                                                                                                              |
| `task`                               | Task line and claims it settles (`build`, `review-task`)                                                                 |
| `resolved`                           | Deleted findings as `- [<part> / <anchor>] <text>`, or `(none)` (`review-doc`)                                           |
| `task_diff`                          | `git diff --cached HEAD` after `git add -A` (`review-task`)                                                              |
| `delta_diff`                         | `git diff <base> HEAD` (`review-task`)                                                                                   |
| `findings`                           | Review findings as `- [<basis>] <text>` (`fix`)                                                                          |

The `plan` prompt includes claim ids assigned by lemma. The `build` prompt directs the runner not to edit delta parts; Lemma writes refs from the reply. Unknown prompt keys prevent `review`, `plan` and `build` at startup. A pack prompt without `{{resolved}}` remains valid; exact-match finding suppression still applies.

Two glob lists overlap when they share a glob, when one glob matches the other's text, or when a repository file matches a glob from each.

### Pack hooks

| Event            | Transition                 | Runs                                   |
| ---------------- | -------------------------- | -------------------------------------- |
| `validate`       | —                          | `lemma lint --hook`                    |
| `pre-transition` | `draft` → `in-progress`    | `lemma lint --hook --gate in-progress` |
| `pre-transition` | `in-progress` → `complete` | `lemma lint --hook --gate complete`    |

The plugin's `hooks.json` registers `SessionStart` → `lemma session-start`, `PostToolUse` (`Edit|Write|MultiEdit`) → `lemma signal`, and `Stop` → `lemma stop`. Each hook is a no-op when `lemma` is not on `PATH`.

### CI

`docs/ci/lemma-validate.yml` runs `lemma lint` with `--base` set to the pull request base ref. On a push to the default branch, it uses the push's `before` SHA when that commit is an ancestor of `HEAD`, or the parent of `HEAD` otherwise. A push to another branch passes `--trunk origin/<default-branch>`. The job fetches full history, then runs `lazyspec validate`.
