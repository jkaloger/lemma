mod common;
use common::*;
use serde_json::{json, Value};

const SPEC_BEHAVIOUR: &str =
    "- [AC-S1-1] Reopen keeps the prior answer.\n  @ref tests/store_test.rs#reopen_keeps_prior\n";
const DELTA_BEHAVIOUR: &str = "- ~~[AC-S1-1]~~ Reopen keeps the prior answer.\n- [AC-D1-1] Reopen keeps it across restarts.\n  @ref tests/store_test.rs#other_test\n";

fn spec_doc(status: &str, behaviour: Option<&str>) -> Value {
    let mut spec = doc("SPEC-001", "spec", status, &["a/**"], &[], INTENT);
    spec["path"] = json!("docs/specs/SPEC-001-s/index.md");
    match behaviour {
        Some(body) => with_parts(spec, &[("behaviour", body)]),
        None => with_parts(spec, &[]),
    }
}

fn delta_doc(claims: &str) -> Value {
    with_parts(
        doc(
            "DELTA-001",
            "delta",
            "in-progress",
            &[],
            &["SPEC-001"],
            INTENT,
        ),
        &[("behaviour.SPEC-001", claims)],
    )
}

fn complete(repo: &Repo, delta: Value, context: Vec<Value>) -> Run {
    let payload = transition_payload("in-progress", "complete", delta, context);
    repo.run(&["apply"], &payload)
}

fn repo() -> Repo {
    let repo = Repo::new();
    repo.spec_file(1, "active", &["a/**"], INTENT);
    repo.spec_behaviour(1, SPEC_BEHAVIOUR);
    repo.commit("spec");
    repo
}

#[test]
fn the_hook_returns_an_update_echoing_the_content_hash() {
    let repo = repo();

    let run = complete(
        &repo,
        delta_doc(DELTA_BEHAVIOUR),
        vec![spec_doc("active", Some(SPEC_BEHAVIOUR))],
    );

    assert_eq!(run.code, 0);
    assert!(run.findings().iter().all(|f| f["severity"] == "warning"));
    let updates = run.json()["updates"].as_array().unwrap().clone();
    assert_eq!(updates.len(), 1);
    assert_eq!(updates[0]["id"], "SPEC-001");
    assert_eq!(updates[0]["part"], "behaviour");
    assert_eq!(updates[0]["hash"], "hash-SPEC-001");
    assert_eq!(
        updates[0]["body"],
        "- [AC-D1-1] Reopen keeps it across restarts.\n  @ref tests/store_test.rs#other_test\n"
    );
}

#[test]
fn the_hook_writes_nothing_itself() {
    let repo = repo();
    let before = repo.read("docs/specs/SPEC-001-s/behaviour.md");

    complete(
        &repo,
        delta_doc(DELTA_BEHAVIOUR),
        vec![spec_doc("active", Some(SPEC_BEHAVIOUR))],
    );

    assert_eq!(repo.read("docs/specs/SPEC-001-s/behaviour.md"), before);
}

#[test]
fn an_existing_id_without_a_strike_is_an_immutability_error_and_no_update() {
    let repo = repo();

    let run = complete(
        &repo,
        delta_doc("- [AC-S1-1] Reworded.\n"),
        vec![spec_doc("active", Some(SPEC_BEHAVIOUR))],
    );

    assert_eq!(run.code, 0);
    assert!(run.json().get("updates").is_none());
    let finding = &run.findings()[0];
    assert_eq!(finding["severity"], "error");
    assert_eq!(finding["id"], "DELTA-001");
    assert_eq!(finding["part"], "behaviour.SPEC-001");
    assert_eq!(finding["line"], 1);
    assert_eq!(
        finding["message"],
        "claims are immutable; strike AC-S1-1 and add a new id"
    );
}

