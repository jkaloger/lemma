mod common;
use common::*;
use serde_json::{json, Value};

const SPEC_CLAIM: &str =
    "- [AC-S1-1] Reopen keeps the prior answer.\n  @ref tests/store_test.rs#reopen_keeps_prior\n";
const SPEC_PART: &str = "docs/specs/SPEC-001-s/behaviour.md";

fn project() -> Repo {
    let repo = Repo::new();
    repo.git(&["branch", "-M", "main"]);
    repo.write("a/x.rs", "fn x() {}\n");
    repo.write("b/y.rs", "fn y() {}\n");
    repo.spec_file(1, "active", &["a/**"], INTENT);
    repo.spec_behaviour(1, SPEC_CLAIM);
    repo.spec_file(2, "active", &["b/**"], INTENT);
    repo.commit("project");
    repo
}

fn check(repo: &Repo, args: &[&str]) -> Run {
    let args: Vec<&str> = ["check"].into_iter().chain(args.iter().copied()).collect();
    repo.run(&args, "")
}

fn touch_both(repo: &Repo) {
    repo.write("a/x.rs", "fn x() { 1; }\n");
    repo.write("b/y.rs", "fn y() { 2; }\n");
}

fn delta(repo: &Repo, n: u32, status: &str, target: &str, claims: &str) {
    repo.delta_file(n, status, &["SPEC-001"], INTENT);
    repo.delta_part(n, target, claims);
}

fn says(run: &Run, needle: &str) -> bool {
    run.stdout.contains(needle)
}

fn lines(run: &Run) -> Vec<&str> {
    run.stdout.lines().collect()
}

fn edit_reopen_body(repo: &Repo) {
    repo.write(
        TEST_FILE,
        "#[test]\nfn reopen_keeps_prior() { assert!(true); }\n\n#[test]\nfn other_test() {}\n",
    );
}

#[test]
fn a_clean_project_is_silent_and_exits_zero() {
    let repo = project();

    let run = check(&repo, &[]);

    assert_eq!(run.code, 0);
    assert_eq!(run.stdout, "");
    assert_eq!(run.stderr, "");
}

#[test]
fn text_output_is_path_line_severity_message() {
    let repo = project();
    edit_reopen_body(&repo);

    let run = check(&repo, &[]);

    assert_eq!(run.code, 2);
    assert_eq!(lines(&run).len(), 1);
    assert!(
        run.stdout
            .starts_with(&format!("{SPEC_PART}:2: error: AC-S1-1: evidence")),
        "{}",
        run.stdout
    );
}

#[test]
fn json_prints_findings_and_exits_two_on_error() {
    let repo = project();
    edit_reopen_body(&repo);

    let run = check(&repo, &["--json"]);

    assert_eq!(run.code, 2);
    let finding = &run.findings()[0];
    assert_eq!(finding["severity"], "error");
    assert_eq!(finding["path"], SPEC_PART);
    assert_eq!(finding["line"], 2);
    assert_eq!(finding["id"], "SPEC-001");
}

#[test]
fn json_on_a_clean_project_has_no_findings_and_exits_zero() {
    let run = check(&project(), &["--json"]);

    assert_eq!(run.code, 0);
    assert_eq!(run.json(), json!({"findings": []}));
}

#[test]
fn json_exits_zero_when_only_warnings_remain() {
    let repo = project();
    repo.spec_behaviour(1, "- [AC-S1-2] Nothing proves this.\n");
    repo.commit("claim without ref");

    let run = check(&repo, &["--json"]);

    assert_eq!(run.code, 0);
    assert_eq!(run.findings()[0]["severity"], "warning");
}

#[test]
fn an_unresolvable_explicit_base_is_an_error() {
    let run = check(&project(), &["--base", "no-such-rev"]);

    assert_eq!(run.code, 2);
    assert!(
        run.stdout.contains("does not resolve to a commit"),
        "{}",
        run.stdout
    );
}

mod coverage {
    use super::*;

