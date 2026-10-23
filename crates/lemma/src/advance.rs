use crate::build;
use crate::git::git_output;
use crate::lint;
use crate::payload::Doc;
use crate::project;
use crate::shape::Gate;
use crate::world::World;
use lazyspec::engine::store::Store;
use std::io::IsTerminal;
use std::path::Path;
use std::process::Command;

pub fn update_command(id: &str, status: &str) -> String {
    format!("lazyspec update {id} --status {status}")
}

fn interactive() -> bool {
    std::io::stdin().is_terminal() && std::io::stderr().is_terminal()
}

pub fn refusal(id: &str, status: &str) -> String {
    format!(
        "build needs an in-progress delta; {id} is {status}; run `{}`",
        update_command(id, "in-progress")
    )
}

fn stayed_draft(id: &str) -> String {
    format!("{id} stayed draft; fix the findings above and run lemma build {id} again")
}

fn confirmed(id: &str) -> bool {
    dialoguer::Confirm::new()
        .with_prompt(format!("{id} is draft; move it to in-progress?"))
        .default(false)
        .interact()
        .unwrap_or(false)
}

struct Move<'a> {
    program: &'a str,
    root: &'a Path,
    id: &'a str,
    dir: &'a str,
}

impl Move<'_> {
    fn require_clean_outside_delta(&self) -> Result<(), String> {
        let prefix = git_output(self.root, &["rev-parse", "--show-prefix"]).unwrap_or_default();
        let allowed = format!("{}{}/", prefix.trim(), self.dir);
        let status = git_output(
            self.root,
            &["status", "--porcelain", "-z", "--untracked-files=all"],
        )
        .unwrap_or_default();
        let mut entries = status.split('\0').filter(|entry| !entry.is_empty());
        while let Some(entry) = entries.next() {
            let mut paths = vec![&entry[3..]];
            if entry[..2].contains(['R', 'C']) {
                paths.extend(entries.next());
            }
            if paths.iter().any(|path| !path.starts_with(&allowed)) {
                return Err(build::DIRTY_TREE.to_string());
            }
        }
        Ok(())
    }

    fn update(&self) -> Result<(), String> {
        let status = Command::new(self.program)
            .args(["update", self.id, "--status", "in-progress"])
            .current_dir(self.root)
            .status()
            .map_err(|e| format!("cannot start `{}`: {e}", self.program))?;
        match status.success() {
            true => Ok(()),
            false => Err(stayed_draft(self.id)),
        }
    }

    fn commit(&self) -> Result<(), String> {
        let id = self.id;
        let moved = git_output(self.root, &["status", "--porcelain", "--", self.dir])
            .is_some_and(|out| !out.trim().is_empty());
        if !moved {
            return Ok(());
        }
        build::git_run(self.root, &["add", "-A", "--", self.dir])
            .and_then(|_| {
                build::git_run(self.root, &["commit", "-q", "-m", &format!("{id}: in-progress")])
            })
            .map_err(|_| {
                format!("{id} moved to in-progress but not committed; commit it and run lemma build {id}")
            })
    }
}

fn advance_with(
    step: &Move,
    interactive: bool,
    ask: impl FnOnce() -> bool,
    gate_findings: impl FnOnce() -> Vec<lint::Finding>,
    status_after: impl FnOnce() -> Option<String>,
) -> Result<(), String> {
    let id = step.id;
    if !interactive {
        return Err(refusal(id, "draft"));
    }
    step.require_clean_outside_delta()?;
    if !ask() {
        return Err(refusal(id, "draft"));
    }
    let findings = gate_findings();
    if lint::has_error(&findings) {
        findings
            .iter()
            .filter(|f| f.is_error())
            .for_each(|f| eprintln!("{}", f.render()));
        return Err(stayed_draft(id));
    }
    step.update()?;
    if status_after().as_deref() != Some("in-progress") {
        return Err(stayed_draft(id));
    }
    step.commit()
}

fn gate_findings(store: &Store, root: &Path, id: &str) -> Vec<lint::Finding> {
    let options = lint::Options {
        delta: Some(id),
        gate: Some(Gate::InProgress),
        ..lint::Options::default()
    };
    lint::run(store, root, &options, None)
}

