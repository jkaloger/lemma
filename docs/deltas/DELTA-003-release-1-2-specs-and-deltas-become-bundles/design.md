Scope in: spec and delta templates as directory templates; claim parsing, validate, transition, fold, receipt and re-pin read and write parts; README and skills. Specs and deltas become lazyspec bundles, and claims move out of `## Behaviour` sections into `behaviour` parts. Back-compat with flat spec and delta files is out: the only adopter is the hydra acceptance clone, re-created at W13.

**18. Spec bundle.** `.lazyspec/templates/spec/` holds `index.md` (frontmatter, `## Intent`) and `behaviour.md` (empty). Spec claims live only in part `behaviour`. The part is the section: no `## Behaviour` heading inside it. A claim-shaped item in a spec's `index.md` is a warning. The empty part ships with the template because lazyspec refuses hook updates to a part a document lacks.

**19. Delta bundle.** `.lazyspec/templates/delta/` holds `index.md` (frontmatter, `## Intent`). Claims go in one part per fold target, named `behaviour.<target-id>` (file `behaviour.<target-id>.md`). `tasks` stays an optional, unread part. `### <spec-id>` partitions are gone. Gate `draft`→`in-progress`: every fold target has its part, every `behaviour.*` part names a fold target, at least one claim across them. Validate reports the same mismatches: a missing part is a warning at `draft` and an error from `in-progress` on; an orphan part is an error.

**20. Fold writes a part.** One update per target: `{id, part: "behaviour", hash, body}`. The body is the merged claim list. A target with no `behaviour` part is an error finding and nothing is written.

**21. Finding locations.** Findings on claims carry `part` and the line in the part file. Part files have no frontmatter, so the offset is zero. Findings on `index.md` keep decision 12 (DELTA-002).

**22. Governing spec.** For a delta claim it is the target named by its part. For a spec claim it is the spec. Replaces the partition rule in decision 5 (DELTA-001).

**23. receipt and re-pin edit part files.** `lemma receipt` finds the claim id across the delta's `behaviour.*` parts and edits that file. `re-pin` rewrites receipts in every `behaviour*` part of the document. The re-pin flag compares each part's claim lines at the old sha.

Later: DELTA-007 removed `receipt` and `re-pin`, so decision 23 no longer applies, and moved the fold to `lemma apply` (decision 55, DELTA-007). The part layout, the `behaviour.<target-id>` naming and the empty `behaviour` part survive. Check 4 (decision 51, DELTA-007) took over the part/target mismatch and claim-in-`index.md` findings. DELTA-009 removed the fold, so delta claim parts are no longer merged into specs.
