mod common;
use common::*;
use serde_json::json;
use std::os::unix::fs::PermissionsExt;

const RUNNER: &str = "#!/bin/sh\ntouch .runner-called\n";

#[test]
fn a_dirty_tree_exits_2_without_calling_a_runner() {
    let repo = Repo::new();
    repo.write("fake-runner.sh", RUNNER);
    std::fs::set_permissions(
        repo.path("fake-runner.sh"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    repo.write(
        "lemma.toml",
        &format!(
            "runner = \"{0}\"\nreviewer = \"{0}\"\n",
            repo.path("fake-runner.sh").display()
        ),
    );
    repo.commit("start");
    repo.write("stray.txt", "x");

    let run = repo.run(&["vibe", "-m", "add a thing"], "");

    assert_eq!(run.code, 2);
    assert!(run
        .stderr
        .contains("lemma: build needs a clean working tree; commit or stash first"));
    assert!(!repo.path(".runner-called").exists());
}

const DIR: &str = "docs/deltas/DELTA-001-add-a-thing";

/// Stands in for lazyspec's create, link and delete, logging each call to `lazyspec.log`.
/// `link` fails while `fail-link` exists.
const LAZYSPEC: &str = r#"#!/bin/sh
dir=docs/deltas/DELTA-001-add-a-thing
case "$1" in
--version) echo 'lazyspec 0.15.1' ;;
create)
  echo "create $2 $3" >> lazyspec.log
  mkdir -p $dir
  printf -- '---\ntitle: "%s"\ntype: delta\nstatus: draft\nauthor: "unknown"\ndate: 2026-01-01\ntags: []\nrelated: []\n---\n\n## Intent\n<!-- template -->\n' "$3" > $dir/index.md
  for part in design claims tasks; do echo '<!-- template -->' > $dir/$part.md; done
  printf '{"id": "DELTA-001", "title": "%s", "type": "delta", "status": "draft", "path": "%s/index.md", "parts": [{"name": "design", "path": "%s/design.md"}, {"name": "claims", "path": "%s/claims.md"}, {"name": "tasks", "path": "%s/tasks.md"}]}\n' "$3" $dir $dir $dir $dir
  ;;
link)
  echo "link $2 $3 $4" >> lazyspec.log
  [ -e fail-link ] && { echo 'link refused' >&2; exit 1; }
  awk -v to="$4" '/^related: \[\]$/ { print "related:"; print "- changes: " to; next } /^related:$/ { print; print "- changes: " to; next } { print }' $dir/index.md > $dir/index.tmp
  mv $dir/index.tmp $dir/index.md
  ;;
delete)
  echo "delete $2" >> lazyspec.log
  rm $dir/index.md
  ;;
update)
  echo "update $2 $3 $4" >> lazyspec.log
  [ -e update-strays ] && echo stray > stray.txt
  sed "s/^status: draft$/status: $4/" $dir/index.md > $dir/index.tmp
  mv $dir/index.tmp $dir/index.md
  ;;
esac
"#;

fn draft_reply(changes: &[&str]) -> String {
    let reply = json!({
        "title": "Add a thing",
        "changes": changes,
        "intent": "Things are missing.",
        "out_of_scope": "- Removing things.",
        "design": "Add the thing to the list.",
        "claims": ["A thing can be added.", "An added thing is listed."],
    });
    json!({ "result": reply.to_string() }).to_string()
}

