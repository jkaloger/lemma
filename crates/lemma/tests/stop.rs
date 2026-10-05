mod common;
use common::*;
use serde_json::json;

const CLAIM: &str =
    "- [AC-S1-1] Reopen keeps the prior answer.\n  @ref tests/store_test.rs#reopen_keeps_prior\n";

fn two_specs() -> Repo {
    let repo = Repo::new();
    repo.git(&["branch", "-M", "main"]);
    repo.write("a/x.rs", "fn x() {}\n");
    repo.write("b/y.rs", "fn y() {}\n");
    repo.spec_file(1, "active", &["a/**"], "## Intent\n\nA.\n");
    repo.spec_behaviour(1, CLAIM);
    repo.spec_file(2, "active", &["b/**"], "## Intent\n\nB.\n");
    repo.commit("specs");
    repo
}

fn stop_with(repo: &Repo, input: serde_json::Value) -> Run {
    repo.run(&["stop"], &input.to_string())
}

fn stop(repo: &Repo) -> Run {
    stop_with(repo, json!({ "session_id": "s", "cwd": repo.root() }))
}

fn touch_both(repo: &Repo) {
    repo.write("a/x.rs", "fn x() { 1; }\n");
    repo.write("b/y.rs", "fn y() { 2; }\n");
}

fn session_start(repo: &Repo, id: &str) -> Run {
    let input = json!({ "session_id": id, "cwd": repo.root(), "source": "startup" });
    repo.run(&["session-start"], &input.to_string())
}

