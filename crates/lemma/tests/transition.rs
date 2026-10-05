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
    let old = "- [AC-OLD-1] removed\n";
    let claims = format!("- [AC-D1-1] bare\n- [AC-D1-2] [intent] vibes\n- ~~[AC-OLD-1]~~ removed\n{}- [AC-D1-3] bare too\n", base_line_for(old, "AC-OLD-1"));
    let d = delta("in-progress", &["SPEC-001"], &claims);

    let run = pre(&repo, "in-progress", "complete", d, vec![spec(1, old)]);

    assert_eq!(run.messages().len(), 2);
    assert!(run.has_message("AC-D1-1: claim has no receipt"));
    assert!(run.has_message("AC-D1-3: claim has no receipt"));
    let findings = run.findings();
    assert_eq!(findings[0]["part"], "behaviour.SPEC-001");
    assert_eq!(findings[0]["line"], 1);
    assert_eq!(findings[1]["line"], 5);
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
    let (b1, b2) = (
        base_line_for(SPEC_CLAIMS, "AC-S-1"),
        base_line_for(SPEC_CLAIMS, "AC-S-2"),
    );
    let claims = format!("- [AC-S-1] new text\n  @ref tests/store_test.rs#reopen_keeps_prior@{sha}\n{b1}- ~~[AC-S-2]~~ doomed\n{b2}- [AC-D1-1] fresh claim\n  @ref tests/store_test.rs#other_test@{sha}\n");
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
                &format!(
                    "- [AC-A-1] [intent] for one\n- ~~[AC-A-0]~~ old\n{}",
                    base_line_for("- [AC-A-0] old\n", "AC-A-0")
                ),
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

const BASE_SPEC: &str = "- [AC-S-1] old text\n";

fn severities(run: &Run) -> Vec<String> {
    run.findings()
        .iter()
        .map(|f| f["severity"].as_str().unwrap().to_string())
        .collect()
}

fn fold_claim(repo: &Repo, base: &str, text: &str) -> String {
    format!(
        "- [AC-S-1] {text}\n  @ref {TEST_FILE}#reopen_keeps_prior@{}\n{base}",
        repo.head()
    )
}

#[test]
fn replace_without_base_fails_draft_to_in_progress() {
    let repo = Repo::new();
    let d = delta("draft", &["SPEC-001"], &claim("AC-S-1", "new", None));

    let run = pre(&repo, "draft", "in-progress", d, vec![spec(1, BASE_SPEC)]);

    assert_eq!(severities(&run), ["error"]);
    assert!(run.has_message("AC-S-1: replaces/strikes a spec claim without `@base`"));
    assert!(run.has_message("lemma base DELTA-001 AC-S-1"));
}

#[test]
fn strike_of_an_unknown_id_fails_draft_to_in_progress() {
    let repo = Repo::new();
    let d = delta("draft", &["SPEC-001"], "- ~~[AC-S-9]~~ gone\n");

    let run = pre(&repo, "draft", "in-progress", d, vec![spec(1, BASE_SPEC)]);

    assert!(run.has_message("AC-S-9"));
    assert_eq!(severities(&run), ["error"]);
}

#[test]
fn a_stale_base_is_not_a_draft_gate_error() {
    let repo = Repo::new();
    let d = delta(
        "draft",
        &["SPEC-001"],
        "- [AC-S-1] new\n  @base 000000000000\n",
    );

    let run = pre(&repo, "draft", "in-progress", d, vec![spec(1, BASE_SPEC)]);

    assert!(run.findings().is_empty(), "{:?}", run.messages());
}

#[test]
fn fold_with_matching_base_replaces_and_drops_the_base_line() {
    let repo = Repo::new();
    let base = base_line_for(BASE_SPEC, "AC-S-1");
    let d = delta(
        "in-progress",
        &["SPEC-001"],
        &fold_claim(&repo, &base, "new"),
    );

    let run = pre(
        &repo,
        "in-progress",
        "complete",
        d,
        vec![spec(1, BASE_SPEC)],
    );

    assert!(run.findings().is_empty(), "{:?}", run.messages());
    let body = updates_of(&run)[0]["body"].as_str().unwrap().to_string();
    assert!(body.starts_with("- [AC-S-1] new\n"), "{body}");
    assert!(!body.contains("@base"));
}

