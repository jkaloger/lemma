mod common;
use common::*;
use serde_json::Value;

fn pre(repo: &Repo, from: &str, to: &str, moved: Value, context: Vec<Value>) -> Run {
    repo.run(
        &["pre-transition"],
        &transition_payload(from, to, moved, context),
    )
}

fn spec(n: u32, body: &str) -> Value {
    doc(
        &format!("SPEC-00{n}"),
        "spec",
        "active",
        &["src/**"],
        &[],
        body,
    )
}

fn delta(status: &str, folds: &[&str], body: &str) -> Value {
    doc("DELTA-001", "delta", status, &[], folds, body)
}

#[test]
fn start_passes_with_intent_claims_and_resolving_target() {
    let repo = Repo::new();
    let d = delta(
        "draft",
        &["SPEC-001"],
        &delta_body(&claim("AC-D1-1", "x", None)),
    );

    let run = pre(&repo, "draft", "in-progress", d, vec![spec(1, "")]);

    assert_eq!(run.code, 0);
    assert!(run.findings().is_empty());
}

#[test]
fn start_errors_without_intent_claims_or_folds_into() {
    let repo = Repo::new();
    let d = delta("draft", &[], "## Intent\n\n## Behaviour\n\n");

    let run = pre(&repo, "draft", "in-progress", d, vec![]);

    assert!(run.findings().iter().all(|f| f["severity"] == "error"));
    assert!(run.has_message("missing or empty `## Intent`"));
    assert!(run.has_message("no claims"));
    assert!(run.has_message("no folds-into link"));
}

#[test]
fn start_errors_when_fold_target_does_not_resolve() {
    let repo = Repo::new();
    let d = delta(
        "draft",
        &["SPEC-404"],
        &delta_body(&claim("AC-D1-1", "x", None)),
    );

    let run = pre(&repo, "draft", "in-progress", d, vec![spec(1, "")]);

    assert!(run.has_message("SPEC-404 does not resolve"));
}

#[test]
fn start_ignores_claims_inside_fences_and_comments() {
    let repo = Repo::new();
    let body = delta_body("```\n- [AC-D1-1] fenced\n```\n<!--\n- [AC-D1-2] commented\n-->\n");
    let d = delta("draft", &["SPEC-001"], &body);

    assert!(pre(&repo, "draft", "in-progress", d, vec![spec(1, "")]).has_message("no claims"));
}

#[test]
fn complete_errors_per_unreceipted_claim_and_skips_intent_and_strikes() {
    let repo = Repo::new();
    let behaviour = "- [AC-D1-1] bare\n- [AC-D1-2] [intent] vibes\n- ~~[AC-OLD-1]~~ removed\n- [AC-D1-3] bare too\n";
    let d = delta("in-progress", &["SPEC-001"], &delta_body(behaviour));

    let run = pre(&repo, "in-progress", "complete", d, vec![spec(1, "")]);

    assert_eq!(run.messages().len(), 2);
    assert!(run.has_message("AC-D1-1: claim has no receipt"));
    assert!(run.has_message("AC-D1-3: claim has no receipt"));
}

#[test]
fn complete_passes_with_fresh_receipts() {
    let repo = Repo::new();
    let sha = repo.head();
    let d = delta(
        "in-progress",
        &["SPEC-001"],
        &delta_body(&claim("AC-D1-1", "x", Some(&sha))),
    );

    assert!(pre(&repo, "in-progress", "complete", d, vec![spec(1, "")])
        .findings()
        .is_empty());
}

#[test]
fn complete_errors_on_stale_receipt_judged_against_the_partition_spec() {
    let repo = Repo::new();
    let sha = repo.head();
    repo.write("lib2/x.rs", "fn x() {}\n");
    repo.commit("change in spec 2 territory");
    let s1 = spec(1, "");
    let s2 = doc("SPEC-002", "spec", "active", &["lib2/**"], &[], "");
    let body = delta_body(&format!(
        "### SPEC-001\n\n{}\n### SPEC-002\n\n{}",
        claim("AC-D1-1", "a", Some(&sha)),
        claim("AC-D1-2", "b", Some(&sha))
    ));
    let d = delta("in-progress", &["SPEC-001", "SPEC-002"], &body);

    let run = pre(&repo, "in-progress", "complete", d, vec![s1, s2]);

    assert_eq!(run.messages().len(), 1);
    assert!(run.has_message("AC-D1-2: receipt is stale"));
    assert_eq!(run.findings()[0]["severity"], "error");
}

