<!-- How the change works, in prose. No `@ref` lines here; refs live under claims. -->

## Config

`runner::Raw` and `Config` gain `check: Option<String>`. `deny_unknown_fields` stays.

## Where it runs

`build::check_work` runs `check_lint`, then `check_settled`, then a new `check_command` when `config.check` is set. It sits before the review gate, so a red check never spends a reviewer call. `check_command` splits the string with `runner::split`, runs it in the repo root with `config.timeout`, and captures stdout and stderr.

Exit 0 is a pass step `check: pass`. Non-zero, or a timeout, is a `check_failed` failure. Its header line is `check: <command> exited <code>`, or `check: <command> timed out after <n>s` on a timeout, followed by the last 40 lines of combined output. The header and the output lines are the failure's detail, so they print as today.

The failure carries one finding, already built, with basis `task`: the task says to leave the tests passing (`build.md`), and `task` is a basis SPEC-002 allows. Its text is the header plus the output tail. `with_check_findings` changes to keep findings a failure already carries, and only splits detail lines when there are none. So test output that mentions a claim id is never pinned to that claim. `review_gate` turns the failure into a fix round and counts it against `review_rounds` as it does today. `--resume` runs the same `check_work`, so a resumed task is checked too.

Because the chain short-circuits, the check runs only in a round where lint and evidence pass. A red suite behind a lint error shows up in the next round.

A command that cannot start (not found) stops the build with that message, not a fix round, because the runner can't fix config.

## Prompts

`fix.md` already says "Leave the tests passing". The check output reaches the runner through the existing findings list. No prompt change.