#[test]
fn a_struck_id_the_spec_lacks_is_an_error() {
    let repo = repo();

    let run = complete(
        &repo,
        delta_doc("- ~~[AC-NOPE-1]~~ gone\n"),
        vec![spec_doc("active", Some(SPEC_BEHAVIOUR))],
    );

    assert!(run.has_message("AC-NOPE-1 is struck but the spec has no such claim"));
    assert!(run.json().get("updates").is_none());
}

#[test]
fn a_target_without_a_behaviour_part_is_an_error() {
    let repo = repo();

    let run = complete(
        &repo,
        delta_doc(DELTA_BEHAVIOUR),
        vec![spec_doc("active", None)],
    );

    assert!(run.has_message("fold target SPEC-001 has no `behaviour` part"));
    assert!(run.json().get("updates").is_none());
}

#[test]
fn a_target_loaded_from_the_store_cannot_be_written() {
    let repo = repo();

    let run = complete(&repo, delta_doc(DELTA_BEHAVIOUR), vec![]);

    assert!(
        run.has_message("was not in the hook payload"),
        "{}",
        run.stdout
    );
    assert!(run.json().get("updates").is_none());
}

#[test]
fn a_draft_target_warns_and_still_folds() {
    let repo = repo();

    let run = complete(
        &repo,
        delta_doc(DELTA_BEHAVIOUR),
        vec![spec_doc("draft", Some(SPEC_BEHAVIOUR))],
    );

    assert!(run.findings().iter().all(|f| f["severity"] == "warning"));
    assert!(run.has_message("SPEC-001 is draft"));
    assert_eq!(run.json()["updates"].as_array().unwrap().len(), 1);
}

#[test]
fn a_pinned_ref_in_the_delta_is_an_error() {
    let repo = repo();

    let run = complete(
        &repo,
        delta_doc("- [AC-D1-1] New.\n  @ref tests/store_test.rs#other_test@3f9c2e1\n"),
        vec![spec_doc("active", Some(SPEC_BEHAVIOUR))],
    );

    assert!(run.has_message("pinned ref; run lemma migrate"));
    assert!(run.json().get("updates").is_none());
}

#[test]
fn a_delta_with_no_claims_for_a_target_creates_no_update() {
    let repo = repo();

    let run = complete(
        &repo,
        delta_doc(""),
        vec![spec_doc("active", Some(SPEC_BEHAVIOUR))],
    );

    assert_eq!(run.findings(), Vec::<Value>::new());
    assert!(run.json().get("updates").is_none());
}

fn preview_repo() -> Repo {
    let repo = repo();
    repo.delta_file(1, "in-progress", &["SPEC-001"], INTENT);
    repo.delta_part(1, "SPEC-001", DELTA_BEHAVIOUR);
    repo.commit("delta");
    repo
}

#[test]
fn preview_lists_added_and_deleted_ids_per_target_and_writes_nothing() {
    let repo = preview_repo();

    let run = repo.run(&["apply", "DELTA-001"], "");

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert_eq!(
        run.stdout.lines().collect::<Vec<_>>(),
        vec![
            "SPEC-001",
            "  New commitments",
            "    +AC-D1-1 Reopen keeps it across restarts.",
            "      provenance: none; accepting this delta establishes this behaviour",
            "  Struck",
            "    -AC-S1-1 Reopen keeps the prior answer.",
        ]
    );
    assert_eq!(repo.git(&["status", "--porcelain"]), "");
}

#[test]
fn preview_reports_fold_issues_as_diagnostics_and_exits_two() {
    let repo = repo();
    repo.delta_file(1, "in-progress", &["SPEC-001"], INTENT);
    repo.delta_part(1, "SPEC-001", "- [AC-S1-1] Reworded.\n");
    repo.commit("delta");

    let run = repo.run(&["apply", "DELTA-001"], "");

    assert_eq!(run.code, 2);
    assert_eq!(
        run.stdout.trim(),
        "docs/deltas/DELTA-001-d/behaviour.SPEC-001.md:1: error: claims are immutable; strike AC-S1-1 and add a new id"
    );
    assert_eq!(repo.git(&["status", "--porcelain"]), "");
}

