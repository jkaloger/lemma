# RFC-001: lemma

| Field    | Value                                                                 |
| -------- | --------------------------------------------------------------------- |
| Status   | proposed                                                              |
| Date     | 2026-10-02                                                            |
| Author   | Jack Kaloger                                                          |
| Decides  | what lemma is, what it is not, and what release 1 contains            |
| Rationale record | hydra tree `metaframework`, 68 heads, in the lazyspec repository at `.hydra/metaframework.json` (commit `f345349`). This RFC summarises it and does not restate it. |

## 1. Problem

Three pieces exist and none of them binds. lazyspec gives a typed document graph with `governs`, `@ref`, staleness and hooks. The lazyspec plugin gives generic verbs over any DAG. Per-repo configuration gives a starter. Each is advisory: an agent can skip the plan, declare completion without evidence, and leave the spec restating the code. The failure is not missing capability. It is that the pieces are scattered, so the workflow they imply is one an agent can walk around.

lemma layers them: a pack for repeatability, hooks for enforcement, and an opinion about what a spec is for that the pack encodes. Layered, they become a workflow agents cannot skip.

### What a good CLAUDE.md cannot do

Prompting cannot scope context by code locality, cannot know when context has decayed, cannot prove that intent was met, and does not keep rationale in the repository. The value of lemma is scoped, decaying, verifiable context for an agent that remembers nothing: typed nodes anchored to code, with verification built into the graph and enforced at document transitions. Hooks are not a convenience; they are how "no unverified completion" becomes a true sentence. Value scales with the age of the codebase and the number of sessions.

## 2. Thesis

Specs carry intent, rationale and constraints and reference executable contracts. Anything precisely expressible in code is code, not prose.

Of the three goals the earlier notes attached to this thesis, two hold and one is set aside. "Easy to specify" and "no new natural-language programming language" are already practised in dom-test (SPEC principles, mechanised acceptance criteria). "Deterministic multi-agent reconciliation of spec changes" has no evidence in any repository and risks a manifest format that becomes a second source of truth. It is dead for lemma and a candidate for its own experiment repository, as spec-judge was for jev.

### What a spec is for

A spec carries what code cannot, for an agent that remembers nothing: intent, constraints, non-goals, trade-offs, rationale. The reader is the agent at build time and at change time. Brevity and rationale are the quality bar, not completeness; a spec that restates the code is worse than none.

Source of truth by kind of fact:

| Fact              | Owner                                                   |
| ----------------- | ------------------------------------------------------- |
| intent, non-goals, constraints, rationale | the spec                        |
| planned behaviour | the spec, as a promissory note until a test settles it  |
| tested behaviour  | the test suite                                          |
| interfaces        | code, always by `@ref`, never prose                     |
| structure         | dictums                                                 |
| decisions         | decision records                                        |
| change            | git                                                     |

Two lifespans, split by type. Evergreen documents describe what the system is now and go stale on drift under their `governs` globs. Point-in-time documents describe a decision, go stale by age, and are superseded rather than edited.

## 3. Position

Three positions existed on one axis, where planned behaviour lives. Heavy: tagged claims in delta documents, folded into capability specs, with `@ref` receipts. Middle: tagged claims in evergreen specs with targeted drift reconciliation. Light: pending tests are the plan, intent lives in the README, no claims or receipts.

The decision is heavy, with two amendments, reached by a scenario quiz on 2026-09-30 in which every revealed preference went to the structured side while the stated preference was "lightweight". Amendment 1: the Stop hook reads pending tests statically from touched test files, so the light position's one real signal is kept without running anything. Amendment 2: "lightweight" is defined as friction per interaction (a ten-second re-pin, chores free, human edits never blocking), not as artefact count.

Evidence for further change comes from dogfooding the thin slice, not from more argument.

## 4. Identity

lemma is a lazyspec pack plus a hooks binary plus a Claude Code plugin, in one repository, under one version.

A binary rather than hook scripts because enforcement beyond required-sections needs a real language with tests: one JSON contract, one file under the trust hash. Rust rather than TypeScript because per-tool-call hooks need sub-millisecond start and a small binary, because the compiler locks down agent-written code, and because only Rust can link the lazyspec engine as a crate instead of spawning the CLI. jev is one HTTP call and does not need an AI SDK. The binary is size-tuned and is never typed by a person; lazyspec remains the only interface agents learn.