fn status_now(root: &Path, id: &str) -> Option<String> {
    let store = project::load_store(root).ok()?;
    World::load(&store, None)
        .docs()
        .iter()
        .find(|d| d.doc_type == "delta" && d.id == id)
        .map(|d| d.status.clone())
}

fn advance_delta(
    root: &Path,
    store: &Store,
    delta: &Doc,
    interactive: bool,
    ask: impl FnOnce() -> bool,
) -> Result<(), String> {
    let dir = Path::new(&delta.path)
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    let step = Move {
        program: "lazyspec",
        root,
        id: &delta.id,
        dir: &dir,
    };
    advance_with(
        &step,
        interactive,
        ask,
        || gate_findings(store, root, &delta.id),
        || status_now(root, &delta.id),
    )
}

pub fn offer(root: &Path, store: &Store, delta: &Doc) -> Result<(), String> {
    advance_delta(root, store, delta, interactive(), || confirmed(&delta.id))
}

/// `lemma vibe` moves the delta it drafted without asking (DECISION-015).
pub fn unasked(root: &Path, store: &Store, delta: &Doc) -> Result<(), String> {
    advance_delta(root, store, delta, true, || true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testgit;
    use std::cell::Cell;
    use std::os::unix::fs::PermissionsExt;

    const DIR: &str = "docs/deltas/DELTA-001";

    fn fake_lazyspec(dir: &Path, body: &str) -> String {
        let script = dir.join("fake-lazyspec");
        std::fs::write(&script, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        script.to_string_lossy().into_owned()
    }

    fn clean_repo() -> testgit::Repo {
        let repo = testgit::Repo::new();
        repo.write("a.txt", "a\n");
        repo.write(&format!("{DIR}/index.md"), "delta\n");
        repo.commit("init");
        repo
    }

    fn step<'a>(program: &'a str, repo: &'a testgit::Repo) -> Move<'a> {
        Move {
            program,
            root: repo.root(),
            id: "DELTA-001",
            dir: DIR,
        }
    }

    fn moved() -> Option<String> {
        Some("in-progress".to_string())
    }

    fn error_finding() -> lint::Finding {
        lint::Finding::bare_error("claim has no proof")
    }

    fn advance(
        step: &Move,
        interactive: bool,
        findings: Vec<lint::Finding>,
        status: Option<String>,
    ) -> Result<(), String> {
        advance_with(step, interactive, || true, || findings, || status)
    }

    #[test]
    fn off_a_terminal_the_move_is_refused_naming_the_command_before_the_tree_is_read() {
        let repo = clean_repo();
        repo.write("stray.txt", "x\n");
        let bin = tempfile::tempdir().unwrap();
        let program = fake_lazyspec(bin.path(), "exit 0");

        let error = advance(&step(&program, &repo), false, vec![], moved()).unwrap_err();

        assert!(
            error.contains("`lazyspec update DELTA-001 --status in-progress`"),
            "{error}"
        );
    }

    #[test]
    fn a_declined_move_is_refused_naming_the_command() {
        let repo = clean_repo();
        let bin = tempfile::tempdir().unwrap();
        let program = fake_lazyspec(bin.path(), "exit 0");

        let error =
            advance_with(&step(&program, &repo), true, || false, Vec::new, moved).unwrap_err();

        assert_eq!(error, refusal("DELTA-001", "draft"));
    }

    #[test]
    fn every_refusal_names_the_real_command() {
        for status in ["draft", "complete"] {
            assert!(
                refusal("DELTA-001", status)
                    .contains("`lazyspec update DELTA-001 --status in-progress`"),
                "{status}"
            );
        }
    }

    #[test]
    fn a_zero_exit_commits_what_the_move_wrote() {
        let repo = clean_repo();
        let bin = tempfile::tempdir().unwrap();
        let program = fake_lazyspec(bin.path(), &format!("echo moved > {DIR}/moved.txt"));

        advance(&step(&program, &repo), true, vec![], moved()).unwrap();

        assert_eq!(
            repo.git(&["log", "-1", "--format=%s"]),
            "DELTA-001: in-progress"
        );
        assert_eq!(repo.git(&["status", "--porcelain"]), "");
    }

    #[test]
    fn the_advance_commit_stages_only_the_delta_directory() {
        let repo = clean_repo();
        let bin = tempfile::tempdir().unwrap();
        let program = fake_lazyspec(
            bin.path(),
            &format!("echo moved > {DIR}/moved.txt; echo cache > cache.txt"),
        );

        advance(&step(&program, &repo), true, vec![], moved()).unwrap();

        assert_eq!(
            repo.git(&["show", "--name-only", "--format=", "HEAD"]),
            format!("{DIR}/moved.txt")
        );
        assert_eq!(repo.git(&["status", "--porcelain"]), "?? cache.txt");
    }

    #[test]
    fn a_dirty_file_inside_the_delta_directory_is_committed_with_the_move() {
        let repo = clean_repo();
        repo.write(&format!("{DIR}/claims.md"), "[proposed] edited\n");
        let bin = tempfile::tempdir().unwrap();
        let program = fake_lazyspec(bin.path(), "exit 0");

        advance(&step(&program, &repo), true, vec![], moved()).unwrap();

        assert_eq!(
            repo.git(&["show", "--name-only", "--format=%s", "HEAD"]),
            format!("DELTA-001: in-progress\n\n{DIR}/claims.md")
        );
        assert_eq!(repo.git(&["status", "--porcelain"]), "");
    }

    #[test]
    fn a_dirty_file_outside_the_delta_directory_is_refused_before_any_prompt() {
        let repo = clean_repo();
        repo.write("dirty.txt", "x\n");
        let bin = tempfile::tempdir().unwrap();
        let program = fake_lazyspec(bin.path(), "exit 0");

        let error = advance_with(
            &step(&program, &repo),
            true,
            || panic!("asked on a dirty tree"),
            Vec::new,
            moved,
        )
        .unwrap_err();

        assert!(error.contains("clean working tree"), "{error}");
    }

    #[test]
    fn a_non_zero_exit_stops_and_leaves_the_delta_draft() {
        let repo = clean_repo();
        let bin = tempfile::tempdir().unwrap();
        let program = fake_lazyspec(bin.path(), "echo blocked >&2; exit 1");

        let error = advance(&step(&program, &repo), true, vec![], moved()).unwrap_err();

        assert_eq!(
            error,
            "DELTA-001 stayed draft; fix the findings above and run lemma build DELTA-001 again"
        );
    }

    #[test]
    fn gate_findings_stop_the_move_without_calling_lazyspec() {
        let repo = clean_repo();
        let bin = tempfile::tempdir().unwrap();
        let called = bin.path().join("called");
        let program = fake_lazyspec(bin.path(), &format!("touch {}", called.display()));

        let error =
            advance(&step(&program, &repo), true, vec![error_finding()], moved()).unwrap_err();

        assert_eq!(error, stayed_draft("DELTA-001"));
        assert!(!called.exists());
    }

    #[test]
    fn a_status_that_is_not_in_progress_after_a_zero_exit_stops_the_build() {
        let repo = clean_repo();
        let bin = tempfile::tempdir().unwrap();
        let program = fake_lazyspec(bin.path(), &format!("echo moved > {DIR}/moved.txt"));
        let head = repo.head();

        let error = advance(
            &step(&program, &repo),
            true,
            vec![],
            Some("draft".to_string()),
        )
        .unwrap_err();

        assert_eq!(error, stayed_draft("DELTA-001"));
        assert_eq!(repo.head(), head);
    }

    #[test]
    fn a_failed_commit_says_the_move_happened_but_was_not_committed() {
        let repo = clean_repo();
        let hook = repo.path(".git/hooks/pre-commit");
        std::fs::write(&hook, "#!/bin/sh\nexit 1\n").unwrap();
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();
        let bin = tempfile::tempdir().unwrap();
        let program = fake_lazyspec(bin.path(), &format!("echo moved > {DIR}/moved.txt"));
        let asked = Cell::new(false);

        let error = advance_with(
            &step(&program, &repo),
            true,
            || {
                asked.set(true);
                true
            },
            Vec::new,
            moved,
        )
        .unwrap_err();

        assert!(asked.get());
        assert_eq!(
            error,
            "DELTA-001 moved to in-progress but not committed; commit it and run lemma build DELTA-001"
        );
    }
}