#[test]
fn preview_of_something_that_is_not_a_delta_fails() {
    let repo = preview_repo();

    let run = repo.run(&["apply", "SPEC-001"], "");

    assert_eq!(run.code, 2);
    assert!(run.stderr.contains("not a delta"), "{}", run.stderr);
}

const ASSUMED_DELTA: &str = "- [AC-D1-1] [assumed] Probably retries.\n";

#[test]
fn the_hook_refuses_a_delta_with_an_assumed_claim_and_writes_no_update() {
    let repo = repo();

    let run = complete(
        &repo,
        delta_doc(ASSUMED_DELTA),
        vec![spec_doc("active", Some(SPEC_BEHAVIOUR))],
    );

    assert!(
        run.has_message("AC-D1-1: assumption unresolved;"),
        "{}",
        run.stdout
    );
    assert_eq!(run.findings()[0]["severity"], "error");
    assert!(run.json().get("updates").is_none());
}

#[test]
fn preview_refuses_a_delta_with_an_assumed_claim_and_writes_nothing() {
    let repo = repo();
    repo.delta_file(1, "in-progress", &["SPEC-001"], INTENT);
    repo.delta_part(1, "SPEC-001", ASSUMED_DELTA);
    repo.commit("assumed delta");
    let before = repo.read("docs/specs/SPEC-001-s/behaviour.md");

    let run = repo.run(&["apply", "DELTA-001"], "");

    assert_eq!(run.code, 2);
    assert!(
        run.stdout.contains("AC-D1-1: assumption unresolved;"),
        "{}",
        run.stdout
    );
    assert!(!run.stdout.contains("SPEC-001: +"), "{}", run.stdout);
    assert_eq!(repo.read("docs/specs/SPEC-001-s/behaviour.md"), before);
}

fn decision(repo: &Repo, status: &str) {
    repo.write(
        "docs/decisions/DECISION-001-x.md",
        &format!("---\ntitle: \"D\"\ntype: decision\nstatus: {status}\nauthor: \"t\"\ndate: 2026-01-01\ntags: []\n---\n\nbody\n"),
    );
}

fn delta_committed(repo: &Repo, claims: &str) {
    repo.delta_file(1, "in-progress", &["SPEC-001"], INTENT);
    repo.delta_part(1, "SPEC-001", claims);
    repo.commit("delta");
}

fn preview(repo: &Repo) -> Vec<String> {
    let run = repo.run(&["apply", "DELTA-001"], "");
    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    run.stdout.lines().map(str::to_string).collect()
}

const NONE: &str = "provenance: none; accepting this delta establishes this behaviour";

#[test]
fn preview_groups_claims_per_target_and_omits_empty_groups() {
    let repo = repo();
    decision(&repo, "accepted");
    repo.commit("decision");
    delta_committed(
        &repo,
        "- [A-1] [assumed] Maybe retries.\n- [A-2] [assumed] Maybe batches.\n- [AC-D1-3] Keeps order.\n- [AC-D1-4] [observed] Reads from cache.\n",
    );
    repo.delta_part(
        1,
        "SPEC-001",
        "- ~~[AC-S1-1]~~ Reopen keeps the prior answer.\n- [A-1] Retries twice.\n  @per DECISION-001\n- [AC-D1-3] Keeps order.\n- [AC-D1-4] [observed] Reads from cache.\n",
    );
    repo.commit("resolve");

    assert_eq!(
        preview(&repo),
        vec![
            "SPEC-001".to_string(),
            "  New commitments".to_string(),
            "    +AC-D1-3 Keeps order.".to_string(),
            format!("      {NONE}"),
            "  Observed behaviour".to_string(),
            "    +AC-D1-4 Reads from cache.".to_string(),
            "  Struck".to_string(),
            "    -AC-S1-1 Reopen keeps the prior answer.".to_string(),
            "  Resolved assumptions".to_string(),
            "    A-1: was assumed, now a commitment".to_string(),
            "      +A-1 Retries twice.".to_string(),
            "        provenance: DECISION-001 (accepted)".to_string(),
            "  Discarded assumptions".to_string(),
            "    A-2: Maybe batches.".to_string(),
        ]
    );
}

