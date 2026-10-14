mod common;
use common::*;

fn repo() -> Repo {
    let repo = Repo::new();
    repo.spec_file(1, "active", &["a/**"], INTENT);
    repo.delta_file(1, "in-progress", &["SPEC-001"], INTENT);
    repo.delta_claims(
        1,
        "- [AC-1] New.\n  @ref tests/store_test.rs#other_test\n- [AC-2] Settled.\n  @ref tests/store_test.rs#reopen_keeps_prior\n- [AC-3] Broken.\n  @ref tests/store_test.rs#missing\n  @ref src/lib.rs#a\n  @ref tests/store_test.rs#other_test\n- [AC-4] [intent] Feels fast.\n",
    );
    repo.delta_file(2, "complete", &["SPEC-001"], INTENT);
    repo.delta_claims(2, "- [AC-1] Old.\n  @ref tests/store_test.rs#missing\n");
    repo.commit("fixtures");
    repo
}

fn refs(repo: &Repo, claim: &str) -> Run {
    repo.run(&["refs", claim], "")
}

#[test]
fn a_resolving_ref_is_ok_and_exits_zero() {
    let run = refs(&repo(), "DELTA-001/AC-2");

    assert_eq!(run.code, 0);
    assert_eq!(
        run.stdout,
        "DELTA-001/AC-2 tests/store_test.rs#reopen_keeps_prior ok\n"
    );
}

#[test]
fn each_ref_gets_its_own_status_and_any_failure_exits_two() {
    let run = refs(&repo(), "DELTA-001/AC-3");

    assert_eq!(run.code, 2);
    assert_eq!(
        run.stdout.lines().collect::<Vec<_>>(),
        vec![
            "DELTA-001/AC-3 tests/store_test.rs#missing unresolved",
            "DELTA-001/AC-3 src/lib.rs#a not-a-test",
            "DELTA-001/AC-3 tests/store_test.rs#other_test ok",
        ]
    );
}

#[test]
fn a_delta_claim_is_cited_with_its_delta() {
    let run = refs(&repo(), "DELTA-001/AC-1");

    assert_eq!(run.code, 0);
    assert_eq!(
        run.stdout,
        "DELTA-001/AC-1 tests/store_test.rs#other_test ok\n"
    );
}

#[test]
fn a_bare_id_lists_it_in_every_delta_complete_ones_included() {
    let run = refs(&repo(), "AC-1");

    assert_eq!(run.code, 2);
    assert_eq!(
        run.stdout.lines().collect::<Vec<_>>(),
        vec![
            "DELTA-001/AC-1 tests/store_test.rs#other_test ok",
            "DELTA-002/AC-1 tests/store_test.rs#missing unresolved",
        ]
    );
}

#[test]
fn an_unknown_claim_exits_two() {
    let run = refs(&repo(), "AC-NOPE-1");

    assert_eq!(run.code, 2);
    assert_eq!(run.stdout, "");
    assert!(run.stderr.contains("unknown claim AC-NOPE-1"));
}

#[test]
fn a_claim_with_no_refs_prints_nothing_and_exits_zero() {
    let run = refs(&repo(), "DELTA-001/AC-4");

    assert_eq!(run.code, 0);
    assert_eq!(run.stdout, "");
}

const CLAIMS: &str = "docs/deltas/DELTA-001-d/claims.md";

fn refs_json(repo: &Repo, file: &str) -> Run {
    repo.run(&["refs", "--json", file], "")
}

fn entries(run: &Run) -> Vec<serde_json::Value> {
    run.json()["entries"].as_array().cloned().unwrap()
}

#[test]
fn json_lists_every_ref_and_per_with_span_text_and_status() {
    let repo = repo();
    repo.delta_claims(
        1,
        "- [AC-1] New.\n  @ref tests/store_test.rs#other_test\n  @per DECISION-003\n```\n@ref tests/fenced.rs#x\n```\n- [AC-2] Two.\n  see @ref src/lib.rs#a here\n",
    );

    let run = refs_json(&repo, CLAIMS);

    assert_eq!(run.code, 0);
    assert_eq!(
        entries(&run),
        vec![
            serde_json::json!({"kind": "ref", "line": 2, "col": 2, "end_col": 37,
                "text": "@ref tests/store_test.rs#other_test", "status": "ok",
                "target": {"path": "tests/store_test.rs", "line": 5}}),
            serde_json::json!({"kind": "per", "line": 3, "col": 2, "end_col": 19,
                "text": "@per DECISION-003", "status": "ok"}),
            serde_json::json!({"kind": "ref", "line": 8, "col": 6, "end_col": 23,
                "text": "@ref src/lib.rs#a",
                "status": "AC-2: ref src/lib.rs#a is not in the governing spec's `tests` globs; point it at a test or widen `tests`"}),
        ]
    );
}

#[test]
fn json_ref_status_is_the_diagnostic_lint_reports() {
    let repo = repo();

    let listed: Vec<String> = entries(&refs_json(&repo, CLAIMS))
        .iter()
        .map(|e| e["status"].as_str().unwrap().to_string())
        .filter(|s| s != "ok")
        .collect();
    let linted = repo.run(&["lint", "DELTA-001", "--json"], "").messages();

    assert_eq!(
        listed,
        vec![
            "AC-3: ref tests/store_test.rs#missing does not resolve: symbol missing not found in tests/store_test.rs; fix the ref (see lemma refs DELTA-001/AC-3)",
            "AC-3: ref src/lib.rs#a is not in the governing spec's `tests` globs; point it at a test or widen `tests`",
        ]
    );
    for status in &listed {
        assert!(linted.contains(status), "{status} not in {linted:?}");
    }
}

