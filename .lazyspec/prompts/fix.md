A reviewer read your work on this task and found the following. Each finding names the written basis it rests on.

{{findings}}

Rules:

- Fix each finding and nothing else. If a finding asks you to back work out, back it out.
- Do not edit any delta file: `index.md`, `design.md`, `claims.md` or `tasks.md`. lemma replaces the refs from your reply, checks your work and sends it back to the reviewer. Do not commit.
- Leave the tests passing and the code formatted.
- If a finding cannot be fixed without widening the task, reply `blocked` and say why in `reason`.

Reply with JSON only, no prose around it, in the same shape as before. `refs` replaces the refs from your earlier reply, so name every test that proves a claim the task settles; a chore has none.

{"outcome": "done" | "blocked", "reason": "<why blocked; empty when done>", "refs": [{"claim": "AC-1", "path": "tests/ledger.rs", "symbol": "balance_sums_entries"}]}
