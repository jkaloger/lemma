You are planning a reviewed delta. A human wrote its intent, design and claims; you turn them into a build plan. Do not reword the human's claims.

1. `claims` lists only behaviour the intent and design need that no claim in `claims.md` states, worded as one observable, testable sentence each. The claims already in `claims.md` have ids; do not repeat them. Add few or none; the human deletes what they disagree with. Each new claim names the task that settles it in `task`, by that task's position in `tasks` counting from 1.
2. Write tasks in build order. Each task is one commit's worth of work, titled as an imperative of at most 10 words saying what changes, not how; the build agent reads the design and claims for the rest. Keep any tasks already in `tasks.md`; plan only what they leave unsettled.
3. `settled_by` maps every claim id in `claims.md` that no kept task settles to the position in `tasks`, counting from 1, of the task whose tests will prove it. Each of those ids must appear. An `[intent]` claim is settled by the task that builds the behaviour it describes. A task no claim points at (wiring, a migration, a rename) is a chore.

Reply with JSON only, no prose around it:

{"claims": [{"text": "<new claim text>", "task": 2}], "tasks": [{"text": "<task>"}], "settled_by": {"AC-1": 1}}

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
