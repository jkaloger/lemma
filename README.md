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
lemma build <delta> [--task T<n>] [--resume] [--dry-run]
lemma refs <claim>
lemma session-start | lemma signal | lemma stop
```

## Description

Lemma is a tool to help propose & implement changes in small steps that can be checked. You describe what should change and how you will know it works, then use Lemma to review the plan, break it into tasks, and work through those tasks one at a time. It uses lazyspec to link the change, its design, claims and test references connected, and emphasises human decision making.

Lemma has three parts

- A [lazyspec](https://github.com/jkaloger/lazyspec) pack with prompt templates, hooks and the document graph
- A plugin for your LLM harness
- A binary `lemma` that runs linting, reviews, planning and builds; using both LLMs and deterministic functions

Lemma checks change coverage, document shape and the links between claims and tests. It does not run tests or decide whether a test proves a claim. The test runner and CI report test results.

> [!WARNING]
> `lemma` is not a vibe coding tool, and as such requires careful consideration, planning and time to use.
> The result is much more laborious than spec driven/software factory/orchestration/vibe coding patterns, but I think results in better quality engineering outcomes.

## Document types

| Type         | Statuses                               | Written by                                                    |
| ------------ | -------------------------------------- | ------------------------------------------------------------- |
| `delta`      | `draft` → `in-progress` → `complete`   | humans write; reviewers annotate; planners and runners assist |
| `decision`   | `proposed` → `accepted` → `superseded` | humans and agents; accepted decisions are immutable           |
| `convention` | `draft` → `accepted`                   | humans                                                        |

A **delta** is the unit of change. It links each changed spec using the `changes` relationship, for example `related: [- changes: SPEC-002]`. The `delta-changes-spec` edge is required. A delta is in flight while `draft` or `in-progress`. A `complete` delta is immutable.

A **decision** records one choice and links the documents it affects or constrains. A superseding decision links the decision it replaces.

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