    #[test]
    fn a_change_inside_one_area_needs_no_delta() {
        let repo = project();
        repo.write("a/x.rs", "fn x() { 1; }\n");

        assert_eq!(check(&repo, &[]).stdout, "");
    }

    #[test]
    fn changes_across_two_areas_without_a_delta_error_for_each_spec() {
        let repo = project();
        touch_both(&repo);

        let run = check(&repo, &[]);

        assert_eq!(run.code, 2);
        assert_eq!(lines(&run).len(), 2);
        assert!(run
            .stdout
            .contains("docs/specs/SPEC-001-s/index.md: error:"));
        assert!(run
            .stdout
            .contains("docs/specs/SPEC-002-s/index.md: error:"));
        assert!(run
            .stdout
            .contains("create a delta that folds into SPEC-002"));
    }

    #[test]
    fn one_file_governed_by_two_overlapping_specs_is_one_area() {
        let repo = project();
        repo.spec_file(3, "active", &["a/**", "b/**"], INTENT);
        repo.write("a/x.rs", "fn x() { 1; }\n");
        repo.commit("overlapping spec");
        repo.write("a/x.rs", "fn x() { 2; }\n");

        let run = check(&repo, &[]);

        assert_eq!(run.code, 0);
        assert_eq!(run.stdout, "");
    }

    #[test]
    fn files_sharing_a_governing_spec_stay_within_one_area() {
        let repo = project();
        repo.spec_file(3, "active", &["a/**", "b/**"], INTENT);
        repo.commit("overlapping spec");
        touch_both(&repo);

        assert_eq!(check(&repo, &[]).stdout, "");
    }

    #[test]
    fn disjoint_areas_error_for_each_touched_spec() {
        let repo = project();
        repo.write("c/z.rs", "fn z() {}\n");
        repo.spec_file(3, "active", &["c/**"], INTENT);
        repo.commit("third area");
        touch_both(&repo);
        repo.write("c/z.rs", "fn z() { 1; }\n");

        let run = check(&repo, &[]);

        assert_eq!(run.code, 2);
        assert_eq!(lines(&run).len(), 3);
    }

    #[test]
    fn untracked_files_count() {
        let repo = project();
        repo.write("a/new.rs", "fn n() {}\n");
        repo.write("b/new.rs", "fn m() {}\n");

        assert_eq!(check(&repo, &[]).code, 2);
    }

    #[test]
    fn an_in_flight_delta_covers_only_the_spec_it_folds_into() {
        let repo = project();
        delta(
            &repo,
            1,
            "in-progress",
            "SPEC-001",
            "- [AC-D1-1] New.\n  @ref tests/store_test.rs#other_test\n",
        );
        repo.commit("delta");
        touch_both(&repo);

        let run = check(&repo, &[]);

        assert_eq!(run.code, 2);
        assert_eq!(lines(&run).len(), 1);
        assert!(run.stdout.contains("SPEC-002"), "{}", run.stdout);
    }

    #[test]
    fn a_draft_delta_covers() {
        let repo = project();
        repo.delta_file(1, "draft", &["SPEC-001", "SPEC-002"], INTENT);
        repo.commit("delta");
        touch_both(&repo);

        assert_eq!(check(&repo, &[]).code, 0);
    }

    #[test]
    fn a_complete_delta_does_not_cover() {
        let repo = project();
        repo.delta_file(1, "complete", &["SPEC-001", "SPEC-002"], INTENT);
        repo.commit("delta");
        touch_both(&repo);

        assert_eq!(check(&repo, &[]).code, 2);
    }

    #[test]
    fn a_delta_completed_within_the_range_covers_its_specs() {
        let repo = project();
        repo.git(&["checkout", "-q", "-b", "feature"]);
        repo.delta_file(1, "complete", &["SPEC-001", "SPEC-002"], INTENT);
        touch_both(&repo);
        repo.commit("both areas under a completed delta");

        let run = check(&repo, &[]);

        assert_eq!((run.code, run.stdout.as_str()), (0, ""));
    }

