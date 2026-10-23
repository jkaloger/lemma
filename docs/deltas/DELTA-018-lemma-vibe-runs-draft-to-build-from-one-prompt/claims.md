<!-- One bullet per behaviour, in plain words: `- An empty account has balance zero.` `lemma review` assigns ids (`- [AC-1] ...`). Mark a behaviour no test can settle `[intent]` after its id. -->

- `lemma vibe -m <prompt>` on a dirty tree exits 2 without calling a runner.
- The draft stage creates a new draft delta whose parts hold the runner reply's intent, out of scope, design and claims, linked `changes` to each spec in the reply.
- A draft reply with no `changes` spec, or an unknown one, stops the run before any delta is created.
- A review round that writes findings is followed by a revise call whose replacement parts lemma writes, keeping the index frontmatter as it was.
- Review stops early on the first clean round.
- When `--rounds` runs out with review lines left, lemma removes every review line, prints each as a warning, and runs plan.
- `--rounds` defaults to 3.
- After plan, no claim in `claims.md` carries `[proposed]`, and the count accepted is printed.
- Vibe moves the delta to in-progress and commits it without asking, refusing if the gate lint has errors.
- A build that stops exits 2 and prints `lemma build <id> --resume`.
- Vibe never moves the delta to complete; a successful run ends with `next: lazyspec update <id> --status complete`.
- A failing stage exits 2 and names the manual command for that stage.
- In plain mode vibe output carries no ANSI colour codes.
- [intent] In live mode each stage opens with a coloured banner, and the run ends with a summary of per-stage time, rounds, accepted claims and committed tasks.
