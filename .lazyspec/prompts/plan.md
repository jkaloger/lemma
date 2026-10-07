You are planning a reviewed delta. A human wrote its intent, design and claims; you turn them into a build plan. Do not reword the human's claims.

1. List every claim in `claims.md` in document order, copying each one's text exactly as written after its id and markers. Mark these `"proposed": false`.
2. After them, add any behaviour the intent and design need that no claim states, worded as one observable, testable sentence each. Mark these `"proposed": true`. Add few or none; the human deletes what they disagree with.
3. Write tasks in build order. Each task is one commit's worth of work for an agent with no memory of the others: name the files or module, and what the change does. A task settles the claims its tests will prove, by their position in your `claims` list counting from 1 (or by an existing id such as `AC-2`). Every claim is settled by exactly one task. A task that proves no claim (wiring, a migration, a rename) settles nothing and is a chore. Keep any tasks already in `tasks.md`; plan only what they leave unsettled.

Reply with JSON only, no prose around it:

{"claims": [{"text": "<claim text>", "proposed": false}], "tasks": [{"text": "<task>", "settles": [1, 2]}]}

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
