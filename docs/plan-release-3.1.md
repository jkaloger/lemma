# Plan: release 3.1

Dogfood fixes from the first full delta run in lemma-playground (DELTA-001, review to build). Runs were silent for minutes, status moves were typed by hand against a command that does not exist, and tasks read like prose diffs.

## Scope

In: spinner and watch line for every runner call; `{session}` runner placeholder; colour; `lemma build` offers the draft to in-progress move; `next:` lines name a real command; short task titles from `plan` and diff review.
Out: live typed progress events (the schema reply is one `StructuredOutput` call at the end, verified 2026-10-08; a `lemma event` side channel is deferred, trigger: the watch line is not enough in two dogfood runs); spec feedback from completed deltas (needs thought); offering `complete` from `lemma review`.

## Findings

- `lazyspec advance` is not a lazyspec 0.15.1 command. Status moves are `lazyspec update <id> --status <status>`, which runs the `pre-transition` hooks and exits non-zero when a hook blocks.
- `claude -p` persists its transcript to `~/.claude/projects/<encoded cwd>/<session-id>.jsonl`, written as the run goes, one JSON object per line with `timestamp` and tool calls. `--session-id <uuid>` picks the id.
- The `plan` prompt asks each task to "name the files or module, and what the change does". That is the source of the prose-diff tasks.

## Decisions

83. **Session placeholder.** Runner and reviewer commands accept `{session}`, replaced by a fresh v4 UUID per runner call. Default runner becomes `claude -p --session-id {session} --output-format json --json-schema {schema}`. A command without `{session}` runs as now.
84. **Progress.** Every runner call (review, plan, build) shows a spinner on stderr with a label and elapsed time, only when stderr is a terminal. Before it, when the command has `{session}`, one stdout line: `   watch: tail -f ~/.claude/projects/*/<uuid>.jsonl | jq -r '<filter>'`, the filter printing one line per tool call (`<name> <input, first 100 chars>`). lemma does not read the transcript. After the call the spinner clears and the existing summary line prints.
85. **Colour.** `console` styles: task heading bold, `commit` and `lint: clean` green, `stopped` and failures red, `next:` dim label. `console` disables colour off a terminal and under `NO_COLOR`. Test output stays plain.
86. **Advance at build.** `lemma build` on a `draft` delta: on a terminal, prompt `<id> is draft; move it to in-progress? [y/N]`. Yes runs `lazyspec update <id> --status in-progress` with inherited stdio, so hook findings print; non-zero exit stops the build with `<id> stayed draft; fix the findings above and run lemma build <id> again`. No, or no terminal, errors as now, naming `lazyspec update <id> --status in-progress`. The clean-tree check runs before the prompt. On success build commits what the move wrote (`git add -A`, message `<id>: in-progress`) before the first task, so the runner-edit check never sees it.
87. **Real commands.** Every `next:` line and README mention of `lazyspec advance` becomes `lazyspec update <id> --status <status>`.
88. **Short tasks.** `plan` and `review-diff` prompts: a task is an imperative title, at most 10 words, saying what changes, not how; the build agent reads the design and claims. Schemas set `maxLength: 80` on task `text`. The commit subject stays `<id> T<n>: <text>`.

## Work

- [x] W76 session+progress+colour: dec 83, 84, 85. `{session}` subst in runner.rs; default runner; spinner helper wraps runner calls in build, review, plan; watch line; console styles. Tests: placeholder replaced w/ uuid; no placeholder no watch line; default runner has `--session-id {session}`; watch line printed for build w/ fake runner using `{session}`.
- [x] W77 advance at build: dec 86, 87. Tests: draft delta off-tty errors naming `lazyspec update`; prompt decision fn (draft+tty asks, else no); fake `lazyspec` on PATH: exit 0 proceeds, non-zero stops w/ message; next: lines + README name real cmd.
- [x] W78 short tasks: dec 88. plan.md, review-diff.md prompts (pack + binary defaults); schema maxLength. Tests: plan + review-diff schemas carry maxLength 80 on task text; prompt text asserts no "name the files".
- [ ] W79 review: whole diff vs this plan + DECISION-003. README synopsis/build section updated for spinner, watch, advance prompt.
