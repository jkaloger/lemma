mod common;
use common::*;

fn validate(repo: &Repo, docs: Vec<serde_json::Value>) -> Run {
    repo.run(&["validate"], &validate_payload(docs))
}

fn findings_besides_intent_counts(run: &Run) -> Vec<serde_json::Value> {
    run.findings()
        .into_iter()
        .filter(|f| !f["message"].as_str().unwrap().ends_with("[intent]"))
        .collect()
}

fn spec(body: &str) -> serde_json::Value {
    doc("SPEC-001", "spec", "active", &["src/**"], &[], body)
}

fn spec_with_claims(claims: &str) -> serde_json::Value {
    with_parts(spec(""), &[("behaviour", claims)])
}

fn delta_in(id: &str, status: &str, folds: &[&str], parts: &[(&str, &str)]) -> serde_json::Value {
    with_parts(doc(id, "delta", status, &[], folds, INTENT), parts)
}

fn delta(id: &str, status: &str, folds: &[&str], claims: &str) -> serde_json::Value {
    delta_in(id, status, folds, &[("behaviour.SPEC-001", claims)])
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
    assert_eq!(findings[0]["part"], "behaviour.SPEC-001");
    assert_eq!(findings[0]["line"], 2);
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
        "in-progress",
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

    let findings = findings_besides_intent_counts(&run);
    assert_eq!(findings.len(), 1);
    assert!(run.has_message("AC-D1-1: claim has no receipt"));
    assert_eq!(findings[0]["part"], "behaviour.SPEC-001");
    assert_eq!(findings[0]["line"], 1);
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
    let s = spec_with_claims(&claim("AC-S-1", "holds", Some(&sha)));

    let run = validate(&repo, vec![s]);

    assert_eq!(run.findings()[0]["id"], "SPEC-001");
    assert_eq!(run.findings()[0]["part"], "behaviour");
    assert!(run.has_message("stale"));
}

#[test]
fn claim_in_a_spec_index_is_an_error() {
    let repo = Repo::new();
    let s = spec("## Intent\n\nx\n\n- [AC-S-1] [intent] stray\n");

    let run = validate(&repo, vec![s]);

    let findings = findings_besides_intent_counts(&run);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0]["severity"], "error");
    assert!(run.has_message("AC-S-1: claim-shaped item is in index.md"));
}

#[test]
fn claim_in_a_delta_index_gets_the_finding() {
    let repo = Repo::new();
    let d = doc(
        "DELTA-001",
        "delta",
        "draft",
        &[],
        &["SPEC-001"],
        "## Intent\n\nx\n\n- [AC-D1-1] [intent] stray\n",
    );
    let d = with_parts(d, &[("behaviour.SPEC-001", "- [AC-D1-2] [intent] a\n")]);

    let run = validate(&repo, vec![spec(""), d]);

    let findings = findings_besides_intent_counts(&run);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0]["severity"], "error");
    assert!(run.has_message("AC-D1-1: claim-shaped item is in index.md"));
}

#[test]
fn a_fold_target_without_its_part_is_an_error() {
    let repo = Repo::new();
    for (status, severity) in [("draft", "warning"), ("in-progress", "error")] {
        let d = delta_in("DELTA-001", status, &["SPEC-001"], &[]);

        let run = validate(&repo, vec![spec(""), d]);

        let findings = run.findings();
        assert_eq!(findings.len(), 1, "{status}");
        assert_eq!(findings[0]["severity"], severity, "{status}");
        assert!(run.has_message("fold target SPEC-001 has no `behaviour.SPEC-001` part"));
    }
}

#[test]
fn an_unresolved_target_is_not_also_reported_as_a_missing_part() {
    let repo = Repo::new();
    let d = delta_in("DELTA-001", "in-progress", &["docs/specs/x.md"], &[]);

    let run = validate(&repo, vec![d]);

    assert_eq!(run.findings().len(), 1);
    assert!(run.has_message("does not resolve to a spec"));
}

