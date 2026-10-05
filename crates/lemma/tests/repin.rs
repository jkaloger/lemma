mod common;
use common::*;

fn delta_with_receipt(repo: &Repo, sha: &str, text: &str) -> String {
    repo.delta_file(1, "in-progress", &["SPEC-001"], INTENT);
    repo.delta_part(1, "SPEC-001", &claim("AC-D1-1", text, Some(sha)));
    repo.spec_file(1, "active", &["src/**"], "## Intent\n\nA.\n");
    repo.commit("delta");
    delta_part_path(1, "SPEC-001")
}

#[test]
fn repin_rewrites_receipts_to_head() {
    let repo = Repo::new();
    let old = repo.head();
    let path = delta_with_receipt(&repo, &old, "works");
    repo.write("src/lib.rs", "pub fn changed() {}\n");
    let new = repo.commit("code moved");

    let run = repo.run(&["re-pin", "DELTA-001"], "");

    assert_eq!(run.code, 0, "{}", run.stdout);
    assert!(run.findings().is_empty());
    let text = repo.read(&path);
    assert!(
        text.contains(&format!("@ref {TEST_FILE}#reopen_keeps_prior@{new}")),
        "{text}"
    );
    assert!(!text.contains(&old));
    assert!(!text.starts_with("---"));
}

#[test]
fn repin_refuses_and_writes_nothing_when_a_symbol_is_gone() {
    let repo = Repo::new();
    let old = repo.head();
    let path = delta_with_receipt(&repo, &old, "works");
    repo.write(TEST_FILE, "#[test]\nfn renamed() {}\n");
    repo.commit("rename test");
    let before = repo.read(&path);

    let run = repo.run(&["re-pin", "DELTA-001"], "");

    assert_eq!(run.code, 2);
    assert_eq!(run.findings()[0]["severity"], "error");
    assert!(run.has_message("symbol reopen_keeps_prior not found"));
    assert_eq!(repo.read(&path), before);
}

#[test]
fn repin_flags_when_the_test_changed_but_the_claim_did_not() {
    let repo = Repo::new();
    let old = repo.head();
    let path = delta_with_receipt(&repo, &old, "works");
    repo.write(
        TEST_FILE,
        "#[test]\nfn reopen_keeps_prior() { assert!(true); }\n",
    );
    let new = repo.commit("weaken test");

    let run = repo.run(&["re-pin", "DELTA-001"], "");

    assert_eq!(run.code, 0);
    let findings = run.findings();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0]["severity"], "warning");
    assert_eq!(findings[0]["id"], "DELTA-001");
    assert!(run.has_message("AC-D1-1"));
    assert!(repo.read(&path).contains(&format!("@{new}")));
}

#[test]
fn repin_does_not_flag_when_the_claim_changed_with_the_test() {
    let repo = Repo::new();
    let old = repo.head();
    let path = delta_with_receipt(&repo, &old, "works");
    repo.write(
        TEST_FILE,
        "#[test]\nfn reopen_keeps_prior() { assert!(true); }\n",
    );
    let text = repo.read(&path).replace("works", "works differently");
    repo.write(&path, &text);
    repo.commit("test and claim move together");

    let run = repo.run(&["re-pin", "DELTA-001"], "");

    assert_eq!(run.code, 0);
    assert!(run.findings().is_empty());
}

#[test]
fn repin_unknown_document_refuses() {
    let repo = Repo::new();

    let run = repo.run(&["re-pin", "DELTA-777"], "");

    assert_eq!(run.code, 2);
    assert!(run.has_message("DELTA-777"));
}

const EDITED_TEST: &str = "#[test]\nfn reopen_keeps_prior() { assert!(true); }\n";

fn claim_committed_after_pin(repo: &Repo) -> (String, String) {
    let pin = repo.head();
    let path = delta_with_receipt(repo, &pin, "works");
    (pin, path)
}

#[test]
fn repin_flags_a_test_only_edit_made_after_the_claim_commit() {
    let repo = Repo::new();
    let (old, path) = claim_committed_after_pin(&repo);
    repo.write("README.md", "unrelated\n");
    repo.commit("unrelated");
    repo.write(TEST_FILE, EDITED_TEST);
    let new = repo.commit("edit only the test");

    let run = repo.run(&["re-pin", "DELTA-001"], "");

    assert_eq!(run.code, 0);
    let findings = run.findings();
    assert_eq!(findings.len(), 1, "{:?}", run.messages());
    assert_eq!(findings[0]["severity"], "warning");
    assert!(run.has_message("AC-D1-1"));
    assert!(run.has_message(TEST_FILE));
    let text = repo.read(&path);
    assert!(text.contains(&format!("@{new}")));
    assert!(!text.contains(&old));
}

#[test]
fn repin_does_not_flag_when_a_commit_edits_test_and_claim_together() {
    let repo = Repo::new();
    let (_, path) = claim_committed_after_pin(&repo);
    repo.write(TEST_FILE, EDITED_TEST);
    let text = repo.read(&path).replace("works", "works differently");
    repo.write(&path, &text);
    repo.commit("test and claim together");

    let run = repo.run(&["re-pin", "DELTA-001"], "");

    assert!(run.findings().is_empty(), "{:?}", run.messages());
}

#[test]
fn repin_flags_when_the_claim_changed_in_a_commit_that_left_the_test_alone() {
    let repo = Repo::new();
    let (_, path) = claim_committed_after_pin(&repo);
    let text = repo.read(&path).replace("works", "works differently");
    repo.write(&path, &text);
    repo.commit("reword claim");
    repo.write(TEST_FILE, EDITED_TEST);
    repo.commit("edit test");

    let run = repo.run(&["re-pin", "DELTA-001"], "");

    assert_eq!(run.findings().len(), 1);
}