#[test]
fn fold_with_a_changed_base_errors_naming_both_hashes() {
    let repo = Repo::new();
    let base = base_line_for(BASE_SPEC, "AC-S-1");
    let base_hash = spec_block_hash(BASE_SPEC, "AC-S-1");
    let moved = "- [AC-S-1] edited elsewhere\n";
    let d = delta(
        "in-progress",
        &["SPEC-001"],
        &fold_claim(&repo, &base, "new"),
    );

    let run = pre(&repo, "in-progress", "complete", d, vec![spec(1, moved)]);

    assert!(run.has_message(&format!("base {base_hash}")));
    assert!(run.has_message(&format!("spec {}", spec_block_hash(moved, "AC-S-1"))));
    assert!(updates_of(&run).is_empty());
}

#[test]
fn a_new_id_colliding_with_a_spec_id_errors_without_writing() {
    let repo = Repo::new();
    let d = delta(
        "in-progress",
        &["SPEC-001"],
        &fold_claim(&repo, "", "collides"),
    );

    let run = pre(
        &repo,
        "in-progress",
        "complete",
        d,
        vec![spec(1, BASE_SPEC)],
    );

    assert!(run.has_message("AC-S-1: replaces/strikes a spec claim without `@base`"));
    assert!(updates_of(&run).is_empty());
}

#[test]
fn strike_with_a_stale_base_errors() {
    let repo = Repo::new();
    let d = delta(
        "in-progress",
        &["SPEC-001"],
        "- ~~[AC-S-1]~~ old text\n  @base 000000000000\n",
    );

    let run = pre(
        &repo,
        "in-progress",
        "complete",
        d,
        vec![spec(1, BASE_SPEC)],
    );

    assert!(run.has_message("spec block changed since `@base`"));
    assert!(updates_of(&run).is_empty());
}

#[test]
fn base_on_an_id_the_spec_lacks_errors_at_complete() {
    let repo = Repo::new();
    let d = delta(
        "in-progress",
        &["SPEC-001"],
        &fold_claim(&repo, "  @base 000000000000\n", "new"),
    );

    let run = pre(&repo, "in-progress", "complete", d, vec![spec(1, "")]);

    assert!(run.has_message("has `@base` but SPEC-001 has no claim AC-S-1"));
}

fn fresh_delta(repo: &Repo) -> Value {
    let sha = repo.head();
    delta(
        "in-progress",
        &["SPEC-001"],
        &claim("AC-D1-1", "x", Some(&sha)),
    )
}

fn complete_run(repo: &Repo) -> Run {
    pre(
        repo,
        "in-progress",
        "complete",
        fresh_delta(repo),
        vec![spec(1, "")],
    )
}

#[test]
fn a_dirty_governed_file_blocks_complete_without_updates() {
    let repo = Repo::new();
    let d = fresh_delta(&repo);
    repo.write("src/lib.rs", "pub fn a() { /* wip */ }\n");

    let run = pre(&repo, "in-progress", "complete", d, vec![spec(1, "")]);

    assert!(run.has_message("src/lib.rs has uncommitted changes; commit before completing"));
    assert_eq!(run.findings()[0]["severity"], "error");
    assert!(updates_of(&run).is_empty());
}

#[test]
fn a_dirty_untracked_governed_file_blocks_complete() {
    let repo = Repo::new();
    let d = fresh_delta(&repo);
    repo.write("src/new/mod.rs", "pub fn b() {}\n");

    let run = pre(&repo, "in-progress", "complete", d, vec![spec(1, "")]);

    assert!(run.has_message("src/new/mod.rs has uncommitted changes"));
}

