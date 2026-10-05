# Fix

Behaviour is wrong against an existing claim or an obvious expectation.

1. Diagnose first. Reproduce, find the cause, state it. No fix without a credible diagnosis.
2. Write the regression test and watch it fail for the diagnosed reason.
3. Direct fix, no delta needed, when the violated claim is right and the change stays in one `governs` area. Commit the test, fix, and report the test as evidence. If the fix edits a test a spec claim references, `lemma check` flags it: open a delta.
4. Open a delta when the claim itself was wrong (strike its id and add a new id with the corrected text, carrying the `@ref` lines) or the fix crosses areas or is complex or risky. Then follow `build.md`.
5. A plan is warranted only for complex, risky or boundary-crossing fixes.
