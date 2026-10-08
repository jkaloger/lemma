mod common;
use common::*;
use serde_json::{json, Value};
use std::os::unix::fs::PermissionsExt;

const INDEX_BODY: &str = "## Intent\n\nDelta intent.\n";
const DESIGN: &str = "Design prose.\n";
const CLAIMS: &str = "- [AC-1] Reopen keeps the prior answer.\n- [AC-2] Writes are atomic.\n";
const MAIN_TASKS: &str = "- [x] T1 reopen (settles AC-1)\n- [ ] T2 atomic (settles AC-2)\n";

fn project(tasks: &str) -> Repo {
    let repo = Repo::new();
    repo.with_dictums();
    repo.spec_file(1, "active", &["a/**"], "## Intent\n\nSpec one intent.\n");
    repo.write("a/x.rs", "fn x() {}\n");
    repo.write("b/y.rs", "fn y() {}\n");
    repo.delta_file(1, "in-progress", &["SPEC-001"], INDEX_BODY);
    repo.delta_part(1, "design", DESIGN);
    repo.delta_claims(1, CLAIMS);
    repo.delta_tasks(1, tasks);
    repo.commit("start");
    repo
}

fn reviewer(repo: &Repo, body: &str) {
    let path = repo.path("reviewer.sh");
    repo.write("reviewer.sh", &format!("#!/bin/sh\n{body}\n"));
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    repo.write(
        "lemma.toml",
        &format!("reviewer = \"{}\"\n", path.display()),
    );
}

fn reply_with(repo: &Repo, reply: Value) {
    repo.write(
        "reply.json",
        &json!({ "result": reply.to_string() }).to_string(),
    );
    reviewer(repo, "cat > /dev/null\ncat reply.json");
}

fn review(repo: &Repo) -> Run {
    repo.run(&["review", "DELTA-001"], "")
}

fn tasks(repo: &Repo) -> String {
    repo.read(&delta_part_path(1, "tasks"))
}

fn two_findings() -> Value {
    json!({"findings": [
        {"text": "Prove   the reopen\nacross restarts.", "settles": ["AC-1", "AC-1", "AC-2"]},
        {"text": "Drop the unused helper."},
    ]})
}

#[test]
fn the_diff_is_scoped_to_governs_and_tests() {
    let repo = project(MAIN_TASKS);
    let base = repo.head();
    repo.git(&["checkout", "-q", "-b", "feature"]);
    repo.write("a/x.rs", "fn x() { governed_marker(); }\n");
    repo.write("tests/store_test.rs", "fn t() { tests_marker(); }\n");
    repo.write("b/y.rs", "fn y() { elsewhere_marker(); }\n");
    repo.commit("work");
    reviewer(
        &repo,
        "cat > stdin.txt\necho '{\"result\": \"{\\\"findings\\\": []}\"}'",
    );

    let run = review(&repo);

    assert_eq!(run.code, 0, "{}", run.stderr);
    assert!(run.stdout.starts_with(&format!(
        "DELTA-001 is in-progress: reviewing the diff against {}",
        &base[..base.len().min(8)]
    )));
    let prompt = repo.read("stdin.txt");
    assert!(prompt.contains("governed_marker"), "{prompt}");
    assert!(prompt.contains("tests_marker"), "{prompt}");
    assert!(!prompt.contains("elsewhere_marker"), "{prompt}");
}

#[test]
fn follow_ups_are_written_under_a_new_heading_with_settles() {
    let repo = project(MAIN_TASKS);
    reply_with(&repo, two_findings());

    let run = review(&repo);

    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(
        tasks(&repo),
        format!(
            "{MAIN_TASKS}\n## Follow-ups\n- [ ] T3 Prove the reopen across restarts. (settles AC-1, AC-2)\n- [ ] T4 Drop the unused helper. (chore)\n"
        )
    );
}

#[test]
fn follow_ups_continue_numbering_and_append_to_the_existing_section() {
    let existing =
        format!("{MAIN_TASKS}\n## Follow-ups\n- [x] T3 earlier (chore)\n\n## Notes\n\nKept.\n");
    let repo = project(&existing);
    reply_with(&repo, two_findings());

    let run = review(&repo);

    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(
        tasks(&repo),
        format!(
            "{MAIN_TASKS}\n## Follow-ups\n- [x] T3 earlier (chore)\n- [ ] T4 Prove the reopen across restarts. (settles AC-1, AC-2)\n- [ ] T5 Drop the unused helper. (chore)\n\n## Notes\n\nKept.\n"
        )
    );
}

#[test]
fn nothing_outside_tasks_is_written() {
    let repo = project(MAIN_TASKS);
    reply_with(&repo, two_findings());
    let before: Vec<String> = ["index", "design", "claims"]
        .iter()
        .map(|p| repo.read(&delta_part_path(1, p)))
        .collect();

    review(&repo);

    let after: Vec<String> = ["index", "design", "claims"]
        .iter()
        .map(|p| repo.read(&delta_part_path(1, p)))
        .collect();
    assert_eq!(before, after);
}

#[test]
fn the_count_is_reported_and_a_rerun_writes_nothing() {
    let repo = project(MAIN_TASKS);
    reply_with(&repo, two_findings());

    let first = review(&repo);
    let written = tasks(&repo);
    let second = review(&repo);

    assert!(first.stdout.contains("2 follow-up tasks written\n"));
    assert!(first
        .stdout
        .ends_with("next: delete the follow-ups you reject, then lemma build DELTA-001\n"));
    assert!(second.stdout.contains("0 follow-up tasks written\n"));
    assert!(second
        .stdout
        .ends_with("next: lazyspec update DELTA-001 --status complete\n"));
    assert_eq!(tasks(&repo), written);
}

#[test]
fn a_single_finding_is_singular() {
    let repo = project(MAIN_TASKS);
    reply_with(&repo, json!({"findings": [{"text": "One thing."}]}));

    let run = review(&repo);

    assert!(run.stdout.contains("1 follow-up task written\n"));
}

#[test]
fn a_reviewer_failure_exits_two_and_writes_nothing() {
    let repo = project(MAIN_TASKS);
    reviewer(&repo, "cat > /dev/null\necho broken >&2\nexit 3");

    let run = review(&repo);

    assert_eq!(run.code, 2);
    assert!(run.stderr.starts_with("lemma: "), "{}", run.stderr);
    assert_eq!(tasks(&repo), MAIN_TASKS);
}