#[test]
fn a_dirty_ungoverned_file_does_not_block_complete() {
    let repo = Repo::new();
    let d = fresh_delta(&repo);
    repo.write("docs/notes.md", "wip\n");
    repo.write("scripts/build.sh", "echo\n");

    let run = pre(&repo, "in-progress", "complete", d, vec![spec(1, "")]);

    assert!(run.findings().is_empty(), "{:?}", run.messages());
    assert!(!updates_of(&run).is_empty());
}

#[test]
fn a_dirty_receipt_test_file_blocks_complete() {
    let repo = Repo::new();
    let d = fresh_delta(&repo);
    repo.write(TEST_FILE, &format!("{TEST_SRC}// wip\n"));

    let run = pre(&repo, "in-progress", "complete", d, vec![spec(1, "")]);

    assert!(run.has_message(&format!("{TEST_FILE} has uncommitted changes")));
    assert!(updates_of(&run).is_empty());
}

#[test]
fn a_dirty_file_is_reported_once_even_when_governed_and_a_receipt() {
    let repo = Repo::new();
    let sha = repo.head();
    let body = format!("- [AC-D1-1] x\n  @ref {TEST_FILE}#reopen_keeps_prior@{sha}\n");
    let d = delta("in-progress", &["SPEC-001"], &body);
    let governing = with_parts(
        doc("SPEC-001", "spec", "active", &["tests/**"], &[], INTENT),
        &[("behaviour", "")],
    );
    repo.write(TEST_FILE, &format!("{TEST_SRC}// wip\n"));

    let run = pre(&repo, "in-progress", "complete", d, vec![governing]);

    let dirty = run
        .messages()
        .iter()
        .filter(|m| m.contains("uncommitted"))
        .count();
    assert_eq!(dirty, 1, "{:?}", run.messages());
}

#[test]
fn the_by_hand_dry_run_reports_a_dirty_governed_file() {
    let repo = Repo::new();
    let d = fresh_delta(&repo);
    repo.write("src/lib.rs", "pub fn a() { /* wip */ }\n");

    let run = pre(&repo, "in-progress", "in-progress", d, vec![spec(1, "")]);

    assert!(run.has_message("src/lib.rs has uncommitted changes; commit before completing"));
    assert!(!run.has_message("ready"));
}

#[test]
fn a_clean_tree_completes() {
    let repo = Repo::new();

    assert!(complete_run(&repo).findings().is_empty());
}

fn production_receipt(repo: &Repo) -> Value {
    let sha = repo.head();
    delta(
        "in-progress",
        &["SPEC-001"],
        &format!("- [AC-D1-1] x\n  @ref src/lib.rs#a@{sha}\n"),
    )
}

#[test]
fn complete_errors_on_a_receipt_outside_the_tests_globs() {
    let repo = Repo::new();
    let d = production_receipt(&repo);

    let run = pre(&repo, "in-progress", "complete", d, vec![spec(1, "")]);

    assert!(run.has_message(
        "AC-D1-1: receipt src/lib.rs is not a test file under SPEC-001's `tests` globs"
    ));
    assert_eq!(run.findings()[0]["line"], 2);
    assert!(updates_of(&run).is_empty());
}

#[test]
fn complete_accepts_a_receipt_matching_the_spec_tests_override() {
    let repo = Repo::new();
    let d = production_receipt(&repo);
    let overridden = with_tests(spec(1, ""), &["src/**"]);

    let run = pre(&repo, "in-progress", "complete", d, vec![overridden]);

    assert!(run.findings().is_empty(), "{:?}", run.messages());
}

#[test]
fn a_tests_override_rejects_the_default_test_path() {
    let repo = Repo::new();
    let d = fresh_delta(&repo);
    let overridden = with_tests(spec(1, ""), &["checks/**"]);

    let run = pre(&repo, "in-progress", "complete", d, vec![overridden]);

    assert!(run.has_message(&format!("receipt {TEST_FILE} is not a test file")));
}

#[test]
fn a_dirty_governed_file_blocks_complete_when_the_root_is_a_git_subdirectory() {
    let repo = Repo::in_subdir("proj");
    let d = fresh_delta(&repo);
    repo.write("src/lib.rs", "pub fn a() { /* wip */ }\n");

    let run = pre(&repo, "in-progress", "complete", d, vec![spec(1, "")]);

    assert!(run.has_message("src/lib.rs has uncommitted changes"));
    assert!(updates_of(&run).is_empty());
}

