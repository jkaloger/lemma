use crate::build;
use crate::git::git_output;
use std::io::IsTerminal;
use std::path::Path;
use std::process::Command;

pub fn update_command(id: &str, status: &str) -> String {
    format!("lazyspec update {id} --status {status}")
}

pub fn should_prompt(status: &str, interactive: bool) -> bool {
    status == "draft" && interactive
}

fn interactive() -> bool {
    std::io::stdin().is_terminal() && std::io::stderr().is_terminal()
}

fn refusal(id: &str, status: &str) -> String {
    format!(
        "build needs an in-progress delta; {id} is {status}; run `{}`",
        update_command(id, "in-progress")
    )
}

fn confirmed(id: &str) -> bool {
    dialoguer::Confirm::new()
        .with_prompt(format!("{id} is draft; move it to in-progress?"))
        .default(false)
        .interact()
        .unwrap_or(false)
}

fn lazyspec_update(program: &str, root: &Path, id: &str) -> Result<(), String> {
    let status = Command::new(program)
        .args(["update", id, "--status", "in-progress"])
        .current_dir(root)
        .status()
        .map_err(|e| format!("cannot start `{program}`: {e}"))?;
    match status.success() {
        true => Ok(()),
        false => Err(format!(
            "{id} stayed draft; fix the findings above and run lemma build {id} again"
        )),
    }
}

fn commit_move(root: &Path, id: &str) -> Result<(), String> {
    if git_output(root, &["status", "--porcelain"])
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        return Ok(());
    }
    build::git_run(root, &["add", "-A"])?;
    build::git_run(root, &["commit", "-q", "-m", &format!("{id}: in-progress")])
}

fn advance_with(
    program: &str,
    root: &Path,
    id: &str,
    status: &str,
    ask: impl FnOnce() -> bool,
    interactive: bool,
) -> Result<(), String> {
    if status != "draft" {
        return Err(refusal(id, status));
    }
    build::require_clean(root)?;
    if !should_prompt(status, interactive) || !ask() {
        return Err(refusal(id, status));
    }
    lazyspec_update(program, root, id)?;
    commit_move(root, id)
}

pub fn offer(root: &Path, id: &str, status: &str) -> Result<(), String> {
    advance_with(
        "lazyspec",
        root,
        id,
        status,
        || confirmed(id),
        interactive(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testgit;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn only_a_draft_on_a_terminal_asks() {
        assert!(should_prompt("draft", true));
        assert!(!should_prompt("draft", false));
        assert!(!should_prompt("complete", true));
        assert!(!should_prompt("in-progress", true));
    }

    fn fake_lazyspec(dir: &Path, body: &str) -> String {
        let script = dir.join("fake-lazyspec");
        std::fs::write(&script, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        script.to_string_lossy().into_owned()
    }

    fn clean_repo() -> testgit::Repo {
        let repo = testgit::Repo::new();
        repo.git(&["config", "user.name", "t"]);
        repo.git(&["config", "user.email", "t@example.com"]);
        repo.git(&["config", "commit.gpgsign", "false"]);
        repo.write("a.txt", "a\n");
        repo.commit("init");
        repo
    }

    #[test]
    fn a_non_draft_or_declined_move_names_the_real_command() {
        let repo = clean_repo();
        let bin = tempfile::tempdir().unwrap();
        let program = fake_lazyspec(bin.path(), "exit 0");

        let off_tty = advance_with(&program, repo.root(), "DELTA-001", "draft", || true, false);
        let declined = advance_with(&program, repo.root(), "DELTA-001", "draft", || false, true);

        for error in [off_tty.unwrap_err(), declined.unwrap_err()] {
            assert!(
                error.contains("`lazyspec update DELTA-001 --status in-progress`"),
                "{error}"
            );
        }
    }

    #[test]
    fn a_zero_exit_commits_what_the_move_wrote() {
        let repo = clean_repo();
        let bin = tempfile::tempdir().unwrap();
        let program = fake_lazyspec(bin.path(), "echo moved > moved.txt");

        advance_with(&program, repo.root(), "DELTA-001", "draft", || true, true).unwrap();

        assert_eq!(
            repo.git(&["log", "-1", "--format=%s"]),
            "DELTA-001: in-progress"
        );
        assert_eq!(repo.git(&["status", "--porcelain"]), "");
    }

    #[test]
    fn a_non_zero_exit_stops_and_leaves_the_delta_draft() {
        let repo = clean_repo();
        let bin = tempfile::tempdir().unwrap();
        let program = fake_lazyspec(bin.path(), "echo blocked >&2; exit 1");

        let error =
            advance_with(&program, repo.root(), "DELTA-001", "draft", || true, true).unwrap_err();

        assert_eq!(
            error,
            "DELTA-001 stayed draft; fix the findings above and run lemma build DELTA-001 again"
        );
    }

    #[test]
    fn a_dirty_tree_is_refused_before_any_prompt() {
        let repo = clean_repo();
        repo.write("dirty.txt", "x\n");
        let bin = tempfile::tempdir().unwrap();
        let program = fake_lazyspec(bin.path(), "exit 0");

        let error = advance_with(
            &program,
            repo.root(),
            "DELTA-001",
            "draft",
            || panic!("asked on a dirty tree"),
            true,
        )
        .unwrap_err();

        assert!(error.contains("clean working tree"), "{error}");
    }
}
