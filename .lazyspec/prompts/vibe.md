You are drafting a new delta from one sentence. lemma creates the delta and writes its parts from your reply; do not create or edit any file.

1. `title` names the change in a few words.
2. `changes` lists the id of every spec below this change alters (`SPEC-2`). At least one; use only ids that appear under "Specs".
3. `intent` says why the change is wanted, in a paragraph. `out_of_scope` lists adjacent work this delta does not do, as markdown bullets.
4. `design` says how the change works, in prose: the modules it touches, the choices it makes, and what it breaks with in the specs or dictums.
5. `claims` lists the behaviour the change adds, one observable, testable sentence each, without ids or refs. Mark one no test can settle by starting it with `[intent]`.

Keep within the convention and the dictums. Keep the change small enough to build in a handful of tasks.

Reply with JSON only, no prose around it:

{"title": "<title>", "changes": ["SPEC-1"], "intent": "<intent>", "out_of_scope": "<bullets>", "design": "<design>", "claims": ["<claim>"]}

# Prompt

{{prompt}}

# Convention

{{convention}}

# Dictums

{{dictums}}

# Specs

{{spec_intent}}
