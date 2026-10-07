mod common;
use common::*;
use serde_json::{json, Value};

const SPEC_INDEX: &str = "docs/specs/SPEC-001-s/index.md";

const DECISION_CONFIG: &str = r#"
[[types]]
name = "decision"
plural = "decisions"
dir = "docs/decisions"
prefix = "DECISION"
lifecycle = { states = ["proposed", "accepted"], edges = [{ from = "proposed", to = "accepted" }] }

[[relationships]]
name = "affects"
inverse = "affected-by"
"#;

const DECISION: &str = "---\ntitle: \"Store reopen\"\ntype: decision\nstatus: proposed\nauthor: \"t\"\ndate: 2026-01-01\ntags: []\nrelated:\n- affects: SPEC-001\n---\n\nWhy.\n";

fn project() -> Repo {
    let repo = Repo::new();
    repo.git(&["branch", "-M", "main"]);
    repo.write(".lazyspec.toml", &format!("{CONFIG}{DECISION_CONFIG}"));
    repo.write("a/x.rs", "fn x() {}\n");
    repo.write("a/more.rs", "fn m() {}\n");
    repo.write("b/y.rs", "fn y() {}\n");
    repo.write("scripts/tool.sh", "echo\n");
    repo.write("docs/decisions/DECISION-001-store.md", DECISION);
    repo.spec_file(1, "active", &["a/**"], INTENT);
    repo.landed_delta(LANDED_CLAIM);
    repo.spec_file(2, "active", &["b/**"], INTENT);
    repo.commit("project");
    repo
}

fn signal_as(repo: &Repo, tool: &str, rel: &str, session: &str) -> Run {
    let input = json!({
        "session_id": session,
        "cwd": repo.root(),
        "hook_event_name": "PostToolUse",
        "tool_name": tool,
        "tool_input": { "file_path": repo.path(rel), "content": "x" },
    });
    repo.run(&["signal"], &input.to_string())
}

fn edit(repo: &Repo, rel: &str) -> Run {
    signal_as(repo, "Edit", rel, "s")
}

fn context_of(run: &Run) -> String {
    let output = run.json();
    assert_eq!(output["hookSpecificOutput"]["hookEventName"], "PostToolUse");
    output["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap()
        .to_string()
}

fn silent(run: &Run) -> bool {
    run.code == 0 && run.stdout.is_empty() && run.stderr.is_empty()
}

const CONTEXT: &str = "a/x.rs → SPEC-001 Spec 1 · DECISION-001 Store reopen";

#[test]
fn only_edit_write_and_multiedit_are_acted_on() {
    let repo = project();
    repo.write("a/x.rs", "fn x() { 1; }\n");

    for tool in ["Read", "Bash", "Grep", "NotebookEdit"] {
        assert!(silent(&signal_as(&repo, tool, "a/x.rs", "s")), "{tool}");
    }
    for (n, tool) in ["Edit", "Write", "MultiEdit"].into_iter().enumerate() {
        let run = signal_as(&repo, tool, "a/x.rs", &format!("s{n}"));
        assert_eq!(context_of(&run), CONTEXT, "{tool}");
    }
}

#[test]
fn a_file_outside_the_repo_is_ignored() {
    let repo = project();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("f.rs"), "x").unwrap();
    let input = json!({
        "session_id": "s",
        "cwd": repo.root(),
        "tool_name": "Edit",
        "tool_input": { "file_path": outside.path().join("f.rs") },
    });

    assert!(silent(&repo.run(&["signal"], &input.to_string())));
}

#[test]
fn the_context_pointer_is_one_line_with_ids_and_titles() {
    let repo = project();

    let run = edit(&repo, "a/x.rs");

    assert_eq!(run.code, 0);
    assert_eq!(run.stderr, "");
    assert_eq!(context_of(&run), CONTEXT);
    assert_eq!(
        run.json()["hookSpecificOutput"].as_object().unwrap().len(),
        2
    );
}

