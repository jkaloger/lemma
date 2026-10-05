# Adopt

The first delta into a spec whose `behaviour` part is empty, over code that already has tests. No production change.

1. Write the delta's Intent and claims to describe what the existing tests already settle: one claim per tested behaviour. No claim without an existing or planned test.
2. Link the spec, then advance to `in-progress`.
3. Per claim, receipt the existing test: `lemma receipt <delta> <claim> <path>#<symbol>`. No commit needed.
4. A claim with no test gets a characterisation test through the real boundary. Commit, then receipt.
5. Do not change production code. A defect found is a separate delta.
6. Finish at review-ready and report.