fn drafting(changes: &[&str]) -> Repo {
    let repo = Repo::new();
    repo.spec_file(1, "active", &["a/**"], "## Intent\n\nSpec one.\n");
    repo.spec_file(2, "active", &["b/**"], "## Intent\n\nSpec two.\n");
    repo.fake_lazyspec(LAZYSPEC);
    repo.write("draft.json", &draft_reply(changes));
    repo.write("runner.sh", "#!/bin/sh\ncat > /dev/null\ncat draft.json\n");
    std::fs::set_permissions(
        repo.path("runner.sh"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    repo.write(
        "lemma.toml",
        &format!("runner = \"{}\"\n", repo.path("runner.sh").display()),
    );
    repo.commit("start");
    repo
}

fn lazyspec_calls(repo: &Repo) -> Vec<String> {
    std::fs::read_to_string(repo.path("lazyspec.log"))
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

#[test]
fn the_draft_creates_a_delta_holding_the_reply_linked_to_its_specs() {
    let repo = drafting(&["SPEC-001", "SPEC-002"]);

    let run = repo.run(&["vibe", "-m", "add a thing"], "");

    assert_eq!(
        lazyspec_calls(&repo),
        [
            "create delta Add a thing",
            "link DELTA-001 changes SPEC-001",
            "link DELTA-001 changes SPEC-002",
        ],
        "{}",
        run.stderr
    );
    let index = repo.read(&format!("{DIR}/index.md"));
    assert!(index.contains("status: draft\n"), "{index}");
    assert!(index.contains("- changes: SPEC-001\n"), "{index}");
    assert!(index.contains("- changes: SPEC-002\n"), "{index}");
    assert!(
        index.ends_with(
            "---\n\n## Intent\n\nThings are missing.\n\n## Out of scope\n\n- Removing things.\n"
        ),
        "{index}"
    );
    assert_eq!(
        repo.read(&format!("{DIR}/design.md")),
        "Add the thing to the list.\n"
    );
    assert_eq!(
        repo.read(&format!("{DIR}/claims.md")),
        "- A thing can be added.\n- An added thing is listed.\n"
    );
}

#[test]
fn a_draft_reply_with_no_or_an_unknown_spec_stops_before_any_delta_is_created() {
    for (changes, message) in [
        (&[][..], "draft reply changes no spec"),
        (
            &["SPEC-009"][..],
            "draft reply changes unknown spec SPEC-009",
        ),
    ] {
        let repo = drafting(changes);

        let run = repo.run(&["vibe", "-m", "add a thing"], "");

        assert_eq!(run.code, 2, "{changes:?}");
        assert!(run.stderr.contains(message), "{}", run.stderr);
        assert_eq!(lazyspec_calls(&repo), Vec::<String>::new());
        assert!(!repo.path("docs/deltas").exists());
    }
}

#[test]
fn a_draft_that_fails_after_create_deletes_the_delta() {
    let repo = drafting(&["SPEC-001"]);
    repo.write("fail-link", "");
    repo.commit("fail the link");

    let run = repo.run(&["vibe", "-m", "add a thing"], "");

    assert_eq!(run.code, 2);
    assert!(run.stderr.contains("link refused"), "{}", run.stderr);
    assert_eq!(
        lazyspec_calls(&repo),
        [
            "create delta Add a thing",
            "link DELTA-001 changes SPEC-001",
            "delete DELTA-001",
        ]
    );
    assert!(!repo.path(DIR).exists());
}

/// Answers each prompt by its opening line, logging `draft`, `review`, `revise` or `plan` to
/// `calls.log`. Review call n replies `review-n.json`, or clean when there is none. A build
/// task replies `built.json`; its task review is clean.
const DISPATCH: &str = r#"#!/bin/sh
cat > prompt.txt
case "$(head -1 prompt.txt)" in
"You are drafting"*) echo draft >> calls.log; cat draft.json ;;
"You are reviewing a draft"*)
  echo review >> calls.log
  n=$(grep -c '^review$' calls.log)
  if [ -e review-$n.json ]; then cat review-$n.json; else cat clean.json; fi ;;
"You are revising"*) echo revise >> calls.log; cat revised.json ;;
"You are planning"*) echo plan >> calls.log; cat planned.json ;;
"You are carrying out"*) cat built.json ;;
"You are reviewing one task"*) echo '{"structured_output": {"stop": "", "findings": []}}' ;;
esac
"#;

fn wrapped(reply: serde_json::Value) -> String {
    json!({ "result": reply.to_string() }).to_string()
}

fn finding(text: &str) -> String {
    wrapped(json!({"findings": [{"part": "index", "anchor": "Intent", "text": text}]}))
}

