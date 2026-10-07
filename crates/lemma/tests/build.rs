mod common;
use common::*;
use std::os::unix::fs::PermissionsExt;

const CLAIMS_PATH: &str = "docs/deltas/DELTA-001-d/claims.md";
const BARE: &str = "- [AC-1] Reopen keeps the prior answer.\n- [AC-2] Writes are atomic.\n";
const REF_1: &str = "  @ref tests/store_test.rs#reopen_keeps_prior\n";
const REF_2: &str = "  @ref tests/store_test.rs#other_test\n";
const TWO_TASKS: &str = "- [ ] T1 reopen (settles AC-1)\n- [ ] T2 atomic (settles AC-2)\n";
const RUNNER: &str = "#!/bin/sh\ncat > /dev/null\ncp -R \".fixture/$LEMMA_TASK/.\" .\necho '{\"result\":\"done\"}'\n";

fn claims(ac1: bool, ac2: bool) -> String {
    let ref_of = |on: bool, line: &'static str| if on { line } else { "" };
    format!(
        "- [AC-1] Reopen keeps the prior answer.\n{}- [AC-2] Writes are atomic.\n{}",
        ref_of(ac1, REF_1),
        ref_of(ac2, REF_2)
    )
}

fn project(status: &str, tasks: &str) -> Repo {
    let repo = Repo::new();
    repo.git(&["config", "user.name", "t"]);
    repo.git(&["config", "user.email", "t@example.com"]);
    repo.git(&["config", "commit.gpgsign", "false"]);
    repo.with_dictums();
    repo.spec_file(1, "active", &["src/**"], "## Intent\n\nSpec one intent.\n");
    repo.delta_file(1, status, &["SPEC-001"], "## Intent\n\nDelta intent.\n");
    repo.delta_part(1, "design", "Design prose.\n");
    repo.delta_claims(1, BARE);
    repo.delta_tasks(1, tasks);
    let script = repo.path("fake-runner.sh");
    repo.write("fake-runner.sh", RUNNER);
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    repo.write(
        "lemma.toml",
        &format!("runner = \"{}\"\n", script.display()),
    );
    repo.write(".gitignore", ".fixture/\n");
    repo.commit("start");
    repo
}

fn fixture(repo: &Repo, task: &str, rel: &str, content: &str) {
    repo.write(&format!(".fixture/{task}/{rel}"), content);
}

fn build(repo: &Repo, args: &[&str]) -> Run {
    let mut full = vec!["build", "DELTA-001"];
    full.extend_from_slice(args);
    repo.run(&full, "")
}

fn tasks(repo: &Repo) -> String {
    repo.read(&delta_part_path(1, "tasks"))
}

fn subjects(repo: &Repo) -> Vec<String> {
    repo.git(&["log", "--format=%s"])
        .lines()
        .map(str::to_string)
        .collect()
}

fn two_task_fixtures(repo: &Repo) {
    fixture(repo, "T1", CLAIMS_PATH, &claims(true, false));
    fixture(repo, "T1", "src/one.rs", "pub fn one() {}\n");
    fixture(repo, "T2", CLAIMS_PATH, &claims(true, true));
    fixture(repo, "T2", "src/two.rs", "pub fn two() {}\n");
}