Rejected: a wrapper CLI fronting lazyspec (a second interface to learn), and a pack with shell hooks only (untestable logic, many files under trust).

### Boundary with lazyspec

Mechanism in lazyspec, policy in lemma. The test: if a game-development pack could want it, it belongs in lazyspec. The expectation is that lazyspec already has everything required (validate and pre-transition hooks, trust, `governs`, staleness, bundles, body updates from hooks), so release 1 raises no lazyspec RFC. The rule exists for the exception.

### Distribution

Three native channels, no new tooling: `lazyspec init --template <repo-url>` for the pack, `claude plugin install` from the repository's marketplace for skills and Claude Code hooks, `cargo install` for the binary. Version coherence is one version number across all three, released together, with compatibility in the release notes. No mismatch check in release 1.

## 5. Documentation model

### 5.1 What needs a durable home

A document earns its place only when the memoryless agent cannot reconstruct the information from code, tests and git. Six things qualify: intent and constraints (non-goals included); rationale and rejected alternatives; structural rules; work in flight; defect triage state; ideas. Planned behaviour is durable only until a test settles it.

Not documented: current tested behaviour (the suite), interfaces (`@ref`), verification evidence (CI plus the receipt), operational knowledge (README, not spec). A glossary is small enough to fold into the convention.

### 5.2 Survey

Taken: OpenSpec's capability-scoped evergreen spec and explicit delta unit; doctrine's first-class rationale and its memory kind, typed records anchored to code; dom-test's mechanised principles and the hydra tree as rationale record; the lazyspec repository's convention and dictums with `governs`. Memory records and structured requirements are both modelled as typed nodes in the lazyspec graph with `governs`, not new machinery.

Left: delta micro-syntax, data-model and contracts files that restate code, nine-kind lifecycles, the rfc + story + iteration chain. Details in `.local/metaframework-survey-frameworks.md` in the lazyspec repository.

### 5.3 Types

Seven types in two families. Evergreen, drift-stale, never driven: `spec`, `convention`, `dictum`. Point-in-time, age-stale: `delta` (the only driven type), `decision` (superseded, never edited), `bug` and `idea` (intake, never driven). No audit type: a finding is a bug or an idea, and a clean run is a fact about a date, not about the system. `rfc`, `story`, `iteration` and `adr` are not carried over.

A spec is one capability; its `governs` lists every code area that implements it. Overlapping globs are expected and `why <path>` returning several specs is correct, because that file serves several capabilities. Dictums stay module-shaped. Rejected: one spec per module (intent would have several homes) and one per user-facing feature (does not map onto `governs`).

A spec's template has a required non-empty `Intent` section and an optional `Behaviour` section that is the sole legal home for tagged claims. Constraints and non-goals live inside `Intent` until they earn an optional heading. A pure-intent spec is legal so infrastructure capabilities carry no filler. Rejected: a required `Behaviour` section, which reintroduces empty-section filler.

### 5.4 The delta unit

A delta document is the unit of in-flight work. It replaces the rfc + story + iteration chain. It carries claims as tagged list items exactly as they will appear in the spec. A fresh id adds, an existing id replaces, a struck id removes. Receipts live under the claim in the delta while it is in flight. At `archived`, a hook appends, replaces and deletes spec lines by id and moves the receipts. The spec is written only by that hook.

Ids are namespaced by the delta (`AC-C17-1`) so parallel deltas never collide; two in-flight deltas touching one id is a validate finding. Semantic drift inside a replaced claim remains a review concern.

Why not edit the spec directly in the same PR as the code: parallel deltas would conflict on the spec file, and a delta must be buildable without the whole spec. Why not OpenSpec's ADDED/MODIFIED/REMOVED micro-format: stable ids make the delta implicit. lazyspec pre-transition hooks already return body updates for other documents (lazyspec RFC-075 names fold-into-another as a use case), so no push-down is needed.

Fold targets are frontmatter links, the native graph edge. A single-target delta has no further ceremony. A delta folding into several specs partitions `Behaviour` with `### <spec-id>` subheadings; validate errors on a subheading naming a spec not in frontmatter, or on unpartitioned claims in a multi-target delta. A delta targeting a new capability creates the spec at draft with `Intent` only and links it, so the fold always appends to an existing node and the new intent is reviewed before claims land.

### 5.5 Where rationale lives