    #[test]
    fn a_delta_completed_before_the_range_does_not_cover_it() {
        let repo = project();
        repo.delta_file(1, "complete", &["SPEC-001", "SPEC-002"], INTENT);
        repo.commit("completed earlier");
        repo.git(&["checkout", "-q", "-b", "feature"]);
        touch_both(&repo);
        repo.commit("both areas");

        assert_eq!(check(&repo, &[]).code, 2);
    }

    #[test]
    fn on_a_branch_committed_changes_are_in_range() {
        let repo = project();
        repo.git(&["checkout", "-q", "-b", "feature"]);
        touch_both(&repo);
        repo.commit("both areas");

        assert_eq!(check(&repo, &[]).code, 2);
    }

    #[test]
    fn on_trunk_committed_changes_are_out_of_range_unless_the_base_says_so() {
        let repo = project();
        touch_both(&repo);
        repo.commit("both areas on main");

        assert_eq!(check(&repo, &[]).code, 0);
        assert_eq!(check(&repo, &["--base", "HEAD~1"]).code, 2);
    }

    #[test]
    fn master_is_found_without_a_trunk_flag() {
        let repo = project();
        repo.git(&["branch", "-M", "master"]);
        repo.git(&["checkout", "-q", "-b", "feature"]);
        touch_both(&repo);
        repo.commit("both areas");

        assert_eq!(check(&repo, &[]).code, 2);
    }

    #[test]
    fn trunk_is_configurable() {
        let repo = project();
        repo.git(&["branch", "-M", "develop"]);
        repo.git(&["checkout", "-q", "-b", "feature"]);
        touch_both(&repo);
        repo.commit("both areas");

        assert_eq!(check(&repo, &[]).code, 0);
        assert_eq!(check(&repo, &["--trunk", "develop"]).code, 2);
    }
}

mod evidence_changed {
    use super::*;

    #[test]
    fn editing_a_referenced_test_body_is_an_error_naming_the_claim() {
        let repo = project();
        edit_reopen_body(&repo);

        let run = check(&repo, &[]);

        assert_eq!(run.code, 2);
        assert!(says(
            &run,
            "AC-S1-1: evidence tests/store_test.rs#reopen_keeps_prior changed"
        ));
        assert!(says(&run, "strike AC-S1-1 in a delta"));
    }

    #[test]
    fn editing_a_sibling_test_is_clean() {
        let repo = project();
        repo.write(
            TEST_FILE,
            "#[test]\nfn reopen_keeps_prior() {}\n\n#[test]\nfn other_test() { assert!(true); }\n",
        );

        assert_eq!(check(&repo, &[]).stdout, "");
    }

    #[test]
    fn an_in_flight_delta_striking_the_claim_excuses_the_change() {
        let repo = project();
        delta(
            &repo,
            1,
            "in-progress",
            "SPEC-001",
            "- ~~[AC-S1-1]~~ Reopen keeps the prior answer.\n- [AC-D1-1] Reopen keeps it, again.\n  @ref tests/store_test.rs#reopen_keeps_prior\n",
        );
        edit_reopen_body(&repo);

        assert_eq!(check(&repo, &[]).stdout, "");
    }

    #[test]
    fn a_complete_delta_that_struck_it_before_the_range_does_not_excuse_it() {
        let repo = project();
        delta(&repo, 1, "complete", "SPEC-001", "- ~~[AC-S1-1]~~ gone\n");
        repo.commit("completed earlier");
        edit_reopen_body(&repo);

        assert!(says(&check(&repo, &[]), "AC-S1-1: evidence"));
    }

    #[test]
    fn a_symbol_absent_at_the_base_is_not_a_change() {
        let repo = project();
        repo.spec_behaviour(
            1,
            &format!(
                "{SPEC_CLAIM}- [AC-S1-2] Added later.\n  @ref tests/store_test.rs#other_test\n"
            ),
        );
        repo.write(
            TEST_FILE,
            "#[test]\nfn reopen_keeps_prior() {}\n\n#[test]\nfn renamed_test() {}\n",
        );

        let run = check(&repo, &[]);

        assert!(!says(&run, "AC-S1-1: evidence"));
        assert!(!says(&run, "AC-S1-2: evidence"));
        assert!(says(
            &run,
            "AC-S1-2: ref tests/store_test.rs#other_test does not resolve"
        ));
    }