#[test]
fn each_task_is_ticked_and_committed_in_order() {
    let repo = project("in-progress", TWO_TASKS);
    two_task_fixtures(&repo);

    let run = build(&repo, &[]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert_eq!(
        &subjects(&repo)[..2],
        ["DELTA-001 T2: atomic", "DELTA-001 T1: reopen"]
    );
    assert_eq!(
        tasks(&repo),
        "- [x] T1 reopen (settles AC-1)\n- [x] T2 atomic (settles AC-2)\n"
    );
    assert_eq!(repo.git(&["status", "--porcelain"]), "");
    assert!(
        run.stdout.ends_with("all tasks complete\n"),
        "{}",
        run.stdout
    );
}

#[test]
fn the_report_follows_the_documented_shape() {
    let repo = project("in-progress", TWO_TASKS);
    two_task_fixtures(&repo);

    let run = build(&repo, &[]);

    let lines: Vec<&str> = run.stdout.lines().collect();
    assert_eq!(lines[0], "T1 reopen (settles AC-1)");
    assert!(lines[1].starts_with("   runner: "), "{}", run.stdout);
    assert!(lines[1].ends_with("s, 2 files"), "{}", run.stdout);
    assert_eq!(lines[2], "   lint: clean");
    assert_eq!(
        lines[3],
        "   AC-1 @ref tests/store_test.rs#reopen_keeps_prior resolves"
    );
    let sha = repo.git(&["log", "--format=%h", "--skip=1", "-1"]);
    assert_eq!(lines[4], format!("   commit {sha} DELTA-001 T1: reopen"));
}

#[test]
fn a_lint_error_stops_with_the_tree_kept() {
    let repo = project("in-progress", TWO_TASKS);
    let broken = format!("{BARE}  @ref tests/store_test.rs#missing_fn\n");
    fixture(&repo, "T1", CLAIMS_PATH, &broken);
    fixture(&repo, "T1", "src/one.rs", "pub fn one() {}\n");
    let before = repo.head();

    let run = build(&repo, &[]);

    assert_eq!(run.code, 2, "{}", run.stdout);
    assert!(run.stdout.contains("   lint: "), "{}", run.stdout);
    assert!(run.stdout.contains("missing_fn"), "{}", run.stdout);
    assert!(run
        .stdout
        .ends_with("   stopped. T1 left unticked; working tree kept.\n"));
    assert_eq!(tasks(&repo), TWO_TASKS);
    assert_eq!(repo.read("src/one.rs"), "pub fn one() {}\n");
    assert_eq!(repo.read(CLAIMS_PATH), broken);
    assert_eq!(repo.head(), before);
}

#[test]
fn a_settled_claim_without_a_ref_stops() {
    let repo = project("in-progress", TWO_TASKS);
    fixture(&repo, "T1", "src/one.rs", "pub fn one() {}\n");
    let before = repo.head();

    let run = build(&repo, &[]);

    assert_eq!(run.code, 2, "{}", run.stdout);
    assert!(
        run.stdout.contains("   AC-1 has no @ref\n"),
        "{}",
        run.stdout
    );
    assert!(run
        .stdout
        .ends_with("   stopped. T1 left unticked; working tree kept.\n"));
    assert_eq!(tasks(&repo), TWO_TASKS);
    assert_eq!(repo.head(), before);
}

#[test]
fn an_intent_claim_needs_no_ref() {
    let repo = project("in-progress", "- [ ] T1 reopen (settles AC-1)\n");
    fixture(
        &repo,
        "T1",
        CLAIMS_PATH,
        "- [AC-1] [intent] Reopen keeps the prior answer.\n- [AC-2] Writes are atomic.\n",
    );

    let run = build(&repo, &[]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert!(run.stdout.contains("   AC-1 [intent]\n"), "{}", run.stdout);
}

#[test]
fn a_rerun_resumes_at_the_first_unticked_task() {
    let repo = project("in-progress", TWO_TASKS);
    two_task_fixtures(&repo);
    fixture(&repo, "T2", CLAIMS_PATH, &claims(true, false));
    let stopped = build(&repo, &[]);
    assert_eq!(stopped.code, 2, "{}", stopped.stdout);
    assert_eq!(subjects(&repo)[0], "DELTA-001 T1: reopen");
    repo.write(CLAIMS_PATH, &claims(true, true));
    repo.commit("human fix");
    fixture(&repo, "T2", CLAIMS_PATH, &claims(true, true));

    let run = build(&repo, &[]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert!(run.stdout.starts_with("T2 atomic (settles AC-2)\n"));
    assert!(run.stdout.ends_with("all tasks complete\n"));
    assert_eq!(subjects(&repo)[0], "DELTA-001 T2: atomic");
}

#[test]
fn task_runs_only_that_task() {
    let repo = project("in-progress", TWO_TASKS);
    fixture(&repo, "T2", CLAIMS_PATH, &claims(false, true));
    fixture(&repo, "T2", "src/two.rs", "pub fn two() {}\n");

    let run = build(&repo, &["--task", "T2"]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert_eq!(
        tasks(&repo),
        "- [ ] T1 reopen (settles AC-1)\n- [x] T2 atomic (settles AC-2)\n"
    );
    assert_eq!(subjects(&repo)[0], "DELTA-001 T2: atomic");
}

#[test]
fn a_ticked_task_is_reported_and_left_alone() {
    let repo = project("in-progress", "- [x] T1 reopen (settles AC-1)\n");
    let before = repo.head();

    let run = build(&repo, &["--task", "T1"]);

    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(run.stdout, "T1 is already ticked\n");
    assert_eq!(repo.head(), before);
}

#[test]
fn an_unknown_task_is_refused() {
    let repo = project("in-progress", TWO_TASKS);

    let run = build(&repo, &["--task", "T9"]);

    assert_eq!(run.code, 2);
    assert!(run.stderr.contains("lemma: unknown task T9"));
}

#[test]
fn a_chore_skips_the_ref_check() {
    let repo = project("in-progress", "- [ ] T1 wire it (chore)\n");
    fixture(&repo, "T1", "src/wire.rs", "pub fn wire() {}\n");

    let run = build(&repo, &[]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert!(run.stdout.contains("T1 wire it (chore)\n"));
    assert!(!run.stdout.contains("@ref"), "{}", run.stdout);
    assert_eq!(subjects(&repo)[0], "DELTA-001 T1: wire it");
}

#[test]
fn a_dirty_tree_is_refused() {
    let repo = project("in-progress", TWO_TASKS);
    repo.write("stray.txt", "x");

    let run = build(&repo, &[]);

    assert_eq!(run.code, 2);
    assert!(run
        .stderr
        .contains("lemma: build needs a clean working tree; commit or stash first"));
}

#[test]
fn a_delta_that_is_not_in_progress_is_refused() {
    let repo = project("draft", TWO_TASKS);

    let run = build(&repo, &[]);

    assert_eq!(run.code, 2);
    assert!(run
        .stderr
        .contains("lemma: build needs an in-progress delta; DELTA-001 is draft"));
}

#[test]
fn follow_ups_run_after_the_main_tasks() {
    let tasks = "- [ ] T1 reopen (settles AC-1)\n\n## Follow-ups\n- [ ] T2 tidy (chore)\n";
    let repo = project("in-progress", tasks);
    fixture(&repo, "T1", CLAIMS_PATH, &claims(true, false));
    fixture(&repo, "T2", "src/tidy.rs", "pub fn tidy() {}\n");

    let run = build(&repo, &[]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert_eq!(
        &subjects(&repo)[..2],
        ["DELTA-001 T2: tidy", "DELTA-001 T1: reopen"]
    );
}
