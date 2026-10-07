# Plan: release 3

Implements DECISION-003: nothing folds; specs hold intent; deltas are hand-written, agent-annotated, binary-built. Starts from 2.1 at W54 (the uncommitted dictum work lands first). W55 to W60 of release 2.1 are superseded.

## Scope

In: delete fold and 2.1 marker machinery; four-part delta; `changes` edge; `[proposed]`; review-line grammar; `lemma lint` with shape rules and gates; `lemma.toml` runner; prompt files; `lemma review` (doc review, plan on clean, diff review); `lemma build`; pack, plugin, CI, README, version 3.0.0; dogfood on this repo.
Out, each with a trigger in DECISION-003: lint via reviewer for prose rules; parallel tasks; runner adapters beyond a `result` JSON field; derived commitments report; `migrate` from 2.x.

## Open questions

- `claude -p --output-format json` shape and stability across versions. W66 pins what is read (`result`, `is_error`) and ignores the rest.
- Whether runner sessions need the plugin hooks at all once build runs lint itself. W72 decides from dogfood; default is keep `signal` (context) and drop nothing.
- What `lemma stop` guards when transitions are human-typed. W72.

## Decisions

71. **Cut.** Delete `apply.rs`, `fold.rs`, `folded.rs`, `migrate.rs`, check 4, the `readiness` surface, assumption history and provenance resolution in `range.rs` and `review.rs`, `[assumed]`, `[observed]`, the `lemma-fold` hook, `plugin/skills/**`, `.lazyspec/templates/spec/behaviour.md`, `tests/apply.rs`, `tests/migrate.rs`. `lemma refs` stays. `@per` parses as a citation line and is otherwise ignored.
72. **Delta parts.** `index.md`: frontmatter (`changes`, `related`), `## Intent`, `## Out of scope`. `design.md`: free prose; an `@ref` line is a shape error. `claims.md`: claims, the part is the section. `tasks.md`: `- [ ] T<n> <text> (settles AC-1, AC-2)` or `(chore)`; optional `## Follow-ups` heading, same grammar. Ids `AC-<n>` and `T<n>`, unique within the delta. Relationship `changes` (inverse `changed-by`) replaces `folds-into`; edge `delta-changes-spec`, `required = "error"`.
73. **Review lines.** `@review <text>` indented under a claim or task, parsed like `@ref`. `> review: <text>` as a blockquote directly after a paragraph in any part. `[proposed]` is a marker after the id, exclusive with `[intent]`. All three are shape errors at any gate. A reviewer finding carries `part`, `anchor` (claim id, task id, or heading text) and `text`; prose anchors append the blockquote at the end of the heading's section. Writing is idempotent: an identical line under the same anchor is not duplicated.
74. **Lint.** `lemma lint [<delta>] [--gate <status>] [--hook] [--json]`. Without a delta: coverage, history and dictum checks on the working tree against base. With a delta: those plus evidence and shape for that delta. `--gate in-progress`: shape; intent; at least one claim; every claim settled by exactly one task; no review or `[proposed]` lines. `--gate complete`: in-progress rules plus every task ticked, every claim's refs resolve or `[intent]`. Exit 2 on error. History finding wording: `test <path>::<sym> changed; DELTA-004/AC-2 claims it; confirm intended`.
75. **Runner.** `lemma.toml` at repo root: `runner`, `reviewer` (default `claude -p --output-format json`; reviewer defaults to runner), `timeout` seconds (default 1800). Prompt on stdin, cwd repo root, env `LEMMA_DELTA`, `LEMMA_TASK` (build only). Stdout parsed as JSON; `result` is the text, `is_error` true or non-zero exit is a failure. Tests use a fake runner script (I/O seam).
76. **Prompts.** `.lazyspec/prompts/review-doc.md`, `plan.md`, `review-diff.md`, `build.md` shipped by the pack; the binary holds the same defaults and uses the pack file when present. Substitution is `{{convention}}`, `{{dictums}}`, `{{spec_intent}}`, `{{related}}`, `{{delta}}`, `{{design}}`, `{{claims}}`, `{{tasks}}`, `{{task}}`, `{{diff}}`. Unknown keys are a lint error at startup. `--dry-run` on review and build prints the assembled prompt and exits.
77. **Context assembly.** Convention body; dictums whose `governs` overlap any changed spec's `governs`, else all dictums; each changed spec's `index.md`; every doc in `related:` by id, body only; the delta parts. Diff review adds `git diff <base>` limited to files under changed specs' `governs` plus test globs.
78. **Review, draft.** Reviewer returns `{"findings":[{"part","anchor","text"}]}`. Zero findings: plan runs in the same invocation. Plan returns `{"claims":[{"text","proposed":bool}],"tasks":[{"text","settles":[..]}]}`; lemma assigns `AC-<n>` to claims in document order matching the human's bullets by exact text, appends proposed claims with `[proposed]`, writes `tasks.md` after any existing tasks, rewrites frontmatter keys in canonical order and normalises headings. Plan refuses when any task is ticked. Output: counts per part.
79. **Review, in-progress.** Reviewer returns findings with `settles` per finding; lemma writes each as an unticked task under `## Follow-ups`. Nothing is written to `claims.md`. Output: count.
80. **Build.** For each unticked task in order (follow-ups after main tasks): assemble prompt (decision 77 plus the task line, the claims it settles, instruction to write `@ref` under each settled claim); run; `lemma lint <delta>`; each settled claim's refs resolve; tick; `git add -A`, commit `DELTA-004 T2: <text>`. Any step failing stops with the tree intact and the task unticked. `--task T<n>` runs one. Chore tasks skip the ref check. Build refuses when the delta is not `in-progress` or the tree is dirty at start.
81. **Pack and plugin.** Hooks: `lemma-validate` runs `lemma lint --hook`; `lemma-intent-gate` runs `lemma lint --hook --gate in-progress`; `lemma-check-gate` runs `lemma lint --hook --gate complete`; `lemma-fold` removed. `plugin.json` 3.0.0, hooks only. CI template runs `lemma lint`. Convention: principle 2 unchanged; add "Humans write documents; agents annotate; a human deletes an annotation to accept it." CHG-9 struck, replaced: "Build: the human writes intent, design and behaviour; the plan step names tasks; the build loop produces evidence." CHG-10 struck, no replacement (adopt is gone). TEST-1 edited in place: drop the `[assumed]` sentence.
82. **Version 3.0.0** in crate, plugin, README. README rewritten from `docs/examples/tally/README.md`.

