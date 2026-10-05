# Adopt

The first delta into a spec whose `behaviour` part is empty, over code that already has tests. No production change.

1. Write the delta's Intent and claims to describe what the existing tests already settle: one claim per tested behaviour. No claim without an existing or planned test.
2. Link the spec, then advance to `in-progress`.
3. Per claim, the receipt is a plain `@ref <path>#<symbol>` to the existing test. No commit needed. Check with `lemma refs <claim>`.
4. A claim with no test gets a characterisation test through the real boundary. Then add its `@ref`.
5. Do not change production code. A defect found is a separate delta.
6. Finish at review-ready (`lemma check --require evidence --require clean`) and report.