#[test]
fn a_part_naming_no_fold_target_is_an_error_carrying_the_part() {
    let repo = Repo::new();
    let d = delta_in(
        "DELTA-001",
        "draft",
        &["SPEC-001"],
        &[
            ("behaviour.SPEC-001", "- [AC-D1-1] [intent] a\n"),
            ("behaviour.SPEC-009", "- [AC-D1-2] [intent] b\n"),
            ("behaviour", "- [AC-D1-3] [intent] c\n"),
            ("tasks", "- [AC-D1-4] not read\n"),
        ],
    );

    let run = validate(&repo, vec![spec(""), d]);

    let errors: Vec<_> = findings_besides_intent_counts(&run);
    let parts: Vec<&str> = errors.iter().map(|f| f["part"].as_str().unwrap()).collect();
    assert_eq!(parts, ["behaviour.SPEC-009", "behaviour"]);
    assert!(errors.iter().all(|f| f["severity"] == "error"));
    assert!(run.has_message("part behaviour.SPEC-009 does not name"));
}

#[test]
fn a_multi_target_delta_with_a_part_per_target_is_clean() {
    let repo = Repo::new();
    let d = delta_in(
        "DELTA-001",
        "draft",
        &["SPEC-001", "SPEC-002"],
        &[
            ("behaviour.SPEC-001", "- [AC-D1-1] [intent] a\n"),
            ("behaviour.SPEC-002", "- [AC-D1-2] [intent] b\n"),
        ],
    );
    let s2 = doc("SPEC-002", "spec", "active", &[], &[], "");

    let run = validate(&repo, vec![spec(""), s2, d]);

    assert!(findings_besides_intent_counts(&run).is_empty());
}

#[test]
fn receipts_are_judged_against_the_spec_named_by_the_part() {
    let repo = Repo::new();
    let sha = repo.head();
    repo.write("lib2/x.rs", "fn x() {}\n");
    repo.commit("spec 2 territory");
    let s2 = doc("SPEC-002", "spec", "active", &["lib2/**"], &[], "");
    let d = delta_in(
        "DELTA-001",
        "in-progress",
        &["SPEC-001", "SPEC-002"],
        &[
            ("behaviour.SPEC-001", &claim("AC-D1-1", "a", Some(&sha))),
            ("behaviour.SPEC-002", &claim("AC-D1-2", "b", Some(&sha))),
        ],
    );

    let run = validate(&repo, vec![spec(""), s2, d]);

    let findings = run.findings();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0]["part"], "behaviour.SPEC-002");
}

