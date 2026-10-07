mod common;
use common::*;

fn repo() -> Repo {
    let repo = Repo::new();
    repo.spec_file(1, "active", &["a/**"], INTENT);
    repo.spec_behaviour(
        1,
        "- [AC-S1-1] Settled.\n  @ref tests/store_test.rs#reopen_keeps_prior\n- [AC-S1-2] Broken.\n  @ref tests/store_test.rs#missing\n  @ref src/lib.rs#a\n  @ref tests/store_test.rs#other_test\n",
    );
    repo.delta_file(1, "in-progress", &["SPEC-001"], INTENT);
    repo.delta_claims(1, "- [AC-1] New.\n  @ref tests/store_test.rs#other_test\n");
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
    let run = refs(&repo(), "AC-S1-1");

    assert_eq!(run.code, 0);
    assert_eq!(
        run.stdout,
        "AC-S1-1 tests/store_test.rs#reopen_keeps_prior ok\n"
    );
}

#[test]
fn each_ref_gets_its_own_status_and_any_failure_exits_two() {
    let run = refs(&repo(), "AC-S1-2");

    assert_eq!(run.code, 2);
    assert_eq!(
        run.stdout.lines().collect::<Vec<_>>(),
        vec![
            "AC-S1-2 tests/store_test.rs#missing unresolved",
            "AC-S1-2 src/lib.rs#a not-a-test",
            "AC-S1-2 tests/store_test.rs#other_test ok",
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
    let repo = repo();
    repo.spec_behaviour(1, "- [AC-S1-3] [intent] Feels fast.\n");
    repo.commit("intent");

    let run = refs(&repo, "AC-S1-3");

    assert_eq!(run.code, 0);
    assert_eq!(run.stdout, "");
}