A decision record type, one per consequential choice, linked to the spec it affects; superseded, never edited. Hydra trees are the long-form rationale and are referenced, never restated. Commit messages carry nothing durable. Chosen over a Decisions section inside the spec, the archived delta, or `verified-by` sign-off: rationale outranks sign-off.

### 5.6 Where work in flight lives

The delta document. Claims with receipts are the only gating progress. An optional `tasks` bundle part holds non-claim work (scaffolding, migrations) as a scratchpad no hook reads. Session sizing is choosing a claim subset, not minting a document. Rejected: separate iteration documents (a second work unit), GitHub issues (outside the graph), the PR (not every project has one).

### 5.7 Structural rules

Convention plus module-shaped dictums. The convention holds the principles and is injected as the prompt preamble; CLAUDE.md shrinks to that injection. Dictums carry `governs` and go stale on drift like any evergreen node. A dictum rule is a tagged claim; a rule the codebase enforces carries an `@ref` receipt to its lint rule, clippy rule or architecture test; a rule with no enforcer is marked `[intent]`. Enforceable versus advisory is therefore structural and the ratio is visible in the same info finding as for specs.

## 6. Verification model

Verification is in the graph, not in a runner. The framework never runs tests. It checks that receipts exist and are fresh; the codebase runs the tests and CI owns pass and fail. A spec tool that re-runs the suite at Stop is neither fast nor lightweight and duplicates an existing gate.

### 6.1 Claims and receipts

The document is canonical. A claim is a tagged list item with a stable id. A receipt is an `@ref path#symbol@sha` line under the claim. Tests carry nothing: no annotations, no naming convention. Release 1 evidence is test symbols only. A claim with no receipt is a validate warning while the delta is in progress and a pre-transition error at `complete`.

The objection to document-side receipts, that an agent forgets the document edit after writing the test, is a behaviour problem, and hooks solve behaviour problems: validate warns continuously, pre-transition errors. The objections to test-side annotations and to a separate manifest are machinery, which does not go away.

### 6.2 Freshness

Existence plus freshness. A receipt is pinned at a commit and is stale when the test file or the code under the spec's `governs` changed since the pin. This reuses lazyspec's pin, reviewed and staleness facilities with zero project setup. Report ingestion (JUnit XML matched to symbols at HEAD) is the planned second slice, deferred until freshness shows whether stale receipts are the real failure mode.

### 6.3 Implementation references

`governs` only in release 1. A claim carries its test receipt and nothing else; the test symbol already anchors it to code. Drift is spec-wide: any change under the globs stales every receipt in the spec and the agent narrows by reading. If that proves noisy, the back-pocket amendment is an optional `@ref impl#symbol` per claim that narrows freshness to that symbol plus its test.

### 6.4 Intentional change versus regression

The distinction is structural. An intentional requirement change is a claim replaced or removed by id in an in-flight delta. A regression is a governed-code change with no delta document touching the affected claims: receipts go stale while claims are unchanged. The question "is there a delta naming this claim id" decides it.

### 6.5 Unverifiable claims

Two trials run together. An `[intent]` marker excludes a claim from the receipt check. `verified-by: <name> @ <sha>` records human sign-off as a receipt that goes stale by sha. validate reports the count of intent-only claims as an info finding so the ratio is visible. Which gets abused is empirical. jev grading of intent-only claims is the third trial (section 10).

### 6.6 Hand-maintained receipts

The agent writes the `@ref` line when it writes the test. The one mechanical subcommand is `re-pin`, scoped to a document, updating receipt commits after drift is resolved. Receipt suggestion from test names is deferred, gated on evidence that agents fumble the line.

## 7. Workflow

### 7.1 Governing principle

No unbounded code changes; no unverified completion.

An agent may make a scoped change directly when the intended outcome is clear, the execution permissions are appropriate and relevant verification is available. A reviewed technical approach is required when uncertainty, coordination or risk warrants it. This replaces "no code without a plan": the oracle outranks the plan. A plan without verification proves nothing; verification without a plan still bounds the change.

Per workflow. Fix: credible diagnosis and regression evidence where practical; plan only if complex, risky or crossing significant boundaries. Chore: direct execution with relevant automated checks; escalate on compatibility, security or production behaviour. Refactor: preserve the observable contract; characterisation tests where existing behaviour is insufficiently verified. Spike: agree the question, the cost boundary and the evidence; keep experimental code isolated and disposable. Build: the planned case; the delta document is the reviewed approach and its claims name the verification.

