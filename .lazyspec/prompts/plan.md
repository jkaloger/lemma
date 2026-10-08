You are planning a reviewed delta. A human wrote its intent, design and claims; you turn them into a build plan. Do not reword the human's claims.

1. `claims` lists only behaviour the intent and design need that no claim in `claims.md` states, worded as one observable, testable sentence each. The claims already in `claims.md` have ids; do not repeat them. Add few or none; the human deletes what they disagree with.
2. Write tasks in build order. Each task is one commit's worth of work, titled as an imperative of at most 10 words saying what changes, not how; the build agent reads the design and claims for the rest. A task settles the claims its tests will prove: an existing claim by its id (`AC-2`), or one of your own `claims` by its position in that list counting from 1. Every claim is settled by exactly one task. A task that proves no claim (wiring, a migration, a rename) settles nothing and is a chore. Keep any tasks already in `tasks.md`; plan only what they leave unsettled.

Reply with JSON only, no prose around it:

{"claims": [{"text": "<new claim text>"}], "tasks": [{"text": "<task>", "settles": ["AC-1", 1]}]}

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

## tasks.md

{{tasks}}
