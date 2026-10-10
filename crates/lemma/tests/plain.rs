mod common;
use common::*;
use serde_json::json;
use std::os::unix::fs::PermissionsExt;

const INDEX_BODY: &str = "## Intent\n\nDelta intent.\n";

fn project(status: &str, claims: &str, tasks: &str) -> Repo {
    let repo = Repo::new();
    repo.git(&["config", "user.name", "t"]);
    repo.git(&["config", "user.email", "t@example.com"]);
    repo.git(&["config", "commit.gpgsign", "false"]);
    repo.with_dictums();
    repo.spec_file(1, "active", &["src/**"], "## Intent\n\nSpec one intent.\n");
    repo.delta_file(1, status, &["SPEC-001"], INDEX_BODY);
    repo.delta_part(1, "design", "Design prose.\n");
    repo.delta_claims(1, claims);
    repo.delta_tasks(1, tasks);
    repo.write(".gitignore", "replies/\n");
    repo
}

/// One script for every runner call: it answers from the reply file the prompt's marker names.
fn agent(repo: &Repo, replies: &[(&str, serde_json::Value)]) {
    let branches: String = replies
        .iter()
        .enumerate()
        .map(|(n, (marker, reply))| {
            let file = format!("replies/{n}.json");
            repo.write(&file, &json!({ "structured_output": reply }).to_string());
            format!("if grep -q '{marker}' replies/stdin; then cat {file}; exit 0; fi\n")
        })
        .collect();
    repo.write(
        "agent.sh",
        &format!("#!/bin/sh\nmkdir -p replies\ncat > replies/stdin\n[ -n \"$1\" ] && mkdir -p src && echo 'pub fn one() {{}}' > src/one.rs\n{branches}exit 1\n"),
    );
    let path = repo.path("agent.sh");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    repo.write(
        "lemma.toml",
        &format!(
            "runner = \"{0} build {{session}}\"\nreviewer = \"{0} '' {{session}}\"\n",
            path.display()
        ),
    );
    repo.commit("start");
}

/// The session uuid and the runner's wall time vary each run; every other byte is pinned.
fn varying_blanked(stdout: &str) -> String {
    let session = regex::Regex::new(r"/[0-9a-f-]{36}\.jsonl").unwrap();
    let elapsed = regex::Regex::new(r"runner: \d+(m\d+)?s,").unwrap();
    let stdout = session.replace_all(stdout, "/<session>.jsonl");
    elapsed
        .replace_all(&stdout, "runner: <elapsed>,")
        .into_owned()
}

fn watch(repo: &Repo) -> String {
    let encoded: String = repo
        .root()
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    format!(
        "   watch: tail -n +1 -F \"$HOME/.claude/projects/{encoded}/<session>.jsonl\" | jq -r 'select(.type == \"assistant\") | .message.content[]? | select(.type == \"tool_use\") | \"\\(.name) \\(.input | tostring | .[0:100])\"'\n"
    )
}

#[test]
fn piped_review_plan_and_build_print_the_plain_report_with_watch_lines() {
    let review = project("draft", "- Reopen keeps the prior answer.\n", "");
    agent(&review, &[("", json!({"findings": []}))]);
    let run = review.run(&["review", "DELTA-001"], "");
    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert_eq!(run.stderr, "");
    assert_eq!(
        varying_blanked(&run.stdout),
        format!(
            "DELTA-001 is draft: reviewing the document\n{}review clean\nnext: lemma plan DELTA-001\n",
            watch(&review)
        )
    );

    let plan = project("draft", "- Reopen keeps the prior answer.\n", "");
    agent(
        &plan,
        &[(
            "You are planning a reviewed delta",
            json!({"claims": [], "tasks": [{"text": "reopen", "settles": ["AC-1"]}]}),
        )],
    );
    let run = plan.run(&["plan", "DELTA-001"], "");
    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert_eq!(run.stderr, "");
    assert_eq!(
        varying_blanked(&run.stdout),
        format!(
            "{}review clean, planning\nclaims.md: 1 claim tagged AC-1\ntasks.md: 1 task written\nnext: read the diff, accept or reject each [proposed] claim, then lemma build DELTA-001\n",
            watch(&plan)
        )
    );

    let build = project(
        "in-progress",
        "- [AC-1] Reopen keeps the prior answer.\n",
        "- [ ] T1 reopen (settles AC-1)\n",
    );
    agent(
        &build,
        &[
            (
                "You are reviewing one task",
                json!({"stop": "", "findings": []}),
            ),
            (
                "You are carrying out one task",
                json!({"outcome": "done", "reason": "", "refs": [
                    {"claim": "AC-1", "path": "tests/store_test.rs", "symbol": "reopen_keeps_prior"}
                ]}),
            ),
        ],
    );
    let run = build.run(&["build", "DELTA-001"], "");
    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert_eq!(run.stderr, "");
    assert_eq!(varying_blanked(&run.stdout), format!(
        "T1 reopen (settles AC-1)\n{w}   runner: <elapsed>, 1 files\n   lint: clean\n   AC-1 @ref tests/store_test.rs#reopen_keeps_prior resolves\n{w}   review: clean\n   commit {sha} DELTA-001 T1: reopen\nall tasks complete\nnext: lazyspec update DELTA-001 --status complete\n",
        w = watch(&build),
        sha = build.head()
    ));
}