#[test]
fn json_target_is_the_symbols_first_line_or_line_one_without_a_symbol() {
    let repo = repo();
    repo.write(
        "tests/jump.test.ts",
        "import x from 'x';\n\ndescribe('a', () => {\n  it('b', () => {});\n});\n\nfunction later() {}\n",
    );
    repo.write(
        "tests/spec.lua",
        "local T = {}\n\nT['jumps'] = function() end\n",
    );
    repo.delta_claims(
        1,
        "- [AC-1] A.\n  @ref tests/store_test.rs#reopen_keeps_prior\n  @ref tests/store_test.rs#other_test\n  @ref tests/jump.test.ts#later\n  @ref tests/spec.lua#jumps\n  @ref tests/store_test.rs\n  @ref tests/store_test.rs#missing\n",
    );

    let run = refs_json(&repo, CLAIMS);

    assert_eq!(run.code, 0);
    let targets: Vec<serde_json::Value> =
        entries(&run).iter().map(|e| e["target"].clone()).collect();
    assert_eq!(
        targets,
        vec![
            serde_json::json!({"path": "tests/store_test.rs", "line": 2}),
            serde_json::json!({"path": "tests/store_test.rs", "line": 5}),
            serde_json::json!({"path": "tests/jump.test.ts", "line": 7}),
            serde_json::json!({"path": "tests/spec.lua", "line": 3}),
            serde_json::json!({"path": "tests/store_test.rs", "line": 1}),
            serde_json::Value::Null,
        ]
    );
}

#[test]
fn json_exits_zero_whatever_the_status_and_two_on_bad_input() {
    let repo = repo();
    let listed = refs_json(&repo, CLAIMS);
    assert_eq!(listed.code, 0);
    assert!(entries(&listed).iter().any(|e| e["status"] != "ok"));

    let missing = refs_json(&repo, "docs/deltas/DELTA-001-d/nope.md");
    assert_eq!(missing.code, 2);
    assert_eq!(missing.stdout, "");

    let elsewhere = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(elsewhere.path(), "@ref tests/store_test.rs#other_test\n").unwrap();
    let outside = refs_json(&repo, elsewhere.path().to_str().unwrap());
    assert_eq!(outside.code, 2);
    assert_eq!(outside.stdout, "");
    assert!(outside.stderr.contains("outside the lazyspec root"));
}

#[test]
fn json_lists_doc_ids_with_a_configured_prefix_outside_brackets_only() {
    let repo = repo();
    repo.delta_claims(
        1,
        "- [AC-1] Per TEST-1 see SPEC-001, [SPEC-002], DECISION-003, XSPEC-001 and DELTA-002.\n  @ref tests/store_test.rs#other_test\n```\nSPEC-001 in a fence\n```\n",
    );

    let run = refs_json(&repo, CLAIMS);

    assert_eq!(run.code, 0);
    let docs: Vec<serde_json::Value> = entries(&run)
        .into_iter()
        .filter(|e| e["kind"] == "doc")
        .map(|e| serde_json::json!({"line": e["line"], "col": e["col"], "end_col": e["end_col"], "text": e["text"]}))
        .collect();
    assert_eq!(
        docs,
        vec![
            serde_json::json!({"line": 1, "col": 24, "end_col": 32, "text": "SPEC-001"}),
            serde_json::json!({"line": 1, "col": 74, "end_col": 83, "text": "DELTA-002"}),
        ]
    );
}

#[test]
fn json_doc_id_targets_its_index_line_one_or_is_unknown() {
    let repo = repo();
    repo.delta_claims(1, "- [AC-1] See SPEC-001, DELTA-002 and SPEC-009.\n");

    let run = refs_json(&repo, CLAIMS);

    assert_eq!(run.code, 0);
    let docs: Vec<serde_json::Value> = entries(&run)
        .into_iter()
        .map(|e| serde_json::json!({"text": e["text"], "status": e["status"], "target": e["target"]}))
        .collect();
    assert_eq!(
        docs,
        vec![
            serde_json::json!({"text": "SPEC-001", "status": "ok",
                "target": {"path": "docs/specs/SPEC-001-s/index.md", "line": 1}}),
            serde_json::json!({"text": "DELTA-002", "status": "ok",
                "target": {"path": "docs/deltas/DELTA-002-d/index.md", "line": 1}}),
            serde_json::json!({"text": "SPEC-009", "status": "unknown", "target": null}),
        ]
    );
}

#[test]
fn json_doc_id_span_is_in_bytes_after_a_multibyte_character() {
    let repo = repo();
    repo.delta_claims(
        1,
        "- [AC-1] `draft` → `in-progress` → `complete`; see SPEC-001.\n",
    );

    let run = refs_json(&repo, CLAIMS);

    assert_eq!(run.code, 0, "{}", run.stderr);
    let docs: Vec<serde_json::Value> = entries(&run)
        .into_iter()
        .map(|e| serde_json::json!({"col": e["col"], "end_col": e["end_col"], "text": e["text"]}))
        .collect();
    assert_eq!(
        docs,
        vec![serde_json::json!({"col": 55, "end_col": 63, "text": "SPEC-001"})]
    );
}