#[test]
fn dirty_governed_files_with_spaces_or_non_ascii_names_block_complete() {
    let repo = Repo::in_subdir("proj");
    let d = fresh_delta(&repo);
    repo.write("src/my file.rs", "pub fn b() {}\n");
    repo.write("src/caf\u{e9}.rs", "pub fn c() {}\n");

    let run = pre(&repo, "in-progress", "complete", d, vec![spec(1, "")]);

    assert!(run.has_message("src/my file.rs has uncommitted changes"));
    assert!(run.has_message("src/caf\u{e9}.rs has uncommitted changes"));
}

#[test]
fn an_invalid_governs_or_tests_glob_blocks_complete() {
    let repo = Repo::new();
    let d = fresh_delta(&repo);
    let broken = with_tests(
        doc("SPEC-001", "spec", "active", &["src/["], &[], INTENT),
        &["checks/["],
    );

    let run = pre(&repo, "in-progress", "complete", d, vec![broken]);

    assert!(run.has_message("governs glob `src/[` is invalid:"));
    assert!(run.has_message("tests glob `checks/[` is invalid:"));
    assert!(updates_of(&run).is_empty());
}

fn body_of_fold(spec_claims: &str, delta_claim_text: &str) -> String {
    let repo = Repo::new();
    let sha = repo.head();
    repo.spec_file(1, "active", &["src/**"], "## Intent\n\nA.\n");
    repo.spec_behaviour(1, spec_claims);
    repo.delta_file(1, "in-progress", &["SPEC-001"], INTENT);
    repo.delta_part(
        1,
        "SPEC-001",
        &format!("- [AC-S-1] {delta_claim_text}\n  @ref {TEST_FILE}#reopen_keeps_prior@{sha}\n"),
    );
    repo.commit("fixtures");
    let based = repo.run(&["base", "DELTA-001"], "");
    assert_eq!(based.code, 0, "{}", based.stdout);
    repo.commit("based");

    let spec_doc = with_parts(
        doc("SPEC-001", "spec", "active", &["src/**"], &[], INTENT),
        &[("behaviour", &repo.read(&spec_part_path(1)))],
    );
    let delta_doc = with_parts(
        doc(
            "DELTA-001",
            "delta",
            "in-progress",
            &[],
            &["SPEC-001"],
            INTENT,
        ),
        &[(
            "behaviour.SPEC-001",
            &repo.read(&delta_part_path(1, "SPEC-001")),
        )],
    );
    let run = pre(&repo, "in-progress", "complete", delta_doc, vec![spec_doc]);

    assert!(run.findings().is_empty(), "{:?}", run.messages());
    let updates = updates_of(&run);
    assert_eq!(updates.len(), 1);
    updates[0]["body"].as_str().unwrap().to_string()
}

fn has_base_line(body: &str) -> bool {
    body.lines().any(|l| l.trim_start().starts_with("@base "))
}

#[test]
fn based_delta_folds_after_lemma_base_with_no_base_line_in_the_spec() {
    let body = body_of_fold("- [AC-S-1] old\n", "new");

    assert!(body.starts_with("- [AC-S-1] new\n"), "{body}");
    assert!(!has_base_line(&body), "{body}");
}

#[test]
fn a_claim_mentioning_base_keeps_its_id_line_through_base_and_fold() {
    let body = body_of_fold(
        "- [AC-S-1] the fold strips `@base` lines\n",
        "the fold strips `@base` lines, now precisely",
    );

    assert!(
        body.starts_with("- [AC-S-1] the fold strips `@base` lines, now precisely\n"),
        "{body}"
    );
    assert!(!has_base_line(&body), "{body}");
}

const FORCED_WARNING: &str =
    "AC-D1-1: receipt was force re-pinned; review the test against the claim, then remove [forced] or add @checked";

