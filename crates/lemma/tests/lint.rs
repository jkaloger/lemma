mod common;
use common::*;
use serde_json::{json, Value};

fn project() -> Repo {
    let repo = Repo::new();
    repo.git(&["branch", "-M", "main"]);
    repo.write("a/x.rs", "fn x() {}\n");
    repo.write("b/y.rs", "fn y() {}\n");
    repo.spec_file(1, "active", &["a/**"], INTENT);
    repo.landed_delta(LANDED_CLAIM);
    repo.spec_file(2, "active", &["b/**"], INTENT);
    repo.commit("project");
    repo
}

fn check(repo: &Repo, args: &[&str]) -> Run {
    let args: Vec<&str> = ["lint"].into_iter().chain(args.iter().copied()).collect();
    repo.run(&args, "")
}

fn touch_both(repo: &Repo) {
    repo.write("a/x.rs", "fn x() { 1; }\n");
    repo.write("b/y.rs", "fn y() { 2; }\n");
}

fn delta(repo: &Repo, n: u32, status: &str, claims: &str) {
    repo.delta_file(n, status, &["SPEC-001"], INTENT);
    repo.delta_claims(n, claims);
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

    assert_eq!(run.code, 0);
    assert_eq!(
        lines(&run),
        vec![format!("{LANDED_CLAIMS}:2: warning: {HISTORY}")]
    );
}