#[test]
fn complete_errors_when_receipt_does_not_resolve() {
    let repo = Repo::new();
    let sha = repo.head();
    let body = format!("- [AC-D1-1] x\n  @ref {TEST_FILE}#gone@{sha}\n");
    let d = delta("in-progress", &["SPEC-001"], &delta_body(&body));

    assert!(
        pre(&repo, "in-progress", "complete", d, vec![spec(1, "")]).has_message("does not resolve")
    );
}

const SPEC_BODY: &str = "## Intent\n\nWhy.\n\n## Behaviour\n\n- [AC-S-1] old text\n  @ref tests/store_test.rs#reopen_keeps_prior@1111111\n- [AC-S-2] doomed\n  @ref tests/store_test.rs#other_test@1111111\n- [AC-S-3] untouched\n";

#[test]
fn archive_appends_replaces_and_strikes_with_receipts_moving() {
    let repo = Repo::new();
    let behaviour = "- [AC-S-1] new text\n  @ref tests/store_test.rs#reopen_keeps_prior@2222222\n- ~~[AC-S-2]~~ doomed\n- [AC-D1-1] fresh claim\n  @ref tests/store_test.rs#other_test@2222222\n";
    let d = delta("complete", &["SPEC-001"], &delta_body(behaviour));

    let run = pre(&repo, "complete", "archived", d, vec![spec(1, SPEC_BODY)]);

    assert!(run.findings().is_empty());
    let updates = run.json()["updates"].as_array().unwrap().clone();
    assert_eq!(updates.len(), 1);
    assert_eq!(updates[0]["id"], "SPEC-001");
    assert_eq!(updates[0]["hash"], "hash-SPEC-001");
    assert_eq!(
        updates[0]["body"],
        "## Intent\n\nWhy.\n\n## Behaviour\n\n- [AC-S-1] new text\n  @ref tests/store_test.rs#reopen_keeps_prior@2222222\n- [AC-S-3] untouched\n- [AC-D1-1] fresh claim\n  @ref tests/store_test.rs#other_test@2222222\n"
    );
}

#[test]
fn archive_partitioned_delta_updates_each_target_with_its_own_hash() {
    let repo = Repo::new();
    let body =
        delta_body("### SPEC-001\n\n- [AC-A-1] for one\n\n### SPEC-002\n\n- [AC-B-1] for two\n");
    let d = delta("complete", &["SPEC-001", "SPEC-002"], &body);

    let run = pre(
        &repo,
        "complete",
        "archived",
        d,
        vec![spec(1, "## Intent\n\nx\n"), spec(2, "## Intent\n\ny\n")],
    );

    let updates = run.json()["updates"].as_array().unwrap().clone();
    assert_eq!(updates.len(), 2);
    assert_eq!(updates[0]["hash"], "hash-SPEC-001");
    assert_eq!(
        updates[0]["body"],
        "## Intent\n\nx\n\n## Behaviour\n\n- [AC-A-1] for one\n"
    );
    assert_eq!(updates[1]["hash"], "hash-SPEC-002");
    assert_eq!(
        updates[1]["body"],
        "## Intent\n\ny\n\n## Behaviour\n\n- [AC-B-1] for two\n"
    );
}

#[test]
fn archive_errors_when_a_target_spec_is_missing() {
    let repo = Repo::new();
    let d = delta(
        "complete",
        &["SPEC-001", "SPEC-009"],
        &delta_body("### SPEC-001\n\n- [AC-A-1] x\n"),
    );

    let run = pre(&repo, "complete", "archived", d, vec![spec(1, "")]);

    assert!(run.has_message("SPEC-009 does not resolve"));
    assert!(run.json().get("updates").is_none());
}

