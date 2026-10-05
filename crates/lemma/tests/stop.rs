mod common;
use common::*;
use serde_json::json;

fn two_specs() -> Repo {
    let repo = Repo::new();
    repo.write("a/x.rs", "fn x() {}\n");
    repo.write("b/y.rs", "fn y() {}\n");
    repo.spec_file(1, "active", &["a/**"], "## Intent\n\nA.\n");
    repo.spec_file(2, "active", &["b/**"], "## Intent\n\nB.\n");
    repo.commit("specs");
    repo
}

fn stop(repo: &Repo, active: bool) -> Run {
    let input = json!({
        "session_id": "s",
        "transcript_path": "/dev/null",
        "cwd": repo.root(),
        "stop_hook_active": active,
    });
    repo.run(&["stop"], &input.to_string())
}

fn touch_both(repo: &Repo) {
    repo.write("a/x.rs", "fn x() { 1; }\n");
    repo.write("b/y.rs", "fn y() { 2; }\n");
}

#[test]
fn change_within_one_governs_area_passes() {
    let repo = two_specs();
    repo.write("a/x.rs", "fn x() { 1; }\n");

    assert_eq!(stop(&repo, false).code, 0);
}

#[test]
fn ungoverned_changes_are_ignored() {
    let repo = two_specs();
    repo.write("a/x.rs", "fn x() { 1; }\n");
    repo.write("scripts/tool.sh", "echo\n");

    assert_eq!(stop(&repo, false).code, 0);
}

#[test]
fn change_across_two_areas_is_flagged_on_stderr() {
    let repo = two_specs();
    touch_both(&repo);

    let run = stop(&repo, false);

    assert_eq!(run.code, 2);
    assert!(
        run.stderr.contains("more than one governs area"),
        "{}",
        run.stderr
    );
    assert!(run.stderr.contains("SPEC-001") && run.stderr.contains("SPEC-002"));
}

#[test]
fn untracked_files_count_toward_the_session_diff() {
    let repo = two_specs();
    repo.write("a/new.rs", "fn n() {}\n");
    repo.write("b/new.rs", "fn m() {}\n");

    assert_eq!(stop(&repo, false).code, 2);
}

#[test]
fn in_progress_delta_covering_both_specs_bounds_the_diff() {
    let repo = two_specs();
    repo.delta_file(
        1,
        "in-progress",
        &["SPEC-001", "SPEC-002"],
        "## Intent\n\nBoth.\n",
    );
    repo.commit("delta");
    touch_both(&repo);

    assert_eq!(stop(&repo, false).code, 0);
}

#[test]
fn delta_covering_only_one_spec_does_not_bound_the_diff() {
    let repo = two_specs();
    repo.delta_file(1, "in-progress", &["SPEC-001"], "## Intent\n\nOne.\n");
    repo.commit("delta");
    touch_both(&repo);

    assert_eq!(stop(&repo, false).code, 2);
}

#[test]
fn draft_delta_does_not_cover() {
    let repo = two_specs();
    repo.delta_file(
        1,
        "draft",
        &["SPEC-001", "SPEC-002"],
        "## Intent\n\nBoth.\n",
    );
    repo.commit("delta");
    touch_both(&repo);

    assert_eq!(stop(&repo, false).code, 2);
}

#[test]
fn stop_hook_active_is_the_escape() {
    let repo = two_specs();
    touch_both(&repo);

    let run = stop(&repo, true);

    assert_eq!(run.code, 0);
    assert!(run.stderr.is_empty());
}

#[test]
fn bare_ignore_is_a_pending_test_but_ignore_with_reason_is_not() {
    let repo = two_specs();
    repo.write(
        "a/x.rs",
        "#[test]\n#[ignore = \"needs network\"]\nfn slow() {}\n",
    );
    assert_eq!(stop(&repo, false).code, 0);

    repo.write("a/x.rs", "#[test]\n#[ignore]\nfn pending() {}\n");
    let run = stop(&repo, false);

    assert_eq!(run.code, 2);
    assert!(
        run.stderr.contains("a/x.rs:2: pending test"),
        "{}",
        run.stderr
    );
}

#[test]
fn it_todo_in_a_touched_file_is_pending() {
    let repo = two_specs();
    repo.write("a/spec.test.ts", "it.todo('later')\n");

    assert_eq!(stop(&repo, false).code, 2);
}

#[test]
fn pending_tests_in_untouched_files_are_ignored() {
    let repo = two_specs();
    repo.write("c/old.rs", "#[ignore]\nfn old() {}\n");
    repo.commit("old pending");
    repo.write("a/x.rs", "fn x() { 1; }\n");

    assert_eq!(stop(&repo, false).code, 0);
}

#[test]
fn outside_a_lazyspec_project_stop_is_silent() {
    let dir = tempfile::tempdir().unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_lemma"))
        .arg("stop")
        .current_dir(dir.path())
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(0));
    assert!(out.stderr.is_empty());
}

#[test]
fn empty_stdin_uses_the_process_cwd() {
    let repo = two_specs();
    touch_both(&repo);

    assert_eq!(repo.run(&["stop"], "").code, 2);
}

#[test]
fn prose_mentioning_ignore_in_markdown_is_not_pending() {
    let repo = two_specs();
    repo.write("a/README.md", "Use `#[ignore]` sparingly.\n#[ignore]\n");
    repo.write("a/notes.txt", "it.todo(x)\n");

    assert_eq!(stop(&repo, false).code, 0);
}

#[test]
fn ignore_attribute_must_start_the_line_in_rust() {
    let repo = two_specs();
    repo.write("a/x.rs", "// see #[ignore] docs\nfn x() {}\n");

    assert_eq!(stop(&repo, false).code, 0);
}

fn complete_delta_setup(repo: &Repo, status: &str) {
    repo.spec_file(1, "active", &["a/**"], "## Intent\n\nA.\n");
    repo.delta_file(1, status, &["SPEC-001"], "## Intent\n\nOne.\n");
    repo.commit("delta");
}

fn set_status(repo: &Repo, from: &str, to: &str) {
    let rel = "docs/deltas/DELTA-001-d/index.md";
    let text = repo
        .read(rel)
        .replace(&format!("status: {from}"), &format!("status: {to}"));
    repo.write(rel, &text);
}

#[test]
fn working_tree_complete_over_head_in_progress_errors() {
    let repo = Repo::new();
    complete_delta_setup(&repo, "in-progress");
    set_status(&repo, "in-progress", "complete");

    let run = stop(&repo, false);

    assert_eq!(run.code, 2);
    assert!(
        run.stderr.contains("DELTA-001 is complete"),
        "{}",
        run.stderr
    );
}

#[test]
fn a_delta_already_complete_at_head_passes() {
    let repo = Repo::new();
    complete_delta_setup(&repo, "complete");

    assert_eq!(stop(&repo, false).code, 0);
}

#[test]
fn a_new_delta_already_complete_in_the_working_tree_errors() {
    let repo = Repo::new();
    repo.spec_file(1, "active", &["a/**"], "## Intent\n\nA.\n");
    repo.commit("spec");
    repo.delta_file(1, "complete", &["SPEC-001"], "## Intent\n\nOne.\n");

    assert_eq!(stop(&repo, false).code, 2);
}
