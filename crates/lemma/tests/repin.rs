mod common;
use common::*;

const EDITED_TEST: &str = "#[test]\nfn reopen_keeps_prior() { assert!(true); }\n";

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
fn repin_refuses_when_the_test_changed_but_the_claim_did_not() {
    let repo = Repo::new();
    let old = repo.head();
    let path = delta_with_receipt(&repo, &old, "works");
    repo.write(TEST_FILE, EDITED_TEST);
    repo.commit("weaken test");
    let before = repo.read(&path);

    let run = repo.run(&["re-pin", "DELTA-001"], "");

    assert_eq!(run.code, 2);
    let findings = run.findings();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0]["severity"], "error");
    assert_eq!(findings[0]["id"], "DELTA-001");
    assert!(run.has_message("AC-D1-1"));
    assert!(run.has_message("re-pin refused (pass --force to override)"));
    assert_eq!(repo.read(&path), before);
}

#[test]
fn repin_force_writes_and_warns_when_the_claim_did_not_change() {
    let repo = Repo::new();
    let old = repo.head();
    let path = delta_with_receipt(&repo, &old, "works");
    repo.write(TEST_FILE, EDITED_TEST);
    let new = repo.commit("weaken test");

    let run = repo.run(&["re-pin", "DELTA-001", "--force"], "");

    assert_eq!(run.code, 0, "{}", run.stdout);
    let findings = run.findings();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0]["severity"], "warning");
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

fn claim_committed_after_pin(repo: &Repo) -> (String, String) {
    let pin = repo.head();
    let path = delta_with_receipt(repo, &pin, "works");
    (pin, path)
}

