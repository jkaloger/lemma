mod common;
use common::*;
use serde_json::Value;

fn pre(repo: &Repo, from: &str, to: &str, moved: Value, context: Vec<Value>) -> Run {
    repo.run(
        &["pre-transition"],
        &transition_payload(from, to, moved, context),
    )
}

fn spec(n: u32, claims: &str) -> Value {
    let spec = doc(
        &format!("SPEC-00{n}"),
        "spec",
        "active",
        &["src/**"],
        &[],
        "## Intent\n\nWhy.\n",
    );
    with_parts(spec, &[("behaviour", claims)])
}

fn delta_in(status: &str, folds: &[&str], parts: &[(&str, &str)]) -> Value {
    with_parts(doc("DELTA-001", "delta", status, &[], folds, INTENT), parts)
}

fn delta(status: &str, folds: &[&str], claims: &str) -> Value {
    delta_in(status, folds, &[("behaviour.SPEC-001", claims)])
}

#[test]
fn start_passes_with_intent_claims_and_resolving_target() {
    let repo = Repo::new();
    let d = delta("draft", &["SPEC-001"], &claim("AC-D1-1", "x", None));

    let run = pre(&repo, "draft", "in-progress", d, vec![spec(1, "")]);

    assert_eq!(run.code, 0);
    assert!(run.findings().is_empty());
}

#[test]
fn start_errors_without_intent_claims_or_folds_into() {
    let repo = Repo::new();
    let d = with_parts(
        doc("DELTA-001", "delta", "draft", &[], &[], "## Intent\n\n"),
        &[],
    );

    let run = pre(&repo, "draft", "in-progress", d, vec![]);

    assert!(run.findings().iter().all(|f| f["severity"] == "error"));
    assert!(run.has_message("missing or empty `## Intent`"));
    assert!(run.has_message("no claims"));
    assert!(run.has_message("no folds-into link"));
}

#[test]
fn start_errors_when_fold_target_does_not_resolve() {
    let repo = Repo::new();
    let d = delta_in(
        "draft",
        &["SPEC-404"],
        &[("behaviour.SPEC-404", &claim("AC-D1-1", "x", None))],
    );

    let run = pre(&repo, "draft", "in-progress", d, vec![spec(1, "")]);

    assert!(run.has_message("SPEC-404 does not resolve"));
}

#[test]
fn start_ignores_claims_inside_fences_and_comments() {
    let repo = Repo::new();
    let claims = "```\n- [AC-D1-1] fenced\n```\n<!--\n- [AC-D1-2] commented\n-->\n";
    let d = delta("draft", &["SPEC-001"], claims);

    assert!(pre(&repo, "draft", "in-progress", d, vec![spec(1, "")]).has_message("no claims"));
}

#[test]
fn start_errors_when_a_fold_target_has_no_part() {
    let repo = Repo::new();
    let d = delta_in(
        "draft",
        &["SPEC-001", "SPEC-002"],
        &[("behaviour.SPEC-001", &claim("AC-D1-1", "x", None))],
    );

    let run = pre(
        &repo,
        "draft",
        "in-progress",
        d,
        vec![spec(1, ""), spec(2, "")],
    );

    assert_eq!(run.messages().len(), 1);
    assert!(run.has_message("fold target SPEC-002 has no `behaviour.SPEC-002` part"));
}

#[test]
fn start_errors_on_a_part_naming_no_fold_target() {
    let repo = Repo::new();
    let d = delta_in(
        "draft",
        &["SPEC-001"],
        &[
            ("behaviour.SPEC-001", &claim("AC-D1-1", "x", None)),
            ("behaviour.SPEC-009", &claim("AC-D1-2", "y", None)),
        ],
    );

    let run = pre(&repo, "draft", "in-progress", d, vec![spec(1, "")]);

    assert_eq!(run.messages().len(), 1);
    assert_eq!(run.findings()[0]["part"], "behaviour.SPEC-009");
    assert_eq!(run.findings()[0]["severity"], "error");
}

