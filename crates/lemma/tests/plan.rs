mod common;
use common::*;
use serde_json::{json, Value};
use std::os::unix::fs::PermissionsExt;

const INDEX_BODY: &str = "## Intent\n\nDelta intent.\n";
const DESIGN: &str = "Design prose.\n";
const PLAN_MARKER: &str = "You are planning a reviewed delta";

fn project(claims: &str, tasks: &str) -> Repo {
    let repo = Repo::new();
    repo.with_dictums();
    repo.spec_file(1, "active", &["a/**"], "## Intent\n\nSpec one intent.\n");
    repo.delta_file(1, "draft", &["SPEC-001"], INDEX_BODY);
    repo.delta_part(1, "design", DESIGN);
    repo.delta_claims(1, claims);
    repo.delta_tasks(1, tasks);
    repo.write("review.json", &envelope(json!({"findings": []})));
    repo.write("plan.json", &envelope(json!({"claims": [], "tasks": []})));
    let path = repo.path("reviewer.sh");
    repo.write(
        "reviewer.sh",
        &format!(
            "#!/bin/sh\ncat > stdin.txt\nif grep -q '{PLAN_MARKER}' stdin.txt; then\n  echo called >> plan-calls\n  cat plan.json\nelse\n  cat review.json\nfi\n"
        ),
    );
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    repo.write(
        "lemma.toml",
        &format!("reviewer = \"{}\"\n", path.display()),
    );
    repo
}

fn envelope(reply: Value) -> String {
    json!({ "result": reply.to_string() }).to_string()
}

fn plan_reply(repo: &Repo, reply: Value) {
    repo.write("plan.json", &envelope(reply));
}

fn claim(text: &str) -> Value {
    json!({"text": text, "proposed": false})
}

fn proposed(text: &str) -> Value {
    json!({"text": text, "proposed": true})
}

fn task(text: &str, settles: Value) -> Value {
    json!({"text": text, "settles": settles})
}

fn review(repo: &Repo) -> Run {
    repo.run(&["review", "DELTA-001"], "")
}

fn plan_calls(repo: &Repo) -> usize {
    std::fs::read_to_string(repo.path("plan-calls"))
        .map(|text| text.lines().count())
        .unwrap_or(0)
}

fn snapshot(repo: &Repo) -> Vec<String> {
    ["index", "design", "claims", "tasks"]
        .iter()
        .map(|part| repo.read(&delta_part_path(1, part)))
        .collect()
}

fn claims_of(repo: &Repo) -> String {
    repo.read(&delta_part_path(1, "claims"))
}

fn tasks_of(repo: &Repo) -> String {
    repo.read(&delta_part_path(1, "tasks"))
}

const HEADER: &str = "DELTA-001 is draft: reviewing the document\n";

#[test]
fn ids_are_assigned_in_document_order_continuing_from_the_highest() {
    let repo = project("- First.\n- [AC-2] Second.\n- Third.\n- Fourth.\n", "");
    plan_reply(
        &repo,
        json!({
            "claims": [claim("First."), claim("Second."), claim("Third."), claim("Fourth.")],
            "tasks": [task("Build it", json!([1, 2, 3, 4]))]
        }),
    );

    let run = review(&repo);

    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(
        claims_of(&repo),
        "- [AC-3] First.\n- [AC-2] Second.\n- [AC-4] Third.\n- [AC-5] Fourth.\n"
    );
    assert_eq!(
        run.stdout,
        format!(
            "{HEADER}review clean, planning\nclaims.md: 3 claims tagged AC-3..AC-5\ntasks.md: 1 task written\n"
        )
    );
    assert_eq!(
        tasks_of(&repo),
        "- [ ] T1 Build it (settles AC-3, AC-2, AC-4, AC-5)\n"
    );
}

#[test]
fn a_single_new_id_is_named_alone() {
    let repo = project("- Only.\n", "");
    plan_reply(
        &repo,
        json!({"claims": [claim("Only.")], "tasks": [task("Do", json!([1]))]}),
    );

    let run = review(&repo);

    assert!(
        run.stdout.contains("claims.md: 1 claim tagged AC-1\n"),
        "{}",
        run.stdout
    );
}

#[test]
fn human_bullet_text_is_preserved_byte_for_byte() {
    let bullet =
        "- Reopen keeps `Head::prior`, even after a crash!  \n  continuation line with  spacing.\n";
    let repo = project(bullet, "");
    plan_reply(
        &repo,
        json!({
            "claims": [claim("Reopen keeps `Head::prior`, even after a crash!")],
            "tasks": [task("Do", json!([1]))]
        }),
    );

    let run = review(&repo);

    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(
        claims_of(&repo),
        "- [AC-1] Reopen keeps `Head::prior`, even after a crash!  \n  continuation line with  spacing.\n"
    );
    assert_eq!(tasks_of(&repo), "- [ ] T1 Do (settles AC-1)\n");
}

