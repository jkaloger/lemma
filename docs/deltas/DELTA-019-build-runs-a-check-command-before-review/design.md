<!-- How the change works, in prose. No `@ref` lines here; refs live under claims. -->

## Config

`runner::Raw` and `Config` gain `check: Option<String>`. `deny_unknown_fields` stays.

## Where it runs

`build::check_work` runs `check_lint`, then `check_settled`, then a new `check_command` when `config.check` is set. It sits before the review gate, so a red check never spends a reviewer call. `check_command` splits the string with `runner::split`, runs it in the repo root with `config.timeout`, and captures stdout and stderr.

Exit 0 is a pass step `check: pass`. Non-zero, or a timeout, is a `check_failed` failure. Its detail is `check: <command> exited <code>` plus the last 40 lines of combined output. Its findings have basis `check`. `review_gate` already turns a check failure into a fix round and counts it against `review_rounds`, so nothing changes there. `--resume` runs the same `check_work`, so a resumed task is checked too.

A command that cannot start (not found) stops the build with that message, not a fix round, because the runner can't fix config.

## Prompts

`fix.md` already says "Leave the tests passing". The check output reaches the runner through the existing findings list. No prompt change.