#[test]
fn a_commitment_without_per_says_so() {
    let repo = repo();
    delta_committed(&repo, "- [AC-D1-1] New.\n");

    let lines = preview(&repo);

    assert!(lines.contains(&format!("      {NONE}")), "{lines:?}");
}

#[test]
fn an_intent_commitment_carries_provenance_too() {
    let repo = repo();
    delta_committed(&repo, "- [AC-D1-1] [intent] Feels quick.\n");

    assert!(preview(&repo).contains(&format!("      {NONE}")));
}

#[test]
fn a_decision_accepted_at_the_base_is_accepted() {
    let repo = repo();
    decision(&repo, "accepted");
    repo.commit("decision");
    delta_committed(&repo, "- [AC-D1-1] New.\n  @per DECISION-001\n");

    assert!(preview(&repo).contains(&"      provenance: DECISION-001 (accepted)".to_string()));
}

#[test]
fn a_decision_accepted_after_the_base_is_not_accepted_at_base() {
    let repo = repo();
    decision(&repo, "proposed");
    repo.commit("decision");
    delta_committed(&repo, "- [AC-D1-1] New.\n  @per DECISION-001\n");
    decision(&repo, "accepted");
    repo.commit("accept it");

    assert!(preview(&repo)
        .contains(&"      provenance: DECISION-001 (not accepted at base)".to_string()));
}

#[test]
fn a_decision_created_after_the_base_is_not_accepted_at_base() {
    let repo = repo();
    delta_committed(&repo, "- [AC-D1-1] New.\n  @per DECISION-001\n");
    decision(&repo, "accepted");
    repo.commit("decision");

    assert!(preview(&repo)
        .contains(&"      provenance: DECISION-001 (not accepted at base)".to_string()));
}

#[test]
fn a_citation_that_is_not_a_decision_is_unverified() {
    let repo = repo();
    delta_committed(
        &repo,
        "- [AC-D1-1] New.\n  @per the Slack thread on retries\n",
    );

    assert!(preview(&repo)
        .contains(&"      provenance: the Slack thread on retries (unverified)".to_string()));
}

#[test]
fn an_assumption_resolved_in_place_says_it_was_assumed() {
    let repo = repo();
    delta_committed(&repo, "- [A-1] [assumed] Retries.\n");
    repo.delta_part(1, "SPEC-001", "- [A-1] Retries.\n");
    repo.commit("resolve");

    let lines = preview(&repo);

    assert!(lines.contains(&"    A-1: was assumed, now a commitment".to_string()));
    assert!(!lines.iter().any(|l| l.contains("New commitments")));
}

#[test]
fn an_assumption_resolved_in_place_as_observed_says_so() {
    let repo = repo();
    delta_committed(&repo, "- [A-1] [assumed] Retries.\n");
    repo.delta_part(1, "SPEC-001", "- [A-1] [observed] Retries.\n");
    repo.commit("resolve");

    assert!(preview(&repo).contains(&"    A-1: was assumed, now observed".to_string()));
}

#[test]
fn an_assumption_replaced_by_a_new_id_names_the_replacement() {
    let repo = repo();
    delta_committed(&repo, "- [A-1] [assumed] Retries.\n");
    repo.delta_part(1, "SPEC-001", "- [AC-D1-1] Retries twice.\n");
    repo.commit("resolve");

    let lines = preview(&repo);

    assert!(
        lines.contains(&"    A-1 → AC-D1-1".to_string()),
        "{lines:?}"
    );
    assert!(!lines.iter().any(|l| l.contains("Discarded")));
}

