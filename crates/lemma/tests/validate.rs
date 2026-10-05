mod common;
use common::*;

fn validate(repo: &Repo, docs: Vec<serde_json::Value>) -> Run {
    repo.run(&["validate"], &validate_payload(docs))
}

fn spec(body: &str) -> serde_json::Value {
    doc("SPEC-001", "spec", "active", &["src/**"], &[], body)
}

fn delta(id: &str, status: &str, folds: &[&str], behaviour: &str) -> serde_json::Value {
    doc(id, "delta", status, &[], folds, &delta_body(behaviour))
}

#[test]
fn fresh_receipt_has_no_findings() {
    let repo = Repo::new();
    let sha = repo.head();
    let d = delta(
        "DELTA-001",
        "in-progress",
        &["SPEC-001"],
        &claim("AC-D1-1", "works", Some(&sha)),
    );

    let run = validate(&repo, vec![spec(""), d]);

    assert_eq!(run.code, 0);
    assert!(run.findings().is_empty(), "{:?}", run.messages());
}

#[test]
fn receipt_goes_stale_after_a_governed_change() {
    let repo = Repo::new();
    let sha = repo.head();
    repo.write("src/lib.rs", "pub fn a() { /* changed */ }\n");
    repo.commit("touch governed code");
    let d = delta(
        "DELTA-001",
        "in-progress",
        &["SPEC-001"],
        &claim("AC-D1-1", "works", Some(&sha)),
    );

    let run = validate(&repo, vec![spec(""), d]);

    let findings = run.findings();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0]["severity"], "warning");
    assert_eq!(findings[0]["id"], "DELTA-001");
    assert_eq!(findings[0]["line"], 8);
    assert!(run.has_message("stale"));
}

#[test]
fn receipt_goes_stale_after_the_test_file_changes() {
    let repo = Repo::new();
    let sha = repo.head();
    repo.write(
        TEST_FILE,
        &format!("{TEST_SRC}\n#[test]\nfn added() {{}}\n"),
    );
    repo.commit("edit test");
    let d = delta(
        "DELTA-001",
        "complete",
        &["SPEC-001"],
        &claim("AC-D1-1", "works", Some(&sha)),
    );

    assert!(validate(&repo, vec![spec(""), d]).has_message("stale"));
}

#[test]
fn ungoverned_change_leaves_receipt_fresh() {
    let repo = Repo::new();
    let sha = repo.head();
    repo.write("README.md", "hi\n");
    repo.commit("docs");
    let d = delta(
        "DELTA-001",
        "in-progress",
        &["SPEC-001"],
        &claim("AC-D1-1", "works", Some(&sha)),
    );

    assert!(validate(&repo, vec![spec(""), d]).findings().is_empty());
}

#[test]
fn missing_symbol_does_not_resolve() {
    let repo = Repo::new();
    let sha = repo.head();
    let body = format!("- [AC-D1-1] works\n  @ref {TEST_FILE}#gone@{sha}\n");
    let d = delta("DELTA-001", "in-progress", &["SPEC-001"], &body);

    let run = validate(&repo, vec![spec(""), d]);

    assert!(run.has_message("does not resolve"));
    assert!(run.has_message("symbol gone not found"));
}

#[test]
fn unknown_commit_and_missing_file_do_not_resolve() {
    let repo = Repo::new();
    let body = "- [AC-D1-1] a\n  @ref tests/store_test.rs#reopen_keeps_prior@deadbee\n- [AC-D1-2] b\n  @ref tests/nope.rs#x@deadbee\n";
    let d = delta("DELTA-001", "in-progress", &["SPEC-001"], body);

    let run = validate(&repo, vec![spec(""), d]);

    assert!(run.has_message("commit deadbee does not exist"));
}

#[test]
fn claim_without_receipt_is_flagged_but_intent_is_skipped() {
    let repo = Repo::new();
    let body = "- [AC-D1-1] bare\n- [AC-D1-2] [intent] feels right\n";
    let d = delta("DELTA-001", "in-progress", &["SPEC-001"], body);

    let run = validate(&repo, vec![spec(""), d]);

    let findings = run.findings();
    assert_eq!(findings.len(), 1);
    assert!(run.has_message("AC-D1-1: claim has no receipt"));
    assert_eq!(findings[0]["line"], 7);
}

#[test]
fn draft_delta_claims_need_no_receipt() {
    let repo = Repo::new();
    let d = delta("DELTA-001", "draft", &["SPEC-001"], "- [AC-D1-1] bare\n");

    assert!(validate(&repo, vec![spec(""), d]).findings().is_empty());
}

#[test]
fn stale_receipt_in_an_active_spec_is_flagged() {
    let repo = Repo::new();
    let sha = repo.head();
    repo.write("src/lib.rs", "pub fn b() {}\n");
    repo.commit("change");
    let s = spec(&format!(
        "## Behaviour\n\n{}",
        claim("AC-S-1", "holds", Some(&sha))
    ));

    let run = validate(&repo, vec![s]);

    assert_eq!(run.findings()[0]["id"], "SPEC-001");
    assert!(run.has_message("stale"));
}