#[test]
fn the_context_is_said_once_per_area_then_silent() {
    let repo = project();

    assert_eq!(context_of(&edit(&repo, "a/x.rs")), CONTEXT);
    assert!(silent(&edit(&repo, "a/x.rs")));
    assert!(silent(&edit(&repo, "a/more.rs")));

    let hashes = repo.read(".git/lemma/sessions/s.said");
    assert_eq!(hashes.lines().count(), 1);
}

#[test]
fn another_session_hears_it_again() {
    let repo = project();
    edit(&repo, "a/x.rs");

    let run = signal_as(&repo, "Edit", "a/x.rs", "other");

    assert_eq!(context_of(&run), CONTEXT);
}

#[test]
fn another_area_gets_its_own_pointer() {
    let repo = project();
    edit(&repo, "a/x.rs");

    let run = edit(&repo, "b/y.rs");

    assert_eq!(context_of(&run), "b/y.rs → SPEC-002 Spec 2");
}

#[test]
fn a_clean_edit_to_an_ungoverned_file_is_silent() {
    let repo = project();
    repo.write("scripts/tool.sh", "echo changed\n");

    assert!(silent(&edit(&repo, "scripts/tool.sh")));
}

#[test]
fn a_changed_test_names_the_landed_delta_and_is_said_once() {
    let repo = project();
    edit(&repo, "a/x.rs");
    repo.write(
        TEST_FILE,
        "#[test]\nfn reopen_keeps_prior() { assert!(true); }\n\n#[test]\nfn other_test() {}\n",
    );

    let run = edit(&repo, TEST_FILE);

    let said = context_of(&run);
    assert!(
        said.contains(&format!("{LANDED_CLAIMS}:2: warning: {HISTORY}")),
        "{said}"
    );
    assert_eq!(said.lines().count(), 1);
    assert!(silent(&edit(&repo, TEST_FILE)));
}

#[test]
fn editing_a_sibling_test_is_silent() {
    let repo = project();
    repo.write(
        TEST_FILE,
        &format!("{TEST_SRC}\n#[test]\nfn added() {{}}\n"),
    );

    assert!(silent(&edit(&repo, TEST_FILE)));
}

#[test]
fn the_file_that_makes_the_change_span_two_areas_is_flagged() {
    let repo = project();
    repo.write("a/x.rs", "fn x() { 1; }\n");
    assert_eq!(context_of(&edit(&repo, "a/x.rs")), CONTEXT);
    repo.write("a/more.rs", "fn m() { 1; }\n");
    assert!(silent(&edit(&repo, "a/more.rs")));

    repo.write("b/y.rs", "fn y() { 1; }\n");
    let said = context_of(&edit(&repo, "b/y.rs"));

    assert!(said.contains("b/y.rs → SPEC-002"), "{said}");
    assert!(said.contains("more than one governed area"), "{said}");
    assert!(
        said.contains("create a delta that changes SPEC-001"),
        "{said}"
    );
    assert!(
        said.contains("create a delta that changes SPEC-002"),
        "{said}"
    );
}

#[test]
fn coverage_is_said_once_per_spec_even_when_more_files_change() {
    let repo = project();
    repo.write("c/z.rs", "fn z() {}\n");
    repo.spec_file(3, "active", &["c/**"], INTENT);
    repo.commit("third area");
    repo.write("a/x.rs", "fn x() { 1; }\n");
    edit(&repo, "a/x.rs");
    repo.write("b/y.rs", "fn y() { 1; }\n");
    let second = context_of(&edit(&repo, "b/y.rs"));
    assert!(second.contains("create a delta that changes SPEC-002"));
    repo.write("c/z.rs", "fn z() { 1; }\n");
    edit(&repo, "c/z.rs");

    repo.write("b/y.rs", "fn y() { 2; }\n");

    assert!(silent(&edit(&repo, "b/y.rs")));
}