### 7.2 Two tracks

Shaping, human-paced: idea → optional interview → specify → decision. Delivery, agent-paced and hook-enforced: delta `draft` → `in-progress` → `complete` → `archived`. The tracks join at the spec: shaping produces specs and decisions; delivery produces claims and receipts that fold into specs.

The human reviews and decides on the shaping track and at `in-progress` → `complete`. The agent authors and produces evidence, and drives delivery transitions up to `complete`.

### 7.3 Nobody classifies

Twenty-four prompt workflows in six groups are named (orientation, shaping, changing code, verifying, maintaining, operating), and the set is provisional. But the framework reads only two structural signals: is there an in-progress delta, and does the diff cross `governs` areas. The fix / chore / refactor / spike distinction lives in a dictum and in the agent's judgement. Misjudgement is caught by the codebase's own checks and by the cross-governs flag, not by a label. Sensitive-path lists and diff-size ceilings were deleted with the strawman.

### 7.4 Bound and gates

The bound on a direct change is one `governs` area. Crossing areas requires an in-progress delta, flagged at Stop. The agent self-advances `draft` → `in-progress` when `Intent` is present, at least one claim exists and fold targets resolve. "No unbounded change" is therefore enforced after the fact, at Stop and at `complete`, not before editing. A PreToolUse edit gate stays in the back pocket if practice shows that is too late.

The exit gate is human, at `in-progress` → `complete`, movable to the front by pre-approval. A human may write `approved-by: <name> @ <sha>` on the delta, the sha pinning the claims section; the hook then accepts an agent completion while the claims are unchanged since that commit. Any claim added, replaced or removed after approval stales it and the gate returns to the end. The gate is never absent. "Ready for review" is derived (in-progress, all claims receipted, validate clean), not a status. The agent never writes `approved-by`; Stop flags an approval written during the agent's own session. `complete` → `archived` is the fold, with no judgement. PR merge, where the project has one, is orthogonal. The gate reads only the document so it survives PR-based and direct-to-main projects alike.

### 7.5 Creating specs

A spec is minted only by the specify workflow, a router over three inputs: a promoted idea, a hydra tree, or a prompt. The agent authors `Intent` at the highest authorship level the input supports (generate from a tree, co-write from an idea or prompt); the human reviews; the spec goes active with no `Behaviour`. Claims arrive only by delta archive.

Rationale is required and its form is free. A spec must link a hydra tree, a promoted idea or a decision. The dictum recommends an interview for anything worth one; the hook never names the tool.

### 7.6 Ideas and bugs

An idea is a thin record, `open` → `promoted` | `dropped`, intent only, in the lazyspec graph so links and age staleness apply. Promotion to an interview mints a hydra tree the idea links; promotion to a delta links the delta. Hydra heads were rejected as the idea store: their states map onto, rather than fit, the idea lifecycle, and a backlog tree holding every interview becomes the whole design history in one file.

A bug is a thin record, `open` → `closed`, with one reproduction claim and a link to the spec claim id it violates. Direct fix: the regression test is an `@ref` receipt on the bug and the close requires it to resolve. Delta: when the violated claim itself was wrong, or the fix trips the escalation trigger, the bug spawns a delta carrying the replaced claim; the bug closes when the delta completes. The bug never writes a spec. Tracker sync is out of scope.

### 7.7 Volatility

Human edits never break the flow. Drift is detected from changed files against `governs`. Later agent work in an affected area may start immediately and must resolve only the drift relevant to its task before completing; unrelated drift stays visibly stale. Mechanical repairs (renames, moves) are automated where lazyspec supports them; semantic discrepancies are presented for review scoped to the claims relied on. Never re-pin to silence a finding, and section 8 makes that a check.

### 7.8 Test harness

A core `testing` dictum carries the stack-agnostic principles the receipt model depends on: every claim is mechanised, doubles only at I/O seams, test through the real boundary. Stack-specific dictums (`testing-web`: MSW-only doubles, real browser) are optional files in the same pack, in a directory the setup step prunes. One repository, one version, no lazyspec change; pack layering is revisited only if that directory outgrows pruning.

## 8. Enforcement

### 8.1 Surfaces and precedence