## Work

- [x] W61 land W54: commit the working-tree dictum work with its tests green. No 3.0 change.
- [x] W62 accept DECISION-003; this plan. Human transition.
- [x] W63 cut: decision 71. One commit; crate builds; remaining tests green; `lemma check` still runs checks 1 to 3 on the old shape so W64 has a baseline.
- [ ] W64 delta shape and lint: decisions 72, 73, 74. Parts, `changes` edge, templates, `AC-<n>` ids, `[proposed]`, review-line parsing, shape rules, `check` renamed `lint` with `--gate`. Tests: each shape rule; each gate; review line under claim, task and prose; `[proposed]` with `[intent]` errors; `@ref` in design errors; `tasks.md` parse including follow-ups and chores.
- [ ] W65 checks re-cut: decision 74 history and evidence. World indexes completed deltas' claims by ref path. Tests: changed test reffed by a completed delta warns and names it; evidence at `complete` on the delta only; coverage via `changes`; dictum variant unchanged.
- [ ] W66 runner and prompts: decisions 75, 76, 77. `runner.rs`, `prompts.rs`, `context.rs`, `lemma.toml` loading. Tests: fake runner echoes its stdin; timeout; non-zero exit; `is_error`; pack prompt overrides default; unknown key errors; `--dry-run` prints; dictum selection by governs overlap.
- [ ] W67 review on draft: decision 78 findings half. Tests: findings land under the right anchor in each part; idempotent on re-run; unknown anchor is an error naming it; zero findings reports clean.
- [ ] W68 plan on clean: decision 78 plan half. Tests: ids assigned in order; human bullet text preserved; proposed appended with marker; tasks appended after hand-written ones; refuses with a ticked task; frontmatter canonicalised; headings normalised; re-run on a planned delta changes nothing.
- [ ] W69 review on in-progress: decision 79. Tests: diff scoped to governs and tests; follow-ups written with settles; nothing in claims; count reported.
- [ ] W70 build: decision 80. Tests with a fake runner that writes files from a fixture: ticks and commits per task; stops on lint failure with tree kept; stops on unresolved ref; resume from first unticked; `--task`; chore skips ref check; refuses dirty tree; refuses non in-progress; follow-ups run after main tasks; commit message format.
- [ ] W71 pack, plugin, CI, convention: decision 81. Tests: `tests/pack.rs` adopts the pack and lints clean; hook table matches; no skills dir.
- [ ] W72 `signal` and `stop`: signal pointers add `related:` docs and the delta's parts; stop drops the in-session completion rule if W70 makes it dead, else rewords it. Tests updated.
- [ ] W73 README and version: decision 82.
- [ ] W74 review: whole-diff review against DECISION-003 and this plan.
- [ ] W75 dogfood (human): `lazyspec init --template . --force` on this repo; hand-write DELTA-001 for one 3.1 item; run review, plan, advance, build, review, advance end to end; log to `.scratch/dogfood-3.0.md`; decide each DECISION-003 trigger.
