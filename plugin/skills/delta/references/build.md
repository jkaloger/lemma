# Build

The planned case: new behaviour. The delta records what is known going in and preserves what is not; its claims name the verification as they settle.

1. Record what is known before code: constraints the work must honour, commitments with `@per` where authority exists, and assumptions as `[assumed]`. Preserve unknowns as you meet them; do not invent an acceptance criterion to make the work specifiable. Intent goes in `index.md`, claims in one `behaviour.<spec-id>` part per fold target. New capability: create the spec at `draft` first.
2. Size the delta to one session: a coherent claim subset, one `governs` area. Split otherwise.
3. Per claim, red-green: test through the real boundary, doubles only at I/O seams, then write `@ref <path>#<symbol>` under the claim. The path must be a test file per the spec's `tests` globs. Run `lemma check`.
4. Non-claim work (scaffolding, migrations) goes in an optional `tasks` part. No hook reads it.
5. Finish at review-ready and report.