#[test]
fn other_transitions_return_no_findings() {
    let repo = Repo::new();
    let d = delta("archived", &[], "");

    let run = pre(&repo, "archived", "archived", d, vec![]);

    assert_eq!(run.code, 0);
    assert_eq!(run.json(), serde_json::json!({"findings": []}));
}

fn fresh_claim(repo: &Repo, id: &str) -> String {
    claim(id, "x", Some(&repo.head()))
}

#[test]
fn typo_partition_is_an_error_on_every_gate() {
    let repo = Repo::new();
    let body = delta_body(&format!(
        "### SPEC-009\n\n{}",
        fresh_claim(&repo, "AC-D1-1")
    ));
    for (from, to) in [
        ("draft", "in-progress"),
        ("in-progress", "complete"),
        ("complete", "archived"),
    ] {
        let d = delta(from, &["SPEC-001"], &body);

        let run = pre(&repo, from, to, d, vec![spec(1, "")]);

        assert!(run.has_message("would not fold"), "{from}->{to}");
        assert!(run.has_message("AC-D1-1"), "{from}->{to}");
        assert!(run.findings().iter().all(|f| f["severity"] == "error"));
        assert!(run.json()["updates"]
            .as_array()
            .is_none_or(|u| u.is_empty()));
    }
}

#[test]
fn unpartitioned_claim_in_multi_target_delta_would_not_fold() {
    let repo = Repo::new();
    let body = delta_body(&fresh_claim(&repo, "AC-D1-1"));
    let d = delta("in-progress", &["SPEC-001", "SPEC-002"], &body);

    let run = pre(
        &repo,
        "in-progress",
        "complete",
        d,
        vec![spec(1, ""), spec(2, "")],
    );

    assert!(run.has_message("AC-D1-1: claim would not fold"));
}

#[test]
fn edge_cases_subheading_in_single_target_delta_would_not_fold() {
    let repo = Repo::new();
    let body = delta_body(&format!(
        "### Edge cases\n\n{}",
        fresh_claim(&repo, "AC-D1-1")
    ));
    let d = delta("draft", &["SPEC-001"], &body);

    let run = pre(&repo, "draft", "in-progress", d, vec![spec(1, "")]);

    assert!(run.has_message("partition Edge is not one of"));
    assert!(run.has_message("AC-D1-1: claim would not fold"));
}

#[test]
fn struck_claims_in_unknown_partitions_are_not_reported_as_claims() {
    let repo = Repo::new();
    let body = delta_body("### SPEC-009\n\n- ~~[AC-OLD-1]~~ gone\n");
    let d = delta("draft", &["SPEC-001"], &body);

    let run = pre(&repo, "draft", "in-progress", d, vec![spec(1, "")]);

    assert!(!run.has_message("AC-OLD-1"));
}

#[test]
fn by_hand_in_progress_to_in_progress_runs_the_completion_gate() {
    let repo = Repo::new();
    let d = delta(
        "in-progress",
        &["SPEC-001"],
        &delta_body(&claim("AC-D1-1", "bare", None)),
    );

    let run = pre(&repo, "in-progress", "in-progress", d, vec![spec(1, "")]);

    assert!(run.has_message("AC-D1-1: claim has no receipt"));
}

#[test]
fn by_hand_draft_to_draft_runs_the_start_gate() {
    let repo = Repo::new();
    let d = delta("draft", &[], "## Intent\n\n## Behaviour\n\n");

    let run = pre(&repo, "draft", "draft", d, vec![]);

    assert!(run.has_message("no folds-into link"));
}

#[test]
fn same_status_archived_returns_nothing() {
    let repo = Repo::new();
    let d = delta(
        "archived",
        &["SPEC-001"],
        &delta_body(&fresh_claim(&repo, "AC-D1-1")),
    );

    let run = pre(&repo, "archived", "archived", d, vec![spec(1, "")]);

    assert_eq!(run.json(), serde_json::json!({"findings": []}));
}

