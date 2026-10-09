In: per-task review gate in `build`; a bounded fix round that resumes the builder's session; findings that cite a written basis; `review-task` and `fix` prompts; removal of `review` on in-progress deltas, the `review-diff` prompt and its schema; DECISION-004.

**91. Review gate.** In `build`, after the existing checks pass (part edits, refs, lint, settled) and before the tick, lemma stages the tree (`git add -A`) and runs the reviewer with the `review-task` prompt.

- The prompt's values are the task, the claims it settles, `design`, `convention`, `dictums`, `spec_intent`, `related`, the task's diff (`git diff --cached HEAD`) and the delta's diff so far (base..HEAD), labelled read-only context.
- Reply: `{"stop":"<reason or empty>","findings":[{"text","basis"}]}`.
- `basis` is a schema enum built from the context: `task`, `design`, `convention`, every claim id of the delta, every dictum rule id in `{{dictums}}` and every `related` doc id. A finding with no written basis cannot be expressed, and style nits fall out by construction.
- No findings and an empty `stop` pass, and the task ticks and commits as today.
- The reviewer judges only the task's diff. The delta-so-far diff is there to catch the task undoing earlier work.

**92. Scope creep is a finding.** Work the task did not ask for is reported with `basis: task`. The prompt says to back it out, not to justify it. A task that pursues a different design from `design.md` is `stop`, not a finding.

**93. Fix round.** Findings with an empty `stop` run the builder again with the `fix` prompt, which lists each finding with its basis. After a fix round, lemma reruns the existing checks and the review gate.

- **Bound.** At most `review_rounds` fix rounds per task (`lemma.toml`, default 1). Findings left after the last round stop the build. No commit goes around a finding.
- **Resume.** When the runner command has `{session}`, the fix round runs the `resume` command (`lemma.toml`, default `claude -p --resume {session} --output-format json --json-schema {schema}`) with the build session's id, so the builder keeps its context.
- **No `{session}`.** The fix round is a fresh runner with the `build` prompt plus the `fix` section.
- **Reply.** The fix reply is the build reply schema, and its refs replace the task's refs.

**94. Stops.**

- **Exhausted rounds.** Prints each finding (`  <basis>: <text>`) and `stopped. T<n> left unticked after <k> fix rounds; working tree kept.`, then exits 2.
- **`stop`.** Prints `stopped. T<n> review: <reason>; working tree kept.`, then exits 2.
- **Record.** On a stop, findings are written to `<git-dir>/lemma/reviews/<delta>-T<n>.json`, which is not committed. Nothing is written to `tasks.md` or `claims.md`.
- **Tree.** The tree is never reset, as today.

**95. Trailer.** Each commit carries `Lemma-Review: clean` or `Lemma-Review: <n> findings fixed in <k> rounds`. git keeps the record that the gate ran. No delta part does.

**96. Progress.** Each reviewer and fix call gets the existing spinner and watch line (decisions 84 and 90, DELTA-010). Each one prints a summary line under the task heading, `review: clean` or `review: <n> findings, fix round <k>`.

**97. In-progress review removed.** `lemma review` on an in-progress delta errors with `review needs a draft delta; <id> is in-progress, and lemma build reviews each task`. The following are removed:

- the `review-diff` prompt, from the pack and the binary default
- its schema
- the follow-up writer
- `tests/review_diff.rs`

`## Follow-ups` stays as a section humans write, and `build` still runs it after the main tasks. A leftover `.lazyspec/prompts/review-diff.md` is an unknown prompt file. It follows whatever rule the loader already applies to unknown files: if the loader errors, the error names the file and says it can be deleted. As built, the loader ignores the leftover file.

**98. DECISION-004** supersedes DECISION-003's clause "On an in-progress delta `review` reads the diff and writes follow-up tasks", and its prompt list. Review gates each task in `build` against written sources. Findings go back to the builder within a round bound and are never captured as tasks. The rest of DECISION-003 stands.

The version was settled as 3.2.0 (see Out of scope for the question as it stood). W80 recorded DECISION-004 as accepted by the human ("go then build"). W84 updated the README workflow to three steps, the build section (gate, fix, stops, trailer), the config keys and the tally example transcripts. W85 was a whole-diff review against DECISION-004 and the plan.

Later: DELTA-009 (release 3.0, DELTA-009) had already removed fold, so this delta's DECISION-003 clause refers to the post-3.0 state. No later delta in this set changed the gate.