#[test]
fn a_covering_delta_silences_the_coverage_finding() {
    let repo = project();
    repo.delta_file(1, "draft", &["SPEC-001", "SPEC-002"], INTENT);
    repo.commit("delta");
    repo.write("a/x.rs", "fn x() { 1; }\n");
    edit(&repo, "a/x.rs");
    repo.write("b/y.rs", "fn y() { 1; }\n");

    let said = context_of(&edit(&repo, "b/y.rs"));

    assert_eq!(
        said,
        "b/y.rs → SPEC-002 Spec 2 · DELTA-001 Delta 1 (docs/deltas/DELTA-001-d: index.md)"
    );
}

#[test]
fn pointers_name_an_in_flight_deltas_parts_and_related_documents() {
    let repo = project();
    repo.write(
        "docs/deltas/DELTA-001-d/index.md",
        "---\ntitle: \"Retry\"\ntype: delta\nstatus: in-progress\nauthor: \"t\"\ndate: 2026-01-01\ntags: []\nrelated:\n- changes: SPEC-001\n- affects: DECISION-002\n---\n\n## Intent\n\nBecause.\n",
    );
    repo.write(
        "docs/decisions/DECISION-002-retry.md",
        "---\ntitle: \"Retry budget\"\ntype: decision\nstatus: accepted\nauthor: \"t\"\ndate: 2026-01-01\ntags: []\n---\n\nOnce.\n",
    );
    repo.delta_part(1, "design", "Retry once.\n");
    repo.delta_claims(1, "- [AC-1] Retries once.\n");
    repo.delta_tasks(1, "- [ ] T1 retry (settles AC-1)\n");
    repo.delta_file(2, "complete", &["SPEC-001"], INTENT);
    repo.delta_file(3, "draft", &["SPEC-002"], INTENT);
    repo.commit("deltas");
    repo.write("a/x.rs", "fn x() { 1; }\n");

    let said = context_of(&edit(&repo, "a/x.rs"));

    assert_eq!(
        said,
        "a/x.rs → SPEC-001 Spec 1 · DECISION-001 Store reopen · DELTA-001 Retry (docs/deltas/DELTA-001-d: index.md claims.md design.md tasks.md) · DECISION-002 Retry budget"
    );
}

#[test]
fn editing_a_spec_reports_its_invalid_globs() {
    let repo = project();
    repo.spec_file_with_tests(1, "active", &["a/**"], &["[bad"], INTENT);

    let said = context_of(&edit(&repo, SPEC_INDEX));

    assert!(said.contains("tests glob `[bad` is invalid"), "{said}");
}

#[test]
fn editing_a_delta_part_reports_its_claims() {
    let repo = project();
    repo.delta_file(1, "in-progress", &["SPEC-001"], INTENT);
    repo.delta_claims(1, "- [AC-1] Retries.\n  @ref tests/store_test.rs#missing\n");

    let said = context_of(&edit(&repo, &delta_part_path(1, "claims")));

    assert!(
        said.contains("AC-1: ref tests/store_test.rs#missing does not resolve"),
        "{said}"
    );
}

#[test]
fn a_clean_spec_is_silent() {
    let repo = project();

    assert!(silent(&edit(&repo, SPEC_INDEX)));
}

#[test]
fn it_never_blocks_on_bad_input() {
    let repo = project();

    for stdin in ["", "not json", "{}", "{\"tool_name\":\"Edit\"}"] {
        assert!(silent(&repo.run(&["signal"], stdin)), "{stdin:?}");
    }
}

#[test]
fn outside_a_lazyspec_project_it_is_silent() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("f.rs"), "x").unwrap();
    let input: Value = json!({
        "session_id": "s",
        "cwd": dir.path(),
        "tool_name": "Edit",
        "tool_input": { "file_path": dir.path().join("f.rs") },
    });
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_lemma"))
        .arg("signal")
        .current_dir(dir.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    std::io::Write::write_all(
        &mut child.stdin.take().unwrap(),
        input.to_string().as_bytes(),
    )
    .unwrap();
    let out = child.wait_with_output().unwrap();

    assert_eq!(out.status.code(), Some(0));
    assert!(out.stdout.is_empty() && out.stderr.is_empty());
}
