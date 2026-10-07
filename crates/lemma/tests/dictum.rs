mod common;
use common::*;
use serde_json::json;

const CLAIM: &str =
    "- [DT-1] Reopen keeps the prior answer.\n  @ref tests/store_test.rs#reopen_keeps_prior\n";
const CHANGED_TEST: &str =
    "#[test]\nfn reopen_keeps_prior() { assert!(true); }\n\n#[test]\nfn other_test() {}\n";

fn project(body: &str) -> Repo {
    let repo = Repo::new();
    repo.git(&["branch", "-M", "main"]);
    repo.with_dictums();
    repo.write("a/x.rs", "fn x() {}\n");
    repo.write("b/y.rs", "fn y() {}\n");
    repo.spec_file(1, "active", &["a/**"], INTENT);
    repo.spec_file(2, "active", &["b/**"], INTENT);
    repo.dictum_file(&["a/**"], body);
    repo.commit("project");
    repo
}

fn check(repo: &Repo) -> Run {
    repo.run(&["check"], "")
}

#[test]
fn an_unresolved_dictum_ref_is_a_check_3_error() {
    let repo = project("- [DT-1] Reopen keeps it.\n  @ref tests/store_test.rs#no_such_test\n");

    let run = check(&repo);

    assert_eq!(run.code, 2);
    assert!(run.stdout.contains(DICTUM_PATH));
    assert!(run
        .stdout
        .contains("DT-1: ref tests/store_test.rs#no_such_test does not resolve"));
}

#[test]
fn a_dictum_claim_without_a_ref_is_flagged_unless_intent() {
    let repo = project("- [DT-1] Bare.\n- [DT-2] [intent] Aspirational.\n");

    let run = check(&repo);

    assert!(run.stdout.contains("DT-1: claim has no `@ref`"));
    assert!(!run.stdout.contains("DT-2"));
}

#[test]
fn an_assumed_dictum_claim_is_an_error() {
    let repo = project("- [DT-1] [assumed] Maybe.\n");

    let run = check(&repo);

    assert_eq!(run.code, 2);
    assert!(run.stdout.contains("DT-1: [assumed] claim in a dictum"));
}

#[test]
fn a_dictum_claim_with_a_resolving_ref_is_clean() {
    let repo = project(CLAIM);

    assert_eq!(check(&repo).stdout, "");
}

#[test]
fn a_changed_region_a_dictum_claim_references_is_a_check_2_error() {
    let repo = project(CLAIM);
    repo.write(TEST_FILE, CHANGED_TEST);

    let run = check(&repo);

    assert_eq!(run.code, 2);
    assert!(run.stdout.contains(DICTUM_PATH));
    assert!(run
        .stdout
        .contains("DT-1: evidence tests/store_test.rs#reopen_keeps_prior changed"));
    assert!(run.stdout.contains("strike DT-1 in the dictum"));
}

#[test]
fn the_dictum_striking_the_id_and_adding_a_new_one_excuses_the_change() {
    let repo = project(CLAIM);
    repo.write(TEST_FILE, CHANGED_TEST);
    repo.dictum_file(
        &["a/**"],
        "- ~~[DT-1]~~ Reopen keeps the prior answer.\n- [DT-2] Reopen keeps the prior answer.\n  @ref tests/store_test.rs#reopen_keeps_prior\n",
    );

    assert_eq!(check(&repo).stdout, "");
}

#[test]
fn striking_without_adding_a_new_id_does_not_excuse_the_change() {
    let repo = project(CLAIM);
    repo.write(TEST_FILE, CHANGED_TEST);
    repo.dictum_file(&["a/**"], "- ~~[DT-1]~~ Reopen keeps the prior answer.\n");

    assert!(check(&repo).stdout.contains("DT-1: evidence"));
}

#[test]
fn a_dictum_is_not_an_area_for_check_1() {
    let repo = project(CLAIM);
    repo.write("a/x.rs", "fn x() { 1; }\n");
    repo.write("b/y.rs", "fn y() { 2; }\n");

    let run = check(&repo);

    assert!(run.stdout.contains("SPEC-001"));
    assert!(!run.stdout.contains("DICTUM-001, "));
    assert!(!run.stdout.contains(", DICTUM-001"));
}

#[test]
fn a_dictum_governing_a_path_alone_needs_no_delta() {
    let repo = project(CLAIM);
    repo.write("src/lib.rs", "pub fn a() { 1; }\n");

    assert_eq!(check(&repo).stdout, "");
}

#[test]
fn a_hand_edit_to_a_dictum_claim_is_not_a_check_4_error() {
    let repo = project(CLAIM);
    repo.dictum_file(
        &["a/**"],
        "- [DT-1] Reopen keeps the prior answer, always.\n  @ref tests/store_test.rs#reopen_keeps_prior\n",
    );

    assert_eq!(check(&repo).stdout, "");
}

#[test]
fn a_legacy_line_in_a_dictum_is_still_flagged() {
    let repo = project(
        "- [DT-1] Claim.\n  @ref tests/store_test.rs#reopen_keeps_prior\n  @base abc1234\n",
    );

    assert!(check(&repo).code == 2);
}

#[test]
fn an_invalid_dictum_governs_glob_is_flagged() {
    let repo = project(CLAIM);
    repo.dictum_file(&["a/[**"], CLAIM);

    let run = check(&repo);

    assert_eq!(run.code, 2);
    assert!(run.stdout.contains("governs glob"));
}

fn signal(repo: &Repo, rel: &str) -> Run {
    let input = json!({
        "session_id": "s",
        "cwd": repo.root(),
        "tool_name": "Edit",
        "tool_input": { "file_path": repo.path(rel), "content": "x" },
    });
    repo.run(&["signal"], &input.to_string())
}

#[test]
fn signal_points_at_the_dictum_governing_the_touched_path_on_first_touch() {
    let repo = project(CLAIM);
    repo.write("a/x.rs", "fn x() { 1; }\n");

    let first = signal(&repo, "a/x.rs");
    let said = first.json()["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap()
        .to_string();
    let second = signal(&repo, "a/x.rs");

    assert_eq!(said, "a/x.rs → SPEC-001 Spec 1 · DICTUM-001 Testing");
    assert!(second.stdout.is_empty());
}
