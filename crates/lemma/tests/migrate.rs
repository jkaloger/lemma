mod common;
use common::*;

fn project() -> Repo {
    let repo = Repo::new();
    repo.spec_file(1, "active", &["src/**"], INTENT);
    repo
}

fn migrate(repo: &Repo) -> Run {
    repo.run(&["migrate"], "")
}

fn spec_part(repo: &Repo) -> String {
    repo.read(&spec_part_path(1))
}

#[test]
fn a_commit_pinned_ref_loses_its_pin() {
    let repo = project();
    repo.spec_behaviour(
        1,
        "- [A-1] one\n  @ref tests/store_test.rs#reopen_keeps_prior@3f9c2e1\n",
    );

    let run = migrate(&repo);

    assert_eq!(run.code, 0);
    assert_eq!(run.stdout, format!("{}\n", spec_part_path(1)));
    assert_eq!(
        spec_part(&repo),
        "- [A-1] one\n  @ref tests/store_test.rs#reopen_keeps_prior\n"
    );
}

#[test]
fn a_blob_pinned_ref_loses_its_pin() {
    let repo = project();
    repo.spec_behaviour(
        1,
        "- [A-1] one\n  @ref tests/store_test.rs#reopen_keeps_prior@{blob:3f9c2e1}\n",
    );

    migrate(&repo);

    assert_eq!(
        spec_part(&repo),
        "- [A-1] one\n  @ref tests/store_test.rs#reopen_keeps_prior\n"
    );
}

#[test]
fn a_forced_marker_is_stripped_with_the_pin() {
    let repo = project();
    repo.spec_behaviour(
        1,
        "- [A-1] one\n  @ref tests/store_test.rs#reopen_keeps_prior@3f9c2e1 [forced]\n  @ref tests/store_test.rs#other_test@3f9c2e1 [forced] trailing\n",
    );

    migrate(&repo);

    assert_eq!(
        spec_part(&repo),
        "- [A-1] one\n  @ref tests/store_test.rs#reopen_keeps_prior\n  @ref tests/store_test.rs#other_test trailing\n"
    );
}

#[test]
fn base_lines_are_deleted_from_delta_parts() {
    let repo = project();
    repo.spec_behaviour(1, "- [A-1] one\n");
    repo.delta_file(1, "draft", &["SPEC-001"], INTENT);
    repo.delta_part(
        1,
        "SPEC-001",
        "- ~~[A-1]~~ one\n  @base 0123456789ab\n- [A-2] two\n  @ref tests/store_test.rs#other_test@abc1234\n  @base 0123456789ab\n",
    );

    let run = migrate(&repo);

    assert_eq!(run.stdout, format!("{}\n", delta_part_path(1, "SPEC-001")));
    assert_eq!(
        repo.read(&delta_part_path(1, "SPEC-001")),
        "- ~~[A-1]~~ one\n- [A-2] two\n  @ref tests/store_test.rs#other_test\n"
    );
}

#[test]
fn a_claim_with_only_checked_receipts_becomes_intent() {
    let repo = project();
    repo.spec_behaviour(
        1,
        "- [A-1] Feels fast.\n  @checked jack@3f9c2e1\n- [A-2] Looks right.\n  more detail\n  @checked jack@3f9c2e1\n  @checked ana@abc1234\n",
    );

    migrate(&repo);

    assert_eq!(
        spec_part(&repo),
        "- [A-1] [intent] Feels fast.\n- [A-2] [intent] Looks right.\n  more detail\n"
    );
}

#[test]
fn a_claim_with_a_ref_and_a_checked_line_keeps_the_ref_and_stays_evidenced() {
    let repo = project();
    repo.spec_behaviour(
        1,
        "- [A-1] one\n  @ref tests/store_test.rs#reopen_keeps_prior@3f9c2e1\n  @checked jack@3f9c2e1\n- [A-2] two\n  @ref tests/store_test.rs#other_test\n  @checked jack@3f9c2e1\n",
    );

    migrate(&repo);

    assert_eq!(
        spec_part(&repo),
        "- [A-1] one\n  @ref tests/store_test.rs#reopen_keeps_prior\n- [A-2] two\n  @ref tests/store_test.rs#other_test\n"
    );
}

#[test]
fn an_existing_intent_marker_is_not_doubled() {
    let repo = project();
    repo.spec_behaviour(1, "- [A-1] [intent] Feels fast.\n  @checked jack@3f9c2e1\n");

    migrate(&repo);

    assert_eq!(spec_part(&repo), "- [A-1] [intent] Feels fast.\n");
}

#[test]
fn a_struck_claim_is_not_marked_intent() {
    let repo = project();
    repo.delta_file(1, "draft", &["SPEC-001"], INTENT);
    repo.spec_behaviour(1, "- [A-1] one\n");
    repo.delta_part(1, "SPEC-001", "- ~~[A-1]~~ one\n  @checked jack@3f9c2e1\n");

    migrate(&repo);

    assert_eq!(
        repo.read(&delta_part_path(1, "SPEC-001")),
        "- ~~[A-1]~~ one\n"
    );
}

#[test]
fn legacy_markers_in_fences_and_prose_are_left_alone() {
    let repo = project();
    let body = "- [A-1] one `@base`\n  ```\n  @base 0123456789ab\n  ```\n";
    repo.spec_behaviour(1, body);

    let run = migrate(&repo);

    assert_eq!(run.stdout, "");
    assert_eq!(spec_part(&repo), body);
}

#[test]
fn it_is_idempotent_and_leaves_a_checkable_project() {
    let repo = project();
    repo.spec_behaviour(
        1,
        "- [A-1] one\n  @ref tests/store_test.rs#reopen_keeps_prior@3f9c2e1 [forced]\n  @base 0123456789ab\n- [A-2] two\n  @checked jack@3f9c2e1\n",
    );
    assert_eq!(repo.run(&["check"], "").code, 2);
    repo.commit("legacy");

    let first = migrate(&repo);
    let after_first = spec_part(&repo);
    let second = migrate(&repo);

    assert_eq!(first.stdout.lines().count(), 1);
    assert_eq!((second.code, second.stdout.as_str()), (0, ""));
    assert_eq!(spec_part(&repo), after_first);
    assert_eq!(repo.run(&["check"], "").code, 0);
}

#[test]
fn every_rewritten_file_is_printed_once() {
    let repo = project();
    repo.spec_file(2, "active", &["lib/**"], INTENT);
    repo.spec_behaviour(1, "- [A-1] one\n  @checked jack@abc1234\n");
    repo.spec_behaviour(2, "- [B-1] one\n  @base 0123456789ab\n");
    repo.spec_file(3, "active", &["other/**"], INTENT);
    repo.spec_behaviour(3, "- [C-1] fine\n");

    let run = migrate(&repo);

    assert_eq!(
        run.stdout,
        format!("{}\n{}\n", spec_part_path(1), spec_part_path(2))
    );
}

#[test]
fn without_a_lazyspec_project_it_exits_two_with_a_message() {
    let dir = tempfile::tempdir().unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_lemma"))
        .arg("migrate")
        .current_dir(dir.path())
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("no .lazyspec.toml"));
    assert!(out.stdout.is_empty());
}
