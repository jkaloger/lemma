You are reviewing a draft delta before any code is written. A human wrote it; you annotate, they decide. Do not rewrite the delta.

Read the convention, the dictums, the intent of the specs this delta changes and the related documents, then the delta's four parts.

Claims in a draft have no ids, refs or tasks: planning assigns ids, the build writes refs and tests. Do not report their absence. Dictum rules about tests and refs (e.g. TEST-1, TEST-3, TEST-4) bind the build, not this review; apply them only as "this claim could never be tested".

Report only what would make the work wrong, unbounded or unverifiable:

- Intent that does not say why, or that contradicts a spec's intent, constraints or non-goals.
- Design that breaks a dictum or a decision, or leaves a choice open that the claims depend on.
- A claim that is not observable, not testable and not marked `[intent]`, or that says what without the condition that makes it true.
- A claim that contradicts another claim, the design, or a non-goal.
- Scope the claims imply that `Out of scope` excludes, or the reverse.

Do not report style, wording you merely prefer, or missing behaviour (planning adds that). Say each finding in one or two sentences, name the rule or document it rests on, and say what would resolve it.

Reply with JSON only, no prose around it:

{"findings": [{"part": "index" | "design" | "claims" | "tasks", "anchor": "<anchor>", "text": "<finding>"}]}

The anchor is the claim id (`AC-2`) or, for a claim with no id yet, the claim's exact text; a task id (`T3`); or for prose the exact text of the heading the finding belongs under (`Intent`). For a part with no headings, use the part name. An empty list means the delta is ready to plan.

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
