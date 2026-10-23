<!-- One bullet per behaviour, in plain words: `- An empty account has balance zero.` `lemma review` assigns ids (`- [AC-1] ...`). Mark a behaviour no test can settle `[intent]` after its id. -->

- With no `check` in `lemma.toml`, build runs no check command and behaves as before.
- With `check` set, build runs it in the repo root after each task, only when the task's lint and evidence checks pass, and before the task review.
- A check that exits non-zero is one fix-round finding with basis `task`, whose text holds `check: <command> exited <code>` and the last 40 lines of output; the reviewer is not called that round.
- A claim id in the check output does not pin the finding to that claim.
- A check failure counts against `review_rounds`; when the budget runs out the task stops unticked.
- A check that times out is one fix-round finding whose text holds `check: <command> timed out after <n>s` and the last 40 lines of output.
- A check command that cannot start stops the build with that message and no fix round.
- A resumed task runs the check before its review.
