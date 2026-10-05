# Chore

Maintenance with no intended behaviour change: dependency bumps, config, tooling, renames.

1. Execute directly. Run the project's own checks (build, lint, tests) as the evidence.
2. Escalate to a delta or ask the user if it touches compatibility, security or production behaviour.
3. Stay inside one `governs` area. A change across areas needs a delta.
4. If a spec claim's receipt goes stale only because of the chore, resolve, then `lemma re-pin <id>`.