fn complete_delta_setup(repo: &Repo, status: &str) {
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

mod on_a_branch {
    use super::*;

    fn branch(repo: &Repo) {
        repo.git(&["checkout", "-q", "-b", "feature"]);
    }

    #[test]
    fn a_clean_branch_is_silent() {
        let repo = two_specs();
        branch(&repo);

        let run = stop(&repo);

        assert_eq!(
            (run.code, run.stdout.as_str(), run.stderr.as_str()),
            (0, "", "")
        );
    }

    #[test]
    fn a_change_within_one_area_passes() {
        let repo = two_specs();
        branch(&repo);
        repo.write("a/x.rs", "fn x() { 1; }\n");
        repo.commit("one area");

        assert_eq!(stop(&repo).code, 0);
    }

    #[test]
    fn committed_changes_across_two_areas_are_errors_in_check_format() {
        let repo = two_specs();
        branch(&repo);
        touch_both(&repo);
        repo.commit("both");

        let run = stop(&repo);

        assert_eq!(run.code, 2);
        assert_eq!(run.stdout, "");
        assert!(
            run.stderr
                .contains("docs/specs/SPEC-001-s/index.md: error: changes to"),
            "{}",
            run.stderr
        );
        assert!(run
            .stderr
            .contains("create a delta that folds into SPEC-002"));
    }

    #[test]
    fn an_in_flight_delta_covering_both_specs_bounds_the_change() {
        let repo = two_specs();
        branch(&repo);
        repo.delta_file(
            1,
            "in-progress",
            &["SPEC-001", "SPEC-002"],
            "## Intent\n\nBoth.\n",
        );
        repo.commit("delta");
        touch_both(&repo);

        assert_eq!(stop(&repo).code, 0);
    }

    #[test]
    fn a_delta_covering_one_spec_leaves_the_other_an_error() {
        let repo = two_specs();
        branch(&repo);
        repo.delta_file(1, "draft", &["SPEC-001"], "## Intent\n\nOne.\n");
        repo.commit("delta");
        touch_both(&repo);

        let run = stop(&repo);

        assert_eq!(run.code, 2);
        assert_eq!(run.stderr.lines().count(), 1);
        assert!(run.stderr.contains("SPEC-002"), "{}", run.stderr);
    }

    #[test]
    fn a_changed_evidence_region_is_an_error_naming_the_claim() {
        let repo = two_specs();
        branch(&repo);
        repo.write(
            TEST_FILE,
            "#[test]\nfn reopen_keeps_prior() { assert!(true); }\n\n#[test]\nfn other_test() {}\n",
        );

        let run = stop(&repo);

        assert_eq!(run.code, 2);
        assert!(run.stderr.contains("AC-S1-1: evidence"), "{}", run.stderr);
    }

    #[test]
    fn an_unresolved_ref_is_an_error_but_a_claim_without_a_ref_is_not() {
        let repo = two_specs();
        branch(&repo);
        repo.spec_behaviour(1, "- [AC-S1-2] Nothing proves this.\n");
        assert_eq!(stop(&repo).code, 0);
        assert_eq!(stop(&repo).stderr, "");

        repo.spec_behaviour(1, "- [AC-S1-2] Gone.\n  @ref tests/store_test.rs#missing\n");
        let run = stop(&repo);

        assert_eq!(run.code, 2);
        assert!(run.stderr.contains("does not resolve"), "{}", run.stderr);
    }

    #[test]
    fn a_broken_ref_in_an_untouched_spec_is_silent() {
        let repo = two_specs();
        repo.spec_behaviour(2, "- [AC-S2-1] Gone.\n  @ref b/y.rs#y\n");
        repo.commit("pre-existing broken ref");
        branch(&repo);
        repo.write("a/x.rs", "fn x() { 1; }\n");

        let run = stop(&repo);

        assert_eq!((run.code, run.stderr.as_str()), (0, ""));
    }

    #[test]
    fn a_broken_ref_in_a_touched_delta_is_an_error() {
        let repo = two_specs();
        branch(&repo);
        repo.delta_file(1, "in-progress", &["SPEC-001"], "## Intent\n\nOne.\n");
        repo.delta_part(
            1,
            "SPEC-001",
            "- [AC-D1-1] Gone.\n  @ref tests/store_test.rs#missing\n",
        );

        let run = stop(&repo);

        assert_eq!(run.code, 2);
        assert!(run.stderr.contains("does not resolve"), "{}", run.stderr);
    }

    #[test]
    fn a_broken_ref_in_a_spec_governing_a_touched_file_is_an_error() {
        let repo = two_specs();
        repo.spec_behaviour(2, "- [AC-S2-1] Gone.\n  @ref b/y.rs#y\n");
        repo.commit("pre-existing broken ref");
        branch(&repo);
        repo.write("b/y.rs", "fn y() { 2; }\n");

        let run = stop(&repo);

        assert_eq!(run.code, 2);
        assert!(
            run.stderr.contains("is not in the governing spec"),
            "{}",
            run.stderr
        );
    }

    #[test]
    fn integrity_findings_are_left_to_check() {
        let repo = two_specs();
        branch(&repo);
        repo.spec_behaviour(
            1,
            "- [AC-S1-2] Pinned.\n  @ref tests/store_test.rs#reopen_keeps_prior@3f9c2e1\n",
        );

        assert_eq!(stop(&repo).code, 0);
        assert_eq!(repo.run(&["check"], "").code, 2);
    }

    #[test]
    fn pending_tests_are_no_longer_detected() {
        let repo = two_specs();
        branch(&repo);
        repo.write("a/x.rs", "#[test]\n#[ignore]\nfn pending() {}\n");

        assert_eq!(stop(&repo).code, 0);
    }
}

mod on_trunk {
    use super::*;

    #[test]
    fn committed_changes_before_the_session_are_out_of_range() {
        let repo = two_specs();
        touch_both(&repo);
        repo.commit("both, no session recorded");

        assert_eq!(stop(&repo).code, 0);
    }

    #[test]
    fn the_session_base_brings_commits_made_in_the_session_into_range() {
        let repo = two_specs();
        session_start(&repo, "s");
        touch_both(&repo);
        repo.commit("both");

        let run = stop(&repo);

        assert_eq!(run.code, 2);
        assert!(
            run.stderr.contains("more than one governed area"),
            "{}",
            run.stderr
        );
    }

    #[test]
    fn uncommitted_and_untracked_files_count() {
        let repo = two_specs();
        session_start(&repo, "s");
        repo.write("a/new.rs", "fn n() {}\n");
        repo.write("b/new.rs", "fn m() {}\n");

        assert_eq!(stop(&repo).code, 2);
    }

    #[test]
    fn a_session_that_stays_in_one_area_passes() {
        let repo = two_specs();
        session_start(&repo, "s");
        repo.write("a/x.rs", "fn x() { 1; }\n");
        repo.commit("one area");

        assert_eq!(stop(&repo).code, 0);
    }
}

mod hook_input {
    use super::*;

    #[test]
    fn stop_hook_active_exits_zero_without_checking() {
        let repo = two_specs();
        touch_both(&repo);

        let run = stop_with(
            &repo,
            json!({ "session_id": "s", "cwd": repo.root(), "stop_hook_active": true }),
        );

        assert_eq!((run.code, run.stderr.as_str()), (0, ""));
    }

    #[test]
    fn stop_hook_active_may_be_absent_or_false() {
        let repo = two_specs();
        repo.git(&["checkout", "-q", "-b", "feature"]);
        touch_both(&repo);

        let absent = stop_with(&repo, json!({ "session_id": "s", "cwd": repo.root() }));
        let falsy = stop_with(
            &repo,
            json!({ "session_id": "s", "cwd": repo.root(), "stop_hook_active": false }),
        );

        assert_eq!((absent.code, falsy.code), (2, 2));
    }

    #[test]
    fn empty_stdin_uses_the_process_cwd() {
        let repo = two_specs();
        repo.git(&["checkout", "-q", "-b", "feature"]);
        touch_both(&repo);

        assert_eq!(repo.run(&["stop"], "").code, 2);
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
}

mod completion {
    use super::*;

    #[test]
    fn working_tree_complete_over_head_in_progress_errors() {
        let repo = two_specs();
        complete_delta_setup(&repo, "in-progress");
        set_status(&repo, "in-progress", "complete");

        let run = stop(&repo);

        assert_eq!(run.code, 2);
        assert!(
            run.stderr.contains("DELTA-001 is complete"),
            "{}",
            run.stderr
        );
    }

    #[test]
    fn a_delta_already_complete_at_head_passes() {
        let repo = two_specs();
        complete_delta_setup(&repo, "complete");

        assert_eq!(stop(&repo).code, 0);
    }

    #[test]
    fn a_new_delta_already_complete_in_the_working_tree_errors() {
        let repo = two_specs();
        repo.delta_file(1, "complete", &["SPEC-001"], "## Intent\n\nOne.\n");

        assert_eq!(stop(&repo).code, 2);
    }

    #[test]
    fn a_delta_completed_and_committed_in_the_session_passes() {
        let repo = two_specs();
        complete_delta_setup(&repo, "in-progress");
        session_start(&repo, "s");
        set_status(&repo, "in-progress", "complete");
        repo.commit("complete it");

        assert_eq!(stop(&repo).code, 0);
    }
}

mod session_start_hook {
    use super::*;

    fn base_file(repo: &Repo, id: &str) -> std::path::PathBuf {
        repo.path(&format!(".git/lemma/sessions/{id}"))
    }

    #[test]
    fn records_the_base_once() {
        let repo = two_specs();
        let first = repo.git(&["rev-parse", "HEAD"]);

        let run = session_start(&repo, "s");
        assert_eq!(run.code, 0);
        assert!(run.stdout.is_empty() && run.stderr.is_empty());
        assert_eq!(
            std::fs::read_to_string(base_file(&repo, "s")).unwrap(),
            first
        );

        repo.write("a/x.rs", "fn x() { 1; }\n");
        repo.commit("later");
        session_start(&repo, "s");

        assert_eq!(
            std::fs::read_to_string(base_file(&repo, "s")).unwrap(),
            first
        );
    }

    #[test]
    fn ignores_unsafe_session_ids() {
        let repo = two_specs();

        let run = session_start(&repo, "../escape");

        assert_eq!(run.code, 0);
        assert!(!repo.path(".git/lemma/escape").exists());
    }

    fn run_outside_a_project(dir: &std::path::Path) -> std::process::Output {
        let input = json!({ "session_id": "s", "cwd": dir }).to_string();
        let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_lemma"))
            .arg("session-start")
            .current_dir(dir)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        std::io::Write::write_all(&mut child.stdin.take().unwrap(), input.as_bytes()).unwrap();
        child.wait_with_output().unwrap()
    }

    #[test]
    fn is_silent_outside_a_repo() {
        let dir = tempfile::tempdir().unwrap();

        let out = run_outside_a_project(dir.path());

        assert_eq!(out.status.code(), Some(0));
        assert!(out.stdout.is_empty() && out.stderr.is_empty());
    }

    #[test]
    fn writes_no_base_in_a_repo_without_a_lazyspec_project() {
        let dir = tempfile::tempdir().unwrap();
        let git = |args: &[&str]| {
            let out = std::process::Command::new("git")
                .arg("-C")
                .arg(dir.path())
                .args(args)
                .output()
                .unwrap();
            assert!(out.status.success());
        };
        git(&["init", "-q"]);
        git(&[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "init",
        ]);

        let out = run_outside_a_project(dir.path());

        assert_eq!(out.status.code(), Some(0));
        assert!(!dir.path().join(".git/lemma/sessions/s").exists());
    }
}
