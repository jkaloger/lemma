mod common;
use common::*;

const INDEX: &str = "docs/deltas/DELTA-001-d/index.md";
const DELTA: &str = "docs/deltas/DELTA-001-d/behaviour.SPEC-001.md";

fn delta_with_bare_claims(repo: &Repo) {
    repo.delta_file(1, "in-progress", &["SPEC-001"], INTENT);
    repo.delta_part(
        1,
        "SPEC-001",
        "- [AC-D1-1] first\n- [AC-D1-2] second\n  note\n",
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
    let index_before = repo.read(INDEX);

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
    assert!(!text.starts_with("---"));
    assert_eq!(repo.read(INDEX), index_before);
}

#[test]
fn receipt_lands_in_the_part_holding_the_claim() {
    let repo = Repo::new();
    delta_with_bare_claims(&repo);
    repo.delta_part(1, "SPEC-002", "- [AC-D1-7] elsewhere\n");
    repo.spec_file(2, "active", &["src/**"], "## Intent\n\nB.\n");
    repo.commit("second part");
    let other_part = delta_part_path(1, "SPEC-002");
    let first_before = repo.read(DELTA);
    let head = repo.head();

    let run = repo.run(&["receipt", "DELTA-001", "AC-D1-7", &target()], "");

    assert_eq!(run.code, 0, "{}", run.stdout);
    assert_eq!(
        repo.read(&other_part),
        format!("- [AC-D1-7] elsewhere\n  @ref {TEST_FILE}#reopen_keeps_prior@{head}\n")
    );
    assert_eq!(repo.read(DELTA), first_before);
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

    assert!(run.has_message("no live claim AC-D1-9 in any behaviour part"));
}

#[test]
fn receipt_refuses_a_claim_that_only_sits_in_a_non_behaviour_part() {
    let repo = Repo::new();
    delta_with_bare_claims(&repo);
    repo.write("docs/deltas/DELTA-001-d/tasks.md", "- [AC-D1-5] a task\n");
    repo.commit("tasks part");

    let run = refused(&repo, &["receipt", "DELTA-001", "AC-D1-5", &target()]);

    assert!(run.has_message("AC-D1-5"));
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
    repo.delta_file(1, "in-progress", &["SPEC-001"], INTENT);
    repo.delta_part(
        1,
        "SPEC-001",
        "- [AC-D1-1] first\r\n- [AC-D1-2] second\r\n  note\r\n",
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

#[test]
fn receipt_ignores_a_struck_claim_and_lands_on_the_live_one() {
    let repo = Repo::new();
    delta_with_bare_claims(&repo);
    repo.delta_part(1, "SPEC-002", "- ~~[AC-D1-2]~~ moved away\n");
    repo.commit("struck copy in another part");
    let struck_before = repo.read(&delta_part_path(1, "SPEC-002"));

    let run = repo.run(&["receipt", "DELTA-001", "AC-D1-2", &target()], "");

    assert_eq!(run.code, 0, "{}", run.stdout);
    assert!(repo.read(DELTA).contains("@ref"));
    assert_eq!(repo.read(&delta_part_path(1, "SPEC-002")), struck_before);
}

#[test]
fn receipt_refuses_a_claim_that_is_only_struck() {
    let repo = Repo::new();
    delta_with_bare_claims(&repo);
    repo.delta_part(1, "SPEC-002", "- ~~[AC-D1-8]~~ gone\n");
    repo.commit("struck only");

    refused(&repo, &["receipt", "DELTA-001", "AC-D1-8", &target()]);
}

#[test]
fn receipt_refuses_a_claim_live_in_two_parts() {
    let repo = Repo::new();
    delta_with_bare_claims(&repo);
    repo.delta_part(1, "SPEC-002", "- [AC-D1-2] twice\n");
    repo.commit("duplicate");

    let run = refused(&repo, &["receipt", "DELTA-001", "AC-D1-2", &target()]);

    assert!(run.has_message("more than one part"));
}

fn based_delta(repo: &Repo, delta_claims: &str) {
    repo.delta_file(1, "in-progress", &["SPEC-001"], INTENT);
    repo.delta_part(1, "SPEC-001", delta_claims);
    repo.spec_file(1, "active", &["src/**"], "## Intent\n\nA.\n");
    repo.spec_behaviour(1, "- [AC-S-1] old\n");
    repo.commit("delta");
}

#[test]
fn receipt_on_a_replacing_claim_inserts_base_after_the_ref() {
    let repo = Repo::new();
    based_delta(&repo, "- [AC-S-1] new\n  note\n");
    let head = repo.head();

    let run = repo.run(&["receipt", "DELTA-001", "AC-S-1", &target()], "");

    assert_eq!(run.code, 0, "{}", run.stdout);
    assert_eq!(
        repo.read(DELTA),
        format!(
            "- [AC-S-1] new\n  @ref {TEST_FILE}#reopen_keeps_prior@{head}\n{}  note\n",
            base_line_for("- [AC-S-1] old\n", "AC-S-1")
        )
    );
}

#[test]
fn receipt_on_a_new_id_claim_adds_no_base() {
    let repo = Repo::new();
    based_delta(&repo, "- [AC-D1-1] fresh\n");

    let run = repo.run(&["receipt", "DELTA-001", "AC-D1-1", &target()], "");

    assert_eq!(run.code, 0, "{}", run.stdout);
    assert!(!repo.read(DELTA).contains("@base"));
}

#[test]
fn receipt_leaves_an_existing_base_alone() {
    let repo = Repo::new();
    based_delta(&repo, "- [AC-S-1] new\n  @base 000000000000\n");

    let run = repo.run(&["receipt", "DELTA-001", "AC-S-1", &target()], "");

    assert_eq!(run.code, 0, "{}", run.stdout);
    assert_eq!(repo.read(DELTA).matches("@base").count(), 1);
    assert!(repo.read(DELTA).contains("@base 000000000000"));
}

#[test]
fn re_receipting_a_replaced_claim_puts_base_after_the_id_line() {
    let repo = Repo::new();
    based_delta(
        &repo,
        &format!("- [AC-S-1] new\n  @ref {TEST_FILE}#reopen_keeps_prior@1111111\n"),
    );

    let run = repo.run(&["receipt", "DELTA-001", "AC-S-1", &target()], "");

    assert_eq!(run.code, 0, "{}", run.stdout);
    assert!(repo.read(DELTA).starts_with(&format!(
        "- [AC-S-1] new\n{}",
        base_line_for("- [AC-S-1] old\n", "AC-S-1")
    )));
}

#[test]
fn receipt_refuses_a_production_path_and_writes_nothing() {
    let repo = Repo::new();
    delta_with_bare_claims(&repo);

    let run = refused(&repo, &["receipt", "DELTA-001", "AC-D1-1", "src/lib.rs#a"]);

    assert!(run.has_message(
        "src/lib.rs is not a test file under SPEC-001's `tests` globs; a production symbol is not a receipt"
    ));
}

#[test]
fn a_spec_tests_override_accepts_its_own_paths_and_rejects_the_default() {
    let repo = Repo::new();
    delta_with_bare_claims(&repo);
    repo.spec_file_with_tests(
        1,
        "active",
        &["src/**"],
        &["checks/**"],
        "## Intent\n\nA.\n",
    );
    repo.write("checks/foo.rs", "#[test]\nfn holds() {}\n");
    repo.commit("override");

    let rejected = refused(&repo, &["receipt", "DELTA-001", "AC-D1-1", &target()]);
    assert!(rejected.has_message("is not a test file under SPEC-001's"));

    let run = repo.run(
        &["receipt", "DELTA-001", "AC-D1-1", "checks/foo.rs#holds"],
        "",
    );
    assert_eq!(run.code, 0, "{}", run.stdout);
    assert!(repo.read(DELTA).contains("@ref checks/foo.rs#holds@"));
}

#[test]
fn receipt_refuses_when_the_target_spec_does_not_resolve() {
    let repo = Repo::new();
    repo.delta_file(1, "in-progress", &["SPEC-009"], INTENT);
    repo.delta_part(1, "SPEC-009", "- [AC-D1-1] first\n");
    repo.commit("delta");

    let part = delta_part_path(1, "SPEC-009");
    let before = repo.read(&part);

    let run = repo.run(&["receipt", "DELTA-001", "AC-D1-1", &target()], "");

    assert_eq!(run.code, 2, "{}", run.stdout);
    assert!(run.has_message("SPEC-009"));
    assert_eq!(repo.read(&part), before);
}

#[test]
fn receipt_on_a_specs_own_claim_applies_the_spec_tests_globs() {
    let repo = Repo::new();
    repo.spec_file(1, "active", &["src/**"], "## Intent\n\nA.\n");
    repo.spec_behaviour(1, "- [AC-1] works\n");
    repo.commit("spec");
    let part = spec_part_path(1);
    let before = repo.read(&part);

    let run = repo.run(&["receipt", "SPEC-001", "AC-1", "src/lib.rs#a"], "");

    assert_eq!(run.code, 2, "{}", run.stdout);
    assert!(run.has_message(
        "src/lib.rs is not a test file under SPEC-001's `tests` globs; a production symbol is not a receipt"
    ));
    assert_eq!(repo.read(&part), before);
}
