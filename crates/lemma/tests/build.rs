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
fn a_lint_error_stops_with_the_tree_kept() {
    let repo = project("in-progress", TWO_TASKS);
    let broken = "- [AC-1] Reopen keeps the prior answer.\n  @ref tests/store_test.rs#missing_fn\n- [AC-2] Writes are atomic.\n";
    fixture(&repo, "T1", "src/one.rs", "pub fn one() {}\n");
    done(
        &repo,
        "T1",
        &[("AC-1", "tests/store_test.rs", "missing_fn")],
    );
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
    done(&repo, "T1", &[]);
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
    repo.commit("human fix");

    let run = build(&repo, &[]);

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
fn a_runner_that_edits_a_delta_part_stops_the_build() {
    let repo = project("in-progress", TWO_TASKS);
    fixture(&repo, "T1", CLAIMS_PATH, &claims(true, false));
    fixture(&repo, "T1", "src/one.rs", "pub fn one() {}\n");
    done(&repo, "T1", &[REOPEN]);
    let before = repo.head();

    let run = build(&repo, &[]);

    assert_eq!(run.code, 2, "{}", run.stdout);
    assert!(
        run.stdout.contains(&format!(
            "   runner edited {CLAIMS_PATH}; lemma writes delta parts\n"
        )),
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
fn a_reply_with_no_json_stops_the_build() {
    let repo = project("in-progress", TWO_TASKS);
    repo.write(".fixture/T1.json", r#"{"result": "all done"}"#);

    let run = build(&repo, &[]);

    assert_eq!(run.code, 2, "{}", run.stdout);
    assert!(
        run.stdout
            .contains("   runner: runner reply has no JSON object\n"),
        "{}",
        run.stdout
    );
    assert_eq!(tasks(&repo), TWO_TASKS);
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
        serde_json::from_str(&repo.read(".git/lemma/reviews/DELTA-001-T1.json")).unwrap();
    assert_eq!(record["stop"], "");
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
        serde_json::from_str(&repo.read(".git/lemma/reviews/DELTA-001-T1.json")).unwrap();
    assert_eq!(record["stop"], "this is a different design");
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
fn a_fix_that_breaks_lint_stops_before_the_second_review() {
    let repo = project("in-progress", single_task());
    one_task_fixtures(&repo);
    review_round(&repo, "T1", 1, &[HELPER]);
    fix_reply(
        &repo,
        "T1",
        1,
        &[("AC-1", "tests/store_test.rs", "missing_fn")],
    );
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
    assert_eq!(tasks(&repo), single_task());
    assert_eq!(repo.head(), before);
    let record: Value =
        serde_json::from_str(&repo.read(".git/lemma/reviews/DELTA-001-T1.json")).unwrap();
    assert_eq!(
        record["findings"],
        json!([{"text": "back out the helper", "basis": "task"}])
    );
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
