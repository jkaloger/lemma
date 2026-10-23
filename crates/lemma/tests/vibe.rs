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
