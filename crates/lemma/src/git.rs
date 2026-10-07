use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

pub fn git_output(root: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn nul_separated(root: &Path, args: &[&str]) -> Vec<String> {
    git_output(root, args)
        .unwrap_or_default()
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(str::to_string)
        .collect()
}

pub fn untracked_files(root: &Path) -> Vec<String> {
    nul_separated(root, &["ls-files", "--others", "--exclude-standard", "-z"])
}

pub fn changed_since(root: &Path, rev: &str) -> Vec<String> {
    let tracked = nul_separated(root, &["diff", "--name-only", "--relative", "-z", rev]);
    let all: BTreeSet<String> = tracked.into_iter().chain(untracked_files(root)).collect();
    all.into_iter().collect()
}

pub fn repo_files(root: &Path) -> Vec<String> {
    let tracked = nul_separated(root, &["ls-files", "-z"]);
    let all: BTreeSet<String> = tracked.into_iter().chain(untracked_files(root)).collect();
    all.into_iter().collect()
}
