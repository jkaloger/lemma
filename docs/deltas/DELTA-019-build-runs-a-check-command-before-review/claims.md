<!-- One bullet per behaviour, in plain words: `- An empty account has balance zero.` `lemma review` assigns ids (`- [AC-1] ...`). Mark a behaviour no test can settle `[intent]` after its id. -->

- [AC-1] With no `check` in `lemma.toml`, build runs no check command and behaves as before.
- [AC-2] With `check` set, build runs it in the repo root after each task, only when the task's lint and evidence checks pass, and before the task review.
- [AC-3] A check that exits non-zero is one fix-round finding with basis `task`, whose text holds `check: <command> exited <code>` and the last 40 lines of output; the reviewer is not called that round.
- [AC-4] A claim id in the check output does not pin the finding to that claim.
- [AC-5] A check failure counts against `review_rounds`; when the budget runs out the task stops unticked.
- [AC-6] A check that times out is one fix-round finding whose text holds `check: <command> timed out after <n>s` and the last 40 lines of output.
- [AC-7] A check command that cannot start stops the build with that message and no fix round.
- [AC-8] A resumed task runs the check before its review.
- [AC-9] A check that exits 0 prints the step `check: pass` and the task proceeds to review.