#[test]
fn a_deleted_assumption_is_discarded() {
    let repo = repo();
    delta_committed(
        &repo,
        "- [A-1] [assumed] Retries.\n- [AC-D1-1] Keeps order.\n",
    );
    repo.delta_part(1, "SPEC-001", "- [AC-D1-1] Keeps order.\n");
    repo.commit("drop it");

    let lines = preview(&repo);

    assert!(lines.contains(&"  Discarded assumptions".to_string()));
    assert!(lines.contains(&"    A-1: Retries.".to_string()));
}

#[test]
fn assumptions_that_cannot_be_paired_with_new_ids_are_discarded() {
    let repo = repo();
    delta_committed(&repo, "- [A-1] [assumed] One.\n- [A-2] [assumed] Two.\n");
    repo.delta_part(1, "SPEC-001", "- [AC-D1-1] Three.\n");
    repo.commit("rewrite");

    let lines = preview(&repo);

    assert!(lines.contains(&"    A-1: One.".to_string()), "{lines:?}");
    assert!(lines.contains(&"    A-2: Two.".to_string()));
    assert!(!lines.iter().any(|l| l.contains('→')));
}

#[test]
fn a_replacement_that_is_not_yet_committed_is_paired() {
    let repo = repo();
    delta_committed(&repo, "- [A-1] [assumed] Retries.\n");
    repo.delta_part(1, "SPEC-001", "- [AC-D1-1] Retries twice.\n");

    let lines = preview(&repo);

    assert!(
        lines.contains(&"    A-1 → AC-D1-1".to_string()),
        "{lines:?}"
    );
    assert!(!lines.iter().any(|l| l.contains("Discarded")));
    assert!(!lines.iter().any(|l| l.contains("New commitments")));
}

#[test]
fn an_unrelated_new_claim_does_not_hide_a_discarded_assumption() {
    let repo = repo();
    delta_committed(&repo, "- [A-1] [assumed] Retries failed uploads.\n");
    repo.delta_part(1, "SPEC-001", "- [AC-D1-5] Exports to CSV.\n");
    repo.commit("swap");

    let lines = preview(&repo);

    assert!(lines.contains(&"    A-1: Retries failed uploads.".to_string()));
    assert!(lines.contains(&"    +AC-D1-5 Exports to CSV.".to_string()));
    assert!(!lines.iter().any(|l| l.contains('→')), "{lines:?}");
}

#[test]
fn two_vanished_and_two_new_ids_are_all_discarded() {
    let repo = repo();
    delta_committed(
        &repo,
        "- [A-1] [assumed] Retries twice.\n- [A-2] [assumed] Batches writes.\n",
    );
    repo.delta_part(
        1,
        "SPEC-001",
        "- [AC-D1-1] Retries twice.\n- [AC-D1-2] Batches writes.\n",
    );
    repo.commit("rewrite");

    let lines = preview(&repo);

    assert!(
        lines.contains(&"    A-1: Retries twice.".to_string()),
        "{lines:?}"
    );
    assert!(lines.contains(&"    A-2: Batches writes.".to_string()));
    assert!(!lines.iter().any(|l| l.contains('→')));
}

#[test]
fn a_citation_with_trailing_punctuation_still_resolves_its_decision() {
    let repo = repo();
    decision(&repo, "accepted");
    repo.commit("decision");
    delta_committed(&repo, "- [AC-D1-1] New.\n  @per DECISION-001: because\n");

    assert!(
        preview(&repo).contains(&"      provenance: DECISION-001: because (accepted)".to_string())
    );
}
#[test]
fn a_delta_created_inside_the_range_still_resolves() {
    let repo = repo();
    repo.write("unrelated.txt", "x");
    repo.commit("unrelated work");
    delta_committed(&repo, "- [A-1] [assumed] Retries.\n");
    repo.delta_part(1, "SPEC-001", "- [A-1] Retries.\n");
    repo.commit("resolve");

    assert!(preview(&repo).contains(&"    A-1: was assumed, now a commitment".to_string()));
}