#[test]
fn start_counts_claims_across_parts() {
    let repo = Repo::new();
    let d = delta_in(
        "draft",
        &["SPEC-001", "SPEC-002"],
        &[
            ("behaviour.SPEC-001", ""),
            ("behaviour.SPEC-002", &claim("AC-D1-1", "x", None)),
        ],
    );

    let run = pre(
        &repo,
        "draft",
        "in-progress",
        d,
        vec![spec(1, ""), spec(2, "")],
    );

    assert!(run.findings().is_empty(), "{:?}", run.messages());
}

#[test]
fn complete_errors_per_unreceipted_claim_and_skips_intent_and_strikes() {
    let repo = Repo::new();
    let claims = "- [AC-D1-1] bare\n- [AC-D1-2] [intent] vibes\n- ~~[AC-OLD-1]~~ removed\n- [AC-D1-3] bare too\n";
    let d = delta("in-progress", &["SPEC-001"], claims);

    let run = pre(&repo, "in-progress", "complete", d, vec![spec(1, "")]);

    assert_eq!(run.messages().len(), 2);
    assert!(run.has_message("AC-D1-1: claim has no receipt"));
    assert!(run.has_message("AC-D1-3: claim has no receipt"));
    let findings = run.findings();
    assert_eq!(findings[0]["part"], "behaviour.SPEC-001");
    assert_eq!(findings[0]["line"], 1);
    assert_eq!(findings[1]["line"], 4);
}

#[test]
fn complete_passes_with_fresh_receipts() {
    let repo = Repo::new();
    let sha = repo.head();
    let d = delta(
        "in-progress",
        &["SPEC-001"],
        &claim("AC-D1-1", "x", Some(&sha)),
    );

    assert!(pre(&repo, "in-progress", "complete", d, vec![spec(1, "")])
        .findings()
        .is_empty());
}

#[test]
fn complete_errors_on_stale_receipt_judged_against_the_part_spec() {
    let repo = Repo::new();
    let sha = repo.head();
    repo.write("lib2/x.rs", "fn x() {}\n");
    repo.commit("change in spec 2 territory");
    let s1 = spec(1, "");
    let s2 = with_parts(
        doc("SPEC-002", "spec", "active", &["lib2/**"], &[], ""),
        &[("behaviour", "")],
    );
    let d = delta_in(
        "in-progress",
        &["SPEC-001", "SPEC-002"],
        &[
            ("behaviour.SPEC-001", &claim("AC-D1-1", "a", Some(&sha))),
            ("behaviour.SPEC-002", &claim("AC-D1-2", "b", Some(&sha))),
        ],
    );

    let run = pre(&repo, "in-progress", "complete", d, vec![s1, s2]);

    assert_eq!(run.messages().len(), 1);
    assert!(run.has_message("AC-D1-2: receipt is stale"));
    assert_eq!(run.findings()[0]["severity"], "error");
    assert_eq!(run.findings()[0]["part"], "behaviour.SPEC-002");
    assert_eq!(run.findings()[0]["line"], 2);
    assert!(updates_of(&run).is_empty());
}

#[test]
fn complete_errors_when_receipt_does_not_resolve() {
    let repo = Repo::new();
    let sha = repo.head();
    let claims = format!("- [AC-D1-1] x\n  @ref {TEST_FILE}#gone@{sha}\n");
    let d = delta("in-progress", &["SPEC-001"], &claims);

    assert!(
        pre(&repo, "in-progress", "complete", d, vec![spec(1, "")]).has_message("does not resolve")
    );
}

const SPEC_CLAIMS: &str = "- [AC-S-1] old text\n  @ref tests/store_test.rs#reopen_keeps_prior@1111111\n- [AC-S-2] doomed\n  @ref tests/store_test.rs#other_test@1111111\n- [AC-S-3] untouched\n";