#[test]
fn partition_naming_a_spec_outside_folds_into_is_flagged() {
    let repo = Repo::new();
    let body = "### SPEC-001\n\n- [AC-D1-1] [intent] a\n\n### SPEC-009\n\n- [AC-D1-2] [intent] b\n";
    let d = delta("DELTA-001", "draft", &["SPEC-001", "SPEC-002"], body);
    let s2 = doc("SPEC-002", "spec", "active", &[], &[], "");

    let run = validate(&repo, vec![spec(""), s2, d]);

    assert_eq!(run.findings().len(), 1);
    assert!(run.has_message("partition SPEC-009"));
}

#[test]
fn unpartitioned_claims_in_a_multi_target_delta_are_flagged() {
    let repo = Repo::new();
    let d = delta(
        "DELTA-001",
        "draft",
        &["SPEC-001", "SPEC-002"],
        "- [AC-D1-1] [intent] loose\n",
    );
    let s2 = doc("SPEC-002", "spec", "active", &[], &[], "");

    let run = validate(&repo, vec![spec(""), s2, d]);

    assert!(run.has_message("not under a spec partition"));
}

#[test]
fn fold_target_missing_from_the_payload_is_flagged() {
    let repo = Repo::new();
    let d = delta(
        "DELTA-001",
        "draft",
        &["SPEC-404"],
        "- [AC-D1-1] [intent] x\n",
    );

    let run = validate(&repo, vec![d]);

    assert!(run.has_message("fold target SPEC-404 does not resolve"));
}

#[test]
fn two_in_progress_deltas_touching_one_claim_id_are_both_flagged() {
    let repo = Repo::new();
    let a = delta(
        "DELTA-001",
        "in-progress",
        &["SPEC-001"],
        "- [AC-X-1] [intent] one\n",
    );
    let b = delta(
        "DELTA-002",
        "in-progress",
        &["SPEC-001"],
        "- [AC-X-1] [intent] two\n",
    );

    let run = validate(&repo, vec![spec(""), a, b]);

    let ids: Vec<String> = run
        .findings()
        .iter()
        .map(|f| f["id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(ids, vec!["DELTA-001", "DELTA-002"]);
    assert!(run.has_message("AC-X-1 is touched by in-progress deltas DELTA-001, DELTA-002"));
}

#[test]
fn a_draft_and_an_in_progress_delta_sharing_an_id_are_not_flagged() {
    let repo = Repo::new();
    let a = delta(
        "DELTA-001",
        "in-progress",
        &["SPEC-001"],
        "- [AC-X-1] [intent] one\n",
    );
    let b = delta(
        "DELTA-002",
        "draft",
        &["SPEC-001"],
        "- [AC-X-1] [intent] two\n",
    );

    assert!(validate(&repo, vec![spec(""), a, b]).findings().is_empty());
}

#[test]
fn bad_payload_reports_an_error_finding_and_exits_zero() {
    let repo = Repo::new();

    let run = repo.run(&["validate"], "not json");

    assert_eq!(run.code, 0);
    assert_eq!(run.findings()[0]["severity"], "error");
}

#[test]
fn unpinned_branch_commit_is_unresolved_with_a_distinct_message() {
    let repo = Repo::new();
    let base = repo.git(&["rev-parse", "--abbrev-ref", "HEAD"]);
    repo.git(&["checkout", "-q", "-b", "feature"]);
    repo.write("README.md", "branch\n");
    let sha = repo.commit("branch only");
    repo.git(&["checkout", "-q", &base]);
    let d = delta(
        "DELTA-001",
        "in-progress",
        &["SPEC-001"],
        &claim("AC-D1-1", "works", Some(&sha)),
    );

    let run = validate(&repo, vec![spec(""), d]);

    assert!(
        run.has_message("is not on this branch's history; re-pin after merge"),
        "{:?}",
        run.messages()
    );
    assert!(!run.has_message("stale"));
}

#[test]
fn same_claim_twice_in_one_delta_is_not_an_overlap() {
    let repo = Repo::new();
    let sha = repo.head();
    let both = format!(
        "{}{}",
        claim("AC-D1-1", "once", Some(&sha)),
        claim("AC-D1-1", "twice", Some(&sha))
    );
    let d = delta("DELTA-001", "in-progress", &["SPEC-001"], &both);

    assert!(!validate(&repo, vec![spec(""), d]).has_message("touched by"));
}

#[test]
fn spec_missing_from_payload_is_loaded_from_the_store() {
    let repo = Repo::new();
    repo.spec_file(1, "active", &["src/**"], "## Intent\n\nA.\n");
    repo.commit("spec");
    let d = delta(
        "DELTA-001",
        "in-progress",
        &["SPEC-001"],
        &claim("AC-D1-1", "works", Some(&repo.head())),
    );

    let run = validate(&repo, vec![d]);

    assert!(
        !run.has_message("does not resolve to a spec"),
        "{:?}",
        run.messages()
    );
}