#[test]
fn missing_context_specs_are_loaded_from_the_store() {
    let repo = Repo::new();
    repo.spec_file(1, "active", &["src/**"], "## Intent\n\nA.\n");
    repo.commit("spec");
    let d = delta(
        "in-progress",
        &["SPEC-001"],
        &delta_body(&fresh_claim(&repo, "AC-D1-1")),
    );

    let run = pre(&repo, "in-progress", "in-progress", d, vec![]);

    assert_eq!(run.messages(), ["checked in-progress → complete: ready"]);
}

fn by_hand(repo: &Repo, status: &str, d: Value, context: Vec<Value>) -> Run {
    pre(repo, status, status, d, context)
}

#[test]
fn by_hand_clean_run_says_what_was_checked() {
    let repo = Repo::new();
    let cases = [
        ("draft", "draft → in-progress"),
        ("in-progress", "in-progress → complete"),
        ("complete", "complete → archived"),
    ];
    for (status, edge) in cases {
        let body = delta_body(&fresh_claim(&repo, "AC-D1-1"));
        let d = delta(status, &["SPEC-001"], &body);

        let run = by_hand(&repo, status, d, vec![spec(1, "")]);

        let findings = run.findings();
        let expected = if status == "complete" { 2 } else { 1 };
        assert_eq!(findings.len(), expected, "{status}: {:?}", run.messages());
        assert!(findings.iter().all(|f| f["severity"] == "warning"));
        assert_eq!(
            findings.last().unwrap()["message"],
            format!("checked {edge}: ready").as_str()
        );
        assert!(run.json().get("updates").is_none());
    }
}

#[test]
fn by_hand_with_problems_reports_them_without_the_ready_line() {
    let repo = Repo::new();
    let d = delta(
        "in-progress",
        &["SPEC-001"],
        &delta_body(&claim("AC-D1-1", "bare", None)),
    );

    let run = by_hand(&repo, "in-progress", d, vec![spec(1, "")]);

    assert!(run.has_message("AC-D1-1: claim has no receipt"));
    assert!(!run.has_message("ready"));
}

#[test]
fn by_hand_complete_previews_the_fold_as_findings_only() {
    let repo = Repo::new();
    let d = delta(
        "complete",
        &["SPEC-001"],
        &delta_body(&fresh_claim(&repo, "AC-D1-1")),
    );

    let run = by_hand(&repo, "complete", d, vec![spec(1, "")]);

    assert_eq!(
        run.messages(),
        [
            "would fold 1 claims into SPEC-001",
            "checked complete → archived: ready"
        ]
    );
    assert!(run.findings().iter().all(|f| f["severity"] == "warning"));
    assert!(run.json().get("updates").is_none());
}

#[test]
fn by_hand_complete_preview_counts_replaces_and_strikes() {
    let repo = Repo::new();
    let spec_body = "## Behaviour\n\n- [AC-D1-1] old\n- [AC-D1-2] doomed\n";
    let body = delta_body(&format!(
        "{}- ~~[AC-D1-2]~~ doomed\n- [AC-D1-3] new\n",
        fresh_claim(&repo, "AC-D1-1")
    ));
    let d = delta("complete", &["SPEC-001"], &body);

    let run = by_hand(&repo, "complete", d, vec![spec(1, spec_body)]);

    assert!(run.has_message("would fold 2 claims into SPEC-001, strike 1"));
    assert!(run.json().get("updates").is_none());
}

#[test]
fn by_hand_complete_with_an_unresolved_target_reports_the_gate_error() {
    let repo = Repo::new();
    let d = delta(
        "complete",
        &["SPEC-009"],
        &delta_body(&fresh_claim(&repo, "AC-D1-1")),
    );

    let run = by_hand(&repo, "complete", d, vec![]);

    assert!(run.has_message("SPEC-009 does not resolve"));
    assert!(!run.has_message("ready"));
    assert!(run.json().get("updates").is_none());
}