fn updates_of(run: &Run) -> Vec<Value> {
    run.json()["updates"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

#[test]
fn complete_folds_into_the_behaviour_part_when_the_gate_is_clean() {
    let repo = Repo::new();
    let sha = repo.head();
    let claims = format!("- [AC-S-1] new text\n  @ref tests/store_test.rs#reopen_keeps_prior@{sha}\n- ~~[AC-S-2]~~ doomed\n- [AC-D1-1] fresh claim\n  @ref tests/store_test.rs#other_test@{sha}\n");
    let d = delta("in-progress", &["SPEC-001"], &claims);

    let run = pre(
        &repo,
        "in-progress",
        "complete",
        d,
        vec![spec(1, SPEC_CLAIMS)],
    );

    assert!(run.findings().is_empty(), "{:?}", run.messages());
    let updates = updates_of(&run);
    assert_eq!(updates.len(), 1);
    assert_eq!(updates[0]["id"], "SPEC-001");
    assert_eq!(updates[0]["part"], "behaviour");
    assert_eq!(updates[0]["hash"], "hash-SPEC-001");
    assert_eq!(
        updates[0]["body"],
        format!("- [AC-S-1] new text\n  @ref tests/store_test.rs#reopen_keeps_prior@{sha}\n- [AC-S-3] untouched\n- [AC-D1-1] fresh claim\n  @ref tests/store_test.rs#other_test@{sha}\n")
    );
}

#[test]
fn a_gate_error_returns_no_updates() {
    let repo = Repo::new();
    let d = delta("in-progress", &["SPEC-001"], "- [AC-D1-1] bare\n");

    let run = pre(&repo, "in-progress", "complete", d, vec![spec(1, "")]);

    assert!(run.has_message("AC-D1-1: claim has no receipt"));
    assert!(run.json().get("updates").is_none());
}

#[test]
fn complete_folds_each_targets_own_part_into_its_own_spec() {
    let repo = Repo::new();
    let d = delta_in(
        "in-progress",
        &["SPEC-001", "SPEC-002"],
        &[
            (
                "behaviour.SPEC-001",
                "- [AC-A-1] [intent] for one\n- ~~[AC-A-0]~~ old\n",
            ),
            ("behaviour.SPEC-002", "- [AC-B-1] [intent] for two\n"),
        ],
    );

    let run = pre(
        &repo,
        "in-progress",
        "complete",
        d,
        vec![spec(1, "- [AC-A-0] old\n"), spec(2, "")],
    );

    let updates = updates_of(&run);
    assert_eq!(updates.len(), 2);
    assert_eq!(updates[0]["hash"], "hash-SPEC-001");
    assert_eq!(updates[0]["part"], "behaviour");
    assert_eq!(updates[0]["body"], "- [AC-A-1] [intent] for one\n");
    assert_eq!(updates[1]["hash"], "hash-SPEC-002");
    assert_eq!(updates[1]["part"], "behaviour");
    assert_eq!(updates[1]["body"], "- [AC-B-1] [intent] for two\n");
}

#[test]
fn complete_errors_without_updates_when_a_target_spec_has_no_behaviour_part() {
    let repo = Repo::new();
    let d = delta("in-progress", &["SPEC-001"], "- [AC-A-1] [intent] x\n");
    let flat_spec = doc(
        "SPEC-001",
        "spec",
        "active",
        &["src/**"],
        &[],
        "## Intent\n\nx\n",
    );

    let run = pre(&repo, "in-progress", "complete", d, vec![flat_spec]);

    assert!(run.has_message("SPEC-001 has no `behaviour` part"));
    assert!(run.findings().iter().all(|f| f["severity"] == "error"));
    assert!(run.json().get("updates").is_none());
}

#[test]
fn complete_errors_when_a_target_spec_is_missing() {
    let repo = Repo::new();
    let d = delta_in(
        "in-progress",
        &["SPEC-001", "SPEC-009"],
        &[("behaviour.SPEC-001", "- [AC-A-1] [intent] x\n")],
    );

    let run = pre(&repo, "in-progress", "complete", d, vec![spec(1, "")]);

    assert!(run.has_message("SPEC-009 does not resolve"));
    assert!(run.json().get("updates").is_none());
}

#[test]
fn complete_errors_instead_of_writing_when_a_target_came_from_the_store() {
    let repo = Repo::new();
    repo.spec_file(1, "active", &["src/**"], "## Intent\n\nA.\n");
    repo.spec_behaviour(1, "- [AC-S-1] kept\n");
    repo.commit("spec");
    let d = delta("in-progress", &["SPEC-001"], "- [AC-A-1] [intent] new\n");

    let run = pre(&repo, "in-progress", "complete", d, vec![]);

    assert!(run.has_message("SPEC-001 was not in the hook payload"));
    assert!(run.findings().iter().all(|f| f["severity"] == "error"));
    assert!(run.json().get("updates").is_none());
}

#[test]
fn a_store_loaded_target_still_previews_the_fold_by_hand() {
    let repo = Repo::new();
    repo.spec_file(1, "active", &["src/**"], "## Intent\n\nA.\n");
    repo.spec_behaviour(1, "- [AC-S-1] kept\n");
    repo.commit("spec");
    let d = delta("in-progress", &["SPEC-001"], "- [AC-A-1] [intent] new\n");

    let run = pre(&repo, "in-progress", "in-progress", d, vec![]);

    assert_eq!(
        run.messages(),
        [
            "would fold 1 claims into SPEC-001",
            "checked in-progress → complete: ready"
        ]
    );
}

#[test]
fn transitions_without_a_gate_return_no_findings() {
    let repo = Repo::new();
    let d = delta("complete", &["SPEC-001"], &claim("AC-D1-1", "bare", None));

    for (from, to) in [("complete", "complete"), ("complete", "draft")] {
        let run = pre(&repo, from, to, d.clone(), vec![spec(1, "")]);

        assert_eq!(run.code, 0);
        assert_eq!(run.json(), serde_json::json!({"findings": []}));
    }
}

fn fresh_claim(repo: &Repo, id: &str) -> String {
    claim(id, "x", Some(&repo.head()))
}

#[test]
fn orphan_part_is_an_error_on_every_gate() {
    let repo = Repo::new();
    let parts = [
        ("behaviour.SPEC-001", String::new()),
        ("behaviour.SPEC-009", fresh_claim(&repo, "AC-D1-1")),
    ];
    let parts: Vec<(&str, &str)> = parts.iter().map(|(n, b)| (*n, b.as_str())).collect();
    for (from, to) in [("draft", "in-progress"), ("in-progress", "complete")] {
        let mut with_claim = parts.clone();
        with_claim[0].1 = "- [AC-D1-2] [intent] ok\n";
        let d = delta_in(from, &["SPEC-001"], &with_claim);

        let run = pre(&repo, from, to, d, vec![spec(1, "")]);

        assert!(
            run.has_message("part behaviour.SPEC-009 does not name"),
            "{from}->{to}"
        );
        assert!(run.findings().iter().all(|f| f["severity"] == "error"));
        assert!(run
            .findings()
            .iter()
            .all(|f| f["part"] == "behaviour.SPEC-009"));
        assert!(updates_of(&run).is_empty());
    }
}

#[test]
fn a_plain_behaviour_part_on_a_delta_names_no_target() {
    let repo = Repo::new();
    let d = delta_in(
        "in-progress",
        &["SPEC-001"],
        &[("behaviour", &fresh_claim(&repo, "AC-D1-1"))],
    );

    let run = pre(&repo, "in-progress", "complete", d, vec![spec(1, "")]);

    assert!(run.has_message("part behaviour does not name"));
}

#[test]
fn struck_claims_in_an_orphan_part_still_flag_the_part_only() {
    let repo = Repo::new();
    let d = delta_in(
        "draft",
        &["SPEC-001"],
        &[
            ("behaviour.SPEC-001", "- [AC-D1-1] [intent] ok\n"),
            ("behaviour.SPEC-009", "- ~~[AC-OLD-1]~~ gone\n"),
        ],
    );

    let run = pre(&repo, "draft", "in-progress", d, vec![spec(1, "")]);

    assert_eq!(run.messages().len(), 1);
    assert!(!run.has_message("AC-OLD-1"));
}

#[test]
fn by_hand_in_progress_to_in_progress_runs_the_completion_gate() {
    let repo = Repo::new();
    let d = delta(
        "in-progress",
        &["SPEC-001"],
        &claim("AC-D1-1", "bare", None),
    );

    let run = pre(&repo, "in-progress", "in-progress", d, vec![spec(1, "")]);

    assert!(run.has_message("AC-D1-1: claim has no receipt"));
}

#[test]
fn by_hand_complete_is_a_no_op() {
    let repo = Repo::new();
    let d = delta("complete", &["SPEC-001"], &fresh_claim(&repo, "AC-D1-1"));

    let run = by_hand(&repo, "complete", d, vec![spec(1, "")]);

    assert_eq!(run.json(), serde_json::json!({"findings": []}));
}

#[test]
fn by_hand_draft_to_draft_runs_the_start_gate() {
    let repo = Repo::new();
    let d = delta_in("draft", &[], &[]);

    let run = pre(&repo, "draft", "draft", d, vec![]);

    assert!(run.has_message("no folds-into link"));
}

#[test]
fn missing_context_specs_are_loaded_from_the_store() {
    let repo = Repo::new();
    repo.spec_file(1, "active", &["src/**"], "## Intent\n\nA.\n");
    repo.commit("spec");
    let d = delta("in-progress", &["SPEC-001"], &fresh_claim(&repo, "AC-D1-1"));

    let run = pre(&repo, "in-progress", "in-progress", d, vec![]);

    assert_eq!(
        run.messages(),
        [
            "would fold 1 claims into SPEC-001",
            "checked in-progress → complete: ready"
        ]
    );
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
    ];
    for (status, edge) in cases {
        let d = delta(status, &["SPEC-001"], &fresh_claim(&repo, "AC-D1-1"));

        let run = by_hand(&repo, status, d, vec![spec(1, "")]);

        let findings = run.findings();
        let expected = if status == "in-progress" { 2 } else { 1 };
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
        &claim("AC-D1-1", "bare", None),
    );

    let run = by_hand(&repo, "in-progress", d, vec![spec(1, "")]);

    assert!(run.has_message("AC-D1-1: claim has no receipt"));
    assert!(!run.has_message("ready"));
}

#[test]
fn by_hand_in_progress_previews_the_fold_as_findings_only() {
    let repo = Repo::new();
    let d = delta("in-progress", &["SPEC-001"], &fresh_claim(&repo, "AC-D1-1"));

    let run = by_hand(&repo, "in-progress", d, vec![spec(1, "")]);

    assert_eq!(
        run.messages(),
        [
            "would fold 1 claims into SPEC-001",
            "checked in-progress → complete: ready"
        ]
    );
    assert!(run.findings().iter().all(|f| f["severity"] == "warning"));
    assert!(run.json().get("updates").is_none());
}

#[test]
fn by_hand_preview_counts_replaces_and_strikes() {
    let repo = Repo::new();
    let spec_claims = "- [AC-D1-1] old\n- [AC-D1-2] doomed\n";
    let claims = format!(
        "{}- ~~[AC-D1-2]~~ doomed\n- [AC-D1-3] new\n",
        fresh_claim(&repo, "AC-D1-1")
    );
    let d = delta("in-progress", &["SPEC-001"], &claims);

    let run = by_hand(&repo, "in-progress", d, vec![spec(1, spec_claims)]);

    assert!(run.has_message("would fold 2 claims into SPEC-001, strike 1"));
    assert!(run.json().get("updates").is_none());
}

#[test]
fn by_hand_in_progress_with_an_unresolved_target_reports_the_gate_error() {
    let repo = Repo::new();
    let d = delta_in(
        "in-progress",
        &["SPEC-009"],
        &[("behaviour.SPEC-009", &fresh_claim(&repo, "AC-D1-1"))],
    );

    let run = by_hand(&repo, "in-progress", d, vec![]);

    assert!(run.has_message("SPEC-009 does not resolve"));
    assert!(!run.has_message("ready"));
    assert!(run.json().get("updates").is_none());
}

#[test]
fn a_checked_receipt_satisfies_the_gate() {
    let repo = Repo::new();
    let d = delta(
        "in-progress",
        &["SPEC-001"],
        &format!("- [AC-D1-1] by hand\n  @checked jack@{}\n", repo.head()),
    );

    let run = pre(&repo, "in-progress", "complete", d, vec![spec(1, "")]);

    assert!(run.findings().is_empty(), "{:?}", run.messages());
    assert_eq!(updates_of(&run).len(), 1);
}