    #[test]
    fn a_broken_ref_that_landed_on_main_is_not_flagged_by_an_unrelated_branch_edit() {
        let repo = project();
        repo.spec_behaviour(
            1,
            &format!("{SPEC_CLAIM}- [AC-S1-2] Gone.\n  @ref tests/store_test.rs#gone_test\n"),
        );
        repo.commit("broken ref on main");
        repo.git(&["checkout", "-q", "-b", "feature"]);
        repo.write(
            TEST_FILE,
            "#[test]\nfn reopen_keeps_prior() {}\n\n#[test]\nfn other_test() { assert!(true); }\n",
        );
        repo.commit("unrelated edit");

        let run = check(&repo, &[]);

        assert!(!says(&run, "evidence"), "{}", run.stdout);
        assert!(says(
            &run,
            "AC-S1-2: ref tests/store_test.rs#gone_test does not resolve"
        ));
    }

    fn strike_project(status: &str, folds: &[&str], part: &str) -> Repo {
        let repo = project();
        repo.delta_file(1, status, folds, INTENT);
        repo.delta_part(1, part, "- ~~[AC-S1-1]~~ Reopen keeps the prior answer.\n");
        repo.commit("delta");
        edit_reopen_body(&repo);
        repo
    }

    #[test]
    fn a_draft_delta_striking_the_claim_does_not_excuse_the_change() {
        let repo = strike_project("draft", &["SPEC-001"], "SPEC-001");

        assert!(says(&check(&repo, &[]), "AC-S1-1: evidence"));
    }

    #[test]
    fn a_strike_from_a_delta_folding_into_another_spec_does_not_excuse_the_change() {
        let repo = strike_project("in-progress", &["SPEC-002"], "SPEC-001");

        assert!(says(&check(&repo, &[]), "AC-S1-1: evidence"));
    }

    #[test]
    fn a_strike_in_the_part_for_another_spec_does_not_excuse_the_change() {
        let repo = strike_project("in-progress", &["SPEC-001", "SPEC-002"], "SPEC-002");

        assert!(says(&check(&repo, &[]), "AC-S1-1: evidence"));
    }

    #[test]
    fn an_in_progress_delta_folding_into_the_spec_excuses_the_change() {
        let repo = strike_project("in-progress", &["SPEC-001"], "SPEC-001");

        assert!(!says(&check(&repo, &[]), "AC-S1-1: evidence"));
    }

    #[test]
    fn a_delta_that_completed_in_the_range_excuses_the_change() {
        let repo = project();
        repo.git(&["checkout", "-q", "-b", "feature"]);
        delta(&repo, 1, "complete", "SPEC-001", "- ~~[AC-S1-1]~~ gone\n");
        edit_reopen_body(&repo);
        repo.commit("completed in range");

        assert!(!says(&check(&repo, &[]), "AC-S1-1: evidence"));
    }

    #[test]
    fn a_ref_deleted_from_the_working_tree_is_changed() {
        let repo = project();
        repo.write(TEST_FILE, "#[test]\nfn other_test() {}\n");

        let run = check(&repo, &[]);

        assert!(says(&run, "AC-S1-1: evidence"));
        assert!(says(
            &run,
            "AC-S1-1: ref tests/store_test.rs#reopen_keeps_prior does not resolve"
        ));
    }

    #[test]
    fn the_base_comes_from_the_merge_base_on_a_branch() {
        let repo = project();
        repo.git(&["checkout", "-q", "-b", "feature"]);
        edit_reopen_body(&repo);
        repo.commit("edit the test");

        assert_eq!(check(&repo, &[]).code, 2);
        assert_eq!(check(&repo, &["--base", "HEAD"]).code, 0);
    }

    #[test]
    fn on_trunk_with_a_clean_tree_nothing_is_compared() {
        let repo = project();
        edit_reopen_body(&repo);
        repo.commit("edit on main");

        assert_eq!(check(&repo, &[]).stdout, "");
    }
}

