mod common;
use common::*;
use serde_json::{json, Value};
use std::os::unix::fs::PermissionsExt;

const INDEX_BODY: &str = "## Intent\n\nDelta intent.\n";
const DESIGN: &str = "Design prose.\n\n## Approach\n\nDo it.\n\n## Risks\n\nNone.\n";
const CLAIMS: &str = "- [AC-1] Reopen keeps the prior answer.\n  @ref tests/store_test.rs#reopen_keeps_prior\n- Reads are paged.\n- [AC-2] Writes are atomic.\n";
const TASKS: &str = "- [ ] T1 reopen (settles AC-1)\n- [ ] T2 paging (settles AC-2)\n";

const PARTS: [&str; 4] = ["index", "design", "claims", "tasks"];

fn project() -> Repo {
    let repo = Repo::new();
    repo.with_dictums();
    repo.spec_file(1, "active", &["a/**"], "## Intent\n\nSpec one intent.\n");
    repo.delta_file(1, "draft", &["SPEC-001"], INDEX_BODY);
    repo.delta_part(1, "design", DESIGN);
    repo.delta_claims(1, CLAIMS);
    repo.delta_tasks(1, TASKS);
    repo
}

fn script(repo: &Repo, body: &str) {
    let path = repo.path("reviewer.sh");
    repo.write(
        "reviewer.sh",
        &format!("#!/bin/sh\ncat > /dev/null\n{body}\n"),
    );
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
    script(repo, "cat reply.json");
}

fn finding(part: &str, anchor: &str, text: &str) -> Value {
    json!({"part": part, "anchor": anchor, "text": text})
}

fn review(repo: &Repo) -> Run {
    repo.run(&["review", "DELTA-001"], "")
}

fn snapshot(repo: &Repo) -> Vec<String> {
    let mut files = vec![repo.read(&delta_part_path(1, "index"))];
    files.extend(PARTS[1..].iter().map(|p| repo.read(&delta_part_path(1, p))));
    files
}

fn all_kinds() -> Value {
    json!({"findings": [
        finding("claims", "AC-2", "Say what atomic means."),
        finding("claims", "Reads are paged.", "Which page size?"),
        finding("tasks", "T1", "Split this."),
        finding("index", "Intent", "Name the user."),
        finding("design", "Approach", "Why this way?"),
        finding("design", "design", "Missing a diagram."),
    ]})
}

#[test]
fn findings_land_under_the_right_anchor_in_each_part() {
    let repo = project();
    reply_with(&repo, all_kinds());
    let index = repo.read(&delta_part_path(1, "index"));

    let run = review(&repo);

    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(
        run.stdout,
        "DELTA-001 is draft: reviewing the document\n6 review lines written\n"
    );
    assert_eq!(
        repo.read(&delta_part_path(1, "index")),
        format!("{index}\n> review: Name the user.\n")
    );
    assert_eq!(
        repo.read(&delta_part_path(1, "design")),
        "Design prose.\n\n## Approach\n\nDo it.\n\n> review: Why this way?\n\n## Risks\n\nNone.\n\n> review: Missing a diagram.\n"
    );
    assert_eq!(
        repo.read(&delta_part_path(1, "claims")),
        "- [AC-1] Reopen keeps the prior answer.\n  @ref tests/store_test.rs#reopen_keeps_prior\n- Reads are paged.\n  @review Which page size?\n- [AC-2] Writes are atomic.\n  @review Say what atomic means.\n"
    );
    assert_eq!(
        repo.read(&delta_part_path(1, "tasks")),
        "- [ ] T1 reopen (settles AC-1)\n  @review Split this.\n- [ ] T2 paging (settles AC-2)\n"
    );
}

#[test]
fn a_missing_part_file_is_created() {
    let repo = project();
    std::fs::remove_file(repo.path(&delta_part_path(1, "design"))).unwrap();
    reply_with(
        &repo,
        json!({"findings": [finding("design", "", "Write a design.")]}),
    );

    let run = review(&repo);

    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(
        repo.read(&delta_part_path(1, "design")),
        "> review: Write a design.\n"
    );
}

#[test]
fn running_twice_with_the_same_reply_changes_nothing_the_second_time() {
    let repo = project();
    let mut reply = all_kinds();
    reply["findings"]
        .as_array_mut()
        .unwrap()
        .push(finding("tasks", "T1", "Split this."));
    reply_with(&repo, reply);

    let first = review(&repo);
    let after_first = snapshot(&repo);
    let second = review(&repo);

    assert!(
        first.stdout.contains("6 review lines written"),
        "{}",
        first.stdout
    );
    assert_eq!(second.code, 0, "{}", second.stderr);
    assert!(second.stdout.contains("0 review lines written"));
    assert_eq!(snapshot(&repo), after_first);
}

#[test]
fn a_single_line_is_reported_in_the_singular() {
    let repo = project();
    reply_with(
        &repo,
        json!({"findings": [finding("tasks", "T2", "Order matters.")]}),
    );

    assert!(review(&repo).stdout.contains("1 review line written\n"));
}

#[test]
fn an_unknown_anchor_is_an_error_naming_it_and_nothing_is_written() {
    let repo = project();
    let mut reply = all_kinds();
    let findings = reply["findings"].as_array_mut().unwrap();
    findings.push(finding("claims", "AC-9", "Which claim?"));
    findings.push(finding("design", "Nowhere", "Which section?"));
    reply_with(&repo, reply);
    let before = snapshot(&repo);

    let run = review(&repo);

    assert_eq!(run.code, 2);
    assert!(
        run.stderr.contains("lemma: claims: no anchor `AC-9`"),
        "{}",
        run.stderr
    );
    assert!(
        run.stderr.contains("lemma: design: no anchor `Nowhere`"),
        "{}",
        run.stderr
    );
    assert_eq!(snapshot(&repo), before);
}

#[test]
fn an_unknown_part_is_an_error_naming_it() {
    let repo = project();
    reply_with(&repo, json!({"findings": [finding("notes", "", "x")]}));

    let run = review(&repo);

    assert_eq!(run.code, 2);
    assert!(run.stderr.contains("notes"), "{}", run.stderr);
}

#[test]
fn a_reply_without_findings_is_an_error() {
    let repo = project();
    reply_with(&repo, json!({"clean": true}));

    let run = review(&repo);

    assert_eq!(run.code, 2);
    assert!(run.stderr.contains("findings"), "{}", run.stderr);
}

#[test]
fn zero_findings_with_review_lines_left_reports_clean_and_changes_nothing() {
    let repo = project();
    repo.delta_claims(1, &format!("{CLAIMS}  @review Still open?\n"));
    reply_with(&repo, json!({"findings": []}));
    let before = snapshot(&repo);

    let run = review(&repo);

    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(
        run.stdout,
        "DELTA-001 is draft: reviewing the document\nreview clean; 1 review lines remain, resolve them and run again\n"
    );
    assert_eq!(snapshot(&repo), before);
}

#[test]
fn a_failing_reviewer_is_exit_two_with_its_stderr_and_nothing_written() {
    let repo = project();
    script(&repo, "echo 'rate limit exceeded' >&2\nexit 1");
    let before = snapshot(&repo);

    let run = review(&repo);

    assert_eq!(run.code, 2);
    assert!(run.stderr.contains("lemma: "), "{}", run.stderr);
    assert!(run.stderr.contains("rate limit exceeded"), "{}", run.stderr);
    assert_eq!(snapshot(&repo), before);
}