#[test]
fn a_refused_fold_shows_only_the_unresolved_assumptions() {
    let repo = repo();
    delta_committed(&repo, "- [AC-D1-2] New.\n- [A-1] [assumed] Probably.\n");

    let run = repo.run(&["apply", "DELTA-001"], "");

    assert_eq!(run.code, 2);
    let lines: Vec<_> = run.stdout.lines().collect();
    assert_eq!(
        &lines[..3],
        ["SPEC-001", "  Unresolved assumptions", "    A-1 Probably."]
    );
    assert!(!run.stdout.contains("New commitments"));
}

fn gate(repo: &Repo, from: &str, to: &str, claims: &str) -> Run {
    let payload = transition_payload(
        from,
        to,
        delta_doc(claims),
        vec![spec_doc("active", Some(SPEC_BEHAVIOUR))],
    );
    repo.run(&["apply"], &payload)
}

#[test]
fn the_complete_gate_warns_once_per_claim() {
    let repo = repo();
    decision(&repo, "accepted");
    repo.commit("decision");

    let run = gate(
        &repo,
        "in-progress",
        "complete",
        "- [AC-D1-1] New.\n  @per DECISION-001\n- [AC-D1-2] Other.\n- [AC-D1-3] [observed] Cached.\n",
    );

    assert_eq!(run.code, 0);
    assert!(run.findings().iter().all(|f| f["severity"] == "warning"));
    assert_eq!(
        run.messages(),
        vec![
            "AC-D1-1: new commitment; provenance: DECISION-001 (accepted)".to_string(),
            format!("AC-D1-2: new commitment; {NONE}"),
            "AC-D1-3: observed behaviour".to_string(),
        ]
    );
    assert_eq!(run.json()["updates"].as_array().unwrap().len(), 1);
}

#[test]
fn the_complete_gate_reports_resolved_and_discarded_assumptions() {
    let repo = repo();
    delta_committed(
        &repo,
        "- [A-1] [assumed] One retry.\n- [A-2] [assumed] Two.\n- [AC-D1-9] Keeps.\n",
    );
    repo.delta_part(
        1,
        "SPEC-001",
        "- [A-1] [assumed] One retry.\n- [AC-D1-9] Keeps.\n",
    );
    repo.commit("drop A-2");
    repo.delta_part(
        1,
        "SPEC-001",
        "- [AC-D1-1] One retry.\n- [AC-D1-9] Keeps.\n",
    );
    repo.commit("resolve");
    let payload = transition_payload(
        "in-progress",
        "complete",
        delta_doc("- [AC-D1-1] One retry.\n- [AC-D1-9] Keeps.\n"),
        vec![spec_doc("active", Some(SPEC_BEHAVIOUR))],
    );
    let mut payload: Value = serde_json::from_str(&payload).unwrap();
    payload["documents"][0]["path"] = json!("docs/deltas/DELTA-001-d/index.md");
    payload["documents"][0]["parts"][0]["path"] = json!(delta_part_path(1, "SPEC-001"));

    let run = repo.run(&["apply"], &payload.to_string());

    let messages = run.messages();
    assert!(
        messages.contains(&format!("A-1 → AC-D1-1: resolved assumption; {NONE}")),
        "{messages:?}"
    );
    assert!(messages.contains(&"A-2: discarded assumption".to_string()));
    assert!(run.findings().iter().all(|f| f["severity"] == "warning"));
}

#[test]
fn other_transitions_emit_no_accountability_findings() {
    let repo = repo();

    for (from, to) in [("draft", "in-progress"), ("in-progress", "draft")] {
        let run = gate(&repo, from, to, "- [AC-D1-1] New.\n");
        assert_eq!(run.findings(), Vec::<Value>::new(), "{from} -> {to}");
    }
}