/// A vibe run whose review calls write `findings[n]` in round n + 1, then come back clean.
fn vibing(findings: &[&str]) -> Repo {
    let repo = Repo::new();
    repo.spec_file(1, "active", &["a/**"], "## Intent\n\nSpec one.\n");
    repo.fake_lazyspec(LAZYSPEC);
    repo.write("draft.json", &draft_reply(&["SPEC-001"]));
    repo.write("clean.json", &wrapped(json!({"findings": []})));
    for (n, text) in findings.iter().enumerate() {
        repo.write(&format!("review-{}.json", n + 1), &finding(text));
    }
    repo.write(
        "revised.json",
        &wrapped(json!({
            "index": "## Intent\n\nThings are missing, and users ask for them.\n\n## Out of scope\n\n- Removing things.",
            "design": "Append the thing to the list.",
            "claims": "- A thing can be added.\n- An added thing is listed last.",
        })),
    );
    repo.write(
        "planned.json",
        &wrapped(
            json!({"claims": [], "tasks": [{"text": "Build it", "settles": ["AC-1", "AC-2"]}]}),
        ),
    );
    repo.write(
        "built.json",
        &json!({"structured_output": {"outcome": "done", "reason": "", "refs": [
            {"claim": "AC-1", "path": TEST_FILE, "symbol": "reopen_keeps_prior"},
            {"claim": "AC-2", "path": TEST_FILE, "symbol": "other_test"},
        ]}})
        .to_string(),
    );
    repo.write(".gitignore", "calls.log\nprompt.txt\nlazyspec.log\n");
    repo.write("runner.sh", DISPATCH);
    std::fs::set_permissions(
        repo.path("runner.sh"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    repo.write(
        "lemma.toml",
        &format!("runner = \"{}\"\n", repo.path("runner.sh").display()),
    );
    repo.commit("start");
    repo
}

fn calls(repo: &Repo) -> Vec<String> {
    repo.read("calls.log").lines().map(str::to_string).collect()
}

fn no_review_lines(repo: &Repo) {
    for part in ["index", "design", "claims", "tasks"] {
        let text = repo.read(&format!("{DIR}/{part}.md"));
        assert!(
            !text.contains("> review:") && !text.contains("@review"),
            "{part}: {text}"
        );
    }
}

#[test]
fn each_review_with_findings_but_the_last_is_revised_and_lemma_writes_the_parts() {
    let repo = vibing(&["Intent does not say who asks.", "Intent is still vague."]);

    let run = repo.run(&["vibe", "-m", "add a thing", "--rounds", "3"], "");

    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(
        calls(&repo),
        ["draft", "review", "revise", "review", "revise", "review", "plan"]
    );
    let index = repo.read(&format!("{DIR}/index.md"));
    assert!(
        index.starts_with("---\ntitle: \"Add a thing\"\ntype: delta\nstatus: in-progress\n"),
        "{index}"
    );
    assert!(index.contains("- changes: SPEC-001\n---\n"), "{index}");
    assert!(
        index.ends_with(
            "---\n\n## Intent\n\nThings are missing, and users ask for them.\n\n## Out of scope\n\n- Removing things.\n"
        ),
        "{index}"
    );
    assert_eq!(
        repo.read(&format!("{DIR}/design.md")),
        "Append the thing to the list.\n"
    );
    assert!(repo
        .read(&format!("{DIR}/claims.md"))
        .contains("An added thing is listed last."));
}

#[test]
fn review_stops_at_the_first_clean_round() {
    let repo = vibing(&["Intent does not say who asks."]);

    let run = repo.run(&["vibe", "-m", "add a thing", "--rounds", "3"], "");

    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(
        calls(&repo),
        ["draft", "review", "revise", "review", "plan"]
    );
    assert!(run.stdout.contains("review clean"), "{}", run.stdout);
}

#[test]
fn findings_from_the_last_review_are_not_revised_but_stripped_warned_and_planned() {
    let repo = vibing(&["Intent does not say who asks.", "Intent is still vague."]);

    let run = repo.run(&["vibe", "-m", "add a thing", "--rounds", "2"], "");

    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(
        calls(&repo),
        ["draft", "review", "revise", "review", "plan"]
    );
    assert!(
        run.stderr
            .contains("warning: DELTA-001 index: unresolved > review: Intent is still vague."),
        "{}",
        run.stderr
    );
    no_review_lines(&repo);
    assert!(repo
        .read(&format!("{DIR}/tasks.md"))
        .contains("- [x] T1 Build it (settles AC-1, AC-2)"));
}

#[test]
fn rounds_defaults_to_three_review_calls() {
    let repo = vibing(&["First.", "Second.", "Third.", "Fourth."]);

    let run = repo.run(&["vibe", "-m", "add a thing"], "");

    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(
        calls(&repo),
        ["draft", "review", "revise", "review", "revise", "review", "plan"]
    );
    no_review_lines(&repo);
}

#[test]
fn after_plan_no_claim_is_proposed_and_the_count_accepted_is_printed() {
    let repo = vibing(&[]);
    repo.write(
        "planned.json",
        &wrapped(json!({
            "claims": [{"text": "A thing can be removed."}, {"text": "A removed thing is gone."}],
            "tasks": [{"text": "Build it", "settles": ["AC-1", "AC-2", 1, 2]}],
        })),
    );
    let refs: Vec<_> = (1..=4)
        .map(|n| json!({"claim": format!("AC-{n}"), "path": TEST_FILE, "symbol": "other_test"}))
        .collect();
    repo.write(
        "built.json",
        &json!({"structured_output": {"outcome": "done", "reason": "", "refs": refs}}).to_string(),
    );
    repo.commit("plan proposes claims");

    let run = repo.run(&["vibe", "-m", "add a thing"], "");

    assert_eq!(run.code, 0, "{}", run.stderr);
    assert!(
        run.stdout.contains("accepted 2 [proposed] claims"),
        "{}",
        run.stdout
    );
    let claims = repo.read(&format!("{DIR}/claims.md"));
    assert!(!claims.contains("[proposed]"), "{claims}");
    assert!(
        claims.contains("- [AC-3] A thing can be removed.\n"),
        "{claims}"
    );
    assert!(
        claims.contains("- [AC-4] A removed thing is gone.\n"),
        "{claims}"
    );
}

const NEXT_COMPLETE: &str = "next: lazyspec update DELTA-001 --status complete\n";

fn status(repo: &Repo) -> String {
    let index = repo.read(&format!("{DIR}/index.md"));
    index
        .lines()
        .find_map(|l| l.strip_prefix("status: "))
        .unwrap_or_default()
        .to_string()
}

fn subjects(repo: &Repo) -> Vec<String> {
    repo.git(&["log", "--format=%s"])
        .lines()
        .map(str::to_string)
        .collect()
}

#[test]
fn vibe_moves_the_delta_to_in_progress_and_commits_it_without_asking() {
    let repo = vibing(&[]);

    let run = repo.run(&["vibe", "-m", "add a thing"], "");

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert!(
        subjects(&repo).contains(&"DELTA-001: in-progress".to_string()),
        "{:?}",
        subjects(&repo)
    );
    assert!(lazyspec_calls(&repo).contains(&"update DELTA-001 --status in-progress".to_string()));
    assert!(
        !run.stderr.contains("move it to in-progress?"),
        "{}",
        run.stderr
    );
}

#[test]
fn vibe_refuses_the_move_when_the_gate_lint_has_errors() {
    let repo = vibing(&[]);
    repo.write(
        "planned.json",
        &wrapped(json!({"claims": [], "tasks": [{"text": "Build it", "settles": ["AC-1"]}]})),
    );
    repo.commit("plan leaves AC-2 unsettled");

    let run = repo.run(&["vibe", "-m", "add a thing"], "");

    assert_eq!(run.code, 2, "{}{}", run.stdout, run.stderr);
    assert!(
        run.stderr.contains("AC-2 is settled by no task"),
        "{}",
        run.stderr
    );
    assert!(!lazyspec_calls(&repo)
        .iter()
        .any(|c| c.starts_with("update")));
    assert_eq!(status(&repo), "draft");
    assert!(!subjects(&repo).contains(&"DELTA-001: in-progress".to_string()));
}

#[test]
fn a_build_that_stops_a_task_exits_2_and_names_the_resume() {
    let repo = vibing(&[]);
    repo.write(
        "built.json",
        &json!({"structured_output": {"outcome": "blocked", "reason": "cannot", "refs": []}})
            .to_string(),
    );
    repo.commit("block the task");

    let run = repo.run(&["vibe", "-m", "add a thing"], "");

    assert_eq!(run.code, 2, "{}{}", run.stdout, run.stderr);
    assert!(
        run.stdout
            .ends_with("next: lemma build DELTA-001 --resume\n"),
        "{}",
        run.stdout
    );
}

#[test]
fn a_build_that_fails_without_a_stopped_task_names_the_plain_build() {
    let repo = vibing(&[]);
    repo.write("update-strays", "");
    repo.write(
        ".gitignore",
        "calls.log\nprompt.txt\nlazyspec.log\nupdate-strays\n",
    );
    repo.commit("the move leaves a stray file");

    let run = repo.run(&["vibe", "-m", "add a thing"], "");

    assert_eq!(run.code, 2, "{}{}", run.stdout, run.stderr);
    assert!(
        run.stderr
            .contains("lemma: build needs a clean working tree; commit or stash first"),
        "{}",
        run.stderr
    );
    assert!(
        run.stdout.ends_with("next: lemma build DELTA-001\n"),
        "{}",
        run.stdout
    );
}

#[test]
fn vibe_leaves_the_delta_in_progress_and_ends_naming_the_complete_step() {
    let repo = vibing(&[]);

    let run = repo.run(&["vibe", "-m", "add a thing"], "");

    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert!(run.stdout.ends_with(NEXT_COMPLETE), "{}", run.stdout);
    assert_eq!(status(&repo), "in-progress");
    assert!(!lazyspec_calls(&repo).iter().any(|c| c.contains("complete")));
    assert!(repo
        .read(&format!("{DIR}/tasks.md"))
        .contains("- [x] T1 Build it (settles AC-1, AC-2)\n"));
}
