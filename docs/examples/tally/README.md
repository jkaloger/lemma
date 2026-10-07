# tally

A small ledger service. This README shows the lemma workflow, not the service.

## Layout

```
.lazyspec.toml                 lazyspec types, edges, hooks (from the lemma pack)
lemma.toml                     runner and reviewer commands
docs/convention/               CONVENTION.md, DICTUM-*.md   human-edited rules, claims with refs
docs/specs/SPEC-002-ledger/    index.md                     intent, constraints, non-goals, open questions, governs
docs/deltas/DELTA-004-.../     index.md design.md claims.md tasks.md
docs/decisions/                DECISION-*.md                point-in-time, superseded not edited
src/ tests/
```

A spec is a paragraph of intent plus a `governs` glob. It never holds claims. A delta is the unit of work and the record of why the tests say what they say. Completed deltas are immutable. Claims are cited as `DELTA-004/AC-2`.

## Setup

```
lazyspec init --template https://github.com/jkaloger/lemma
lazyspec hook trust
cargo install lemma
```

`lemma.toml`:

```toml
runner   = "claude -p --output-format json --permission-mode acceptEdits"
reviewer = "claude -p --output-format json --model opus"
```

Agents never type `lemma`. The lazyspec transition hooks call `lemma lint`; the Claude Code hooks call `lemma signal` and `lemma stop`. The human types three commands.

## Workflow

### 1. Write the delta

```
$ lazyspec create delta "ledger balances" --changes SPEC-002
created docs/deltas/DELTA-004-ledger-balances/{index,design,claims,tasks}.md
```

Fill the human parts by hand: `Intent` and `Out of scope` in `index.md`, design prose in `design.md`, behaviour as plain bullets in `claims.md`. No ids, no refs, no tasks. `related:` in frontmatter lists the RFC or decision the work leans on; every agent in the loop gets those as context.

### 2. Review, then plan

```
$ lemma review DELTA-004
DELTA-004 is draft: reviewing the document
3 review lines written
```

Review lines land inline, in the grammar of the thing they review. Under a bullet:

```
- A balance query for an account with no entries returns zero, not an error.
  @review Says what, not why. SPEC-002 non-goals say absent accounts are errors; state that this account exists with no entries.
```

Under a prose section, a blockquote:

```
> review: Design commits to a running-balance column but DICTUM-003 [DATA-2] forbids derived columns without a rebuild path. Name the rebuild or drop the column.
```

Edit, delete each line as you resolve it, run again. When the review is clean the same run plans:

```
$ lemma review DELTA-004
DELTA-004 is draft: review clean, planning
claims.md: 4 claims tagged AC-1..AC-4, 1 [proposed]
tasks.md: 3 tasks written
index.md: frontmatter and headings tidied
```

Plan gives each bullet an id. A behaviour the planner thinks is missing is added as `[proposed]`; delete the tag to accept it or the line to reject it. Tasks you wrote by hand are kept; plan only appends. Read the diff in git before advancing. Plan refuses to run once any task is ticked.

### 3. Start

```
$ lazyspec advance DELTA-004
lemma lint --gate in-progress: intent present, 4 claims, 3 tasks, every claim settled, no review lines, no [proposed]
DELTA-004 draft → in-progress
```

### 4. Build

```
$ lemma build DELTA-004
T1 add Entry and Balance types (settles AC-1)
   runner: 2m04s, 6 files
   lint: clean
   AC-1 @ref tests/ledger.rs#balance_sums_entries resolves
   commit 3f1c9a2 DELTA-004 T1: add Entry and Balance types
T2 balance endpoint (settles AC-2, AC-3)
   runner: 3m31s, 4 files
   lint: AC-3 @ref tests/ledger.rs#balance_rejects_unknown_account does not resolve
   stopped. T2 left unticked; working tree kept.
```

Each task gets a fresh runner with the task line, `design.md`, the claims it settles, the dictums whose `governs` match, and the spec's intent. After the runner returns, `lint` runs, the settled claims' refs must resolve, the box is ticked and the task is committed. A failure stops the loop with the tree intact. Fix by hand or re-run; `build` resumes from the first unticked task.

```
$ lemma build DELTA-004
T2 balance endpoint (settles AC-2, AC-3)   resuming
   ...
   commit 8ab0e11 DELTA-004 T2: balance endpoint
T3 wire into router (chore)
   commit c42d7f0 DELTA-004 T3: wire into router
all tasks complete
```

### 5. Review the change

```
$ lemma review DELTA-004
DELTA-004 is in-progress: reviewing the diff against main
2 follow-up tasks written
```

A diff review writes tasks, not comments, because the fix is work:

```
## Follow-ups
- [ ] T4 `balance()` loads every entry; DICTUM-003 [DATA-4] caps unbounded reads. Page or aggregate in SQL. (settles AC-2)
- [ ] T5 `balance_sums_entries` asserts on a float; DICTUM-001 [TEST-5]. Use integer cents. (settles AC-1)
```

Delete the ones you disagree with. `lemma build DELTA-004` runs the rest.

### 6. Complete

```
$ lazyspec advance DELTA-004
lemma lint --gate complete: 5 tasks ticked, 4 claims, 3 refs resolve, 1 [intent], no review lines
DELTA-004 in-progress → complete
```

SPEC-002 is now drift-stale because files under its `governs` changed. That is a prompt to re-read twenty lines of intent, not a merge. Edit if the intent moved; `lazyspec touch SPEC-002` if it did not.

### A chore

A change under one spec's `governs` needs no delta. `lint` passes on the working tree and the commit is a normal commit.

## Checks

`lemma lint` runs on the working tree, at transitions, and on every edit through `signal`.

| Check | Rule | Severity |
| --- | --- | --- |
| coverage | a change under more than one spec's `governs` has an in-flight delta that `changes` each | error |
| evidence | at `complete`, every claim's `@ref` resolves, or the claim is `[intent]` | error |
| shape | intent present; every claim settled by exactly one task; every task settles a claim or is `(chore)`; no `@ref` in `design.md`; no review or `[proposed]` lines at a gate | error |
| history | a test changed that a completed delta's claim refs | warning, names the delta |
| dictum | a test changed that a dictum rule refs, and the rule was not struck and replaced in the same change | error |

lemma never runs tests. CI does.

## Commands

```
lemma lint [<delta>] [--gate <status>] [--json]
lemma review <delta>              draft: review the document, plan when clean; in-progress: review the diff
lemma build <delta> [--task T2]   run unticked tasks in order; stop on first failure
lemma signal | lemma stop | lemma session-start     hooks, not typed
```
