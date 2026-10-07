mod common;
use common::*;
use std::process::Command;

fn run_with_path(repo: &Repo, args: &[&str], path: &std::ffi::OsStr) -> Run {
    let out = Command::new(env!("CARGO_BIN_EXE_lemma"))
        .args(args)
        .env("PATH", path)
        .current_dir(repo.root())
        .output()
        .unwrap();
    Run {
        code: out.status.code().unwrap(),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

fn run_with_lazyspec(repo: &Repo, args: &[&str], version_output: &str) -> Run {
    let bin_dir = tempfile::tempdir().unwrap();
    install_fake_lazyspec(bin_dir.path(), version_output);
    run_with_path(repo, args, &path_with_prepended(bin_dir.path()))
}

fn run_without_lazyspec(repo: &Repo, args: &[&str]) -> Run {
    let bin_dir = tempfile::tempdir().unwrap();
    let git = Command::new("which").arg("git").output().unwrap().stdout;
    let git = String::from_utf8(git).unwrap();
    std::os::unix::fs::symlink(git.trim(), bin_dir.path().join("git")).unwrap();
    run_with_path(repo, args, bin_dir.path().as_os_str())
}

const STORE_COMMANDS: [&[&str]; 2] = [&["lint"], &["refs", "AC-1"]];

#[test]
fn store_commands_refuse_an_old_lazyspec_with_an_install_hint() {
    let repo = Repo::new();
    for args in STORE_COMMANDS {
        let run = run_with_lazyspec(&repo, args, "lazyspec 0.14.2");

        assert_eq!(run.code, 2, "{args:?}");
        assert!(run.stderr.contains("0.15"), "{args:?}: {}", run.stderr);
        assert!(run.stderr.contains("cargo install lazyspec"));
    }
}

#[test]
fn store_commands_refuse_a_missing_lazyspec() {
    let repo = Repo::new();
    for args in STORE_COMMANDS {
        let run = run_without_lazyspec(&repo, args);

        assert_eq!(run.code, 2, "{args:?}");
        assert!(run.stderr.contains("not found"), "{args:?}: {}", run.stderr);
    }
}

#[test]
fn stop_warns_about_an_old_lazyspec_without_blocking() {
    let repo = Repo::new();

    let run = run_with_lazyspec(&repo, &["stop"], "lazyspec 0.14.2");

    assert_eq!(run.code, 0);
    assert!(run.stderr.contains("0.15"));
    assert!(run.stderr.contains("cargo install lazyspec"));
}

#[test]
fn stop_warns_about_a_missing_lazyspec_without_blocking() {
    let repo = Repo::new();

    let run = run_without_lazyspec(&repo, &["stop"]);

    assert_eq!(run.code, 0);
    assert!(run.stderr.contains("not found"));
}

#[test]
fn a_current_lazyspec_passes_the_check() {
    let repo = Repo::new();

    let run = run_with_lazyspec(&repo, &["lint"], "lazyspec 0.15.1");

    assert_eq!(run.code, 0);
    assert_eq!(run.stderr, "");
}
