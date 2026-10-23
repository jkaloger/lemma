mod common;
use common::*;
use serde_json::{json, Value};
use std::os::unix::fs::PermissionsExt;

const NEXT: &str = "next: lazyspec update DELTA-001 --status complete\n";

const CLAIMS_PATH: &str = "docs/deltas/DELTA-001-d/claims.md";
const BARE: &str = "- [AC-1] Reopen keeps the prior answer.\n- [AC-2] Writes are atomic.\n";
const REF_1: &str = "  @ref tests/store_test.rs#reopen_keeps_prior\n";
const REF_2: &str = "  @ref tests/store_test.rs#other_test\n";
const TWO_TASKS: &str = "- [ ] T1 reopen (settles AC-1)\n- [ ] T2 atomic (settles AC-2)\n";
const RUNNER: &str = "#!/bin/sh\nmkdir -p .fixture\ncat > \".fixture/$LEMMA_TASK.prompt.cur\"\nif grep -q 'A reviewer read your work' \".fixture/$LEMMA_TASK.prompt.cur\"; then\n  n=$(cat \".fixture/$LEMMA_TASK.fixes\" 2>/dev/null || echo 0)\n  n=$((n+1))\n  echo $n > \".fixture/$LEMMA_TASK.fixes\"\n  key=\"$LEMMA_TASK.fix$n\"\nelse\n  n=0\n  key=\"$LEMMA_TASK\"\nfi\nmv \".fixture/$LEMMA_TASK.prompt.cur\" \".fixture/$LEMMA_TASK.prompt.$n\"\nprintf '%s' \"$*\" > \".fixture/$LEMMA_TASK.args.$n\"\n[ -d \".fixture/$key\" ] && cp -R \".fixture/$key/.\" .\ncat \".fixture/$key.json\"\n";
const REVIEWER: &str = "#!/bin/sh\nmkdir -p .fixture\nn=$(cat \".fixture/$LEMMA_TASK.rcalls\" 2>/dev/null || echo 0)\nn=$((n+1))\necho $n > \".fixture/$LEMMA_TASK.rcalls\"\ncat > \".fixture/$LEMMA_TASK.review-prompt\"\ncp \".fixture/$LEMMA_TASK.review-prompt\" \".fixture/$LEMMA_TASK.review-prompt.$n\"\ncp \"$1\" \".fixture/$LEMMA_TASK.schema\"\nif [ \"$n\" = 1 ]; then f=\".fixture/$LEMMA_TASK.review.json\"; else f=\".fixture/$LEMMA_TASK.review$n.json\"; fi\nif [ -f \"$f\" ]; then cat \"$f\"; else echo '{\"structured_output\":{\"stop\":\"\",\"findings\":[]}}'; fi\n";
const REOPEN: (&str, &str, &str) = ("AC-1", "tests/store_test.rs", "reopen_keeps_prior");
const OTHER: (&str, &str, &str) = ("AC-2", "tests/store_test.rs", "other_test");
const MISSING: (&str, &str, &str) = ("AC-1", "tests/store_test.rs", "missing_fn");

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
    install_fakes(&repo, "");
    repo.write(".gitignore", ".fixture/\n");
    repo.commit("start");
    repo
}

fn install_fakes(repo: &Repo, runner_args: &str) {
    install_fakes_with(repo, runner_args, "");
}

