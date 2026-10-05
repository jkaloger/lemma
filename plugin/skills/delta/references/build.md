# Build

The planned case: new behaviour. The delta is the reviewed approach; its claims name the verification.

1. Write the full delta (Intent in `index.md`, claims in one `behaviour.<spec-id>` part per fold target) before any code. New capability: create the spec at `draft` first.
2. Size the delta to one session: a coherent claim subset, one `governs` area. Split otherwise.
3. Per claim, red-green: test through the real boundary, doubles only at I/O seams, commit, then receipt. The receipt path must be a test file per the spec's `tests` globs.
4. Non-claim work (scaffolding, migrations) goes in an optional `tasks` part. No hook reads it.
5. Finish at review-ready and report.