mod claims_have_evidence {
    use super::*;

    fn without_ref(repo: &Repo) {
        repo.spec_behaviour(
            1,
            &format!("{SPEC_CLAIM}- [AC-S1-2] Nothing proves this.\n"),
        );
        repo.commit("claim without ref");
    }

    #[test]
    fn a_claim_without_a_ref_is_a_warning() {
        let repo = project();
        without_ref(&repo);

        let run = check(&repo, &[]);

        assert_eq!(run.code, 0);
        assert!(
            run.stdout.starts_with(&format!(
                "{SPEC_PART}:3: warning: AC-S1-2: claim has no `@ref`"
            )),
            "{}",
            run.stdout
        );
    }

    #[test]
    fn require_evidence_makes_it_an_error() {
        let repo = project();
        without_ref(&repo);

        let run = check(&repo, &["--require", "evidence"]);

        assert_eq!(run.code, 2);
        assert!(run.stdout.contains(": error: AC-S1-2: claim has no `@ref`"));
    }

    fn intent_project() -> Repo {
        let repo = project();
        repo.spec_behaviour(
            1,
            &format!("{SPEC_CLAIM}- [AC-S1-2] [intent] Feels fast.\n"),
        );
        repo.commit("intent");
        repo
    }

    #[test]
    fn an_intent_claim_is_exempt_and_text_output_stays_silent() {
        let run = check(&intent_project(), &["--require", "evidence"]);

        assert_eq!(run.code, 0);
        assert_eq!(run.stdout, "");
    }

    #[test]
    fn the_intent_count_is_not_reported_in_any_output_mode() {
        let repo = intent_project();
        let payload = validate_payload(vec![with_parts(
            doc("SPEC-001", "spec", "active", &["a/**"], &[], INTENT),
            &[("behaviour", &repo.read(SPEC_PART))],
        )]);

        let json = check(&repo, &["--json"]);
        let hook = repo.run(&["check", "--hook"], &payload);

        assert!(!json.has_message("[intent]"), "{}", json.stdout);
        assert!(!hook.has_message("[intent]"), "{}", hook.stdout);
    }

    #[test]
    fn an_unresolved_ref_is_always_an_error() {
        let repo = project();
        repo.spec_behaviour(
            1,
            "- [AC-S1-1] x\n  @ref tests/store_test.rs#missing_symbol\n",
        );
        repo.commit("bad symbol");

        let run = check(&repo, &[]);

        assert_eq!(run.code, 2);
        assert!(run
            .stdout
            .contains("AC-S1-1: ref tests/store_test.rs#missing_symbol does not resolve"));
        assert!(run.stdout.contains("lemma refs AC-S1-1"));
    }

    #[test]
    fn a_ref_into_production_code_is_always_an_error() {
        let repo = project();
        repo.spec_behaviour(1, "- [AC-S1-1] x\n  @ref src/lib.rs#a\n");
        repo.commit("not a test");

        let run = check(&repo, &[]);

        assert_eq!(run.code, 2);
        assert!(says(
            &run,
            "AC-S1-1: ref src/lib.rs#a is not in the governing spec's `tests` globs"
        ));
    }

    #[test]
    fn the_governing_specs_tests_globs_decide() {
        let repo = project();
        repo.spec_file_with_tests(1, "active", &["a/**"], &["a/**"], INTENT);
        repo.spec_behaviour(1, "- [AC-S1-1] x\n  @ref a/x.rs#x\n");
        repo.commit("custom tests glob");

        assert_eq!(check(&repo, &["--require", "evidence"]).stdout, "");
    }

    #[test]
    fn in_progress_delta_claims_are_checked_and_draft_ones_are_not() {
        let repo = project();
        delta(&repo, 1, "draft", "SPEC-001", "- [AC-D1-1] Planned.\n");
        repo.commit("draft delta");
        assert_eq!(check(&repo, &["--require", "evidence"]).stdout, "");

        delta(
            &repo,
            1,
            "in-progress",
            "SPEC-001",
            "- [AC-D1-1] Planned.\n",
        );
        repo.commit("in progress");
        let run = check(&repo, &["--require", "evidence"]);

        assert_eq!(run.code, 2);
        assert!(run
            .stdout
            .contains("docs/deltas/DELTA-001-d/behaviour.SPEC-001.md:1: error: AC-D1-1"));
    }
}