fn install_fakes_with(repo: &Repo, runner_args: &str, extra: &str) {
    for (name, body) in [("fake-runner.sh", RUNNER), ("fake-reviewer.sh", REVIEWER)] {
        repo.write(name, body);
        std::fs::set_permissions(repo.path(name), std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    repo.write(
        "lemma.toml",
        &format!(
            "runner = \"{}{runner_args}\"\nreviewer = \"{} {{schema_file}}\"\n{extra}",
            repo.path("fake-runner.sh").display(),
            repo.path("fake-reviewer.sh").display()
        ),
    );
}

fn fixture(repo: &Repo, task: &str, rel: &str, content: &str) {
    repo.write(&format!(".fixture/{task}/{rel}"), content);
}

fn reply(repo: &Repo, task: &str, outcome: &str, reason: &str, refs: &[(&str, &str, &str)]) {
    let refs: Vec<Value> = refs
        .iter()
        .map(|(claim, path, symbol)| json!({"claim": claim, "path": path, "symbol": symbol}))
        .collect();
    let output = json!({"structured_output": {"outcome": outcome, "reason": reason, "refs": refs}});
    repo.write(&format!(".fixture/{task}.json"), &output.to_string());
}

fn done(repo: &Repo, task: &str, refs: &[(&str, &str, &str)]) {
    reply(repo, task, "done", "", refs);
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
    fixture(repo, "T1", "src/one.rs", "pub fn one() {}\n");
    done(repo, "T1", &[REOPEN]);
    fixture(repo, "T2", "src/two.rs", "pub fn two() {}\n");
    done(repo, "T2", &[OTHER]);
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
        run.stdout.ends_with(&format!("all tasks complete\n{NEXT}")),
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
    assert!(lines[1].ends_with("s, 1 files"), "{}", run.stdout);
    assert_eq!(lines[2], "   lint: clean");
    assert_eq!(
        lines[3],
        "   AC-1 @ref tests/store_test.rs#reopen_keeps_prior resolves"
    );
    let sha = repo.git(&["log", "--format=%h", "--skip=1", "-1"]);
    assert_eq!(lines[4], "   review: clean");
    assert_eq!(lines[5], format!("   commit {sha} DELTA-001 T1: reopen"));
}

#[test]
fn a_lint_error_left_after_the_last_fix_round_stops_with_the_tree_kept() {
    let repo = project("in-progress", TWO_TASKS);
    let broken = "- [AC-1] Reopen keeps the prior answer.\n  @ref tests/store_test.rs#missing_fn\n- [AC-2] Writes are atomic.\n";
    fixture(&repo, "T1", "src/one.rs", "pub fn one() {}\n");
    done(&repo, "T1", &[MISSING]);
    fix_reply(&repo, "T1", 1, &[MISSING]);
    let before = repo.head();

    let run = build(&repo, &[]);

    assert_eq!(run.code, 2, "{}", run.stdout);
    assert!(
        run.stdout.contains("   check: 1 finding, fix round 1\n"),
        "{}",
        run.stdout
    );
    assert!(run.stdout.contains("   lint: "), "{}", run.stdout);
    assert!(run.stdout.contains("missing_fn"), "{}", run.stdout);
    assert!(run
        .stdout
        .ends_with("   stopped. T1 left unticked; working tree kept.\n"));
    assert_eq!(repo.read(".fixture/T1.fixes").trim(), "1");
    assert!(!repo.path(".fixture/T1.rcalls").exists());
    assert_eq!(tasks(&repo), TWO_TASKS);
    assert_eq!(repo.read("src/one.rs"), "pub fn one() {}\n");
    assert_eq!(repo.read(CLAIMS_PATH), broken);
    assert_eq!(repo.head(), before);
    let record = record(&repo);
    assert_eq!(record["ending"], "unticked");
    let finding = &record["findings"][0];
    assert_eq!(finding["basis"], "AC-1");
    assert!(
        finding["text"].as_str().unwrap().starts_with("lint: "),
        "{finding}"
    );
}

#[test]
fn a_settled_claim_without_a_ref_becomes_a_fix_round_finding_on_the_claim() {
    let repo = project("in-progress", single_task());
    fixture(&repo, "T1", "src/one.rs", "pub fn one() {}\n");
    done(&repo, "T1", &[]);
    fix_reply(&repo, "T1", 1, &[REOPEN]);

    let run = build(&repo, &[]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert!(
        run.stdout.contains("   AC-1 has no @ref\n"),
        "{}",
        run.stdout
    );
    assert!(fix_prompt(&repo).contains("- [AC-1] AC-1 has no @ref\n"));
    assert_eq!(tasks(&repo), "- [x] T1 reopen (settles AC-1)\n");
    assert_eq!(repo.read(CLAIMS_PATH), claims(true, false));
}

#[test]
fn an_intent_claim_needs_no_ref() {
    let repo = project("in-progress", "- [ ] T1 reopen (settles AC-1)\n");
    repo.write(
        CLAIMS_PATH,
        "- [AC-1] [intent] Reopen keeps the prior answer.\n- [AC-2] Writes are atomic.\n",
    );
    repo.commit("mark intent");
    done(&repo, "T1", &[]);

    let run = build(&repo, &[]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert!(run.stdout.contains("   AC-1 [intent]\n"), "{}", run.stdout);
}

#[test]
fn a_rerun_resumes_at_the_first_unticked_task() {
    let repo = project("in-progress", TWO_TASKS);
    two_task_fixtures(&repo);
    done(&repo, "T2", &[]);
    let stopped = build(&repo, &[]);
    assert_eq!(stopped.code, 2, "{}", stopped.stdout);
    assert_eq!(subjects(&repo)[0], "DELTA-001 T1: reopen");
    repo.write(CLAIMS_PATH, &claims(true, true));
    let refused = build(&repo, &[]);
    assert_eq!(refused.code, 2, "{}", refused.stdout);
    assert!(refused.stderr.contains("T2 stopped "), "{}", refused.stderr);

    let run = build(&repo, &["--resume"]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert!(run.stdout.starts_with("T2 atomic (settles AC-2)\n"));
    assert!(run.stdout.ends_with(&format!("all tasks complete\n{NEXT}")));
    assert_eq!(subjects(&repo)[0], "DELTA-001 T2: atomic");
}

#[test]
fn task_runs_only_that_task() {
    let repo = project("in-progress", TWO_TASKS);
    fixture(&repo, "T2", "src/two.rs", "pub fn two() {}\n");
    done(&repo, "T2", &[OTHER]);

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
    assert_eq!(run.stdout, format!("T1 is already ticked\n{NEXT}"));
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
    done(&repo, "T1", &[]);

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
    assert!(run
        .stderr
        .contains("`lazyspec update DELTA-001 --status in-progress`"));
}

#[test]
fn follow_ups_run_after_the_main_tasks() {
    let tasks = "- [ ] T1 reopen (settles AC-1)\n\n## Follow-ups\n- [ ] T2 tidy (chore)\n";
    let repo = project("in-progress", tasks);
    done(&repo, "T1", &[REOPEN]);
    fixture(&repo, "T2", "src/tidy.rs", "pub fn tidy() {}\n");
    done(&repo, "T2", &[]);

    let run = build(&repo, &[]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert_eq!(
        &subjects(&repo)[..2],
        ["DELTA-001 T2: tidy", "DELTA-001 T1: reopen"]
    );
}

#[test]
fn refs_are_written_under_their_claim_once_and_resolve() {
    let repo = project("in-progress", TWO_TASKS);
    fixture(&repo, "T2", "src/two.rs", "pub fn two() {}\n");
    done(&repo, "T2", &[OTHER, OTHER]);

    let run = build(&repo, &["--task", "T2"]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert_eq!(
        repo.read(CLAIMS_PATH),
        format!("- [AC-1] Reopen keeps the prior answer.\n- [AC-2] Writes are atomic.\n{REF_2}")
    );
    let refs = repo.run(&["refs", "DELTA-001/AC-2"], "");
    assert_eq!(refs.code, 0, "{}", refs.stdout);
    assert!(refs.stdout.contains("other_test ok"), "{}", refs.stdout);
}

#[test]
fn a_ref_follows_the_continuation_lines_of_its_claim() {
    let repo = project("in-progress", "- [ ] T1 reopen (settles AC-1)\n");
    repo.write(
        CLAIMS_PATH,
        "- [AC-1] Reopen keeps the prior answer.\n  more detail\n- [AC-2] Writes are atomic.\n",
    );
    repo.commit("detail");
    done(&repo, "T1", &[REOPEN]);

    let run = build(&repo, &[]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert_eq!(
        repo.read(CLAIMS_PATH),
        format!("- [AC-1] Reopen keeps the prior answer.\n  more detail\n{REF_1}- [AC-2] Writes are atomic.\n")
    );
}

#[test]
fn a_blocked_outcome_prints_the_reason_and_leaves_the_task_unticked() {
    let repo = project("in-progress", TWO_TASKS);
    reply(&repo, "T1", "blocked", "the store has no seam to test", &[]);
    let before = repo.head();

    let run = build(&repo, &[]);

    assert_eq!(run.code, 2, "{}", run.stdout);
    assert!(
        run.stdout
            .contains("   runner blocked: the store has no seam to test\n"),
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
fn a_delta_part_edit_becomes_a_fix_round_finding() {
    let repo = project("in-progress", single_task());
    fixture(&repo, "T1", CLAIMS_PATH, &claims(true, false));
    fixture(&repo, "T1", "src/one.rs", "pub fn one() {}\n");
    done(&repo, "T1", &[REOPEN]);
    fixture(&repo, "T1.fix1", CLAIMS_PATH, BARE);
    fix_reply(&repo, "T1", 1, &[REOPEN]);

    let run = build(&repo, &[]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert!(
        run.stdout.contains(&format!(
            "   runner edited {CLAIMS_PATH}; lemma writes delta parts\n"
        )),
        "{}",
        run.stdout
    );
    assert!(fix_prompt(&repo).contains(&format!(
        "- [task] runner edited {CLAIMS_PATH}; lemma writes delta parts\n"
    )));
    assert_eq!(tasks(&repo), "- [x] T1 reopen (settles AC-1)\n");
    assert_eq!(repo.read(CLAIMS_PATH), claims(true, false));
}

#[test]
fn a_reply_without_json_becomes_a_fix_round_finding() {
    let repo = project("in-progress", single_task());
    fixture(&repo, "T1", "src/one.rs", "pub fn one() {}\n");
    repo.write(".fixture/T1.json", r#"{"result": "all done"}"#);
    fix_reply(&repo, "T1", 1, &[REOPEN]);

    let run = build(&repo, &[]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert!(
        run.stdout
            .contains("   runner: runner reply has no JSON object\n"),
        "{}",
        run.stdout
    );
    assert!(fix_prompt(&repo).contains("- [task] runner: runner reply has no JSON object\n"));
    assert_eq!(tasks(&repo), "- [x] T1 reopen (settles AC-1)\n");
}

#[test]
fn a_lint_error_becomes_a_fix_round_finding_on_the_claim_it_names() {
    let repo = project("in-progress", single_task());
    fixture(&repo, "T1", "src/one.rs", "pub fn one() {}\n");
    done(&repo, "T1", &[MISSING]);
    fix_reply(&repo, "T1", 1, &[REOPEN]);

    let run = build(&repo, &[]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    let fix = fix_prompt(&repo);
    let finding = fix
        .lines()
        .find(|l| l.starts_with("- [AC-1] lint: "))
        .unwrap_or_else(|| panic!("{fix}"));
    assert!(finding.contains("missing_fn"), "{finding}");
    assert_eq!(repo.read(CLAIMS_PATH), claims(true, false));
}

#[test]
fn a_not_a_test_ref_becomes_a_fix_round_finding() {
    let repo = project("in-progress", single_task());
    fixture(&repo, "T1", "src/one.rs", "pub fn one() {}\n");
    done(&repo, "T1", &[("AC-1", "src/one.rs", "one")]);
    fix_reply(&repo, "T1", 1, &[REOPEN]);

    let run = build(&repo, &[]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    let fix = fix_prompt(&repo);
    let finding = fix
        .lines()
        .find(|l| l.contains("src/one.rs#one"))
        .unwrap_or_else(|| panic!("{fix}"));
    assert!(finding.starts_with("- [AC-1] "), "{finding}");
    assert!(finding.contains("not"), "{finding}");
}

#[test]
fn a_check_fix_round_spends_the_budget_reviewer_findings_would_use() {
    let repo = project("in-progress", single_task());
    fixture(&repo, "T1", "src/one.rs", "pub fn one() {}\n");
    done(&repo, "T1", &[]);
    fix_reply(&repo, "T1", 1, &[REOPEN]);
    review_round(&repo, "T1", 1, &[HELPER]);

    let run = build(&repo, &[]);

    assert_eq!(run.code, 2, "{}", run.stdout);
    assert!(
        run.stdout.contains("   check: 1 finding, fix round 1\n"),
        "{}",
        run.stdout
    );
    assert!(
        run.stdout
            .ends_with("   stopped. T1 left unticked after 1 fix round; working tree kept.\n"),
        "{}",
        run.stdout
    );
    assert_eq!(repo.read(".fixture/T1.fixes").trim(), "1");
}

#[test]
fn a_check_failure_goes_to_a_fix_round_and_a_clean_review_commits() {
    let repo = project("in-progress", single_task());
    fixture(&repo, "T1", "src/one.rs", "pub fn one() {}\n");
    done(&repo, "T1", &[MISSING]);
    fixture(
        &repo,
        "T1.fix1",
        "src/one.rs",
        "pub fn one() { /* fixed */ }\n",
    );
    fix_reply(&repo, "T1", 1, &[REOPEN]);

    let run = build(&repo, &[]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert!(
        run.stdout.contains("   check: 1 finding, fix round 1\n"),
        "{}",
        run.stdout
    );
    assert!(run.stdout.contains("   review: clean\n"), "{}", run.stdout);
    assert_eq!(repo.read(".fixture/T1.rcalls").trim(), "1");
    assert_eq!(
        repo.git(&["log", "-1", "--format=%B"]).trim_end(),
        "DELTA-001 T1: reopen\n\nLemma-Review: 1 finding fixed in 1 round"
    );
    assert_eq!(repo.read("src/one.rs"), "pub fn one() { /* fixed */ }\n");
    assert_eq!(repo.git(&["status", "--porcelain"]), "");
}

const CHECK: &str = "#!/bin/sh\nfixes=$(cat .fixture/T1.fixes 2>/dev/null || echo 0)\nrcalls=$(cat .fixture/T1.rcalls 2>/dev/null || echo 0)\necho \"$(pwd) fixes=$fixes rcalls=$rcalls\" >> .fixture/check.log\n";

fn with_check(repo: &Repo, extra: &str) {
    repo.write(".fixture/fake-check.sh", CHECK);
    let check = repo.path(".fixture/fake-check.sh");
    std::fs::set_permissions(&check, std::fs::Permissions::from_mode(0o755)).unwrap();
    install_fakes_with(
        repo,
        "",
        &format!("check = \"{}\"\n{extra}", check.display()),
    );
    repo.commit("check");
}

#[test]
fn without_a_check_build_runs_no_check_command() {
    let repo = project("in-progress", single_task());
    repo.write(".fixture/fake-check.sh", CHECK);
    one_task_fixtures(&repo);

    let run = build(&repo, &[]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert!(!run.stdout.contains("check:"), "{}", run.stdout);
    assert!(!repo.path(".fixture/check.log").exists());
}

#[test]
fn the_check_runs_in_the_repo_root_after_lint_and_evidence_pass_and_before_review() {
    let repo = project("in-progress", single_task());
    with_check(&repo, "review_rounds = 2\n");
    fixture(&repo, "T1", "src/one.rs", "pub fn one() {}\n");
    done(&repo, "T1", &[MISSING]);
    fix_reply(&repo, "T1", 1, &[]);
    fix_reply(&repo, "T1", 2, &[REOPEN]);

    let run = build(&repo, &[]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    let root = std::fs::canonicalize(repo.path(".")).unwrap();
    assert_eq!(
        repo.read(".fixture/check.log"),
        format!("{} fixes=2 rcalls=0\n", root.display())
    );
    let lint_failed = run.stdout.find("   lint: ").unwrap();
    let evidence_failed = run.stdout.find("AC-1 has no @ref").unwrap();
    let passed = run.stdout.find("   check: pass").unwrap();
    let reviewed = run.stdout.find("   review: clean").unwrap();
    assert!(
        lint_failed < evidence_failed && evidence_failed < passed && passed < reviewed,
        "{}",
        run.stdout
    );
}

#[test]
fn a_passing_check_prints_check_pass_and_proceeds_to_review() {
    let repo = project("in-progress", single_task());
    with_check(&repo, "");
    one_task_fixtures(&repo);

    let run = build(&repo, &[]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    let lines: Vec<&str> = run.stdout.lines().collect();
    assert_eq!(
        lines[3..6],
        [
            "   AC-1 @ref tests/store_test.rs#reopen_keeps_prior resolves",
            "   check: pass",
            "   review: clean",
        ],
        "{}",
        run.stdout
    );
}

const FAILING_CHECK: &str = "#!/bin/sh\nfixes=$(cat .fixture/T1.fixes 2>/dev/null || echo 0)\n[ \"$fixes\" -ge \"$(cat .fixture/check.passes-at)\" ] && exit 0\ni=1\nwhile [ $i -le 45 ]; do echo \"line $i\"; i=$((i+1)); done\necho 'AC-1 still fails' >&2\nexit 3\n";

fn with_failing_check(repo: &Repo, passes_at: u32, extra: &str) -> String {
    repo.write(".fixture/fake-check.sh", FAILING_CHECK);
    repo.write(".fixture/check.passes-at", &passes_at.to_string());
    let check = repo.path(".fixture/fake-check.sh");
    std::fs::set_permissions(&check, std::fs::Permissions::from_mode(0o755)).unwrap();
    let command = check.display().to_string();
    install_fakes_with(repo, "", &format!("check = \"{command}\"\n{extra}"));
    repo.commit("check");
    command
}

#[test]
fn a_failing_check_is_one_task_finding_with_the_exit_code_and_output_tail() {
    let repo = project("in-progress", single_task());
    let command = with_failing_check(&repo, 1, "review_rounds = 2\n");
    one_task_fixtures(&repo);
    fix_reply(&repo, "T1", 1, &[REOPEN]);

    let run = build(&repo, &[]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert!(
        run.stdout.contains("   check: 1 finding, fix round 1\n"),
        "{}",
        run.stdout
    );
    let tail: String = (7..=45).map(|i| format!("\nline {i}")).collect();
    assert_eq!(
        fix_findings(&repo),
        format!("- [task] check: {command} exited 3{tail}\nAC-1 still fails")
    );
    assert_eq!(repo.read(".fixture/T1.rcalls").trim(), "1");
}

const HANGING_CHECK: &str = "#!/bin/sh\nfixes=$(cat .fixture/T1.fixes 2>/dev/null || echo 0)\n[ \"$fixes\" -ge 1 ] && exit 0\ni=1\nwhile [ $i -le 45 ]; do echo \"line $i\"; i=$((i+1)); done\nexec sleep 30\n";

#[test]
fn a_timed_out_check_is_one_task_finding_with_the_timeout_and_output_tail() {
    let repo = project("in-progress", single_task());
    repo.write(".fixture/fake-check.sh", HANGING_CHECK);
    let check = repo.path(".fixture/fake-check.sh");
    std::fs::set_permissions(&check, std::fs::Permissions::from_mode(0o755)).unwrap();
    let command = check.display().to_string();
    install_fakes_with(
        &repo,
        "",
        &format!("check = \"{command}\"\ntimeout = 3\nreview_rounds = 2\n"),
    );
    repo.commit("check");
    one_task_fixtures(&repo);
    fix_reply(&repo, "T1", 1, &[REOPEN]);

    let run = build(&repo, &[]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert!(
        run.stdout.contains("   check: 1 finding, fix round 1\n"),
        "{}",
        run.stdout
    );
    let tail: String = (6..=45).map(|i| format!("\nline {i}")).collect();
    assert_eq!(
        fix_findings(&repo),
        format!("- [task] check: {command} timed out after 3s{tail}")
    );
}

#[test]
fn a_check_that_cannot_start_stops_the_build_with_that_message_and_no_fix_round() {
    let repo = project("in-progress", single_task());
    let command = repo.path(".fixture/missing-check").display().to_string();
    install_fakes_with(
        &repo,
        "",
        &format!("check = \"{command}\"\nreview_rounds = 2\n"),
    );
    repo.commit("check");
    one_task_fixtures(&repo);
    fix_reply(&repo, "T1", 1, &[REOPEN]);
    let before = repo.head();

    let run = build(&repo, &[]);

    assert_eq!(run.code, 2, "{}", run.stdout);
    assert!(
        run.stdout
            .contains(&format!("   check: cannot start `{command}`: ")),
        "{}",
        run.stdout
    );
    assert!(!run.stdout.contains("fix round"), "{}", run.stdout);
    assert!(
        run.stdout
            .ends_with("   stopped. T1 left unticked; working tree kept.\n"),
        "{}",
        run.stdout
    );
    assert!(!repo.path(".fixture/T1.fixes").exists());
    assert!(!repo.path(".fixture/T1.rcalls").exists());
    assert_eq!(tasks(&repo), single_task());
    assert_eq!(repo.head(), before);
}

#[test]
fn a_claim_id_in_the_check_output_does_not_pin_the_finding_to_that_claim() {
    let repo = project("in-progress", single_task());
    with_failing_check(&repo, 1, "review_rounds = 2\n");
    one_task_fixtures(&repo);
    fix_reply(&repo, "T1", 1, &[REOPEN]);

    let run = build(&repo, &[]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    let findings = fix_findings(&repo);
    assert!(findings.contains("\nAC-1 still fails"), "{findings}");
    assert!(findings.starts_with("- [task] check: "), "{findings}");
    assert!(!findings.contains("- [AC-1]"), "{findings}");
}

#[test]
fn a_check_failure_spends_review_rounds_and_stops_unticked_when_they_run_out() {
    let repo = project("in-progress", single_task());
    with_failing_check(&repo, 99, "review_rounds = 1\n");
    one_task_fixtures(&repo);
    fix_reply(&repo, "T1", 1, &[REOPEN]);
    let before = repo.head();

    let run = build(&repo, &[]);

    assert_eq!(run.code, 2, "{}", run.stdout);
    assert!(
        run.stdout.contains("   check: 1 finding, fix round 1\n"),
        "{}",
        run.stdout
    );
    assert!(
        run.stdout
            .ends_with("   stopped. T1 left unticked; working tree kept.\n"),
        "{}",
        run.stdout
    );
    assert_eq!(repo.read(".fixture/T1.fixes").trim(), "1");
    assert!(!repo.path(".fixture/T1.rcalls").exists());
    assert_eq!(tasks(&repo), single_task());
    assert_eq!(repo.head(), before);
    let record = record(&repo);
    assert_eq!(record["ending"], "unticked");
    assert_eq!(record["findings"].as_array().unwrap().len(), 1);
    assert_eq!(record["findings"][0]["basis"], "task");
}

#[test]
fn a_blocked_reply_stops_at_once_without_a_fix_round() {
    let repo = project("in-progress", single_task());
    fixture(&repo, "T1", CLAIMS_PATH, &claims(true, false));
    blocked(&repo, "T1", "needs a human");
    fix_reply(&repo, "T1", 1, &[REOPEN]);

    let run = build(&repo, &[]);

    assert_eq!(run.code, 2, "{}", run.stdout);
    assert!(
        run.stdout.contains("   runner blocked: needs a human\n"),
        "{}",
        run.stdout
    );
    assert!(!run.stdout.contains("fix round"), "{}", run.stdout);
    assert!(!repo.path(".fixture/T1.fixes").exists());
    assert_eq!(record(&repo)["ending"], "unticked");
}

#[test]
fn a_ref_to_a_claim_the_task_does_not_settle_stops() {
    let repo = project("in-progress", TWO_TASKS);
    done(&repo, "T1", &[REOPEN, OTHER]);

    let run = build(&repo, &[]);

    assert_eq!(run.code, 2, "{}{}", run.stdout, run.stderr);
    assert!(
        run.stdout.contains("T1 does not settle AC-2"),
        "{}",
        run.stdout
    );
    assert_eq!(repo.read(CLAIMS_PATH), BARE);
}

#[test]
fn a_runner_with_session_prints_the_watch_line_before_the_report() {
    let repo = project("in-progress", TWO_TASKS);
    two_task_fixtures(&repo);
    install_fakes(&repo, " {session}");
    repo.commit("session runner");

    let run = build(&repo, &["--task", "T1"]);

    let lines: Vec<&str> = run.stdout.lines().collect();
    assert_eq!(lines[0], "T1 reopen (settles AC-1)");
    let watch = lines[1];
    assert!(
        watch.starts_with("   watch: tail -n +1 -F \"$HOME/.claude/projects/-")
            && watch.contains(".jsonl\" | jq -r '"),
        "{}",
        run.stdout
    );
    let session = watch
        .split(".jsonl")
        .next()
        .and_then(|head| head.rsplit('/').next())
        .unwrap();
    assert_eq!(session.len(), 36, "{session}");
    assert!(lines[2].starts_with("   runner: "), "{}", run.stdout);
}

#[test]
fn a_runner_without_session_prints_no_watch_line() {
    let repo = project("in-progress", TWO_TASKS);
    two_task_fixtures(&repo);

    let run = build(&repo, &["--task", "T1"]);

    assert!(!run.stdout.contains("watch:"), "{}", run.stdout);
}

fn review_reply(repo: &Repo, task: &str, stop: &str, findings: &[(&str, &str)]) {
    let findings: Vec<Value> = findings
        .iter()
        .map(|(basis, text)| json!({"basis": basis, "text": text}))
        .collect();
    let output = json!({"structured_output": {"stop": stop, "findings": findings}});
    repo.write(&format!(".fixture/{task}.review.json"), &output.to_string());
}

fn one_task_fixtures(repo: &Repo) {
    fixture(repo, "T1", "src/one.rs", "pub fn one() {}\n");
    done(repo, "T1", &[REOPEN]);
}

#[test]
fn a_clean_review_commits_with_the_review_trailer() {
    let repo = project("in-progress", TWO_TASKS);
    one_task_fixtures(&repo);

    let run = build(&repo, &["--task", "T1"]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert!(run.stdout.contains("   review: clean\n"), "{}", run.stdout);
    let message = repo.git(&["log", "-1", "--format=%B"]);
    assert_eq!(
        message.trim_end(),
        "DELTA-001 T1: reopen\n\nLemma-Review: clean"
    );
}

#[test]
fn the_basis_enum_is_the_ids_in_the_review_context() {
    let repo = project("in-progress", TWO_TASKS);
    repo.dictum_file(&["src/**"], "- [TEST-1] [intent] Tests are real.\n");
    repo.commit("dictum");
    one_task_fixtures(&repo);

    let run = build(&repo, &["--task", "T1"]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    let schema: Value = serde_json::from_str(&repo.read(".fixture/T1.schema")).unwrap();
    let finding = &schema["properties"]["findings"]["items"];
    assert_eq!(
        finding["properties"]["basis"]["enum"],
        json!(["task", "design", "convention", "AC-1", "AC-2", "TEST-1"])
    );
    assert_eq!(schema["required"], json!(["findings", "stop"]));
}

#[test]
fn a_finding_with_a_basis_outside_the_enum_is_rejected() {
    let repo = project("in-progress", TWO_TASKS);
    one_task_fixtures(&repo);
    review_reply(&repo, "T1", "", &[("taste", "rename it")]);
    let before = repo.head();

    let run = build(&repo, &["--task", "T1"]);

    assert_eq!(run.code, 2, "{}", run.stdout);
    assert!(
        run.stdout
            .contains("   review: reviewer reply: unknown basis `taste`\n"),
        "{}",
        run.stdout
    );
    assert_eq!(tasks(&repo), TWO_TASKS);
    assert_eq!(repo.head(), before);
}

#[test]
fn a_stop_ends_the_build_with_the_tree_kept_and_the_task_unticked() {
    let repo = project("in-progress", TWO_TASKS);
    one_task_fixtures(&repo);
    review_reply(&repo, "T1", "this is a different design", &[]);
    let before = repo.head();

    let run = build(&repo, &[]);

    assert_eq!(run.code, 2, "{}", run.stdout);
    assert!(
        run.stdout
            .ends_with("   stopped. T1 review: this is a different design; working tree kept.\n"),
        "{}",
        run.stdout
    );
    assert_eq!(tasks(&repo), TWO_TASKS);
    assert_eq!(repo.read("src/one.rs"), "pub fn one() {}\n");
    assert_eq!(repo.head(), before);
}

#[test]
fn findings_with_an_empty_stop_list_each_finding_and_stop() {
    let repo = project("in-progress", TWO_TASKS);
    install_fakes_with(&repo, "", "review_rounds = 0\n");
    repo.commit("no fix rounds");
    one_task_fixtures(&repo);
    review_reply(
        &repo,
        "T1",
        "",
        &[
            ("task", "back out the helper"),
            ("AC-2", "atomic is not proven"),
        ],
    );
    let before = repo.head();

    let run = build(&repo, &[]);

    assert_eq!(run.code, 2, "{}", run.stdout);
    assert!(
        run.stdout
            .contains("   task: back out the helper\n   AC-2: atomic is not proven\n"),
        "{}",
        run.stdout
    );
    assert!(
        run.stdout
            .ends_with("   stopped. T1 left unticked after 0 fix rounds; working tree kept.\n"),
        "{}",
        run.stdout
    );
    assert_eq!(tasks(&repo), TWO_TASKS);
    assert_eq!(repo.head(), before);
}

#[test]
fn the_task_diff_includes_files_the_task_created() {
    let repo = project("in-progress", TWO_TASKS);
    one_task_fixtures(&repo);

    let run = build(&repo, &["--task", "T1"]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    let prompt = repo.read(".fixture/T1.review-prompt");
    let task_diff = prompt.split("# Task diff").nth(1).unwrap();
    assert!(task_diff.contains("+++ b/src/one.rs"), "{task_diff}");
    assert!(task_diff.contains("+pub fn one() {}"), "{task_diff}");
}

#[test]
fn the_reviewer_sees_earlier_work_as_read_only_context() {
    let repo = project("in-progress", TWO_TASKS);
    two_task_fixtures(&repo);

    let run = build(&repo, &[]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    let prompt = repo.read(".fixture/T2.review-prompt");
    let (context, task_diff) = prompt.split_once("# Task diff").unwrap();
    let context = context
        .split("# Delta diff so far (read-only context)")
        .nth(1)
        .unwrap();
    assert!(context.contains("+pub fn one() {}"), "{context}");
    assert!(!context.contains("pub fn two()"), "{context}");
    assert!(task_diff.contains("+pub fn two() {}"), "{task_diff}");
    assert!(!task_diff.contains("pub fn one()"), "{task_diff}");
}

fn fix_prompt(repo: &Repo) -> String {
    repo.read(".fixture/T1.prompt.1")
}

fn fix_findings(repo: &Repo) -> String {
    let fix = fix_prompt(repo);
    let start = fix.find("and found the following.").unwrap();
    let end = start + fix[start..].find("\nRules:").unwrap();
    fix[start..end]
        .lines()
        .skip(1)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn fix_reply(repo: &Repo, task: &str, round: u32, refs: &[(&str, &str, &str)]) {
    done(repo, &format!("{task}.fix{round}"), refs);
}

fn review_round(repo: &Repo, task: &str, round: u32, findings: &[(&str, &str)]) {
    let name = match round {
        1 => format!("{task}.review"),
        n => format!("{task}.review{n}"),
    };
    let findings: Vec<Value> = findings
        .iter()
        .map(|(basis, text)| json!({"basis": basis, "text": text}))
        .collect();
    let output = json!({"structured_output": {"stop": "", "findings": findings}});
    repo.write(&format!(".fixture/{name}.json"), &output.to_string());
}

const HELPER: (&str, &str) = ("task", "back out the helper");
const ATOMIC: (&str, &str) = ("AC-2", "atomic is not proven");
const OTHER_REF: (&str, &str, &str) = ("AC-1", "tests/store_test.rs", "other_test");

fn single_task() -> &'static str {
    "- [ ] T1 reopen (settles AC-1)\n"
}

#[test]
fn findings_go_back_to_the_builder_and_a_clean_second_review_commits() {
    let repo = project("in-progress", single_task());
    one_task_fixtures(&repo);
    review_round(&repo, "T1", 1, &[HELPER, ATOMIC]);
    fixture(
        &repo,
        "T1.fix1",
        "src/one.rs",
        "pub fn one() { /* fixed */ }\n",
    );
    fix_reply(&repo, "T1", 1, &[OTHER_REF]);

    let run = build(&repo, &[]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert!(
        run.stdout.contains("   review: 2 findings, fix round 1\n"),
        "{}",
        run.stdout
    );
    assert!(run.stdout.contains("   fix: "), "{}", run.stdout);
    assert!(run.stdout.contains("   review: clean\n"), "{}", run.stdout);
    assert_eq!(
        repo.git(&["log", "-1", "--format=%B"]).trim_end(),
        "DELTA-001 T1: reopen\n\nLemma-Review: 2 findings fixed in 1 round"
    );
    assert_eq!(tasks(&repo), "- [x] T1 reopen (settles AC-1)\n");
    assert_eq!(repo.git(&["status", "--porcelain"]), "");
    let second = repo.read(".fixture/T1.review-prompt.2");
    let task_diff = second.split("# Task diff").nth(1).unwrap();
    assert!(task_diff.contains("/* fixed */"), "{task_diff}");
}

#[test]
fn the_fix_reply_refs_replace_the_refs_the_build_reply_wrote() {
    let repo = project("in-progress", single_task());
    one_task_fixtures(&repo);
    review_round(&repo, "T1", 1, &[HELPER]);
    fix_reply(&repo, "T1", 1, &[OTHER_REF]);

    let run = build(&repo, &[]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert_eq!(
        repo.read(CLAIMS_PATH),
        format!(
            "- [AC-1] Reopen keeps the prior answer.\n{REF_OTHER}- [AC-2] Writes are atomic.\n"
        )
    );
}

const REF_OTHER: &str = "  @ref tests/store_test.rs#other_test\n";

#[test]
fn findings_left_after_the_last_round_stop_with_a_list_and_a_record_file() {
    let repo = project("in-progress", single_task());
    one_task_fixtures(&repo);
    review_round(&repo, "T1", 1, &[HELPER]);
    review_round(&repo, "T1", 2, &[ATOMIC, HELPER]);
    fixture(
        &repo,
        "T1.fix1",
        "src/one.rs",
        "pub fn one() { /* fixed */ }\n",
    );
    fix_reply(&repo, "T1", 1, &[REOPEN]);
    let before = repo.head();

    let run = build(&repo, &[]);

    assert_eq!(run.code, 2, "{}", run.stdout);
    assert!(
        run.stdout
            .contains("   AC-2: atomic is not proven\n   task: back out the helper\n"),
        "{}",
        run.stdout
    );
    assert!(
        run.stdout
            .ends_with("   stopped. T1 left unticked after 1 fix round; working tree kept.\n"),
        "{}",
        run.stdout
    );
    assert_eq!(tasks(&repo), single_task());
    assert_eq!(repo.head(), before);
    assert_eq!(repo.read("src/one.rs"), "pub fn one() { /* fixed */ }\n");
    let record: Value =
        serde_json::from_str(&repo.read(".git/lemma/builds/DELTA-001.json")).unwrap();
    assert_eq!(record["task"], "T1");
    assert_eq!(record["ending"], "reviewed");
    assert_eq!(
        record["findings"],
        json!([
            {"text": "atomic is not proven", "basis": "AC-2"},
            {"text": "back out the helper", "basis": "task"},
        ])
    );
    assert_eq!(repo.git(&["ls-files", ".git"]), "");
}

#[test]
fn a_stop_verdict_writes_the_record_file_too() {
    let repo = project("in-progress", single_task());
    one_task_fixtures(&repo);
    review_reply(&repo, "T1", "this is a different design", &[]);

    let run = build(&repo, &[]);

    assert_eq!(run.code, 2, "{}", run.stdout);
    let record: Value =
        serde_json::from_str(&repo.read(".git/lemma/builds/DELTA-001.json")).unwrap();
    assert_eq!(record["ending"], "reviewed");
    assert_eq!(
        record["findings"],
        json!([{"text": "this is a different design", "basis": "task"}])
    );
}

#[test]
fn the_resume_command_gets_the_session_the_build_call_ran_in() {
    let repo = project("in-progress", single_task());
    let resume = format!(
        "resume = \"{} resumed {{session}}\"\n",
        repo.path("fake-runner.sh").display()
    );
    install_fakes_with(&repo, " {session}", &resume);
    repo.commit("session runner");
    one_task_fixtures(&repo);
    review_round(&repo, "T1", 1, &[HELPER]);
    fix_reply(&repo, "T1", 1, &[REOPEN]);

    let run = build(&repo, &[]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    let session = repo.read(".fixture/T1.args.0");
    assert!(uuid_like(&session), "{session}");
    assert_eq!(
        repo.read(".fixture/T1.args.1"),
        format!("resumed {session}")
    );
    let fix = repo.read(".fixture/T1.prompt.1");
    assert!(fix.contains("- [task] back out the helper"), "{fix}");
    assert!(!fix.contains("# Convention"), "{fix}");
    assert_eq!(run.stdout.matches("watch: ").count(), 2, "{}", run.stdout);
}

#[test]
fn a_resume_command_without_session_makes_the_fix_a_fresh_runner_with_the_build_prompt() {
    let repo = project("in-progress", single_task());
    let resume = format!(
        "resume = \"{} resumed\"\n",
        repo.path("fake-runner.sh").display()
    );
    install_fakes_with(&repo, " {session}", &resume);
    repo.commit("session runner");
    one_task_fixtures(&repo);
    review_round(&repo, "T1", 1, &[HELPER]);
    fix_reply(&repo, "T1", 1, &[REOPEN]);

    let run = build(&repo, &[]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    let build_prompt = repo.read(".fixture/T1.prompt.0");
    let fix = repo.read(".fixture/T1.prompt.1");
    assert!(fix.starts_with(&build_prompt), "{fix}");
    assert!(uuid_like(&repo.read(".fixture/T1.args.1")));
}

#[test]
fn the_fix_line_counts_only_the_files_the_fix_changed() {
    let repo = project("in-progress", single_task());
    one_task_fixtures(&repo);
    fixture(&repo, "T1", "src/extra.rs", "pub fn extra() {}\n");
    review_round(&repo, "T1", 1, &[HELPER]);
    fixture(
        &repo,
        "T1.fix1",
        "src/one.rs",
        "pub fn one() { /* fixed */ }\n",
    );
    fix_reply(&repo, "T1", 1, &[REOPEN]);

    let run = build(&repo, &[]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    let line = |prefix: &str| {
        run.stdout
            .lines()
            .find(|l| l.starts_with(prefix))
            .unwrap()
            .to_string()
    };
    assert!(line("   runner: ").ends_with(", 2 files"), "{}", run.stdout);
    assert!(line("   fix: ").ends_with(", 1 files"), "{}", run.stdout);
}

fn uuid_like(text: &str) -> bool {
    text.len() == 36 && text.matches('-').count() == 4
}

#[test]
fn without_a_session_the_fix_is_a_fresh_runner_with_the_build_prompt_and_the_fix_section() {
    let repo = project("in-progress", single_task());
    one_task_fixtures(&repo);
    review_round(&repo, "T1", 1, &[HELPER]);
    fix_reply(&repo, "T1", 1, &[REOPEN]);

    let run = build(&repo, &[]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert_eq!(repo.read(".fixture/T1.args.1"), "");
    let build_prompt = repo.read(".fixture/T1.prompt.0");
    let fix = repo.read(".fixture/T1.prompt.1");
    assert!(fix.starts_with(&build_prompt), "{fix}");
    assert!(fix.contains("- [task] back out the helper"), "{fix}");
}

#[test]
fn a_fix_that_breaks_lint_after_the_last_round_stops_before_the_second_review() {
    let repo = project("in-progress", single_task());
    one_task_fixtures(&repo);
    review_round(&repo, "T1", 1, &[HELPER]);
    fix_reply(&repo, "T1", 1, &[MISSING]);
    let before = repo.head();

    let run = build(&repo, &[]);

    assert_eq!(run.code, 2, "{}", run.stdout);
    assert!(run.stdout.contains("   lint: "), "{}", run.stdout);
    assert!(run.stdout.contains("missing_fn"), "{}", run.stdout);
    assert!(
        run.stdout
            .ends_with("   stopped. T1 left unticked; working tree kept.\n"),
        "{}",
        run.stdout
    );
    assert_eq!(repo.read(".fixture/T1.rcalls").trim(), "1");
    assert_eq!(repo.read(".fixture/T1.fixes").trim(), "1");
    assert_eq!(tasks(&repo), single_task());
    assert_eq!(repo.head(), before);
    let record: Value =
        serde_json::from_str(&repo.read(".git/lemma/builds/DELTA-001.json")).unwrap();
    assert_eq!(record["ending"], "unticked");
    let first = record["findings"][0]["text"].as_str().unwrap();
    assert!(first.starts_with("lint: "), "{first}");
    assert!(first.contains("missing_fn"), "{first}");
}

#[test]
fn review_rounds_two_allows_two_fix_rounds() {
    let repo = project("in-progress", single_task());
    install_fakes_with(&repo, "", "review_rounds = 2\n");
    repo.commit("two rounds");
    one_task_fixtures(&repo);
    review_round(&repo, "T1", 1, &[HELPER, ATOMIC]);
    review_round(&repo, "T1", 2, &[HELPER]);
    fix_reply(&repo, "T1", 1, &[REOPEN]);
    fix_reply(&repo, "T1", 2, &[REOPEN]);

    let run = build(&repo, &[]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert!(
        run.stdout.contains("   review: 1 finding, fix round 2\n"),
        "{}",
        run.stdout
    );
    assert_eq!(
        repo.git(&["log", "-1", "--format=%B"]).trim_end(),
        "DELTA-001 T1: reopen\n\nLemma-Review: 3 findings fixed in 2 rounds"
    );
}

#[test]
fn a_struck_dictum_rule_is_not_a_basis() {
    let repo = project("in-progress", single_task());
    repo.dictum_file(
        &["src/**"],
        "- [TEST-1] [intent] Tests are real.\n- ~~[TEST-2]~~ [intent] Gone.\n",
    );
    repo.commit("dictum");
    one_task_fixtures(&repo);

    let run = build(&repo, &[]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    let schema: Value = serde_json::from_str(&repo.read(".fixture/T1.schema")).unwrap();
    assert_eq!(
        schema["properties"]["findings"]["items"]["properties"]["basis"]["enum"],
        json!(["task", "design", "convention", "AC-1", "AC-2", "TEST-1"])
    );
}

const RECORD: &str = ".git/lemma/builds/DELTA-001.json";

fn record(repo: &Repo) -> Value {
    serde_json::from_str(&repo.read(RECORD)).unwrap()
}

fn blocked(repo: &Repo, task: &str, reason: &str) {
    reply(repo, task, "blocked", reason, &[]);
}

fn discard(repo: &Repo) {
    repo.git(&["checkout", "--", "."]);
    repo.git(&["clean", "-fdq"]);
}

fn rfc3339_like(text: &str) -> bool {
    let bytes = text.as_bytes();
    text.len() == 20
        && text.ends_with('Z')
        && [4, 7].iter().all(|&i| bytes[i] == b'-')
        && bytes[10] == b'T'
        && [13, 16].iter().all(|&i| bytes[i] == b':')
}

#[test]
fn a_stop_writes_an_uncommitted_record_of_the_task_base_head_session_findings_tree_and_time() {
    let repo = project("in-progress", single_task());
    install_fakes_with(&repo, " {session}", "");
    repo.commit("session runner");
    fixture(&repo, "T1", "src/one.rs", "pub fn one() {}\n");
    blocked(&repo, "T1", "needs a human");
    let base = repo.git(&["rev-parse", "HEAD"]).trim().to_string();

    let run = build(&repo, &[]);

    assert_eq!(run.code, 2, "{}", run.stdout);
    let record = record(&repo);
    assert_eq!(record["task"], "T1");
    assert_eq!(record["ending"], "unticked");
    assert_eq!(record["base"], base.as_str());
    assert_eq!(record["head"], base.as_str());
    assert_eq!(record["session"], repo.read(".fixture/T1.args.0").as_str());
    assert_eq!(
        record["findings"],
        json!([{"text": "runner blocked: needs a human", "basis": "task"}])
    );
    let blob = repo.git(&["hash-object", "src/one.rs"]);
    assert_eq!(record["tree"], json!({"src/one.rs": blob.trim()}));
    let at = record["at"].as_str().unwrap();
    assert!(rfc3339_like(at), "{at}");
    assert_eq!(repo.git(&["ls-files", ".git"]), "");
    assert!(!repo.git(&["status", "--porcelain"]).contains("lemma"));
}

#[test]
fn a_stop_without_a_session_records_a_null_session() {
    let repo = project("in-progress", single_task());
    fixture(&repo, "T1", "src/one.rs", "pub fn one() {}\n");
    blocked(&repo, "T1", "needs a human");

    build(&repo, &[]);

    assert_eq!(record(&repo)["session"], Value::Null);
}

#[test]
fn a_stop_before_the_tick_records_the_ending_unticked() {
    let repo = project("in-progress", single_task());
    fixture(&repo, "T1", "src/one.rs", "pub fn one() {}\n");
    blocked(&repo, "T1", "needs a human");

    build(&repo, &[]);

    assert_eq!(tasks(&repo), single_task());
    assert_eq!(record(&repo)["ending"], "unticked");
}

#[test]
fn a_failed_commit_after_the_tick_records_the_ending_ticked() {
    let repo = project("in-progress", single_task());
    repo.write(".git/hooks/pre-commit", "#!/bin/sh\nexit 1\n");
    std::fs::set_permissions(
        repo.path(".git/hooks/pre-commit"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    one_task_fixtures(&repo);

    let run = build(&repo, &[]);

    assert_eq!(run.code, 2, "{}", run.stdout);
    assert_eq!(tasks(&repo), "- [x] T1 reopen (settles AC-1)\n");
    let record = record(&repo);
    assert_eq!(record["ending"], "ticked");
    assert!(
        record["findings"][0]["text"]
            .as_str()
            .unwrap()
            .starts_with("commit failed"),
        "{record}"
    );
}

#[test]
fn findings_left_after_review_rounds_record_the_ending_reviewed() {
    let repo = project("in-progress", single_task());
    one_task_fixtures(&repo);
    review_round(&repo, "T1", 1, &[HELPER]);
    review_round(&repo, "T1", 2, &[ATOMIC]);
    fix_reply(&repo, "T1", 1, &[REOPEN]);

    build(&repo, &[]);

    let record = record(&repo);
    assert_eq!(record["ending"], "reviewed");
    assert_eq!(
        record["findings"],
        json!([{"text": "atomic is not proven", "basis": "AC-2"}])
    );
}

#[test]
fn a_reviewer_stop_records_the_ending_reviewed() {
    let repo = project("in-progress", single_task());
    one_task_fixtures(&repo);
    review_reply(&repo, "T1", "this is a different design", &[]);

    build(&repo, &[]);

    assert_eq!(record(&repo)["ending"], "reviewed");
}

#[test]
fn a_second_stop_of_the_task_replaces_the_record() {
    let repo = project("in-progress", single_task());
    fixture(&repo, "T1", "src/one.rs", "pub fn one() {}\n");
    blocked(&repo, "T1", "first");
    build(&repo, &[]);
    discard(&repo);
    blocked(&repo, "T1", "second");

    let run = build(&repo, &[]);

    assert_eq!(run.code, 2, "{}", run.stdout);
    assert_eq!(
        record(&repo)["findings"],
        json!([{"text": "runner blocked: second", "basis": "task"}])
    );
}

#[test]
fn the_record_is_deleted_when_the_task_commits() {
    let repo = project("in-progress", single_task());
    fixture(&repo, "T1", "src/one.rs", "pub fn one() {}\n");
    blocked(&repo, "T1", "first");
    build(&repo, &[]);
    assert!(repo.path(RECORD).exists());
    discard(&repo);
    done(&repo, "T1", &[REOPEN]);

    let run = build(&repo, &[]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert!(!repo.path(RECORD).exists());
}

#[test]
fn build_writes_no_per_task_review_file_and_leaves_the_review_record_alone() {
    let repo = project("in-progress", single_task());
    let resolved = "[{\"part\":\"design\",\"anchor\":\"x\",\"text\":\"y\"}]";
    repo.write(".git/lemma/reviews/DELTA-001.json", resolved);
    one_task_fixtures(&repo);
    review_reply(&repo, "T1", "", &[HELPER]);
    review_round(&repo, "T1", 2, &[HELPER]);
    fix_reply(&repo, "T1", 1, &[REOPEN]);

    let run = build(&repo, &[]);

    assert_eq!(run.code, 2, "{}", run.stdout);
    assert!(!repo.path(".git/lemma/reviews/DELTA-001-T1.json").exists());
    assert_eq!(repo.read(".git/lemma/reviews/DELTA-001.json"), resolved);
}

#[test]
fn a_runner_call_that_fails_still_records_its_session() {
    let repo = project("in-progress", single_task());
    install_fakes_with(&repo, " {session}", "");
    repo.commit("session runner");

    let run = build(&repo, &[]);

    assert_eq!(run.code, 2, "{}", run.stdout);
    let session = repo.read(".fixture/T1.args.0");
    assert!(uuid_like(&session), "{session}");
    assert_eq!(record(&repo)["session"], session.as_str());
}

fn stop_t1(repo: &Repo) -> Value {
    fixture(repo, "T1", "src/one.rs", "pub fn one() {}\n");
    blocked(repo, "T1", "needs a human");
    let run = build(repo, &[]);
    assert_eq!(run.code, 2, "{}", run.stdout);
    done(repo, "T1", &[REOPEN]);
    fix_reply(repo, "T1", 1, &[REOPEN]);
    record(repo)
}

#[test]
fn a_dirty_tree_with_a_stop_record_is_refused_naming_the_stop() {
    let repo = project("in-progress", single_task());
    let stopped = stop_t1(&repo);

    let run = build(&repo, &[]);

    assert_eq!(run.code, 2, "{}", run.stdout);
    let at = stopped["at"].as_str().unwrap();
    assert!(
        run.stderr.contains(&format!(
            "lemma: T1 stopped {at}: runner blocked: needs a human; rerun with --resume, or clean the tree to start T1 fresh"
        )),
        "{}",
        run.stderr
    );
    assert_eq!(tasks(&repo), single_task());
    assert!(repo.path(RECORD).exists());
}

#[test]
fn a_dirty_tree_without_a_stop_record_is_refused_as_before() {
    let repo = project("in-progress", single_task());
    repo.write("stray.txt", "x");

    let run = build(&repo, &[]);

    assert_eq!(run.code, 2);
    assert!(!repo.path(RECORD).exists());
    assert!(run
        .stderr
        .contains("lemma: build needs a clean working tree; commit or stash first"));
}

#[test]
fn a_clean_tree_with_a_stop_record_deletes_it_and_starts_the_task_fresh() {
    let repo = project("in-progress", single_task());
    stop_t1(&repo);
    discard(&repo);
    let probe = format!(
        "#!/bin/sh\nmkdir -p .fixture\nif [ -e {RECORD} ]; then echo present; else echo absent; fi > .fixture/record-seen\nexec {} \"$@\"\n",
        repo.path("fake-runner.sh").display()
    );
    repo.write("probe-runner.sh", &probe);
    std::fs::set_permissions(
        repo.path("probe-runner.sh"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    repo.write(
        "lemma.toml",
        &format!(
            "runner = \"{}\"\nreviewer = \"{} {{schema_file}}\"\n",
            repo.path("probe-runner.sh").display(),
            repo.path("fake-reviewer.sh").display()
        ),
    );
    repo.commit("probe runner");
    assert!(repo.path(RECORD).exists());

    let run = build(&repo, &[]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert!(
        run.stdout.starts_with("T1 reopen (settles AC-1)\n"),
        "{}",
        run.stdout
    );
    assert_eq!(repo.read(".fixture/record-seen"), "absent\n");
    assert!(!repo
        .read(".fixture/T1.prompt.0")
        .contains("A reviewer read your work"));
}

#[test]
fn resume_without_a_stop_record_has_nothing_to_resume() {
    let repo = project("in-progress", single_task());

    let run = build(&repo, &["--resume"]);

    assert_eq!(run.code, 2, "{}", run.stdout);
    assert!(run.stderr.contains("nothing to resume"), "{}", run.stderr);
    assert_eq!(subjects(&repo)[0], "start");
}

#[test]
fn resume_names_both_tasks_when_the_record_is_not_for_the_task_given_or_next() {
    let repo = project("in-progress", TWO_TASKS);
    stop_t1(&repo);

    let given = build(&repo, &["--resume", "--task", "T2"]);

    assert_eq!(given.code, 2, "{}", given.stdout);
    assert!(
        given
            .stderr
            .contains("cannot resume: the stop record is for T1, not --task T2"),
        "{}",
        given.stderr
    );

    let mut moved = record(&repo);
    moved["task"] = json!("T2");
    repo.write(RECORD, &moved.to_string());

    let next = build(&repo, &["--resume"]);

    assert_eq!(next.code, 2, "{}", next.stdout);
    assert!(
        next.stderr
            .contains("cannot resume: the stop record is for T2, not the next unticked task T1"),
        "{}",
        next.stderr
    );
    assert_eq!(subjects(&repo)[0], "start");
}

#[test]
fn resume_is_refused_with_both_shas_when_history_changed_under_the_task() {
    let repo = project("in-progress", single_task());
    let stopped = stop_t1(&repo);
    repo.git(&[
        "commit",
        "-q",
        "--amend",
        "--allow-empty",
        "-m",
        "rewritten",
    ]);
    let head = repo.head();

    let run = build(&repo, &["--resume"]);

    assert_eq!(run.code, 2, "{}", run.stdout);
    let at = stopped["head"].as_str().unwrap();
    assert!(run.stderr.contains(at), "{}", run.stderr);
    assert!(run.stderr.contains(&head), "{}", run.stderr);
    assert!(
        run.stderr.contains("not an ancestor of HEAD"),
        "{}",
        run.stderr
    );
    assert_eq!(repo.head(), head);
    assert_eq!(repo.read("src/one.rs"), "pub fn one() {}\n");
    assert_eq!(tasks(&repo), single_task());
}

#[test]
fn resume_is_allowed_after_commits_on_top_of_the_stop() {
    let repo = project("in-progress", single_task());
    stop_t1(&repo);
    repo.write("notes.txt", "during the stop\n");
    repo.git(&["add", "notes.txt"]);
    repo.git(&["commit", "-q", "-m", "human note"]);

    let run = build(&repo, &["--resume"]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert_eq!(subjects(&repo)[..2], ["DELTA-001 T1: reopen", "human note"]);
    assert!(!repo.path(RECORD).exists());
}

#[test]
fn resume_accepts_a_ticked_stop_naming_the_ticked_task() {
    let repo = project("in-progress", TWO_TASKS);
    let hook = ".git/hooks/pre-commit";
    repo.write(hook, "#!/bin/sh\nexit 1\n");
    std::fs::set_permissions(repo.path(hook), std::fs::Permissions::from_mode(0o755)).unwrap();
    two_task_fixtures(&repo);
    assert_eq!(build(&repo, &[]).code, 2);
    assert_eq!(record(&repo)["ending"], "ticked");
    assert!(tasks(&repo).starts_with("- [x] T1"));
    std::fs::remove_file(repo.path(hook)).unwrap();

    let run = build(&repo, &["--resume"]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert!(!run.stderr.contains("cannot resume"), "{}", run.stderr);
    assert_eq!(
        subjects(&repo)[..2],
        ["DELTA-001 T2: atomic", "DELTA-001 T1: reopen"]
    );
}

#[test]
fn resuming_a_ticked_stop_retries_only_the_commit_then_continues() {
    let repo = project("in-progress", TWO_TASKS);
    let hook = ".git/hooks/pre-commit";
    repo.write(hook, "#!/bin/sh\nexit 1\n");
    std::fs::set_permissions(repo.path(hook), std::fs::Permissions::from_mode(0o755)).unwrap();
    two_task_fixtures(&repo);
    assert_eq!(build(&repo, &[]).code, 2);
    assert_eq!(record(&repo)["ending"], "ticked");
    std::fs::remove_file(repo.path(hook)).unwrap();
    std::fs::remove_file(repo.path(".fixture/T1.prompt.0")).unwrap();
    let reviews = repo.read(".fixture/T1.rcalls");

    let run = build(&repo, &["--resume"]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert!(!repo.path(".fixture/T1.prompt.0").exists(), "runner ran");
    assert_eq!(repo.read(".fixture/T1.rcalls"), reviews, "review ran");
    assert!(
        run.stdout.contains("   retrying the commit\n"),
        "{}",
        run.stdout
    );
    assert_eq!(
        subjects(&repo)[..2],
        ["DELTA-001 T2: atomic", "DELTA-001 T1: reopen"]
    );
    let t1 = repo.git(&["show", "--name-only", "--format=", "HEAD~1"]);
    assert!(t1.contains("src/one.rs"), "{t1}");
    assert!(repo.path(".fixture/T2.prompt.0").exists());
    assert!(!repo.path(RECORD).exists());
    assert_eq!(repo.git(&["status", "--porcelain"]), "");
}

#[test]
fn a_resume_prints_the_stop_and_the_paths_changed_since_it() {
    let repo = project("in-progress", single_task());
    fixture(&repo, "T1", "src/kept.rs", "pub fn kept() {}\n");
    let stopped = stop_t1(&repo);
    repo.write("src/one.rs", "pub fn one() { /* human */ }\n");
    repo.write("src/human.rs", "pub fn human() {}\n");
    repo.write("notes.txt", "during the stop\n");
    repo.git(&["add", "notes.txt"]);
    repo.git(&["commit", "-q", "-m", "human note"]);

    let run = build(&repo, &["--resume"]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    let at = stopped["at"].as_str().unwrap();
    assert!(
        run.stdout.contains(&format!(
            "   resuming T1 (stopped {at}: runner blocked: needs a human)\n"
        )),
        "{}",
        run.stdout
    );
    let changed: Vec<&str> = run
        .stdout
        .lines()
        .filter_map(|l| l.strip_prefix("   changed since the stop: "))
        .collect();
    assert_eq!(changed, ["notes.txt", "src/human.rs", "src/one.rs"]);
}

#[test]
fn a_resume_whose_checks_pass_reviews_then_ticks_and_commits() {
    let repo = project("in-progress", single_task());
    stop_t1(&repo);
    repo.write(CLAIMS_PATH, &claims(true, false));
    std::fs::remove_file(repo.path(".fixture/T1.prompt.0")).unwrap();

    let run = build(&repo, &["--resume"]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert!(!repo.path(".fixture/T1.prompt.0").exists(), "runner ran");
    assert!(!repo.path(".fixture/T1.prompt.1").exists(), "fix ran");
    assert_eq!(repo.read(".fixture/T1.rcalls"), "1\n");
    let review = repo.read(".fixture/T1.review-prompt.1");
    let task_diff = review.split("# Task diff").nth(1).unwrap();
    assert!(task_diff.contains("+pub fn one() {}"), "{task_diff}");
    let checks = run
        .stdout
        .find("   lint: clean\n   AC-1 @ref tests/store_test.rs#reopen_keeps_prior resolves\n")
        .expect(&run.stdout);
    let review = run.stdout.find("   review: clean\n").expect(&run.stdout);
    assert!(checks < review, "{}", run.stdout);
    assert_eq!(tasks(&repo), "- [x] T1 reopen (settles AC-1)\n");
    assert_eq!(subjects(&repo)[0], "DELTA-001 T1: reopen");
    assert_eq!(repo.git(&["status", "--porcelain"]), "");
}

#[test]
fn a_resumed_task_runs_the_check_before_its_review() {
    let repo = project("in-progress", single_task());
    with_check(&repo, "");
    stop_t1(&repo);
    repo.write(CLAIMS_PATH, &claims(true, false));
    let _ = std::fs::remove_file(repo.path(".fixture/check.log"));

    let run = build(&repo, &["--resume"]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    let root = std::fs::canonicalize(repo.path(".")).unwrap();
    assert_eq!(
        repo.read(".fixture/check.log"),
        format!("{} fixes=0 rcalls=0\n", root.display())
    );
    let passed = run.stdout.find("   check: pass\n").expect(&run.stdout);
    let reviewed = run.stdout.find("   review: clean\n").expect(&run.stdout);
    assert!(passed < reviewed, "{}", run.stdout);
    assert_eq!(tasks(&repo), "- [x] T1 reopen (settles AC-1)\n");
}

#[test]
fn a_resume_after_the_cap_gets_a_fresh_review_rounds_budget() {
    let repo = project("in-progress", single_task());
    one_task_fixtures(&repo);
    review_round(&repo, "T1", 1, &[HELPER]);
    review_round(&repo, "T1", 2, &[ATOMIC]);
    fix_reply(&repo, "T1", 1, &[REOPEN]);
    assert_eq!(build(&repo, &[]).code, 2);
    assert_eq!(record(&repo)["ending"], "reviewed");
    review_round(&repo, "T1", 3, &[HELPER]);
    fix_reply(&repo, "T1", 2, &[REOPEN]);

    let run = build(&repo, &["--resume"]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert!(
        run.stdout.contains("   review: 1 finding, fix round 1\n"),
        "{}",
        run.stdout
    );
    assert_eq!(
        repo.git(&["log", "-1", "--format=%B"]).trim_end(),
        "DELTA-001 T1: reopen\n\nLemma-Review: 1 finding fixed in 1 round"
    );
}

#[test]
fn a_resume_check_fix_round_spends_the_budget_reviewer_findings_would_use() {
    let repo = project("in-progress", single_task());
    stop_t1(&repo);
    review_round(&repo, "T1", 1, &[HELPER]);

    let run = build(&repo, &["--resume"]);

    assert_eq!(run.code, 2, "{}", run.stdout);
    assert!(
        run.stdout.contains("   check: 1 finding, fix round 1\n"),
        "{}",
        run.stdout
    );
    assert!(
        run.stdout
            .ends_with("   stopped. T1 left unticked after 1 fix round; working tree kept.\n"),
        "{}",
        run.stdout
    );
    assert_eq!(record(&repo)["ending"], "reviewed");
}

#[test]
fn a_resume_whose_checks_fail_fixes_through_the_recorded_session() {
    let repo = project("in-progress", single_task());
    let resume = format!(
        "resume = \"{} resumed {{session}}\"\n",
        repo.path("fake-runner.sh").display()
    );
    install_fakes_with(&repo, " {session}", &resume);
    repo.commit("session runner");
    let stopped = stop_t1(&repo);
    let session = stopped["session"].as_str().unwrap().to_string();
    assert!(uuid_like(&session), "{session}");

    let run = build(&repo, &["--resume"]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert_eq!(
        repo.read(".fixture/T1.args.1"),
        format!("resumed {session}")
    );
    let fix = repo.read(".fixture/T1.prompt.1");
    assert!(fix.contains("- [AC-1] AC-1 has no @ref"), "{fix}");
    assert!(!fix.contains("# Convention"), "{fix}");
    assert_eq!(subjects(&repo)[0], "DELTA-001 T1: reopen");
}

#[test]
fn a_resume_without_a_recorded_session_fixes_in_a_fresh_runner() {
    let repo = project("in-progress", single_task());
    let stopped = stop_t1(&repo);
    assert_eq!(stopped["session"], Value::Null);
    let build_prompt = repo.read(".fixture/T1.prompt.0");

    let run = build(&repo, &["--resume"]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    let fix = repo.read(".fixture/T1.prompt.1");
    assert!(fix.starts_with(&build_prompt), "{fix}");
    assert!(fix.contains("- [AC-1] AC-1 has no @ref"), "{fix}");
}

#[test]
fn a_resume_whose_commands_lack_session_fixes_in_a_fresh_runner() {
    let repo = project("in-progress", single_task());
    let resume = format!(
        "resume = \"{} resumed\"\n",
        repo.path("fake-runner.sh").display()
    );
    install_fakes_with(&repo, " {session}", &resume);
    repo.commit("session runner");
    let stopped = stop_t1(&repo);
    let session = stopped["session"].as_str().unwrap();
    let build_prompt = repo.read(".fixture/T1.prompt.0");

    let run = build(&repo, &["--resume"]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    let fix = repo.read(".fixture/T1.prompt.1");
    assert!(fix.starts_with(&build_prompt), "{fix}");
    let args = repo.read(".fixture/T1.args.1");
    assert!(uuid_like(&args), "{args}");
    assert_ne!(args, session);
}

fn resume_after_human_work(repo: &Repo) {
    stop_t1(repo);
    repo.write("notes.txt", "during the stop\n");
    repo.git(&["add", "notes.txt"]);
    repo.git(&["commit", "-q", "-m", "human note"]);
    repo.write("src/human.rs", "pub fn human() {}\n");
    let run = build(repo, &["--resume"]);
    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
}

fn last_task_diff(repo: &Repo, task: &str) -> String {
    let calls = repo.read(&format!(".fixture/{task}.rcalls"));
    let prompt = repo.read(&format!(".fixture/{task}.review-prompt.{}", calls.trim()));
    prompt.split("# Task diff").nth(1).unwrap().to_string()
}

#[test]
fn a_resumed_review_diffs_from_the_task_base_so_commits_during_the_stop_are_reviewed() {
    let repo = project("in-progress", single_task());

    resume_after_human_work(&repo);

    let task_diff = last_task_diff(&repo, "T1");
    assert!(task_diff.contains("+++ b/notes.txt"), "{task_diff}");
    assert!(task_diff.contains("+during the stop"), "{task_diff}");
    assert!(task_diff.contains("+pub fn one() {}"), "{task_diff}");
}

#[test]
fn human_edits_outside_the_original_diff_are_in_the_resumed_task_diff() {
    let repo = project("in-progress", single_task());

    resume_after_human_work(&repo);

    let task_diff = last_task_diff(&repo, "T1");
    assert!(task_diff.contains("+++ b/src/human.rs"), "{task_diff}");
    assert!(task_diff.contains("+pub fn human() {}"), "{task_diff}");
}

#[test]
fn the_resumed_commit_holds_only_uncommitted_work_and_keeps_human_commits() {
    let repo = project("in-progress", single_task());

    resume_after_human_work(&repo);

    assert_eq!(
        &subjects(&repo)[..2],
        ["DELTA-001 T1: reopen", "human note"]
    );
    let committed = repo.git(&["show", "--name-only", "--format=", "HEAD"]);
    assert!(!committed.contains("notes.txt"), "{committed}");
    assert!(committed.contains("src/human.rs"), "{committed}");
    assert!(committed.contains("src/one.rs"), "{committed}");
    let human = repo.git(&["show", "--name-only", "--format=", "HEAD~1"]);
    assert_eq!(human.trim(), "notes.txt");
    assert_eq!(repo.git(&["status", "--porcelain"]), "");
}

#[test]
fn a_fresh_run_diffs_the_task_against_head() {
    let repo = project("in-progress", TWO_TASKS);
    two_task_fixtures(&repo);

    let run = build(&repo, &[]);

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    let task_diff = last_task_diff(&repo, "T2");
    assert!(task_diff.contains("+pub fn two() {}"), "{task_diff}");
    assert!(!task_diff.contains("pub fn one()"), "{task_diff}");
}

fn fail_commits(repo: &Repo) {
    repo.write(".git/hooks/pre-commit", "#!/bin/sh\nexit 1\n");
    std::fs::set_permissions(
        repo.path(".git/hooks/pre-commit"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();
}

#[test]
fn a_runner_that_cannot_start_is_an_error_that_leaves_a_resumable_record() {
    let repo = project("in-progress", single_task());
    let command = repo.path(".fixture/missing-runner").display().to_string();
    repo.write(
        "lemma.toml",
        &format!(
            "runner = \"{command}\"\nreviewer = \"{} {{schema_file}}\"\n",
            repo.path("fake-reviewer.sh").display()
        ),
    );
    repo.commit("missing runner");

    let run = build(&repo, &[]);

    assert_eq!(run.code, 2, "{}", run.stdout);
    assert!(
        run.stderr.contains(&format!(
            "lemma: T1 stopped on an environment error: runner: cannot start `{command}`: "
        )),
        "{}",
        run.stderr
    );
    let record = record(&repo);
    assert_eq!(record["task"], "T1");
    assert_eq!(record["ending"], "unticked");
}

#[test]
fn a_check_that_cannot_start_is_an_error_that_leaves_a_resumable_record() {
    let repo = project("in-progress", single_task());
    let command = repo.path(".fixture/missing-check").display().to_string();
    install_fakes_with(&repo, "", &format!("check = \"{command}\"\n"));
    repo.commit("check");
    one_task_fixtures(&repo);

    let run = build(&repo, &[]);

    assert_eq!(run.code, 2, "{}", run.stdout);
    assert!(
        run.stderr.contains(&format!(
            "lemma: T1 stopped on an environment error: check: cannot start `{command}`: "
        )),
        "{}",
        run.stderr
    );
    assert_eq!(record(&repo)["task"], "T1");
}

#[test]
fn a_failed_commit_is_an_error_and_resume_retries_the_commit() {
    let repo = project("in-progress", single_task());
    fail_commits(&repo);
    one_task_fixtures(&repo);

    let run = build(&repo, &[]);

    assert_eq!(run.code, 2, "{}", run.stdout);
    assert!(
        run.stderr
            .contains("lemma: T1 stopped on an environment error: commit failed"),
        "{}",
        run.stderr
    );
    assert_eq!(record(&repo)["ending"], "ticked");

    std::fs::remove_file(repo.path(".git/hooks/pre-commit")).unwrap();
    let resumed = build(&repo, &["--resume"]);

    assert_eq!(resumed.code, 0, "{}{}", resumed.stdout, resumed.stderr);
    assert_eq!(subjects(&repo)[0], "DELTA-001 T1: reopen");
    assert!(!repo.path(RECORD).exists());
}