#[test]
fn fold_target_missing_from_the_payload_is_flagged() {
    let repo = Repo::new();
    let d = delta_in(
        "DELTA-001",
        "draft",
        &["SPEC-404"],
        &[("behaviour.SPEC-404", "- [AC-D1-1] [intent] x\n")],
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

    let ids: Vec<String> = findings_besides_intent_counts(&run)
        .iter()
        .map(|f| f["id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(ids, vec!["DELTA-001", "DELTA-002"]);
    assert!(findings_besides_intent_counts(&run)
        .iter()
        .all(|f| f["part"] == "behaviour.SPEC-001" && f["line"] == 1));
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

    let run = validate(&repo, vec![spec(""), a, b]);

    assert!(findings_besides_intent_counts(&run).is_empty());
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

#[test]
fn index_finding_lines_are_file_lines_not_body_lines() {
    let repo = Repo::new();
    repo.spec_file(
        1,
        "active",
        &["src/**"],
        "## Intent\n\nx\n\n- [AC-S-1] [intent] stray\n",
    );
    let rel = "docs/specs/SPEC-001-s/index.md";
    let file = repo.read(rel);
    for body in [
        raw_body_of(&file),
        raw_body_of(&file).trim_start_matches('\n'),
    ] {
        let mut s = spec(body);
        s["path"] = rel.into();

        let run = validate(&repo, vec![s]);

        assert_eq!(
            findings_besides_intent_counts(&run)[0]["line"],
            repo.claim_line(rel, "AC-S-1"),
            "{body:?}"
        );
    }
}

#[test]
fn part_findings_carry_the_part_file_line() {
    let repo = Repo::new();
    repo.delta_file(1, "in-progress", &["SPEC-001"], INTENT);
    repo.delta_part(1, "SPEC-001", "- [AC-D1-1] [intent] a\n- [AC-D1-2] bare\n");
    let rel = delta_part_path(1, "SPEC-001");
    let body = repo.read(&rel);
    let mut d = delta_in(
        "DELTA-001",
        "in-progress",
        &["SPEC-001"],
        &[("behaviour.SPEC-001", &body)],
    );
    d["parts"][0]["path"] = rel.as_str().into();

    let run = validate(&repo, vec![spec(""), d]);

    let finding = &findings_besides_intent_counts(&run)[0];
    assert_eq!(finding["part"], "behaviour.SPEC-001");
    assert_eq!(finding["line"], repo.claim_line(&rel, "AC-D1-2"));
}

#[test]
fn one_of_two_intent_claims_gets_one_warning() {
    let repo = Repo::new();
    let body = "- [AC-D1-1] works\n- [AC-D1-2] [intent] feels right\n";
    let d = delta("DELTA-001", "draft", &["SPEC-001"], body);

    let run = validate(&repo, vec![spec(""), d]);

    let findings = run.findings();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0]["severity"], "warning");
    assert_eq!(findings[0]["id"], "DELTA-001");
    assert_eq!(findings[0]["message"], "1 of 2 claims are [intent]");
}

#[test]
fn no_intent_claims_means_no_intent_warning() {
    let repo = Repo::new();
    let d = delta("DELTA-001", "draft", &["SPEC-001"], "- [AC-D1-1] works\n");

    assert!(validate(&repo, vec![spec(""), d]).findings().is_empty());
}

fn checked_claim(id: &str, sha: &str) -> String {
    format!("- [{id}] verified by hand\n  @checked jack@{sha}\n")
}

#[test]
fn a_checked_receipt_is_fresh_until_governed_code_changes() {
    let repo = Repo::new();
    let sha = repo.head();
    let d = || {
        delta(
            "DELTA-001",
            "in-progress",
            &["SPEC-001"],
            &checked_claim("AC-D1-1", &sha),
        )
    };
    assert!(validate(&repo, vec![spec(""), d()]).findings().is_empty());

    repo.write(TEST_FILE, &format!("{TEST_SRC}\n// edit\n"));
    repo.commit("test file is not governed");
    assert!(validate(&repo, vec![spec(""), d()]).findings().is_empty());

    repo.write("src/lib.rs", "pub fn a() { /* changed */ }\n");
    repo.commit("governed change");
    let run = validate(&repo, vec![spec(""), d()]);
    assert!(run.has_message("receipt is stale"), "{:?}", run.messages());
}

#[test]
fn a_checked_receipt_with_an_unknown_commit_does_not_resolve() {
    let repo = Repo::new();
    let d = delta(
        "DELTA-001",
        "in-progress",
        &["SPEC-001"],
        &checked_claim("AC-D1-1", "deadbee"),
    );

    assert!(validate(&repo, vec![spec(""), d]).has_message("commit deadbee does not exist"));
}

#[test]
fn a_governed_change_after_completion_stales_the_spec_only() {
    let repo = Repo::new();
    let sha = repo.head();
    repo.write("src/lib.rs", "pub fn a() { /* changed */ }\n");
    repo.commit("governed change after completion");
    let s = spec_with_claims(&claim("AC-S-1", "holds", Some(&sha)));
    let d = delta(
        "DELTA-001",
        "complete",
        &["SPEC-001"],
        &claim("AC-D1-1", "works", Some(&sha)),
    );

    let run = validate(&repo, vec![s, d]);

    let findings = run.findings();
    let ids: Vec<&str> = findings.iter().map(|f| f["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["SPEC-001"], "{:?}", run.messages());
}

const BASED_SPEC: &str = "- [AC-S-1] old text\n";

fn based_run(repo: &Repo, status: &str, delta_claims: &str, spec_claims: &str) -> Run {
    let d = delta("DELTA-001", status, &["SPEC-001"], delta_claims);
    validate(repo, vec![spec_with_claims(spec_claims), d])
}

fn severity_of(run: &Run, needle: &str) -> Option<String> {
    run.findings()
        .into_iter()
        .find(|f| f["message"].as_str().unwrap().contains(needle))
        .map(|f| f["severity"].as_str().unwrap().to_string())
}

#[test]
fn replace_without_base_is_an_error_until_complete() {
    let repo = Repo::new();
    for status in ["draft", "in-progress"] {
        let run = based_run(&repo, status, "- [AC-S-1] new\n", BASED_SPEC);
        assert_eq!(
            severity_of(&run, "without `@base`").as_deref(),
            Some("error"),
            "{status}"
        );
    }
    let run = based_run(&repo, "complete", "- [AC-S-1] new\n", BASED_SPEC);
    assert!(severity_of(&run, "without `@base`").is_none());
}

#[test]
fn strike_of_an_unknown_id_is_an_error() {
    let repo = Repo::new();
    let run = based_run(&repo, "draft", "- ~~[AC-S-9]~~ gone\n", BASED_SPEC);
    assert_eq!(
        severity_of(&run, "AC-S-9").as_deref(),
        Some("error"),
        "{:?}",
        run.messages()
    );
}

#[test]
fn base_mismatch_and_orphan_base_are_warnings() {
    let repo = Repo::new();
    let stale = "- [AC-S-1] new\n  @base 000000000000\n";
    let run = based_run(&repo, "draft", stale, BASED_SPEC);
    assert_eq!(
        severity_of(&run, "spec block changed").as_deref(),
        Some("warning")
    );
    let run = based_run(
        &repo,
        "in-progress",
        "- [AC-S-2] new\n  @base 000000000000\n  [intent]\n",
        BASED_SPEC,
    );
    assert_eq!(
        severity_of(&run, "has `@base` but SPEC-001 has no claim AC-S-2").as_deref(),
        Some("warning")
    );
}

#[test]
fn matching_base_is_clean() {
    let repo = Repo::new();
    let delta_claims = format!(
        "- [AC-S-1] new\n  [intent]\n{}",
        base_line_for(BASED_SPEC, "AC-S-1")
    );
    let run = based_run(&repo, "draft", &delta_claims, BASED_SPEC);
    assert!(
        findings_besides_intent_counts(&run).is_empty(),
        "{:?}",
        run.messages()
    );
}

fn in_progress_with(repo: &Repo, claims: &str, spec: serde_json::Value) -> Run {
    let d = delta("DELTA-001", "in-progress", &["SPEC-001"], claims);
    validate(repo, vec![spec, d])
}

#[test]
fn a_claim_without_a_receipt_on_an_in_progress_delta_is_an_error() {
    let repo = Repo::new();

    let run = in_progress_with(&repo, "- [AC-D1-1] bare\n", spec(""));

    assert_eq!(
        severity_of(&run, "claim has no receipt").as_deref(),
        Some("error")
    );
}

#[test]
fn a_receipt_that_does_not_resolve_is_an_error() {
    let repo = Repo::new();
    let sha = repo.head();
    let body = format!("- [AC-D1-1] works\n  @ref {TEST_FILE}#gone@{sha}\n");

    let run = in_progress_with(&repo, &body, spec(""));

    assert_eq!(
        severity_of(&run, "does not resolve").as_deref(),
        Some("error")
    );
}

#[test]
fn an_unresolved_fold_target_is_an_error() {
    let repo = Repo::new();
    let d = delta("DELTA-001", "draft", &["SPEC-404"], "- [AC-D1-1] a\n");

    let run = validate(&repo, vec![d]);

    assert_eq!(
        severity_of(&run, "fold target SPEC-404 does not resolve").as_deref(),
        Some("error")
    );
}

#[test]
fn a_stale_receipt_stays_a_warning() {
    let repo = Repo::new();
    let sha = repo.head();
    repo.write("src/lib.rs", "pub fn a() { /* changed */ }\n");
    repo.commit("touch governed code");

    let run = in_progress_with(&repo, &claim("AC-D1-1", "works", Some(&sha)), spec(""));

    assert_eq!(severity_of(&run, "stale").as_deref(), Some("warning"));
    assert_eq!(run.code, 0);
}

#[test]
fn an_intent_count_stays_a_warning() {
    let repo = Repo::new();

    let run = in_progress_with(&repo, "- [AC-D1-1] [intent] feels right\n", spec(""));

    assert_eq!(severity_of(&run, "[intent]").as_deref(), Some("warning"));
}

fn production_claim(repo: &Repo) -> String {
    let sha = repo.head();
    format!("- [AC-D1-1] works\n  @ref src/lib.rs#a@{sha}\n")
}

#[test]
fn a_production_path_receipt_is_an_error_on_the_receipt_line() {
    let repo = Repo::new();

    let run = in_progress_with(&repo, &production_claim(&repo), spec(""));

    let finding = &run.findings()[0];
    assert_eq!(finding["severity"], "error");
    assert_eq!(finding["line"], 2);
    assert_eq!(finding["part"], "behaviour.SPEC-001");
    assert_eq!(
        finding["message"],
        "AC-D1-1: receipt src/lib.rs is not a test file under SPEC-001's `tests` globs"
    );
}

#[test]
fn a_production_path_receipt_in_a_spec_is_an_error() {
    let repo = Repo::new();
    let s = spec_with_claims(&production_claim(&repo));

    let run = validate(&repo, vec![s]);

    assert!(run.has_message("receipt src/lib.rs is not a test file under SPEC-001's"));
}

#[test]
fn default_tests_globs_accept_the_test_file() {
    let repo = Repo::new();
    let sha = repo.head();

    let run = in_progress_with(&repo, &claim("AC-D1-1", "ok", Some(&sha)), spec(""));

    assert!(run.findings().is_empty(), "{:?}", run.messages());
}

#[test]
fn attributes_tests_override_the_default_globs() {
    let repo = Repo::new();
    let sha = repo.head();
    let overridden = with_tests(spec(""), &["checks/**"]);

    let run = in_progress_with(&repo, &claim("AC-D1-1", "ok", Some(&sha)), overridden);

    assert!(run.has_message(&format!("receipt {TEST_FILE} is not a test file")));
}

#[test]
fn a_checked_receipt_is_exempt_from_the_tests_globs() {
    let repo = Repo::new();
    let sha = repo.head();
    let body = format!("- [AC-D1-1] works\n  @checked manual@{sha}\n");

    let run = in_progress_with(&repo, &body, spec(""));

    assert!(!run.has_message("not a test file"), "{:?}", run.messages());
}

#[test]
fn intent_claims_skip_the_tests_glob_check() {
    let repo = Repo::new();
    let sha = repo.head();
    let body = format!("- [AC-D1-1] [intent] feels fast\n  @ref src/lib.rs#a@{sha}\n");

    let run = in_progress_with(&repo, &body, spec(""));

    assert!(!run.has_message("not a test file"), "{:?}", run.messages());
}

#[test]
fn invalid_governs_and_tests_globs_on_a_spec_are_errors() {
    let repo = Repo::new();
    let broken = with_tests(
        doc("SPEC-001", "spec", "active", &["src/["], &[], INTENT),
        &["checks/["],
    );

    let run = validate(&repo, vec![broken]);

    let errors: Vec<String> = run
        .findings()
        .iter()
        .filter(|f| f["severity"] == "error")
        .map(|f| f["message"].as_str().unwrap().to_string())
        .collect();
    assert!(errors
        .iter()
        .any(|m| m.starts_with("governs glob `src/[` is invalid:")));
    assert!(errors
        .iter()
        .any(|m| m.starts_with("tests glob `checks/[` is invalid:")));
}

#[test]
fn lazyspec_validate_exits_non_zero_on_an_unreceipted_in_progress_claim() {
    let Ok(lazyspec) = which("lazyspec") else {
        if std::env::var_os("CI").is_some() {
            panic!("CI is set but lazyspec is not on PATH; the real-lazyspec validate test must not be skipped in CI");
        }
        eprintln!("skipping: lazyspec is not on PATH");
        return;
    };
    let repo = Repo::new();
    let lemma = env!("CARGO_BIN_EXE_lemma");
    repo.write(
        ".lazyspec.toml",
        &format!(
            "{CONFIG}\n[[hooks]]\nname = \"lemma-validate\"\nevent = \"validate\"\ntypes = [\"spec\", \"delta\"]\nrun = [\"{lemma}\", \"validate\"]\n"
        ),
    );
    repo.spec_file(1, "active", &["src/**"], "## Intent\n\nA.\n");
    repo.delta_file(1, "in-progress", &["SPEC-001"], INTENT);
    repo.delta_part(1, "SPEC-001", "- [AC-D1-1] bare\n");
    repo.commit("fixture");

    let trust = repo.run_real(&lazyspec, &["hook", "trust"]);
    assert_eq!(trust.code, 0, "{}{}", trust.stdout, trust.stderr);
    let run = repo.run_real(&lazyspec, &["validate", "--warnings"]);

    assert_eq!(run.code, 2, "{}{}", run.stdout, run.stderr);
    let output = format!("{}{}", run.stdout, run.stderr);
    assert!(output.contains("has no receipt"), "{output}");
}

fn which(program: &str) -> Result<String, ()> {
    let path = std::env::var_os("PATH").ok_or(())?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(program))
        .find(|candidate| candidate.is_file())
        .map(|p| p.to_string_lossy().into_owned())
        .ok_or(())
}