mod integrity {
    use super::*;

    #[test]
    fn an_invalid_glob_is_an_error() {
        let repo = project();
        repo.spec_file_with_tests(1, "active", &["a/**"], &["[bad"], INTENT);
        repo.spec_behaviour(1, "");

        let run = check(&repo, &[]);

        assert_eq!(run.code, 2);
        assert!(says(&run, "tests glob `[bad` is invalid"));
    }
}

mod require {
    use super::*;

    #[test]
    fn intent_needs_intent_but_no_claim() {
        let repo = project();
        repo.delta_file(1, "draft", &["SPEC-001"], "## Intent\n\n");

        let lax = check(&repo, &[]);
        let strict = check(&repo, &["--require", "intent"]);

        assert_eq!(lax.code, 0);
        assert_eq!(strict.code, 2);
        assert!(says(&strict, "missing or empty `## Intent` section"));
        assert!(!says(&strict, "no claims"));
    }

    #[test]
    fn intent_is_satisfied_by_intent_and_an_empty_part() {
        let repo = project();
        delta(&repo, 1, "draft", "SPEC-001", "");

        assert_eq!(check(&repo, &["--require", "intent"]).stdout, "");
    }

    #[test]
    fn evidence_does_not_imply_claims() {
        let repo = project();
        repo.delta_file(1, "draft", &["SPEC-001"], "## Intent\n\n");

        assert_eq!(check(&repo, &["--require", "evidence"]).code, 0);
    }

    fn in_progress_delta(repo: &Repo) {
        delta(
            repo,
            1,
            "in-progress",
            "SPEC-001",
            "- [AC-D1-1] New.\n  @ref tests/store_test.rs#other_test\n",
        );
        repo.commit("delta");
    }

    #[test]
    fn clean_flags_uncommitted_changes_in_ref_files() {
        let repo = project();
        in_progress_delta(&repo);
        repo.write(
            TEST_FILE,
            "#[test]\nfn reopen_keeps_prior() {}\n\n#[test]\nfn other_test() { assert!(true); }\n",
        );

        let lax = check(&repo, &[]);
        let strict = check(&repo, &["--require", "clean"]);

        assert_eq!(lax.code, 0);
        assert_eq!(strict.code, 2);
        assert!(strict
            .stdout
            .contains("tests/store_test.rs: error: tests/store_test.rs has uncommitted changes"));
    }

    #[test]
    fn clean_flags_uncommitted_changes_under_a_targets_governs() {
        let repo = project();
        in_progress_delta(&repo);
        repo.write("a/x.rs", "fn x() { 1; }\n");

        let run = check(&repo, &["--require", "clean"]);

        assert_eq!(run.code, 2);
        assert!(says(&run, "a/x.rs has uncommitted changes"));
    }

    #[test]
    fn clean_ignores_changes_outside_the_targets() {
        let repo = project();
        in_progress_delta(&repo);
        repo.write("b/y.rs", "fn y() { 1; }\n");

        assert_eq!(check(&repo, &["--require", "clean"]).stdout, "");
    }

    #[test]
    fn require_is_repeatable() {
        let repo = project();
        in_progress_delta(&repo);
        repo.write("a/x.rs", "fn x() { 1; }\n");
        repo.spec_behaviour(
            1,
            &format!("{SPEC_CLAIM}- [AC-S1-2] Nothing proves this.\n"),
        );

        let run = check(&repo, &["--require", "evidence", "--require", "clean"]);

        assert!(says(&run, "a/x.rs has uncommitted changes"));
        assert!(run.stdout.contains(": error: AC-S1-2: claim has no `@ref`"));
    }
}

mod hook {
    use super::*;

