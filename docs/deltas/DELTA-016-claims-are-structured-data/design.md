<!-- How the change works, in prose. No `@ref` lines here; refs live under claims. -->

## Format

YAML. Chosen over TOML, KDL and an own line DSL: lightest syntax that is still off-the-shelf, and `yaml-language-server` plus a JSON Schema gives editors validation and completion without a lemma LSP.

File is a top-level list. An entry is either a bare string (new claim, no id yet) or a map:

```yaml
- A brand new claim.

- id: AC-5
  text: >
    Without `--resume`, a dirty tree with a
    stop record is refused.
  refs:
    - crates/lemma/tests/build.rs#dirty_tree_is_refused
  proposed: split clean vs dirty   # agent annotation; delete to accept

- id: AC-12
  intent: true
  text: Resuming feels instant.

- id: AC-4
  struck: true
  text: Build no longer writes reviews/<delta>-T<n>.json.
```

Map keys: `id`, `text`, `refs`, `per`, `intent`, `struck`, `proposed`. Unknown keys are errors. `lemma review` rewrites a bare string into a map with the next free id.

## YAML footguns to design around

Bare-string claims hit YAML's plain-scalar rules:

- Text starting with a backtick, `[`, `{`, `*`, `&`, `!`, `|`, `>`, `%` or `@` is a parse error or something else. Claims often start with a backtick (`` `lemma stop` reports... ``).
- `: ` inside bare text turns the entry into a map (`- Refused with: an error` is `{Refused with: an error}`).
- ` #` starts a comment, silently truncating text.

Typed deserialize (`deny_unknown_fields`, `text: String`) turns most of these into errors, not silent changes. Lemma lint should map them to a fix hint ("quote the claim, or use `text: >`"). ` #` truncation stays silent; open whether to warn on a bare string ending mid-sentence or just document it.

## Open questions

- Crate. `serde_yaml` is deprecated (already in the lockfile, 0.9.34). Options: `serde_norway`/`serde_yml` forks, `saphyr`. Id assignment needs source spans to rewrite only the bare-string lines and keep the human's formatting; no Rust YAML crate round-trips comments.
- File name: `claims.yaml`. Whether lazyspec parts accept a non-`.md` part, and whether the template comment survives as a YAML comment.
- Schema: ship a JSON Schema in the pack and a `# yaml-language-server: $schema=...` header in the template?
- Migration: `lemma migrate` for open deltas; completed deltas either migrated too or read through the old parser. Completed deltas already carry dead refs (DELTA-009 AC-49, AC-50; DELTA-011 AC-20), so migration must not fail on them.
- Review annotations: one `proposed` string per claim, or a list.