#[test]
fn proposed_claims_are_appended_with_the_marker() {
    let repo = project("- [AC-1] Human claim.\n- Another.", "");
    plan_reply(
        &repo,
        json!({
            "claims": [
                claim("Human claim."),
                claim("Another."),
                proposed("Writes are atomic."),
                claim("Invented but not marked.")
            ],
            "tasks": [task("Do", json!([1, 2, 3, 4]))]
        }),
    );

    let run = review(&repo);

    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(
        claims_of(&repo),
        "- [AC-1] Human claim.\n- [AC-2] Another.\n- [AC-3] [proposed] Writes are atomic.\n- [AC-4] [proposed] Invented but not marked.\n"
    );
    assert!(
        run.stdout
            .contains("claims.md: 1 claim tagged AC-2, 2 [proposed]\n"),
        "{}",
        run.stdout
    );
}

#[test]
fn only_proposed_claims_are_reported_without_a_tag_count() {
    let repo = project("- [AC-1] Human claim.\n", "");
    plan_reply(
        &repo,
        json!({
            "claims": [claim("Human claim."), proposed("Extra.")],
            "tasks": [task("Do", json!([1, 2]))]
        }),
    );

    let run = review(&repo);

    assert!(
        run.stdout.contains("claims.md: 1 [proposed]\n"),
        "{}",
        run.stdout
    );
}

#[test]
fn tasks_are_appended_after_hand_written_ones_and_before_follow_ups() {
    let tasks = "- [ ] T1 hand written (chore)\n\n## Follow-ups\n\n- [ ] T2 later (chore)\n";
    let repo = project("- [AC-1] A.\n- [AC-2] B.\n", tasks);
    plan_reply(
        &repo,
        json!({
            "claims": [claim("A."), claim("B.")],
            "tasks": [task("Build  A", json!([1])), task("Wire B", json!([]))]
        }),
    );

    let run = review(&repo);

    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(
        tasks_of(&repo),
        "- [ ] T1 hand written (chore)\n- [ ] T3 Build A (settles AC-1)\n- [ ] T4 Wire B (chore)\n\n## Follow-ups\n\n- [ ] T2 later (chore)\n"
    );
    assert!(run.stdout.contains("tasks.md: 2 tasks written\n"));
}

#[test]
fn tasks_follow_a_template_comment_on_a_new_line() {
    let repo = project("- [AC-1] A.\n", "<!-- one task per commit -->");
    plan_reply(
        &repo,
        json!({"claims": [claim("A.")], "tasks": [task("Do", json!(["AC-1"]))]}),
    );

    review(&repo);

    assert_eq!(
        tasks_of(&repo),
        "<!-- one task per commit -->\n- [ ] T1 Do (settles AC-1)\n"
    );
}

#[test]
fn a_ticked_task_refuses_and_the_planner_is_not_called() {
    let repo = project(
        "- Plain.\n",
        "- [x] T1 done (chore)\n- [x] T2 also (chore)\n- [ ] T3 open (chore)\n",
    );
    plan_reply(
        &repo,
        json!({"claims": [claim("Plain.")], "tasks": [task("Do", json!([1]))]}),
    );
    let before = snapshot(&repo);

    let run = review(&repo);

    assert_eq!(run.code, 2);
    assert!(
        run.stderr
            .contains("lemma: plan refuses: T1, T2 are ticked"),
        "{}",
        run.stderr
    );
    assert_eq!(snapshot(&repo), before);
    assert_eq!(plan_calls(&repo), 0);
}

#[test]
fn frontmatter_keys_are_put_in_canonical_order() {
    let repo = project("- [AC-1] A.\n", "");
    repo.write(
        &delta_part_path(1, "index"),
        "---\nstatus: draft\ngoverns:\n  - a/**\nrelated:\n- changes: SPEC-001\ntype: delta\ntitle: \"Delta 1\"\ncustom: x\ndate: 2026-01-01\nauthor: \"t\"\ntags: []\nreviewed: yes\n---\n\n## Intent\n\nDelta intent.\n",
    );
    plan_reply(
        &repo,
        json!({"claims": [claim("A.")], "tasks": [task("Do", json!([1]))]}),
    );

    let run = review(&repo);

    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(
        repo.read(&delta_part_path(1, "index")),
        "---\ntitle: \"Delta 1\"\ntype: delta\nstatus: draft\nauthor: \"t\"\ndate: 2026-01-01\ntags: []\nrelated:\n- changes: SPEC-001\ngoverns:\n  - a/**\ncustom: x\nreviewed: yes\n---\n\n## Intent\n\nDelta intent.\n"
    );
    assert!(run
        .stdout
        .ends_with("index.md: frontmatter and headings tidied\n"));
}

