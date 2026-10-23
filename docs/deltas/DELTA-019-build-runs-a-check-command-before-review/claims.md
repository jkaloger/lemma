<!-- One bullet per behaviour, in plain words: `- An empty account has balance zero.` `lemma review` assigns ids (`- [AC-1] ...`). Mark a behaviour no test can settle `[intent]` after its id. -->

- With no `check` in `lemma.toml`, build runs no check command and behaves as before.
- With `check` set, build runs it in the repo root after the lint and evidence checks of each task, and before the task review.
- A check that exits non-zero is a fix-round finding whose text holds the exit code and the last 40 lines of output, and the reviewer is not called that round.
- A check failure counts against `review_rounds`; when the budget runs out the task stops unticked.
- A check that times out is a fix-round finding like a non-zero exit.
- A check command that cannot start stops the build with that message and no fix round.
- A resumed task runs the check before its review.
