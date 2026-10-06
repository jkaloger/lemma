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
    assert_eq!(run.findings(), Vec::<Value>::new());
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

    assert_eq!(run.findings().len(), 1);
    assert_eq!(run.findings()[0]["severity"], "warning");
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
            "SPEC-001: -AC-S1-1 Reopen keeps the prior answer.",
            "SPEC-001: +AC-D1-1 Reopen keeps it across restarts."
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