#[test]
fn json_prints_findings_and_exits_two_on_error() {
    let repo = project();
    touch_both(&repo);

    let run = check(&repo, &["--json"]);

    assert_eq!(run.code, 2);
    let finding = &run.findings()[0];
    assert_eq!(finding["severity"], "error");
    assert_eq!(finding["path"], "docs/specs/SPEC-001-s/index.md");
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
    edit_reopen_body(&repo);

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
        assert!(run.stdout.contains("create a delta that changes SPEC-002"));
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
    fn an_in_flight_delta_covers_only_the_spec_it_changes() {
        let repo = project();
        delta(
            &repo,
            1,
            "in-progress",
            "- [AC-1] New.\n  @ref tests/store_test.rs#other_test\n",
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

mod history {
    use super::*;

    #[test]
    fn editing_a_test_a_landed_delta_claims_warns_and_names_it() {
        let repo = project();
        edit_reopen_body(&repo);

        let run = check(&repo, &[]);

        assert_eq!(run.code, 0);
        assert_eq!(
            lines(&run),
            vec![format!("{LANDED_CLAIMS}:2: warning: {HISTORY}")]
        );
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
    fn every_landed_delta_claiming_the_test_is_named() {
        let repo = project();
        delta(
            &repo,
            1,
            "complete",
            "- [AC-3] Again.\n  @ref tests/store_test.rs#reopen_keeps_prior\n",
        );
        repo.commit("a later delta claims the same test");
        edit_reopen_body(&repo);

        let run = check(&repo, &[]);

        assert!(says(&run, "DELTA-001/AC-3 claims it"), "{}", run.stdout);
        assert!(says(&run, "DELTA-009/AC-1 claims it"), "{}", run.stdout);
    }

    #[test]
    fn claims_of_in_flight_deltas_are_not_history() {
        let repo = project();
        delta(
            &repo,
            1,
            "in-progress",
            "- [AC-1] Other.\n  @ref tests/store_test.rs#other_test\n",
        );
        repo.commit("in flight");
        repo.write(
            TEST_FILE,
            "#[test]\nfn reopen_keeps_prior() {}\n\n#[test]\nfn other_test() { assert!(true); }\n",
        );

        assert_eq!(check(&repo, &[]).stdout, "");
    }

    #[test]
    fn a_delta_that_completed_in_the_range_is_not_history() {
        let repo = project();
        repo.git(&["checkout", "-q", "-b", "feature"]);
        delta(
            &repo,
            1,
            "complete",
            "- [AC-1] Other.\n  @ref tests/store_test.rs#other_test\n",
        );
        repo.write(
            TEST_FILE,
            "#[test]\nfn reopen_keeps_prior() {}\n\n#[test]\nfn other_test() { assert!(true); }\n",
        );
        repo.commit("completed in range");

        assert_eq!(check(&repo, &[]).stdout, "");
    }

    #[test]
    fn a_symbol_absent_at_the_base_is_not_a_change() {
        let repo = project();
        delta(
            &repo,
            1,
            "complete",
            "- [AC-1] Later.\n  @ref tests/store_test.rs#renamed_test\n",
        );
        repo.commit("landed with a broken ref");
        repo.write(
            TEST_FILE,
            "#[test]\nfn reopen_keeps_prior() {}\n\n#[test]\nfn renamed_test() {}\n",
        );

        assert_eq!(check(&repo, &[]).stdout, "");
    }

    #[test]
    fn a_ref_deleted_from_the_working_tree_is_changed() {
        let repo = project();
        repo.write(TEST_FILE, "#[test]\nfn other_test() {}\n");

        assert!(says(&check(&repo, &[]), HISTORY));
    }

    #[test]
    fn the_base_comes_from_the_merge_base_on_a_branch() {
        let repo = project();
        repo.git(&["checkout", "-q", "-b", "feature"]);
        edit_reopen_body(&repo);
        repo.commit("edit the test");

        assert!(says(&check(&repo, &[]), HISTORY));
        assert_eq!(check(&repo, &["--base", "HEAD"]).stdout, "");
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

    fn lint_delta(repo: &Repo) -> Run {
        check(repo, &["DELTA-001"])
    }

    #[test]
    fn an_unresolved_ref_is_always_an_error() {
        let repo = project();
        delta(
            &repo,
            1,
            "in-progress",
            "- [AC-1] x\n  @ref tests/store_test.rs#missing_symbol\n",
        );

        let run = lint_delta(&repo);

        assert_eq!(run.code, 2);
        assert!(says(
            &run,
            "AC-1: ref tests/store_test.rs#missing_symbol does not resolve"
        ));
        assert!(says(&run, "lemma refs DELTA-001/AC-1"));
    }

    #[test]
    fn a_ref_into_production_code_is_always_an_error() {
        let repo = project();
        delta(&repo, 1, "in-progress", "- [AC-1] x\n  @ref src/lib.rs#a\n");

        let run = lint_delta(&repo);

        assert_eq!(run.code, 2);
        assert!(says(
            &run,
            "AC-1: ref src/lib.rs#a is not in the governing spec's `tests` globs"
        ));
    }

    #[test]
    fn the_changed_specs_tests_globs_decide() {
        let repo = project();
        repo.spec_file_with_tests(2, "active", &["b/**"], &["b/**"], INTENT);
        repo.delta_file(1, "in-progress", &["SPEC-001", "SPEC-002"], INTENT);
        repo.delta_claims(
            1,
            "- [AC-1] x\n  @ref b/y.rs#y\n- [AC-2] y\n  @ref tests/store_test.rs#other_test\n",
        );

        assert_eq!(lint_delta(&repo).stdout, "");
    }
}

mod integrity {
    use super::*;

    #[test]
    fn an_invalid_glob_is_an_error() {
        let repo = project();
        repo.spec_file_with_tests(1, "active", &["a/**"], &["[bad"], INTENT);

        let run = check(&repo, &[]);

        assert_eq!(run.code, 2);
        assert!(says(&run, "tests glob `[bad` is invalid"));
    }
}

mod shape {
    use super::*;

    const CLAIMS: &str = "- [AC-1] Retries once.\n  @ref tests/store_test.rs#other_test\n- [AC-2] [intent] Feels fast.\n";
    const TASKS: &str =
        "- [x] T1 retry on timeout (settles AC-1)\n- [x] T2 measure (settles AC-2)\n";

    fn planned(repo: &Repo, claims: &str, tasks: &str) {
        repo.delta_file(1, "in-progress", &["SPEC-001"], INTENT);
        repo.delta_part(1, "design", "Retry once when the upstream times out.\n");
        repo.delta_claims(1, claims);
        repo.delta_tasks(1, tasks);
    }

    fn gate(repo: &Repo, status: &str) -> Run {
        check(repo, &["DELTA-001", "--gate", status])
    }

    fn named(repo: &Repo) -> Run {
        check(repo, &["DELTA-001"])
    }

    fn claims_path() -> String {
        delta_part_path(1, "claims")
    }

    fn tasks_path() -> String {
        delta_part_path(1, "tasks")
    }

    #[test]
    fn a_planned_and_built_delta_passes_both_gates() {
        let repo = project();
        planned(&repo, CLAIMS, TASKS);

        for status in ["in-progress", "complete"] {
            let run = gate(&repo, status);
            assert_eq!((run.code, run.stdout.as_str()), (0, ""), "{status}");
        }
    }

    #[test]
    fn a_gate_needs_intent() {
        let repo = project();
        planned(&repo, CLAIMS, TASKS);
        repo.delta_file(1, "in-progress", &["SPEC-001"], "## Intent\n\n");

        assert_eq!(named(&repo).stdout, "");
        let run = gate(&repo, "in-progress");
        assert_eq!(run.code, 2);
        assert!(says(&run, "missing or empty `## Intent` section"));
    }

    #[test]
    fn a_gate_needs_a_claim() {
        let repo = project();
        planned(&repo, "", "");

        assert!(says(
            &gate(&repo, "in-progress"),
            "no claims; write the behaviour"
        ));
    }

    #[test]
    fn a_bullet_without_an_id_is_an_error_at_a_gate() {
        let repo = project();
        planned(&repo, &format!("{CLAIMS}- Logs the retry.\n"), TASKS);

        assert_eq!(named(&repo).stdout, "");
        assert!(gate(&repo, "in-progress").stdout.contains(&format!(
            "{}:4: error: claim has no id; run `lemma review` to plan",
            claims_path()
        )));
    }

    #[test]
    fn every_claim_is_settled_by_exactly_one_main_task() {
        let repo = project();
        planned(
            &repo,
            CLAIMS,
            "- [x] T1 retry (settles AC-1)\n- [x] T2 again (settles AC-1)\n",
        );

        let run = gate(&repo, "in-progress");

        assert!(says(
            &run,
            "AC-1 is settled by T1, T2; exactly one task settles a claim"
        ));
        assert!(says(&run, "AC-2 is settled by no task"));
    }

    #[test]
    fn follow_ups_may_settle_a_settled_claim_and_must_be_ticked_at_complete() {
        let repo = project();
        planned(
            &repo,
            CLAIMS,
            &format!("{TASKS}- [x] T3 wire it (chore)\n\n## Follow-ups\n- [ ] T4 page the read (settles AC-1)\n"),
        );

        assert_eq!(gate(&repo, "in-progress").stdout, "");
        let run = gate(&repo, "complete");
        assert_eq!(run.code, 2);
        assert!(run.stdout.contains(&format!(
            "{}:6: error: T4 is not ticked; run `lemma build`",
            tasks_path()
        )));
    }

    #[test]
    fn a_task_settling_an_unknown_claim_is_always_an_error() {
        let repo = project();
        planned(
            &repo,
            CLAIMS,
            &format!("{TASKS}- [ ] T3 extra (settles AC-9)\n"),
        );

        assert!(says(
            &named(&repo),
            "T3 settles AC-9, which claims.md does not have"
        ));
    }

    #[test]
    fn a_malformed_task_line_is_always_an_error() {
        let repo = project();
        planned(
            &repo,
            CLAIMS,
            &format!("{TASKS}- [ ] T3 no settles\n- [ ] untagged (chore)\n"),
        );

        let run = named(&repo);

        assert_eq!(run.code, 2);
        for line in [3, 4] {
            assert!(
                run.stdout.contains(&format!(
                    "{}:{line}: error: task is not `- [ ] T<n> <text> (settles AC-<n>, ..)`",
                    tasks_path()
                )),
                "{}",
                run.stdout
            );
        }
    }

    #[test]
    fn duplicate_and_foreign_ids_are_always_errors() {
        let repo = project();
        planned(
            &repo,
            &format!("{CLAIMS}- [AC-1] Again.\n  @ref tests/store_test.rs#other_test\n- [RT-1] Foreign.\n"),
            &format!("{TASKS}- [x] T1 dup (settles AC-1)\n"),
        );

        let run = named(&repo);

        assert!(says(&run, "AC-1 is used twice (lines 1 and 4)"));
        assert!(says(&run, "claim id RT-1 is not AC-<n>"));
        assert!(says(&run, "T1 is used twice (lines 1 and 3)"));
    }

    #[test]
    fn review_lines_under_a_claim_a_task_and_prose_block_a_gate() {
        let repo = project();
        planned(
            &repo,
            &CLAIMS.replace(
                "Retries once.\n",
                "Retries once.\n  @review Say what times out.\n",
            ),
            &TASKS.replace(
                "(settles AC-1)\n",
                "(settles AC-1)\n  @review Name the file.\n",
            ),
        );
        repo.delta_part(1, "design", "Retry once.\n\n> review: Why once?\n");
        repo.delta_file(
            1,
            "in-progress",
            &["SPEC-001"],
            "## Intent\n\nBecause.\n\n> review: Which upstream?\n",
        );

        assert_eq!(named(&repo).code, 0, "{}", named(&repo).stdout);
        let run = gate(&repo, "in-progress");
        for place in [
            format!("{}:2:", claims_path()),
            format!("{}:2:", tasks_path()),
            format!("{}:3:", delta_part_path(1, "design")),
            format!("{}/index.md:16:", delta_dir(1)),
        ] {
            assert!(
                run.stdout
                    .contains(&format!("{place} error: unresolved review line")),
                "{place}\n{}",
                run.stdout
            );
        }
    }

    #[test]
    fn a_proposed_claim_blocks_a_gate_and_never_pairs_with_intent() {
        let repo = project();
        planned(
            &repo,
            &format!("{CLAIMS}- [AC-3] [proposed] Logs the retry.\n- [AC-4] [proposed] [intent] Is quiet.\n"),
            &format!("{TASKS}- [x] T3 log (settles AC-3)\n- [x] T4 hush (settles AC-4)\n"),
        );

        let ungated = named(&repo);
        assert!(says(
            &ungated,
            "AC-4: [proposed] and [intent] are exclusive; keep one"
        ));
        assert!(!says(&ungated, "AC-3 is [proposed]"));
        assert!(says(
            &gate(&repo, "in-progress"),
            "AC-3 is [proposed]; delete the tag to accept it or the claim to reject it"
        ));
    }

    #[test]
    fn a_ref_in_design_is_always_an_error() {
        let repo = project();
        planned(&repo, CLAIMS, TASKS);
        repo.delta_part(
            1,
            "design",
            "Retry.\n\n@ref tests/store_test.rs#other_test\n",
        );

        assert!(named(&repo).stdout.contains(&format!(
            "{}:3: error: design.md holds prose; an `@ref` belongs under a claim in claims.md",
            delta_part_path(1, "design")
        )));
    }

    #[test]
    fn a_missing_ref_is_an_error_only_at_complete() {
        let repo = project();
        planned(
            &repo,
            "- [AC-1] Retries once.\n",
            "- [ ] T1 retry (settles AC-1)\n",
        );

        assert_eq!(gate(&repo, "in-progress").stdout, "");
        let run = gate(&repo, "complete");
        assert!(run.stdout.contains(&format!(
            "{}:1: error: AC-1: claim has no `@ref`",
            claims_path()
        )));
    }

    #[test]
    fn a_written_ref_must_resolve_with_or_without_a_gate() {
        let repo = project();
        planned(
            &repo,
            "- [AC-1] Retries.\n  @ref tests/store_test.rs#missing\n",
            "",
        );

        let run = named(&repo);

        assert_eq!(run.code, 2);
        assert!(says(&run, "lemma refs DELTA-001/AC-1"));
    }

    #[test]
    fn without_a_delta_lint_leaves_deltas_alone() {
        let repo = project();
        planned(
            &repo,
            "- [AC-1] Retries.\n  @ref tests/store_test.rs#missing\n",
            "- [ ] nope\n",
        );

        assert_eq!(check(&repo, &[]).stdout, "");
    }

    #[test]
    fn an_unknown_gate_or_delta_is_refused() {
        let repo = project();
        planned(&repo, CLAIMS, TASKS);

        let gate = gate(&repo, "done");
        let missing = check(&repo, &["DELTA-099"]);

        assert_eq!(gate.code, 2);
        assert!(
            gate.stderr.contains("unknown gate `done`"),
            "{}",
            gate.stderr
        );
        assert_eq!(missing.code, 2);
        assert!(missing.stderr.contains("unknown delta DELTA-099"));
    }
}

mod hook {
    use super::*;

    fn delta_doc(status: &str, claims: &str) -> Value {
        with_parts(
            doc("DELTA-001", "delta", status, &[], &["SPEC-001"], INTENT),
            &[("claims", claims)],
        )
    }

    fn spec_doc() -> Value {
        let mut spec = doc("SPEC-001", "spec", "active", &["a/**"], &[], INTENT);
        spec["path"] = json!("docs/specs/SPEC-001-s/index.md");
        spec
    }

    #[test]
    fn it_exits_zero_and_prints_findings_json_even_with_errors() {
        let repo = project();
        let payload = validate_payload(vec![delta_doc(
            "in-progress",
            "- [AC-1] Retries.\n  @ref tests/store_test.rs#missing\n",
        )]);

        let run = repo.run(&["lint", "--hook"], &payload);

        assert_eq!(run.code, 0);
        assert_eq!(run.findings().len(), 1);
        let finding = &run.findings()[0];
        assert_eq!(finding["severity"], "error");
        assert_eq!(finding["id"], "DELTA-001");
        assert_eq!(finding["part"], "claims");
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
            "in-progress",
            "- [AC-1] Broken.\n  @ref tests/store_test.rs#missing\n",
        );
        let payload = validate_payload(vec![delta_doc("draft", "- [AC-1] Fine.\n")]);

        let run = repo.run(&["lint", "--hook"], &payload);

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
            delta_doc("draft", "- [AC-1] Fine.\n"),
            vec![spec_doc()],
        );

        let run = repo.run(&["lint", "--hook"], &payload);

        assert_eq!(run.code, 0);
        let finding = run
            .findings()
            .into_iter()
            .find(|f| f["message"].as_str().unwrap().contains(HISTORY))
            .unwrap();
        assert!(finding.get("id").is_none());
        assert_eq!(finding["message"], format!("{LANDED_CLAIMS}: {HISTORY}"));
    }

    #[test]
    fn the_gate_applies_to_the_payload_delta() {
        let repo = project();
        let payload = transition_payload(
            "in-progress",
            "complete",
            delta_doc("in-progress", "- [AC-1] No ref yet.\n"),
            vec![spec_doc()],
        );

        let lax = repo.run(&["lint", "--hook"], &payload);
        let strict = repo.run(&["lint", "--hook", "--gate", "complete"], &payload);

        assert_eq!(lax.findings(), Vec::<Value>::new());
        assert!(
            strict.has_message("AC-1: claim has no `@ref`"),
            "{}",
            strict.stdout
        );
        assert!(
            strict.has_message("is settled by no task"),
            "{}",
            strict.stdout
        );
        assert_eq!(strict.code, 0);
    }

    #[test]
    fn on_trunk_the_gate_uses_the_parent_of_the_deltas_first_commit() {
        let repo = project();
        delta(
            &repo,
            1,
            "in-progress",
            "- [AC-1] New.\n  @ref tests/store_test.rs#other_test\n",
        );
        repo.commit("delta");
        edit_reopen_body(&repo);
        repo.commit("edit the evidence");
        let mut moved = delta_doc(
            "in-progress",
            "- [AC-1] New.\n  @ref tests/store_test.rs#other_test\n",
        );
        moved["path"] = json!("docs/deltas/DELTA-001-d/index.md");
        let payload = transition_payload("in-progress", "complete", moved, vec![spec_doc()]);

        let gate = repo.run(&["lint", "--hook"], &payload);
        let plain = check(&repo, &[]);

        assert!(gate.has_message(HISTORY), "{}", gate.stdout);
        assert_eq!(gate.findings()[0]["severity"], "warning");
        assert_eq!(plain.stdout, "");
    }

    fn moved_delta(status: &str) -> Value {
        let mut moved = delta_doc(
            status,
            "- [AC-1] New.\n  @ref tests/store_test.rs#other_test\n",
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

        let run = repo.run(&["lint", "--hook"], &payload);

        assert_eq!(run.code, 0);
        assert!(!run.has_message("governed area"), "{}", run.stdout);
        assert!(!run.has_message("confirm intended"), "{}", run.stdout);
    }

    #[test]
    fn a_transition_limits_coverage_to_the_moved_deltas_specs_and_lets_history_through() {
        let repo = project();
        repo.write("b/tests/t_test.rs", "#[test]\nfn t() {}\n");
        repo.delta_file(8, "complete", &["SPEC-002"], INTENT);
        repo.delta_claims(8, "- [AC-1] B.\n  @ref b/tests/t_test.rs#t\n");
        repo.commit("spec 2 evidence");
        touch_both(&repo);
        repo.write("b/tests/t_test.rs", "#[test]\nfn t() { assert!(true); }\n");
        let payload = transition_payload(
            "in-progress",
            "complete",
            moved_delta("in-progress"),
            vec![spec_doc()],
        );

        let gate = repo.run(&["lint", "--hook"], &payload);
        let plain = check(&repo, &[]);

        let history =
            "test b/tests/t_test.rs::t changed; DELTA-008/AC-1 claims it; confirm intended";
        assert_eq!(
            gate.messages(),
            vec![format!("docs/deltas/DELTA-008-d/claims.md: {history}")]
        );
        assert!(says(&plain, history));
        assert!(says(&plain, "create a delta that changes SPEC-002"));
    }

    #[test]
    fn the_complete_gate_checks_evidence_on_the_moved_delta_only() {
        let repo = project();
        delta(
            &repo,
            2,
            "in-progress",
            "- [AC-1] Broken.\n  @ref tests/store_test.rs#missing\n",
        );
        repo.commit("another delta in flight");
        let mut moved = with_parts(
            doc(
                "DELTA-001",
                "delta",
                "in-progress",
                &[],
                &["SPEC-001"],
                INTENT,
            ),
            &[
                (
                    "claims",
                    "- [AC-1] New.\n  @ref tests/store_test.rs#other_test\n",
                ),
                ("tasks", "- [x] T1 new (settles AC-1)\n"),
            ],
        );
        moved["path"] = json!("docs/deltas/DELTA-001-d/index.md");
        let payload = transition_payload("in-progress", "complete", moved, vec![spec_doc()]);

        let gate = repo.run(&["lint", "--hook", "--gate", "complete"], &payload);

        assert_eq!(gate.findings(), Vec::<Value>::new(), "{}", gate.stdout);
    }

    #[test]
    fn an_invalid_payload_is_reported_with_exit_zero() {
        let run = project().run(&["lint", "--hook"], "not json");

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