#[test]
fn repin_flags_a_dirty_test_file_and_clears_when_the_claim_is_dirty_too() {
    let repo = Repo::new();
    let (_, path) = claim_committed_after_pin(&repo);
    repo.write(TEST_FILE, EDITED_TEST);

    assert_eq!(repo.run(&["re-pin", "DELTA-001"], "").findings().len(), 1);

    repo.git(&["checkout", "--", &path]);
    let text = repo.read(&path).replace("works", "works differently");
    repo.write(&path, &text);

    assert!(repo.run(&["re-pin", "DELTA-001"], "").findings().is_empty());
}

#[test]
fn repin_findings_carry_file_lines() {
    let repo = Repo::new();
    let old = repo.head();
    let path = delta_with_receipt(&repo, &old, "works");
    let receipt_line = repo.claim_line(&path, "AC-D1-1") + 1;

    repo.write(TEST_FILE, EDITED_TEST);
    repo.commit("weaken test");
    let flagged = repo.run(&["re-pin", "DELTA-001"], "");
    assert_eq!(flagged.findings()[0]["line"], receipt_line);

    repo.write(TEST_FILE, "#[test]\nfn renamed() {}\n");
    repo.commit("rename test");
    let refused = repo.run(&["re-pin", "DELTA-001"], "");
    assert_eq!(refused.findings()[0]["line"], receipt_line);
}

#[test]
fn repin_findings_carry_the_part() {
    let repo = Repo::new();
    let old = repo.head();
    delta_with_receipt(&repo, &old, "works");
    repo.write(TEST_FILE, EDITED_TEST);
    repo.commit("weaken test");

    let flagged = repo.run(&["re-pin", "DELTA-001"], "");

    assert_eq!(flagged.findings()[0]["part"], "behaviour.SPEC-001");
}

fn two_part_delta(repo: &Repo, old: &str) -> (String, String) {
    repo.delta_file(1, "in-progress", &["SPEC-001", "SPEC-002"], INTENT);
    repo.delta_part(1, "SPEC-001", &claim("AC-D1-1", "one", Some(old)));
    repo.delta_part(1, "SPEC-002", &claim("AC-D1-2", "two", Some(old)));
    repo.spec_file(1, "active", &["src/**"], "## Intent\n\nA.\n");
    repo.spec_file(2, "active", &["lib/**"], "## Intent\n\nB.\n");
    repo.commit("delta");
    (
        delta_part_path(1, "SPEC-001"),
        delta_part_path(1, "SPEC-002"),
    )
}

#[test]
fn repin_rewrites_every_behaviour_part() {
    let repo = Repo::new();
    let old = repo.head();
    let (first, second) = two_part_delta(&repo, &old);
    repo.write("src/lib.rs", "pub fn changed() {}\n");
    let new = repo.commit("code moved");

    let run = repo.run(&["re-pin", "DELTA-001"], "");

    assert_eq!(run.code, 0, "{}", run.stdout);
    for part in [first, second] {
        assert!(repo.read(&part).contains(&format!("@{new}")), "{part}");
    }
}

#[test]
fn repin_flags_per_part_comparing_each_parts_claim_lines() {
    let repo = Repo::new();
    let old = repo.head();
    let (first, second) = two_part_delta(&repo, &old);
    repo.write(TEST_FILE, EDITED_TEST);
    let reworded = repo.read(&first).replace("one", "one differently");
    repo.write(&first, &reworded);
    repo.commit("test and first claim move together");

    let run = repo.run(&["re-pin", "DELTA-001"], "");

    assert_eq!(run.code, 0);
    let findings = run.findings();
    assert_eq!(findings.len(), 1, "{:?}", run.messages());
    assert_eq!(findings[0]["part"], "behaviour.SPEC-002");
    assert!(run.has_message("AC-D1-2"));
    assert_eq!(findings[0]["line"], repo.claim_line(&second, "AC-D1-2") + 1);
}

#[test]
fn repin_refuses_and_writes_no_part_when_one_symbol_is_gone() {
    let repo = Repo::new();
    let old = repo.head();
    let (first, second) = two_part_delta(&repo, &old);
    let second_text = repo
        .read(&second)
        .replace("reopen_keeps_prior", "other_test");
    repo.write(&second, &second_text);
    repo.commit("second part names another test");
    repo.write(TEST_FILE, "#[test]\nfn reopen_keeps_prior() {}\n");
    repo.commit("drop other_test");
    let before = (repo.read(&first), repo.read(&second));

    let run = repo.run(&["re-pin", "DELTA-001"], "");

    assert_eq!(run.code, 2);
    assert_eq!(run.findings()[0]["part"], "behaviour.SPEC-002");
    assert_eq!((repo.read(&first), repo.read(&second)), before);
}

#[test]
fn repin_leaves_a_checked_receipt_alone() {
    let repo = Repo::new();
    let old = repo.head();
    repo.delta_file(1, "in-progress", &["SPEC-001"], INTENT);
    repo.delta_part(
        1,
        "SPEC-001",
        &format!("- [AC-D1-1] by hand\n  @checked jack@{old}\n"),
    );
    repo.spec_file(1, "active", &["src/**"], "## Intent\n\nA.\n");
    repo.commit("delta");
    repo.write("src/lib.rs", "pub fn changed() {}\n");
    repo.commit("code moved");

    let run = repo.run(&["re-pin", "DELTA-001"], "");

    assert_eq!(run.code, 0, "{}", run.stdout);
    assert!(run.findings().is_empty());
    assert_eq!(
        repo.read(&delta_part_path(1, "SPEC-001")),
        format!("- [AC-D1-1] by hand\n  @checked jack@{old}\n")
    );
}
