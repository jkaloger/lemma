<!-- How the change works, in prose. No `@ref` lines here; refs live under claims. -->

## Command

`lemma vibe -m "<prompt>" [--rounds <n>]`. `--rounds` caps review calls, default 3. Refuses to start on a dirty tree (`build::DIRTY_TREE`), so the in-progress commit and task commits only take vibe's own changes.

New `vibe.rs`, with stages run in order. A stage that fails stops the run, exits 2, and prints `next:` for that stage:

| Stage | Stops with |
| --- | --- |
| draft | `next: lemma vibe -m …` to try again; no delta is left behind |
| review | `next: lemma review <id>` |
| plan | `next: lemma plan <id>` |
| accept | `next: accept or reject each [proposed] claim, then lemma build <id>` |
| in-progress | `next: lazyspec update <id> --status in-progress` |
| build | `next: lemma build <id> --resume` |


## Stages

1. **Draft.** A runner call with the new `vibe.md` prompt (pack file plus binary default, like the others) and the user's prompt, plus the context `context::assemble` already gives: convention, dicta, specs. The reply is checked against a schema: `{ title, changes: [SPEC-n], intent, out_of_scope, design, claims: [text] }`. lemma runs `lazyspec create delta <title>`, writes the parts from the reply, and links each `changes` spec. The runner never writes delta files; lemma does (SPEC-002). An empty `changes` list, or an unknown spec, stops the run before `lazyspec create`, because `delta-changes-spec` is required. A failure after create (a part write or a link) runs `lazyspec delete <id>` before stopping. The delta is uncommitted at that point, so nothing else holds it, and the draft row stays true.
2. **Review.** Up to `--rounds` review calls (`review::review_draft`). A clean call ends the stage. A call that writes findings is followed by a revise call, except the last call: a revise call (new `revise.md` prompt) gets the delta with its review lines and replies with whole-part replacements for index body, design and claims, with every review line resolved and removed. lemma writes them, keeping index frontmatter as it was. `reviews::record` already stops a resolved finding being raised again. So every revision is reviewed, and findings from the last call are never revised: lemma strips every remaining review line, prints each as a warning, and goes on (proceed anyway, DECISION-015). `--rounds 1` is review once, then strip.
3. **Plan.** `plan::plan` as `lemma plan` runs it.
4. **Accept.** Strip the `[proposed]` marker from every claim in `claims.md`, print `accepted <n> [proposed] claims`. New fn beside `claims::split_markers`.
5. **Build.** Move draft to in-progress without asking: reuse `advance::Move` (clean-tree check, gate lint, `lazyspec update`, commit), skipping the prompt. Then `build::run(.., task: None, resume: false, ..)`. Each task gets the check command (DELTA-019) and the task review gate as usual.

On success, a summary table and `next: lazyspec update <id> --status complete`.

## Output

Live mode (`output::Mode::Live`) only. Plain mode keeps today's uncoloured lines, so CI and pipes stay unchanged.

- Each stage opens with a full-width banner in its own colour (draft magenta, review cyan, plan blue, accept yellow, build green), a stage counter `[2/5]` and a glyph.
- The review stage shows `round 2/3` and finding counts per round.
- The existing runner watch line (`activity.rs`) is unchanged under the banners.
- At the end, a summary: delta id, per-stage elapsed time, rounds used, claims accepted, tasks committed. Green on success, red with the failing stage on stop.

Colour via `console`, already a dependency. No new crates.

## Breaks with

All under DECISION-015, and only for the delta a vibe run creates:

- Principle 3 and SPEC-002 "Humans write documents; agents annotate; a human deletes an annotation to accept it." The constraint gets the qualifier "except under `lemma vibe` (DECISION-015)".
- SPEC-002 Intent "A human writes intent, design and behaviour" gets the same qualifier.
- DICTUM-002 CHG-11 "Build: the human writes intent, design and behaviour". CHG-11 is `[intent]` with no refs, so it is edited in place (Governance) to add "except under `lemma vibe` (DECISION-015)". That edit lands before plan, in its own commit.
- SPEC-002 "Review never plans. Plan runs only on a draft delta with no review lines left." `lemma plan` keeps the rule. Vibe meets it by stripping the lines first, not by bypassing the check.
- SPEC-002 "nothing commits around a finding" (DECISION-004), for draft review findings only. Task review findings still block commits under vibe.
- README warning: add a short `lemma vibe` section saying it trades the human gates for speed.


## Open questions

- Whether `--rounds` should also read a `vibe_rounds` key from `lemma.toml`. Left out until someone sets the flag on every run.