lazyspec pre-transition hooks are primary: the document transition is the truth and fires for any agent, editor or git flow because it runs inside lazyspec. Claude Code Stop is early warning for work that never touches a transition. CI running the same binary's validate is the backstop that stops the design being a Claude Code feature. UserPromptSubmit injects the convention preamble and never enforces, so no guardrail depends on a prompt being seen. No pre-push (duplicates CI), no PreToolUse in release 1.

### 8.2 Catalogue

Every check enforces a rule an earlier section states, on a surface already chosen, and each is structural. Rules the framework states but does not check are the failure the design exists to avoid.

validate, continuous: claim without receipt on an in-progress delta; receipt does not resolve; receipt stale; two in-flight deltas touching one claim id; fold target does not resolve; `[intent]` and waiver counts as info; `verified-by` or `approved-by` stale; evergreen spec with no `governs`; spec with no rationale link; contract mutation, meaning a file referenced by a receipt changed while no in-progress delta replaces or removes that claim, naming the affected claims and specs.

pre-transition `draft` → `in-progress`: `Intent` present, at least one claim, fold targets resolve. `in-progress` → `complete`: every non-intent claim has a resolving fresh receipt; agent-invoked completion requires a valid `approved-by`. `complete` → `archived`: the fold into existing specs only.

`re-pin`: refused when the symbol no longer resolves; flagged when it follows a receipt-file change with no matching claim change, which is the silencing move.

Stop: diff crosses `governs` areas with no in-progress delta; validate on session-touched documents; pending tests read statically from touched test files; `approved-by` written this session; contract mutation on session-touched receipt files.

Native lazyspec validation (required sections, relationships, staleness bands, source ownership) is relied on, not duplicated. Contract-mutation detection comes from the LazyDev comparison (`.local/metaframework-lazydev-recommendations.md`, recommendation 2).

### 8.3 Severity

Fixed by surface: validate warns, pre-transition errors, Stop errors with its existing escape, CI as validate. No configuration table. A false positive is waived where it occurs, as a line under the claim with a reason; waivers are counted alongside `[intent]` claims so the ratio stays visible. Contract mutation and a flagged re-pin are warnings at validate and Stop but invalidate `approved-by` at the completion gate, returning the gate to the human. That tie is what makes pre-approval safe: the agent finishes on its own only while the evidence it was approved against has not moved.

### 8.4 Implementation

Subcommands of one binary. lazyspec `[[hooks]]` run entries and Claude Code hook commands both point at it.

## 9. Skills

Framework-owned skills in one plugin, written against the fixed seven-type DAG, adapted from the lazyspec verbs where the shape matches: review-work's three stages, orchestrate's dispatch model, systematic-debugging whole, the authoring ladder. Every code-changing skill speaks claims, receipts, fold and approval directly. The lazyspec plugin is not a dependency; an opinion needs one voice, and the lazyspec verbs are generic over any DAG, the one property lemma does not need. autobots-delivery skills are downstream methodology for the chain the delta unit replaces; an Inlight pack may later be rebuilt on lemma.

Six skills: an entry router plus five group skills (shape, delta, verify, maintain, operate). The twenty-four workflows are reference files inside their group, loaded on demand. Orientation gets no skill. The group is a structural question the agent answers reliably; the workflow within a group is the dictum-guided judgement call, so it belongs in a reference file and not a trigger description. Rejected: one skill per workflow (twenty-four competing trigger descriptions, overlapping, half near-empty, every amendment a plugin change) and a single router taking the workflow as an argument (cannot trigger from symptoms, so fires only when typed).

## 10. jev

Inside the framework, advisory, monotonic, landing after the deterministic core is dogfooded. jev may add a finding, ask for human sign-off, or flag a claim or receipt it thinks is missing. It may never remove a finding, turn a failure into a pass, waive a receipt or complete a document. Output is structured (verdict, confidence, abstain), labelled advisory, never counted as a receipt. It grades `[intent]` claims against the diff and reviews the delta document at the human completion gate. It is a separate subcommand, never invoked by validate, pre-transition or CI. Without an API key it does nothing and the review packet says so. Spec-quality review at specify time is a later trial.

## 11. Multi-agent

Nothing in release 1. lazyspec has no lease mechanism and the orchestrate skill runs units sequentially by design; nothing in the experiments was blocked by that. One agent per delta document; parallelism across deltas is separate agents in separate worktrees on separate deltas. Collision guards are the same-claim-id validate finding and git. Intra-delta parallelism is deferred until dogfooding shows a collision; the back-pocket design is orchestrate writing a claim owner into the delta with validate and Stop checking ownership.

