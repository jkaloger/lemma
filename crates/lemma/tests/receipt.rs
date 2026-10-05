mod common;
use common::*;

const DELTA: &str = "docs/deltas/DELTA-001-d.md";

fn delta_with_bare_claims(repo: &Repo) {
    repo.delta_file(
        1,
        "in-progress",
        &["SPEC-001"],
        &delta_body("- [AC-D1-1] first\n- [AC-D1-2] second\n  note\n"),
    );
    repo.spec_file(1, "active", &["src/**"], "## Intent\n\nA.\n");
    repo.commit("delta");
}

fn target() -> String {
    format!("{TEST_FILE}#reopen_keeps_prior")
}

#[test]
fn receipt_pins_head_as_the_first_line_under_the_claim() {
    let repo = Repo::new();
    delta_with_bare_claims(&repo);
    let head = repo.head();

    let run = repo.run(&["receipt", "DELTA-001", "AC-D1-2", &target()], "");

    assert_eq!(run.code, 0, "{}", run.stdout);
    let text = repo.read(DELTA);
    assert!(
        text.contains(&format!(
            "- [AC-D1-2] second\n  @ref {TEST_FILE}#reopen_keeps_prior@{head}\n  note\n"
        )),
        "{text}"
    );
    assert!(text.contains("- [AC-D1-1] first\n- [AC-D1-2]"));
    assert!(text.starts_with("---\n"));
}

fn refused(repo: &Repo, args: &[&str]) -> Run {
    let before = repo.read(DELTA);
    let run = repo.run(args, "");
    assert_eq!(run.code, 2, "{}", run.stdout);
    assert_eq!(run.findings()[0]["severity"], "error");
    assert_eq!(repo.read(DELTA), before);
    run
}

#[test]
fn receipt_refuses_an_unknown_claim() {
    let repo = Repo::new();
    delta_with_bare_claims(&repo);

    let run = refused(&repo, &["receipt", "DELTA-001", "AC-D1-9", &target()]);

    assert!(run.has_message("AC-D1-9"));
}

#[test]
fn receipt_refuses_an_unknown_document() {
    let repo = Repo::new();
    delta_with_bare_claims(&repo);

    let run = repo.run(&["receipt", "DELTA-777", "AC-D1-1", &target()], "");

    assert_eq!(run.code, 2);
    assert!(run.has_message("DELTA-777"));
}

#[test]
fn receipt_refuses_an_unresolved_symbol() {
    let repo = Repo::new();
    delta_with_bare_claims(&repo);

    let run = refused(
        &repo,
        &[
            "receipt",
            "DELTA-001",
            "AC-D1-1",
            &format!("{TEST_FILE}#gone"),
        ],
    );

    assert!(run.has_message("symbol gone not found"));
}

#[test]
fn receipt_refuses_a_file_with_uncommitted_changes() {
    let repo = Repo::new();
    delta_with_bare_claims(&repo);
    repo.write(TEST_FILE, &format!("{TEST_SRC}\n// edit\n"));

    let run = refused(&repo, &["receipt", "DELTA-001", "AC-D1-1", &target()]);

    assert!(run.has_message("uncommitted"));
}

#[test]
fn receipt_refuses_a_target_without_a_symbol() {
    let repo = Repo::new();
    delta_with_bare_claims(&repo);

    refused(&repo, &["receipt", "DELTA-001", "AC-D1-1", TEST_FILE]);
}

#[test]
fn receipt_rerun_after_a_later_commit_updates_the_existing_ref_in_place() {
    let repo = Repo::new();
    delta_with_bare_claims(&repo);
    assert_eq!(
        repo.run(&["receipt", "DELTA-001", "AC-D1-2", &target()], "")
            .code,
        0
    );
    repo.commit("receipt");
    repo.write(TEST_FILE, &format!("{TEST_SRC}\n// edit\n"));
    let new = repo.commit("touch test");

    let run = repo.run(&["receipt", "DELTA-001", "AC-D1-2", &target()], "");

    assert_eq!(run.code, 0, "{}", run.stdout);
    let text = repo.read(DELTA);
    assert_eq!(text.matches("@ref").count(), 1, "{text}");
    assert!(
        text.contains(&format!(
            "- [AC-D1-2] second\n  @ref {TEST_FILE}#reopen_keeps_prior@{new}\n  note\n"
        )),
        "{text}"
    );
}

#[test]
fn receipt_for_a_different_target_adds_another_ref() {
    let repo = Repo::new();
    delta_with_bare_claims(&repo);
    repo.run(&["receipt", "DELTA-001", "AC-D1-2", &target()], "");
    repo.commit("receipt");

    let other = format!("{TEST_FILE}#other_test");
    let run = repo.run(&["receipt", "DELTA-001", "AC-D1-2", &other], "");

    assert_eq!(run.code, 0, "{}", run.stdout);
    assert_eq!(repo.read(DELTA).matches("@ref").count(), 2);
}

#[test]
fn receipt_keeps_crlf_line_endings() {
    let repo = Repo::new();
    repo.delta_file(
        1,
        "in-progress",
        &["SPEC-001"],
        &delta_body("- [AC-D1-1] first\r\n- [AC-D1-2] second\r\n  note\r\n"),
    );
    repo.spec_file(1, "active", &["src/**"], "## Intent\n\nA.\n");
    let head = repo.commit("delta");

    let run = repo.run(&["receipt", "DELTA-001", "AC-D1-2", &target()], "");

    assert_eq!(run.code, 0, "{}", run.stdout);
    let text = repo.read(DELTA);
    assert!(
        text.contains(&format!(
            "second\r\n  @ref {TEST_FILE}#reopen_keeps_prior@{head}\r\n  note\r\n"
        )),
        "{text:?}"
    );
}