    fn delta_doc(status: &str, claims: &str) -> Value {
        with_parts(
            doc("DELTA-001", "delta", status, &[], &["SPEC-001"], INTENT),
            &[("behaviour.SPEC-001", claims)],
        )
    }

    fn spec_doc() -> Value {
        let mut spec = doc("SPEC-001", "spec", "active", &["a/**"], &[], INTENT);
        spec["path"] = json!("docs/specs/SPEC-001-s/index.md");
        with_parts(spec, &[("behaviour", SPEC_CLAIM)])
    }

    #[test]
    fn it_exits_zero_and_prints_findings_json_even_with_errors() {
        let repo = project();
        let payload = validate_payload(vec![delta_doc(
            "in-progress",
            "- [AC-D1-1] Retries.\n  @ref tests/store_test.rs#missing\n",
        )]);

        let run = repo.run(&["check", "--hook"], &payload);

        assert_eq!(run.code, 0);
        assert_eq!(run.findings().len(), 1);
        let finding = &run.findings()[0];
        assert_eq!(finding["severity"], "error");
        assert_eq!(finding["id"], "DELTA-001");
        assert_eq!(finding["part"], "behaviour.SPEC-001");
        assert_eq!(finding["line"], 2);
        assert!(finding["message"]
            .as_str()
            .unwrap()
            .contains("does not resolve"));
    }

    #[test]
    fn delta_checks_are_scoped_to_the_payload_documents() {
        let repo = project();
        delta(
            &repo,
            2,
            "draft",
            "SPEC-001",
            "- [AC-S1-1] Changed wording.\n",
        );
        let payload = validate_payload(vec![delta_doc("draft", "- [AC-D1-1] Fine.\n")]);

        let run = repo.run(&["check", "--hook"], &payload);

        assert_eq!(run.code, 0);
        assert_eq!(run.findings(), Vec::<Value>::new());
    }

    #[test]
    fn repo_wide_findings_are_not_attributed_to_payload_documents() {
        let repo = project();
        edit_reopen_body(&repo);
        let payload = transition_payload(
            "draft",
            "in-progress",
            delta_doc("draft", "- [AC-D1-1] Fine.\n"),
            vec![spec_doc()],
        );

        let run = repo.run(&["check", "--hook"], &payload);

        assert_eq!(run.code, 0);
        let finding = run
            .findings()
            .into_iter()
            .find(|f| f["message"].as_str().unwrap().contains("AC-S1-1: evidence"))
            .unwrap();
        assert!(finding.get("id").is_none());
        assert!(finding["message"]
            .as_str()
            .unwrap()
            .starts_with(&format!("{SPEC_PART}: AC-S1-1: evidence")));
    }

    #[test]
    fn require_flags_apply_to_the_payload_delta() {
        let repo = project();
        let payload = transition_payload(
            "in-progress",
            "complete",
            delta_doc("in-progress", "- [AC-D1-1] No ref yet.\n"),
            vec![spec_doc()],
        );

        let lax = repo.run(&["check", "--hook"], &payload);
        let strict = repo.run(&["check", "--hook", "--require", "evidence"], &payload);

        assert_eq!(lax.findings()[0]["severity"], "warning");
        assert_eq!(strict.findings()[0]["severity"], "error");
        assert_eq!(strict.code, 0);
    }

    #[test]
    fn on_trunk_the_gate_uses_the_parent_of_the_deltas_first_commit() {
        let repo = project();
        delta(
            &repo,
            1,
            "in-progress",
            "SPEC-001",
            "- [AC-D1-1] New.\n  @ref tests/store_test.rs#other_test\n",
        );
        repo.commit("delta");
        edit_reopen_body(&repo);
        repo.commit("edit the evidence");
        let mut moved = delta_doc(
            "in-progress",
            "- [AC-D1-1] New.\n  @ref tests/store_test.rs#other_test\n",
        );
        moved["path"] = json!("docs/deltas/DELTA-001-d/index.md");
        let payload = transition_payload("in-progress", "complete", moved, vec![spec_doc()]);

        let gate = repo.run(&["check", "--hook"], &payload);
        let plain = check(&repo, &[]);

        assert!(gate.has_message("AC-S1-1: evidence"), "{}", gate.stdout);
        assert_eq!(plain.stdout, "");
    }

