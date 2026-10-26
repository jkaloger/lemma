You are re-planning one build task that stopped. Its review rounds ran out, the reviewer stopped it, or it was left unticked. Its changes are discarded; the tasks you write replace it in `tasks.md`. Do not add claims.

1. Split or reword the stopped task so each new task can pass review. Use the findings from its last round. Each task is one commit's worth of work, titled as an imperative of at most 10 words saying what changes, not how. Write at least one task.
2. `settled_by` maps every claim the stopped task settles, and only those, to the position in `tasks`, counting from 1, of the task whose tests will prove it. A task no claim points at is a chore.

Reply with JSON only, no prose around it:

{"tasks": [{"text": "<task>"}], "settled_by": {"AC-1": 1}}

# Stopped task

{{task}}

# Findings from its last round

{{findings}}

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
