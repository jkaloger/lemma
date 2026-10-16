<!-- One bullet per behaviour, in plain words: `- An empty account has balance zero.` `lemma review` assigns ids (`- [AC-1] ...`). Mark a behaviour no test can settle `[intent]` after its id. -->

- When a runner or fix call changes a delta part, lemma writes back the part as it was before the call, prints `runner edited <path>; restored`, and continues without a fix round.
- A part restored after a resumed call holds the lines lemma wrote before the stop and any human edit made during it.
- Refs from the reply are written after the restore, so a task whose runner reverted `claims.md` still commits with its refs.
- Lint reports an error on a claim whose first line ends with `[intent]` or `[proposed]`, saying the marker goes after the id.
- A `[intent]` or `[proposed]` mentioned mid-text, or in backticks, is not reported.
