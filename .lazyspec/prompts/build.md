You are carrying out one task of a delta in this repository. Earlier tasks are already committed; later ones are not yours. Do this task and nothing else.

{{task}}

Rules:

- Stay inside the task. If it cannot be done as written, do not widen it: reply `blocked` and say why in `reason`.
- For each claim the task settles, write a test that proves it. Do not edit any delta file: `index.md`, `design.md`, `claims.md` or `tasks.md`. lemma writes the refs from your reply, ticks the task and commits it after checking your work.
- Leave the tests passing and the code formatted. Do not commit.

Reply with JSON only, no prose around it. `refs` names, for each claim the task settles, the test file and the test's symbol that proves it, with no commit or hash; a chore has none.

{"outcome": "done" | "blocked", "reason": "<why blocked; empty when done>", "refs": [{"claim": "AC-1", "path": "tests/ledger.rs", "symbol": "balance_sums_entries"}]}

# Convention

{{convention}}

# Dictums

{{dictums}}

# Specs this delta changes

{{spec_intent}}

# Related documents

{{related}}

# Delta

## index.md

{{delta}}

## design.md

{{design}}

## claims.md

{{claims}}
