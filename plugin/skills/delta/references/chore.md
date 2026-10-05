# Chore

Maintenance with no intended behaviour change: dependency bumps, config, tooling, renames.

1. Execute directly. Run the project's own checks (build, lint, tests) as the evidence.
2. Escalate to a delta or ask the user if it touches compatibility, security or production behaviour.
3. Stay inside one `governs` area. A change across areas needs a delta.
4. A chore must not change a test referenced by a landed spec claim; `lemma check` flags it (check 2). If the test must change, open a delta that strikes the claim and adds a new id with its `@ref` lines. A formatting or lint chore that rewrites referenced test bodies hits the same rule.