fn forced_delta(repo: &Repo, status: &str) -> Value {
    let sha = repo.head();
    delta(
        status,
        &["SPEC-001"],
        &format!("- [AC-D1-1] x\n  @ref {TEST_FILE}#reopen_keeps_prior@{sha} [forced]\n"),
    )
}

#[test]
fn a_forced_receipt_warns_at_the_gate_without_blocking_the_fold() {
    let repo = Repo::new();
    let d = forced_delta(&repo, "in-progress");

    let run = pre(&repo, "in-progress", "complete", d, vec![spec(1, "")]);

    assert_eq!(run.code, 0);
    assert!(run.has_message(FORCED_WARNING));
    assert!(run.findings().iter().all(|f| f["severity"] == "warning"));
    assert_eq!(updates_of(&run).len(), 1);
}

#[test]
fn a_forced_receipt_warns_in_the_by_hand_preview() {
    let repo = Repo::new();
    let d = forced_delta(&repo, "in-progress");

    let run = by_hand(&repo, "in-progress", d, vec![spec(1, "")]);

    assert!(run.has_message(FORCED_WARNING));
    assert!(run.has_message("checked in-progress → complete: ready"));
}

#[test]
fn the_fold_carries_the_forced_marker_into_the_spec() {
    let repo = Repo::new();
    let d = forced_delta(&repo, "in-progress");

    let run = pre(&repo, "in-progress", "complete", d, vec![spec(1, "")]);

    let body = updates_of(&run)[0]["body"].as_str().unwrap().to_string();
    assert!(
        body.contains(&format!("#reopen_keeps_prior@{} [forced]", repo.head())),
        "{body}"
    );
}

#[test]
fn by_hand_dry_run_warns_about_an_uncommitted_index_and_still_says_ready() {
    let repo = Repo::new();
    let d = fresh_delta(&repo);
    repo.write("docs/deltas/DELTA-001-x/index.md", "wip\n");
    repo.write("docs/deltas/DELTA-001-x/behaviour.SPEC-001.md", "wip\n");
    repo.write("docs/deltas/DELTA-002-x/index.md", "other\n");

    let run = by_hand(&repo, "in-progress", d, vec![spec(1, "")]);

    assert_eq!(run.code, 0);
    assert!(run.has_message(
        "docs/deltas/DELTA-001-x/index.md has uncommitted changes; commit the delta before asking for review"
    ));
    assert!(
        run.has_message("docs/deltas/DELTA-001-x/behaviour.SPEC-001.md has uncommitted changes")
    );
    assert!(!run.has_message("DELTA-002-x"));
    assert!(run.findings().iter().all(|f| f["severity"] == "warning"));
    assert!(run.has_message("checked in-progress → complete: ready"));
}

#[test]
fn the_real_transition_does_not_warn_about_uncommitted_delta_files() {
    let repo = Repo::new();
    let d = fresh_delta(&repo);
    repo.write("docs/deltas/DELTA-001-x/index.md", "wip\n");

    let run = pre(&repo, "in-progress", "complete", d, vec![spec(1, "")]);

    assert!(!run.has_message("commit the delta"));
}

fn draft_spec(n: u32) -> Value {
    let mut spec = spec(n, "");
    spec["status"] = serde_json::json!("draft");
    spec
}

#[test]
fn folding_into_a_draft_spec_warns_and_still_folds() {
    let repo = Repo::new();
    let d = fresh_delta(&repo);

    let run = pre(&repo, "in-progress", "complete", d, vec![draft_spec(1)]);

    assert_eq!(run.code, 0);
    assert!(run.has_message("SPEC-001 is draft; move it to active once this fold lands"));
    assert_eq!(updates_of(&run).len(), 1);
}

#[test]
fn the_by_hand_preview_warns_about_a_draft_target() {
    let repo = Repo::new();
    let d = fresh_delta(&repo);

    let run = by_hand(&repo, "in-progress", d, vec![draft_spec(1)]);

    assert!(run.has_message("SPEC-001 is draft; move it to active once this fold lands"));
    assert!(run.has_message("checked in-progress → complete: ready"));
}
