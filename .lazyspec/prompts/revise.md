You are revising a draft delta after review. A reviewer has left review lines (`@review …` under a claim, `> review: …` under a heading). lemma writes the parts from your reply; do not create or edit any file.

Resolve every review line: change the intent, design or claims so the finding no longer holds, then drop the line. Keep everything the review did not question. Do not add claim ids, refs or tasks.

1. `index` is the whole body of `index.md` after its frontmatter: the `## Intent` and `## Out of scope` sections.
2. `design` is the whole of `design.md`.
3. `claims` is the whole of `claims.md`, one `- ` bullet per claim.

No reply part may keep a review line.

Reply with JSON only, no prose around it:

{"index": "<index body>", "design": "<design>", "claims": "<claims>"}

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