## 12. Name

lemma. A claim in a delta is a small result proven on the way to a larger one. The name shapes vocabulary minimally: a claim may be called a lemma; receipts, fold, `governs`, drift and re-pin stay as settled. Availability on crates.io and GitHub was not verified at decision time. `delta` is the back-pocket alternative; it is the more exact description of the mechanism but git-delta owns that binary name on most Rust developers' PATH.

## 13. Release plan

### Release 1, the thin slice

Pack: `spec`, `delta`, `decision` types and templates; `testing` and `code-change` dictums. Binary: validate; the `in-progress` → `complete` gate (receipt resolves and is fresh); the fold at `archived`; Stop with the cross-governs bound and the static pending-test read; `re-pin`; a CI job running validate. Plugin: `entry` and `delta`.

Acceptance, in the hydra repository:

1. `lazyspec init --template` from this repository.
2. One evergreen spec for the reopen cascade, `Intent` plus `governs`.
3. A delta with two claims, one `[intent]`, built, with `@ref` receipts to real cargo test symbols.
4. `complete` blocked while a receipt is missing; passes the human gate once added; archive writes the spec lines by id.
5. A governed-code edit with no delta open shows the receipt stale and `re-pin` clears it; an edit across two `governs` areas is flagged at Stop.

### Release 2, on evidence from release 1

Types: `idea` and `bug`; `convention` and `dictum` templates with tagged-claim rules. Skills: `shape`, `verify`, `maintain`, `operate`. Validate remainder: same-id collision, fold-target resolution, `[intent]` and waiver counts, stale `verified-by` and `approved-by`, spec without `governs`, spec without rationale link, contract mutation. Pre-transition `draft` → `in-progress`. `approved-by` at the completion gate with revocation on contract mutation and flagged re-pin. `re-pin` hardening. Stop additions: `approved-by` written this session, contract mutation on touched receipt files. Waivers. `verified-by`. Optional stack dictums with pruning. UserPromptSubmit preamble. Second adopter: this repository self-hosts.

### Release 3 and later, each gated on a concrete need

jev; JUnit ingestion; PreToolUse edit gate; per-claim implementation `@ref`; intra-delta parallelism via claim ownership; GitHub issue sync for bugs; lazyspec adoption and migration from the rfc + story + iteration chain; version-coherence check.

### Adoption order

hydra first: greenfield, real cargo tests, a full lifecycle in days, and an author who can tell framework friction from a code problem. This repository second, as soon as the binary runs. lazyspec after two adopters: its existing document graph needs migrating, and because the binary links the lazyspec engine, engine bugs and policy bugs would otherwise land in one repository.

## 14. Back pocket

Amendments held until dogfooding asks for them, each with its trigger:

| Amendment                                       | Trigger                                              |
| ----------------------------------------------- | ---------------------------------------------------- |
| PreToolUse edit gate bounded to the delta's `governs` | after-the-fact enforcement at Stop proves too late |
| per-claim `@ref impl#symbol`                    | spec-wide staleness is noisy                         |
| receipt suggestion from test names              | agents fumble the `@ref` line                        |
| claim ownership inside one delta                | two agents collide on one delta                      |
| JUnit ingestion                                 | stale receipts are not the real failure mode         |
| `delta` as the name                             | `lemma` is unavailable                               |

## 15. Open items

- Name availability on crates.io and GitHub.
- lazyspec hook trust hashes `run` files inside the docs root. A `cargo install`ed binary is outside it, so trust covers the `[[hooks]]` table only. Whether that is sufficient, or whether lazyspec needs trust-by-path, is the first candidate for the boundary rule in section 4.
- Whether `resume` stays a workflow under `operate` or collapses into `entry`.

## 16. References

- hydra tree `metaframework`: lazyspec repository, `.hydra/metaframework.json` at commit `f345349`
- `.local/metaframework-position.md`, `.local/metaframework-survey-frameworks.md`, `.local/metaframework-lazydev-recommendations.md`, `.local/agent-instruction-intent.md` in the lazyspec repository
- `lazyspec_workflow_binary_spec.md`, the middle position, cited by the tree; not found on disk at the time of writing
- lazyspec RFC-075, pre-transition hook body updates
- dom-test (SPEC principles, mechanised acceptance criteria), spec-judge (jev)
