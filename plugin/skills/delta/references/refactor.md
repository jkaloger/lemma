# Refactor

Change structure, preserve the observable contract.

1. Identify the contract: the spec claims and tests covering the code.
2. Where existing behaviour is not verified enough to trust, add characterisation tests first. Commit them before restructuring.
3. Restructure with those tests green throughout. No claim changes; if one must change, it is not a refactor, open a delta.
4. Stay inside one `governs` area. Moving, renaming or editing a referenced test symbol is flagged by `lemma check` (a renamed symbol no longer resolves; an edited region is check 2). Open a delta that strikes the claim and adds a new id with the new `@ref`.
