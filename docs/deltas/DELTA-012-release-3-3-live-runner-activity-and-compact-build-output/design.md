Two modes, picked once per command: **live** when stdout and stderr are both terminals, **plain** otherwise. Plain is today's output, byte for byte, watch lines included. Everything below is live mode.

**Activity line.** While a runner call with `{session}` waits, lemma polls the transcript at `~/.claude/projects/<encoded root>/<uuid>.jsonl` (the path the watch line names) every 250ms. It reads only bytes appended since the last poll, keeps a partial trailing line for the next poll, and takes the last `tool_use` block of each new assistant message. The spinner message becomes `<label> <elapsed> · <tool> <target>`.

`target` by tool: `Read`, `Edit`, `Write`, `MultiEdit` show `file_path` relative to the repo root; `Bash` shows the first line of `command`; `Grep` and `Glob` show `pattern`; any other tool shows its name only. The whole line is cut to terminal width with `…`.

A missing file, an unreadable file or a line that is not JSON is skipped silently; the spinner shows label and elapsed as today. A command without `{session}` has no transcript and no activity. Polling stops when the call returns. The reply still comes from the runner's stdout only.

**No watch line.** Live mode prints no `watch:` line. When a task stops, the stop block ends with `   transcript: <path>` for the call that failed, so the human can still read it.

**Task block.** Each step line gets a status glyph in a fixed column: `✓` green, `✗` red, `·` dim for information (watch-free counts, the fix-round heading). Lines and wording are otherwise today's:

```
T1 add `Balance` type and the `SUM` query (settles AC-1)
   ✓ runner 2m04s, 6 files
   ✓ lint
   ✓ AC-1 tests/ledger.rs#balance_sums_entries
   ✗ review 1 finding
   · fix round 1
   ✓ fix 48s, 2 files
   ✓ lint
   ✓ AC-1 tests/ledger.rs#balance_sums_entries
   ✓ review clean
```

**Collapse.** When the task commits, lemma clears the block it drew (it counts the lines it printed for the task, moves the cursor up and clears to end of screen) and prints one line:

```
✓ T1 add `Balance` type and the `SUM` query  2m52s · 1 finding fixed · 3f1c9a2
```

Review outcome reads `review clean` or `<n> finding(s) fixed`, matching the trailer counts. Elapsed is wall time from task start to commit. A stopped task is never collapsed; its full block stays, then the `stopped.` line as today. The `next:` line and `all tasks complete` are unchanged.

Collapse needs lemma to own every line in the block. The runner's stderr is already captured, and `lazyspec update` runs before the first task, so nothing else writes inside a block. If a line count is ever unknown, lemma skips the collapse and prints the summary line under the block.

**Colour and glyphs.** `NO_COLOR` drops colour, keeps glyphs. Review and plan get the activity line and no watch line; their summary lines are unchanged.

**Seams.** Two pure pieces carry the logic so tests need no terminal: `activity(line: &str, root) -> Option<String>` maps one transcript line to the `<tool> <target>` text, and a block renderer maps a task's step events to lines, and to the collapsed summary. The runner and build loops feed them. Mode is passed in, not read from the environment inside them.

**Docs.** README's "lemma does not read the transcript" becomes: lemma reads the transcript on a terminal to show activity, and for nothing else. README build sample gains a live-mode example beside the plain one.
