mod common;
use common::*;

const CLAIMS: &str = "- [AC-1] Reopen keeps the prior answer.\n  @ref tests/store_test.rs#reopen_keeps_prior\n- [AC-2] Reads are paged.\n";
const TASKS: &str =
    "- [x] T1 reopen (settles AC-1)\n- [ ] T2 paging (settles AC-2)\n- [ ] T3 wire it (chore)\n";

fn project() -> Repo {
    let repo = Repo::new();
    repo.with_dictums();
    repo.spec_file(1, "active", &["a/**"], "## Intent\n\nSpec one intent.\n");
    repo.write("a/x.rs", "fn x() {}\n");
    repo.write("b/y.rs", "fn y() {}\n");
    repo
}

fn delta(repo: &Repo, status: &str) {
    repo.delta_file(1, status, &["SPEC-001"], "## Intent\n\nDelta intent.\n");
    repo.delta_part(1, "design", "Design prose.\n");
    repo.delta_claims(1, CLAIMS);
    repo.delta_tasks(1, TASKS);
}

fn write_prompt(repo: &Repo, name: &str, text: &str) {
    repo.write(&format!(".lazyspec/prompts/{name}"), text);
}

#[test]
fn review_dry_run_on_a_draft_renders_review_doc_with_claims_and_intent() {
    let repo = project();
    delta(&repo, "draft");

    let run = repo.run(&["review", "DELTA-001", "--dry-run"], "");

    assert_eq!(run.code, 0, "{}", run.stderr);
    assert!(run.stdout.contains("Reopen keeps the prior answer."));
    assert!(run.stdout.contains("Spec one intent."));
    assert!(run.stdout.contains("House rules."));
    assert!(!run.stdout.contains("{{"), "{}", run.stdout);
}

#[test]
fn review_dry_run_on_an_in_progress_delta_includes_the_diff() {
    let repo = project();
    delta(&repo, "in-progress");
    repo.commit("start");
    repo.write("a/x.rs", "fn x() { changed_marker(); }\n");
    repo.write("b/y.rs", "fn y() { elsewhere(); }\n");

    let run = repo.run(&["review", "DELTA-001", "--dry-run"], "");

    assert_eq!(run.code, 0, "{}", run.stderr);
    assert!(run.stdout.contains("changed_marker"), "{}", run.stdout);
    assert!(!run.stdout.contains("elsewhere"));
    assert!(!run.stdout.contains("{{"));
}

#[test]
fn review_needs_a_draft_or_in_progress_delta() {
    let repo = project();
    delta(&repo, "complete");

    let run = repo.run(&["review", "DELTA-001", "--dry-run"], "");

    assert_eq!(run.code, 2);
    assert!(run
        .stderr
        .contains("review needs a draft or in-progress delta"));
}

#[test]
fn an_unknown_delta_exits_two() {
    let repo = project();

    let run = repo.run(&["review", "DELTA-404", "--dry-run"], "");

    assert_eq!(run.code, 2);
    assert!(run.stderr.contains("lemma: unknown delta DELTA-404"));
}

#[test]
fn build_dry_run_prints_the_first_unticked_task_and_its_claim() {
    let repo = project();
    delta(&repo, "in-progress");

    let run = repo.run(&["build", "DELTA-001", "--dry-run"], "");

    assert_eq!(run.code, 0, "{}", run.stderr);
    assert!(run.stdout.contains("- [ ] T2 paging (settles AC-2)"));
    assert!(run.stdout.contains("- [AC-2] Reads are paged."));
    assert!(!run.stdout.contains("T1 reopen (settles AC-1)\n\n- [AC-1]"));
    assert!(!run.stdout.contains("{{"));
}

#[test]
fn build_dry_run_with_a_task_picks_it() {
    let repo = project();
    delta(&repo, "in-progress");

    let run = repo.run(&["build", "DELTA-001", "--task", "T1", "--dry-run"], "");

    assert_eq!(run.code, 0, "{}", run.stderr);
    assert!(run
        .stdout
        .contains("- [x] T1 reopen (settles AC-1)\n\n- [AC-1] Reopen keeps the prior answer."));
}

#[test]
fn build_dry_run_on_a_chore_says_it_settles_no_claim() {
    let repo = project();
    delta(&repo, "in-progress");

    let run = repo.run(&["build", "DELTA-001", "--task", "T3", "--dry-run"], "");

    assert!(run
        .stdout
        .contains("This task is a chore: it settles no claim."));
}

#[test]
fn build_dry_run_with_an_unknown_task_fails() {
    let repo = project();
    delta(&repo, "in-progress");

    let run = repo.run(&["build", "DELTA-001", "--task", "T9", "--dry-run"], "");

    assert_eq!(run.code, 2);
    assert!(run.stderr.contains("unknown task T9"));
}

#[test]
fn build_dry_run_with_every_task_ticked_says_all_complete() {
    let repo = project();
    delta(&repo, "in-progress");
    repo.delta_tasks(1, "- [x] T1 reopen (settles AC-1)\n");

    let run = repo.run(&["build", "DELTA-001", "--dry-run"], "");

    assert_eq!(
        (run.code, run.stdout.as_str()),
        (
            0,
            "all tasks complete\nnext: lemma review DELTA-001 to review the diff, or lazyspec update DELTA-001 --status complete\n"
        )
    );
}