#[test]
fn repin_refuses_a_test_only_edit_made_after_the_claim_commit() {
    let repo = Repo::new();
    let (old, path) = claim_committed_after_pin(&repo);
    repo.write("README.md", "unrelated\n");
    repo.commit("unrelated");
    repo.write(TEST_FILE, EDITED_TEST);
    repo.commit("edit only the test");

    let run = repo.run(&["re-pin", "DELTA-001"], "");

    assert_eq!(run.code, 2);
    let findings = run.findings();
    assert_eq!(findings.len(), 1, "{:?}", run.messages());
    assert_eq!(findings[0]["severity"], "error");
    assert!(run.has_message("AC-D1-1"));
    assert!(run.has_message(TEST_FILE));
    let text = repo.read(&path);
    assert!(text.contains(&old));
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
fn repin_refuses_when_the_claim_changed_in_a_commit_that_left_the_test_alone() {
    let repo = Repo::new();
    let (_, path) = claim_committed_after_pin(&repo);
    let text = repo.read(&path).replace("works", "works differently");
    repo.write(&path, &text);
    repo.commit("reword claim");
    repo.write(TEST_FILE, EDITED_TEST);
    repo.commit("edit test");

    let run = repo.run(&["re-pin", "DELTA-001"], "");

    assert_eq!(run.code, 2);
    assert_eq!(run.findings().len(), 1);
}

#[test]
fn repin_refuses_a_dirty_test_file_even_when_the_claim_is_dirty_too_or_forced() {
    let repo = Repo::new();
    let (_, path) = claim_committed_after_pin(&repo);
    repo.write(TEST_FILE, EDITED_TEST);

    let refused = repo.run(&["re-pin", "DELTA-001"], "");
    assert_eq!(refused.code, 2);
    assert_eq!(refused.findings().len(), 1);
    assert!(refused.has_message(&format!(
        "AC-D1-1: {TEST_FILE} has uncommitted changes; commit before re-pinning"
    )));

    let text = repo.read(&path).replace("works", "works differently");
    repo.write(&path, &text);
    let claim_dirty = repo.run(&["re-pin", "DELTA-001"], "");
    assert_eq!(claim_dirty.code, 2, "{}", claim_dirty.stdout);
    assert!(claim_dirty.has_message("commit before re-pinning"));

    let forced = repo.run(&["re-pin", "DELTA-001", "--force"], "");
    assert_eq!(forced.code, 2, "{}", forced.stdout);
    assert_eq!(repo.read(&path), text);
}

#[test]
fn repin_passes_when_the_claim_is_edited_uncommitted_after_the_test_commit() {
    let repo = Repo::new();
    let (_, path) = claim_committed_after_pin(&repo);
    repo.write(TEST_FILE, EDITED_TEST);
    let new = repo.commit("edit only the test");
    let text = repo.read(&path).replace("works", "works differently");
    repo.write(&path, &text);

    let run = repo.run(&["re-pin", "DELTA-001"], "");

    assert_eq!(run.code, 0, "{}", run.stdout);
    assert!(run.findings().is_empty(), "{:?}", run.messages());
    assert!(repo.read(&path).contains(&format!("@{new}")));
}

#[test]
fn repin_compares_claims_when_the_lazyspec_root_is_a_git_subdirectory() {
    let repo = Repo::in_subdir("proj");
    let (_, path) = claim_committed_after_pin(&repo);
    repo.write(TEST_FILE, EDITED_TEST);
    let text = repo.read(&path).replace("works", "works differently");
    repo.write(&path, &text);
    repo.commit("test and claim together");

    let run = repo.run(&["re-pin", "DELTA-001"], "");

    assert_eq!(run.code, 0, "{}", run.stdout);
    assert!(run.findings().is_empty(), "{:?}", run.messages());
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
    assert_eq!(flagged.code, 2);
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
fn repin_compares_each_parts_claim_and_refuses_the_untouched_one() {
    let repo = Repo::new();
    let old = repo.head();
    let (first, second) = two_part_delta(&repo, &old);
    repo.write(TEST_FILE, EDITED_TEST);
    let reworded = repo.read(&first).replace("one", "one differently");
    repo.write(&first, &reworded);
    repo.commit("test and first claim move together");
    let before = (repo.read(&first), repo.read(&second));

    let run = repo.run(&["re-pin", "DELTA-001"], "");

    assert_eq!(run.code, 2);
    let findings = run.findings();
    assert_eq!(findings.len(), 1, "{:?}", run.messages());
    assert_eq!(findings[0]["part"], "behaviour.SPEC-002");
    assert!(run.has_message("AC-D1-2"));
    assert_eq!(findings[0]["line"], repo.claim_line(&second, "AC-D1-2") + 1);
    assert_eq!((repo.read(&first), repo.read(&second)), before);
}

#[test]
fn repin_rewrites_only_the_named_claims() {
    let repo = Repo::new();
    let old = repo.head();
    let (first, second) = two_part_delta(&repo, &old);
    repo.write("src/lib.rs", "pub fn changed() {}\n");
    let new = repo.commit("code moved");

    let run = repo.run(&["re-pin", "DELTA-001", "AC-D1-1"], "");

    assert_eq!(run.code, 0, "{}", run.stdout);
    assert!(repo.read(&first).contains(&format!("@{new}")));
    assert!(repo.read(&second).contains(&format!("@{old}")));
}

#[test]
fn repin_refuses_an_unknown_claim_id_and_writes_nothing() {
    let repo = Repo::new();
    let old = repo.head();
    let (first, second) = two_part_delta(&repo, &old);
    repo.write("src/lib.rs", "pub fn changed() {}\n");
    repo.commit("code moved");
    let before = (repo.read(&first), repo.read(&second));

    let run = repo.run(&["re-pin", "DELTA-001", "AC-D1-1", "AC-D1-9"], "");

    assert_eq!(run.code, 2);
    assert_eq!(run.findings().len(), 1);
    assert_eq!(run.findings()[0]["severity"], "error");
    assert!(run.has_message("AC-D1-9"));
    assert_eq!((repo.read(&first), repo.read(&second)), before);
}

fn sibling_claims_delta(repo: &Repo, old: &str) -> String {
    repo.delta_file(1, "in-progress", &["SPEC-001"], INTENT);
    let body = format!(
        "{}{}",
        claim("AC-D1-1", "one", Some(old)),
        claim("AC-D1-2", "two", Some(old))
    );
    repo.delta_part(1, "SPEC-001", &body);
    repo.spec_file(1, "active", &["src/**"], "## Intent\n\nA.\n");
    repo.commit("delta");
    delta_part_path(1, "SPEC-001")
}

#[test]
fn repin_compares_per_claim_when_a_sibling_claim_changed_with_the_test() {
    let repo = Repo::new();
    let old = repo.head();
    let path = sibling_claims_delta(&repo, &old);
    repo.write(TEST_FILE, EDITED_TEST);
    let reworded = repo.read(&path).replace("two", "two differently");
    repo.write(&path, &reworded);
    let new = repo.commit("test and sibling claim move together");
    let before = repo.read(&path);

    let refused = repo.run(&["re-pin", "DELTA-001", "AC-D1-1"], "");
    assert_eq!(refused.code, 2);
    assert_eq!(refused.findings().len(), 1);
    assert!(refused.has_message("AC-D1-1"));
    assert_eq!(repo.read(&path), before);

    let ok = repo.run(&["re-pin", "DELTA-001", "AC-D1-2"], "");
    assert_eq!(ok.code, 0, "{}", ok.stdout);
    assert!(ok.findings().is_empty());
    let text = repo.read(&path);
    assert_eq!(text.matches(&format!("@{new}")).count(), 1);
    assert_eq!(text.matches(&format!("@{old}")).count(), 1);
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

const SYMBOL_FILE: &str = "tests/store_test.rs";

fn pinned_at_head(repo: &Repo) -> (String, String) {
    let old = repo.head();
    let path = delta_with_receipt(repo, &old, "works");
    (old, path)
}

#[test]
fn repin_ignores_a_sibling_test_added_to_the_same_file() {
    let repo = Repo::new();
    let (_, path) = pinned_at_head(&repo);
    repo.write(
        SYMBOL_FILE,
        &format!("{TEST_SRC}\n#[test]\nfn added() {{}}\n"),
    );
    let new = repo.commit("add sibling test");

    let run = repo.run(&["re-pin", "DELTA-001"], "");

    assert_eq!(run.code, 0, "{}", run.stdout);
    assert!(run.findings().is_empty(), "{:?}", run.messages());
    assert!(repo.read(&path).contains(&format!("@{new}")));
}

#[test]
fn repin_ignores_a_doc_comment_added_to_the_file() {
    let repo = Repo::new();
    let (_, path) = pinned_at_head(&repo);
    repo.write(
        SYMBOL_FILE,
        &TEST_SRC.replace(
            "#[test]\nfn reopen",
            "/// Reopening keeps the prior.\n#[test]\nfn reopen",
        ),
    );
    let new = repo.commit("doc comment");

    let run = repo.run(&["re-pin", "DELTA-001"], "");

    assert_eq!(run.code, 0, "{}", run.stdout);
    assert!(run.findings().is_empty(), "{:?}", run.messages());
    assert!(repo.read(&path).contains(&format!("@{new}")));
}

#[test]
fn repin_refuses_an_attribute_added_above_the_test() {
    let repo = Repo::new();
    let (_, path) = pinned_at_head(&repo);
    repo.write(
        SYMBOL_FILE,
        &TEST_SRC.replace(
            "#[test]\nfn reopen",
            "#[test]\n#[ignore = \"later\"]\nfn reopen",
        ),
    );
    repo.commit("ignore the test");
    let before = repo.read(&path);

    let run = repo.run(&["re-pin", "DELTA-001"], "");

    assert_eq!(run.code, 2, "{}", run.stdout);
    assert!(run.has_message("re-pin refused"));
    assert_eq!(repo.read(&path), before);
}

#[test]
fn repin_forced_marks_the_ref_not_the_end_of_the_line() {
    let repo = Repo::new();
    let old = repo.head();
    repo.delta_file(1, "in-progress", &["SPEC-001"], INTENT);
    repo.delta_part(
        1,
        "SPEC-001",
        &format!("- [AC-D1-1] works\n  @ref {TEST_FILE}#reopen_keeps_prior@{old} (happy path)\n"),
    );
    repo.spec_file(1, "active", &["src/**"], "## Intent\n\nA.\n");
    repo.commit("delta");
    repo.write(TEST_FILE, EDITED_TEST);
    let new = repo.commit("weaken test");

    let run = repo.run(&["re-pin", "DELTA-001", "--force"], "");

    assert_eq!(run.code, 0, "{}", run.stdout);
    assert!(repo
        .read(&delta_part_path(1, "SPEC-001"))
        .contains(&format!(
            "  @ref {TEST_FILE}#reopen_keeps_prior@{new} [forced] (happy path)\n"
        )));
}

#[test]
fn repin_refuses_when_the_symbol_did_not_exist_at_the_pin() {
    let repo = Repo::new();
    repo.write(SYMBOL_FILE, "#[test]\nfn other_test() {}\n");
    repo.commit("symbol not there yet");
    let (_, path) = pinned_at_head(&repo);
    repo.write(SYMBOL_FILE, TEST_SRC);
    repo.commit("symbol appears after the pin");
    let before = repo.read(&path);

    let run = repo.run(&["re-pin", "DELTA-001"], "");

    assert_eq!(run.code, 2, "{}", run.stdout);
    assert!(run.has_message("re-pin refused"));
    assert_eq!(repo.read(&path), before);
}

fn delta_receipting_a_plain_file(repo: &Repo, sha: &str) -> String {
    repo.delta_file(1, "in-progress", &["SPEC-001"], INTENT);
    repo.delta_part(
        1,
        "SPEC-001",
        &format!("- [AC-D1-1] works\n  @ref tests/notes.txt#alpha@{sha}\n"),
    );
    repo.spec_file(1, "active", &["src/**"], "## Intent\n\nA.\n");
    repo.commit("delta");
    delta_part_path(1, "SPEC-001")
}

#[test]
fn repin_compares_whole_files_when_no_extractor_exists() {
    let repo = Repo::new();
    repo.write("tests/notes.txt", "alpha\nbeta\n");
    let old = repo.commit("notes");
    let path = delta_receipting_a_plain_file(&repo, &old);
    repo.write("tests/notes.txt", "alpha\nbeta\ngamma\n");
    repo.commit("edit another line");
    let before = repo.read(&path);

    let run = repo.run(&["re-pin", "DELTA-001"], "");

    assert_eq!(run.code, 2, "{}", run.stdout);
    assert!(run.has_message("re-pin refused"));
    assert_eq!(repo.read(&path), before);
}

#[test]
fn repin_force_marks_the_receipt_line_as_forced() {
    let repo = Repo::new();
    let (_, path) = pinned_at_head(&repo);
    repo.write(TEST_FILE, EDITED_TEST);
    let new = repo.commit("weaken test");

    let run = repo.run(&["re-pin", "DELTA-001", "--force"], "");

    assert_eq!(run.code, 0, "{}", run.stdout);
    let line = format!("  @ref {TEST_FILE}#reopen_keeps_prior@{new} [forced]");
    assert!(repo.read(&path).contains(&format!("{line}\n")));
    assert_eq!(run.json()["wrote"][0]["text"], line);
}

#[test]
fn repin_force_does_not_mark_a_pin_that_had_not_drifted() {
    let repo = Repo::new();
    let (_, path) = pinned_at_head(&repo);
    repo.write("src/lib.rs", "pub fn changed() {}\n");
    repo.commit("code moved");

    let run = repo.run(&["re-pin", "DELTA-001", "--force"], "");

    assert_eq!(run.code, 0, "{}", run.stdout);
    assert!(!repo.read(&path).contains("[forced]"));
}

#[test]
fn repin_keeps_an_existing_forced_marker() {
    let repo = Repo::new();
    let old = repo.head();
    let path = delta_with_receipt(&repo, &old, "works");
    let marked = repo.read(&path).replace(&old, &format!("{old} [forced]"));
    repo.write(&path, &marked);
    repo.commit("forced pin");
    repo.write("src/lib.rs", "pub fn changed() {}\n");
    let new = repo.commit("code moved");

    let run = repo.run(&["re-pin", "DELTA-001"], "");

    assert_eq!(run.code, 0, "{}", run.stdout);
    assert!(repo.read(&path).contains(&format!("@{new} [forced]\n")));
}

#[test]
fn repin_lists_each_rewritten_line() {
    let repo = Repo::new();
    let old = repo.head();
    let path = sibling_claims_delta(&repo, &old);
    repo.write("src/lib.rs", "pub fn changed() {}\n");
    let new = repo.commit("code moved");

    let run = repo.run(&["re-pin", "DELTA-001", "AC-D1-2"], "");

    assert_eq!(run.code, 0, "{}", run.stdout);
    assert_eq!(
        run.json()["wrote"],
        serde_json::json!([{
            "id": "AC-D1-2",
            "part": "behaviour.SPEC-001",
            "line": 4,
            "text": format!("  @ref {TEST_FILE}#reopen_keeps_prior@{new}"),
        }])
    );
    assert_eq!(
        repo.read(&path).lines().nth(3).unwrap(),
        run.json()["wrote"][0]["text"]
    );
}
