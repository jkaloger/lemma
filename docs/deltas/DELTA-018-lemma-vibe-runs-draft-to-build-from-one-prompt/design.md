<!-- How the change works, in prose. No `@ref` lines here; refs live under claims. -->

## Command

`lemma vibe -m "<prompt>" [--rounds <n>]`. `--rounds` caps review rounds, default 3. Refuses to start on a dirty tree (`build::DIRTY_TREE`), so the in-progress commit and task commits only take vibe's own changes.

New `vibe.rs`, with stages run in order. Any stage that fails stops the run, exits 2, and names the manual command to continue (`lemma review|plan|build <id>`).

## Stages

1. **Draft.** A runner call with the new `vibe.md` prompt (pack file plus binary default, like the others) and the user's prompt, plus the context `context::assemble` already gives: convention, dicta, specs. The reply is checked against a schema: `{ title, changes: [SPEC-n], intent, out_of_scope, design, claims: [text] }`. lemma runs `lazyspec create delta <title>`, writes the parts from the reply, and links each `changes` spec. The runner never writes delta files; lemma does (SPEC-002). An empty `changes` list, or an unknown spec, stops the run because `delta-changes-spec` is required.
2. **Review.** Up to `--rounds` times: `review::review_draft`. On `Clean`, go on. On `Written`, a revise call (new `revise.md` prompt) gets the delta with its review lines and replies with whole-part replacements for index body, design and claims, with every review line resolved and removed. lemma writes them, keeping index frontmatter as it was. `reviews::record` already stops a resolved finding being raised again. If lines remain when rounds run out, lemma strips every review line, prints each stripped line as a warning, and goes on (proceed anyway).
3. **Plan.** `plan::plan` as `lemma plan` runs it.
4. **Accept.** Strip the `[proposed]` marker from every claim in `claims.md`, print `accepted <n> [proposed] claims`. New fn beside `claims::split_markers`.
5. **Build.** Move draft to in-progress without asking: reuse `advance::Move` (clean-tree check, gate lint, `lazyspec update`, commit), skipping the prompt. Then `build::run(.., task: None, resume: false, ..)`. `Stopped` exits 2 with `next: lemma build <id> --resume`.

On success, a summary table and `next: lazyspec update <id> --status complete`.

## Output

Live mode (`output::Mode::Live`) only. Plain mode keeps today's uncoloured lines, so CI and pipes stay unchanged.

- Each stage opens with a full-width banner in its own colour (draft magenta, review cyan, plan blue, accept yellow, build green), a stage counter `[2/5]` and a glyph.
- The review stage shows `round 2/3` and finding counts per round.
- The existing runner watch line (`activity.rs`) is unchanged under the banners.
- At the end, a summary: delta id, per-stage elapsed time, rounds used, claims accepted, tasks committed. Green on success, red with the failing stage on stop.

Colour via `console`, already a dependency. No new crates.

## Breaks with

- SPEC-002 "Humans write documents" and "A human deletes an annotation to accept it" get the qualifier: except under `lemma vibe`, which the human opts into per run.
- README warning: add a short `lemma vibe` section saying it trades the human gates for speed.

## Open questions

- Whether vibe deserves its own DECISION recording the exception to principle 3. Leaning yes; principle 4 says chosen constraints are decisions.
- Whether `--rounds` should also read a `vibe_rounds` key from `lemma.toml`. Left out until someone sets the flag on every run.