#[test]
fn a_pack_prompt_overrides_the_default() {
    let repo = project();
    delta(&repo, "in-progress");
    write_prompt(&repo, "build.md", "custom: {{task}}");

    let run = repo.run(&["build", "DELTA-001", "--dry-run"], "");

    assert!(
        run.stdout.starts_with("custom: - [ ] T2 paging"),
        "{}",
        run.stdout
    );
}

#[test]
fn dry_run_names_an_unknown_prompt_key_and_exits_two() {
    let repo = project();
    delta(&repo, "draft");
    write_prompt(&repo, "plan.md", "fine\n{{bogus}}\n");

    let run = repo.run(&["review", "DELTA-001", "--dry-run"], "");

    assert_eq!(run.code, 2);
    assert!(
        run.stderr
            .contains(".lazyspec/prompts/plan.md:2: unknown prompt key `{{bogus}}`"),
        "{}",
        run.stderr
    );
}

#[test]
fn lint_reports_an_unknown_prompt_key_at_its_path_and_line() {
    let repo = project();
    write_prompt(&repo, "review-doc.md", "a\nb\nuse {{nope}} here\n");

    let run = repo.run(&["lint", "--json"], "");

    assert_eq!(run.code, 2);
    let found: Vec<_> = run
        .findings()
        .into_iter()
        .filter(|f| f["path"] == ".lazyspec/prompts/review-doc.md")
        .collect();
    assert_eq!(found.len(), 1, "{}", run.stdout);
    assert_eq!(found[0]["line"], 3);
    assert_eq!(found[0]["severity"], "error");
    assert!(found[0]["message"]
        .as_str()
        .unwrap()
        .contains("unknown prompt key `{{nope}}`"));
}

#[test]
fn lint_is_silent_about_prompts_that_use_known_keys() {
    let repo = project();
    write_prompt(&repo, "plan.md", "{{ delta }} and {{claims}}\n");

    let run = repo.run(&["lint", "--json"], "");

    assert!(!run.stdout.contains("prompts"), "{}", run.stdout);
}

#[test]
fn a_bad_lemma_toml_fails_the_command() {
    let repo = project();
    delta(&repo, "draft");
    repo.write("lemma.toml", "runer = \"x\"\n");

    let run = repo.run(&["review", "DELTA-001", "--dry-run"], "");

    assert_eq!(run.code, 2);
    assert!(run.stderr.contains("runer"), "{}", run.stderr);
}

fn dictum_repo() -> Repo {
    let repo = project();
    repo.spec_file(2, "active", &["b/**"], "## Intent\n\nSpec two.\n");
    repo.named_dictum_file(1, &["a/**"], "DICTUM_A_BODY");
    repo.named_dictum_file(2, &["b/**"], "DICTUM_B_BODY");
    repo.named_dictum_file(3, &[], "DICTUM_C_BODY");
    delta(&repo, "draft");
    repo
}

#[test]
fn dictums_are_selected_by_governs_overlap_with_the_changed_specs() {
    let repo = dictum_repo();

    let run = repo.run(&["review", "DELTA-001", "--dry-run"], "");

    assert_eq!(run.code, 0, "{}", run.stderr);
    assert!(run.stdout.contains("DICTUM_A_BODY"));
    assert!(run.stdout.contains("DICTUM_C_BODY"));
    assert!(!run.stdout.contains("DICTUM_B_BODY"));
    assert!(run.stdout.contains("## DICTUM-001 Dictum 1"));
}

#[test]
fn every_dictum_is_included_when_none_is_selected() {
    let repo = dictum_repo();
    std::fs::remove_file(repo.path("docs/convention/DICTUM-003-d.md")).unwrap();
    repo.delta_file(1, "draft", &["SPEC-002"], "## Intent\n\nDelta intent.\n");
    repo.named_dictum_file(1, &["c/**"], "DICTUM_A_BODY");
    repo.named_dictum_file(2, &["d/**"], "DICTUM_B_BODY");

    let run = repo.run(&["review", "DELTA-001", "--dry-run"], "");

    assert_eq!(run.code, 0, "{}", run.stderr);
    assert!(run.stdout.contains("DICTUM_A_BODY") && run.stdout.contains("DICTUM_B_BODY"));
}

#[test]
fn dictums_overlap_through_files_both_globs_match() {
    let repo = dictum_repo();
    std::fs::remove_file(repo.path("docs/convention/DICTUM-003-d.md")).unwrap();
    repo.write("a/deep/z.rs", "fn z() {}\n");
    repo.named_dictum_file(1, &["**/*.rs"], "DICTUM_A_BODY");
    repo.named_dictum_file(2, &["docs/**"], "DICTUM_B_BODY");

    let run = repo.run(&["review", "DELTA-001", "--dry-run"], "");

    assert!(run.stdout.contains("DICTUM_A_BODY"));
    assert!(!run.stdout.contains("DICTUM_B_BODY"));
}
