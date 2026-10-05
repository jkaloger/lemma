# Chore

Maintenance with no intended behaviour change: dependency bumps, config, tooling, renames.

1. Execute directly. Run the project's own checks (build, lint, tests) as the evidence.
2. Escalate to a delta or ask the user if it touches compatibility, security or production behaviour.
3. Stay inside one `governs` area. A change across areas needs a delta.
4. A chore commit to a file holding receipts stales them. Resolve, then `lemma re-pin <id>`: no `--force`, because the test symbols did not change. `--force` would mean the test itself changed, which a chore must not do. The one exception is a formatting or lint chore that rewrites test bodies: `re-pin` refuses there, and `--force` is correct after confirming the tests still prove their claims.
