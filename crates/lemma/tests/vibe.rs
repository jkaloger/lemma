mod common;
use common::*;
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