    #[test]
    fn the_moved_deltas_own_strikes_count_at_the_draft_gate() {
        let repo = project();
        edit_reopen_body(&repo);
        let mut moved = delta_doc("draft", "- ~~[AC-S1-1]~~ Reopen keeps the prior answer.\n");
        moved["path"] = json!("docs/deltas/DELTA-001-d/index.md");
        let payload = transition_payload("draft", "in-progress", moved, vec![spec_doc()]);

        let gate = repo.run(&["check", "--hook", "--require", "intent"], &payload);

        assert!(!gate.has_message("AC-S1-1: evidence"), "{}", gate.stdout);
    }

    fn moved_delta(status: &str) -> Value {
        let mut moved = delta_doc(
            status,
            "- [AC-D1-1] New.\n  @ref tests/store_test.rs#other_test\n",
        );
        moved["path"] = json!("docs/deltas/DELTA-001-d/index.md");
        moved
    }

    #[test]
    fn the_validate_event_skips_the_range_checks() {
        let repo = project();
        repo.delta_file(1, "complete", &["SPEC-001", "SPEC-002"], INTENT);
        touch_both(&repo);
        repo.commit("both areas under a completed delta");
        edit_reopen_body(&repo);
        let payload = validate_payload(vec![moved_delta("complete")]);

        let run = repo.run(&["check", "--hook"], &payload);

        assert_eq!(run.code, 0);
        assert!(!run.has_message("governed area"), "{}", run.stdout);
        assert!(!run.has_message("evidence"), "{}", run.stdout);
    }

    #[test]
    fn a_transition_limits_checks_one_and_two_to_the_moved_deltas_targets() {
        let repo = project();
        repo.write("b/tests/t_test.rs", "#[test]\nfn t() {}\n");
        repo.spec_behaviour(2, "- [AC-S2-1] B.\n  @ref b/tests/t_test.rs#t\n");
        repo.commit("spec 2 evidence");
        touch_both(&repo);
        repo.write("b/tests/t_test.rs", "#[test]\nfn t() { assert!(true); }\n");
        let payload = transition_payload(
            "in-progress",
            "complete",
            moved_delta("in-progress"),
            vec![spec_doc()],
        );

        let gate = repo.run(&["check", "--hook"], &payload);
        let plain = check(&repo, &[]);

        assert_eq!(gate.findings(), Vec::<Value>::new(), "{}", gate.stdout);
        assert!(says(&plain, "AC-S2-1: evidence"));
        assert!(says(&plain, "create a delta that folds into SPEC-002"));
    }

    #[test]
    fn an_invalid_payload_is_reported_with_exit_zero() {
        let run = project().run(&["check", "--hook"], "not json");

        assert_eq!(run.code, 0);
        assert!(run.has_message("invalid hook payload"));
    }
}

mod trunk {
    use super::*;

    #[test]
    fn merging_an_advanced_remote_trunk_into_a_branch_is_clean() {
        let repo = project();
        repo.git(&["checkout", "-q", "-b", "upstream"]);
        touch_both(&repo);
        repo.commit("trunk work in two areas");
        repo.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
        repo.git(&["checkout", "-q", "main"]);
        repo.git(&["checkout", "-q", "-b", "feature"]);
        repo.write("c.txt", "c");
        repo.commit("branch work");
        repo.git(&["merge", "-q", "--no-ff", "-m", "merge trunk", "origin/main"]);

        let run = check(&repo, &[]);

        assert_eq!((run.code, run.stdout.as_str()), (0, ""));
    }

    #[test]
    fn a_plain_branch_still_ranges_from_the_merge_base() {
        let repo = project();
        repo.git(&["checkout", "-q", "-b", "feature"]);
        touch_both(&repo);
        repo.commit("two areas");

        assert!(says(&check(&repo, &[]), "governed area"));
    }
}