#[test]
fn headings_are_normalised_outside_fences() {
    let repo = project("- [AC-1] A.\n", "");
    let index = repo.read(&delta_part_path(1, "index"));
    let (front, _) = index.split_once("\n---\n").unwrap();
    repo.write(
        &delta_part_path(1, "index"),
        &format!(
            "{front}\n---\n\n# intent\n\nText.\n\n```\n### OUT OF SCOPE\n```\n\n### OUT OF SCOPE\n\nNope.\n\n## Approach\n"
        ),
    );
    plan_reply(
        &repo,
        json!({"claims": [claim("A.")], "tasks": [task("Do", json!([1]))]}),
    );

    review(&repo);

    assert_eq!(
        repo.read(&delta_part_path(1, "index")),
        format!(
            "{front}\n---\n\n## Intent\n\nText.\n\n```\n### OUT OF SCOPE\n```\n\n## Out of scope\n\nNope.\n\n## Approach\n"
        )
    );
}

#[test]
fn rerunning_on_a_planned_delta_changes_nothing() {
    let repo = project("- First.\n- Second.\n", "");
    plan_reply(
        &repo,
        json!({
            "claims": [claim("First."), claim("Second.")],
            "tasks": [task("Do first", json!([1])), task("Do second", json!([2]))]
        }),
    );

    let first = review(&repo);
    let after_first = snapshot(&repo);
    let second = review(&repo);

    assert_eq!(first.code, 0, "{}", first.stderr);
    assert_eq!(plan_calls(&repo), 1);
    assert_eq!(second.code, 0, "{}", second.stderr);
    assert_eq!(
        second.stdout,
        format!("{HEADER}review clean\nalready planned\n")
    );
    assert_eq!(snapshot(&repo), after_first);
    assert_eq!(plan_calls(&repo), 1);
}

#[test]
fn settles_resolve_by_number_by_id_and_by_text() {
    let repo = project("- [AC-1] Alpha.\n- Beta.\n- Gamma.\n", "");
    plan_reply(
        &repo,
        json!({
            "claims": [claim("Alpha."), claim("Beta."), claim("Gamma."), proposed("Delta.")],
            "tasks": [
                task("One", json!([2, "AC-1", "Gamma.", 2])),
                task("Two", json!(["Delta."]))
            ]
        }),
    );

    let run = review(&repo);

    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(
        tasks_of(&repo),
        "- [ ] T1 One (settles AC-2, AC-1, AC-3)\n- [ ] T2 Two (settles AC-4)\n"
    );
}

#[test]
fn an_unresolvable_settles_entry_errors_and_writes_nothing() {
    let repo = project("- Alpha.\n", "");
    plan_reply(
        &repo,
        json!({
            "claims": [claim("Alpha."), proposed("Extra.")],
            "tasks": [task("Do it", json!([1, "AC-9"]))]
        }),
    );
    let before = snapshot(&repo);

    let run = review(&repo);

    assert_eq!(run.code, 2);
    assert!(
        run.stderr
            .contains("lemma: plan task `Do it` settles unknown claim `AC-9`"),
        "{}",
        run.stderr
    );
    assert_eq!(snapshot(&repo), before);
}

#[test]
fn remaining_review_lines_block_planning() {
    let repo = project("- Plain.\n  @review Which one?\n", "");
    plan_reply(
        &repo,
        json!({"claims": [claim("Plain.")], "tasks": [task("Do", json!([1]))]}),
    );
    let before = snapshot(&repo);

    let run = review(&repo);

    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(
        run.stdout,
        format!("{HEADER}review clean; 1 review lines remain, resolve them and run again\n")
    );
    assert_eq!(snapshot(&repo), before);
    assert_eq!(plan_calls(&repo), 0);
}

#[test]
fn a_marked_bullet_matches_the_planner_by_its_text_without_the_marker() {
    let repo = project(
        "- Reopen keeps the prior answer.\n- [intent] Feels fast.\n",
        "",
    );
    plan_reply(
        &repo,
        json!({
            "claims": [claim("Reopen keeps the prior answer."), claim("Feels fast.")],
            "tasks": [task("reopen", json!([1, 2]))],
        }),
    );

    let run = review(&repo);

    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(
        claims_of(&repo),
        "- [AC-1] Reopen keeps the prior answer.\n- [AC-2] [intent] Feels fast.\n"
    );
    assert_eq!(tasks_of(&repo), "- [ ] T1 reopen (settles AC-1, AC-2)\n");
    assert_eq!(
        repo.run(&["lint", "DELTA-001", "--gate", "in-progress"], "")
            .stdout,
        ""
    );
}
