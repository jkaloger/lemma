---
name: entry
description: Use at the start of any code-changing work in a lemma project (a repo with .lazyspec.toml defining spec, delta and decision types), before editing code. Also use when resuming work, or when asked to shape a capability or record a decision.
---

# lemma entry

Dispatch only. Do not classify the work as fix, chore or refactor; that is the delta skill's judgement.

## Setup (once per project)

1. `lazyspec --version`. If below 0.15 or missing, tell the user to install lazyspec 0.15 or later (`cargo install lazyspec`) and stop.
2. `command -v lemma`. If missing, tell the user to run `cargo install lemma` and stop. Hooks silently do nothing without it.
3. If `.lazyspec/dictums/` is missing, copy it from `.lazyspec/cache/config/.lazyspec/dictums/`.
4. `lazyspec hook trust`.
5. If the plugin was installed this session, end setup by telling the user to restart the Claude Code session; skills and the Stop hook load only after restart. Do no delta work before then.

## Route

1. `lazyspec list --type delta --status in-progress --json`. If one exists, resume it under the `delta` skill. If several, ask the user which.
2. Otherwise route by the prompt:
   - Shaping a capability or choosing between options: the `shape` skill is release 2. Author the spec or decision directly with `lazyspec create spec|decision`, then stop for human review.
     - Spec: one capability. `Intent` in prose: what it is for, why it exists, constraints and non-goals. Set `governs` to every code glob implementing it. Link a rationale artefact (decision or hydra tree). Set `tests` only if the project's test files don't match the default globs. `lazyspec create spec` makes a bundle: `index.md` holds `Intent`, an empty `behaviour` part holds claims. Never write the `behaviour` part or put claims in `index.md`; only the fold writes the part.
     - Decision: `Context` (the forces that made it a choice), `Decision` (one or two sentences), `Consequences` (what gets easier, harder or closed off, and what would void it). Link the spec with `affects`. Never edit after acceptance; replace with a new decision linked by `supersedes`.
   - Changing code: the `delta` skill.
   - Explaining, orienting, investigating, reviewing: no skill. Answer directly.
