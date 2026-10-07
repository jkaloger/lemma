#![allow(dead_code)]

use serde_json::{json, Value};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tempfile::TempDir;

pub const CONFIG: &str = r#"
[[types]]
name = "spec"
plural = "specs"
dir = "docs/specs"
prefix = "SPEC"
subdirectory = true
authorship = "generated"
staleness = "drift"
lifecycle = { states = ["draft", "active"], edges = [{ from = "draft", to = "active" }] }

[[types]]
name = "delta"
plural = "deltas"
dir = "docs/deltas"
prefix = "DELTA"
subdirectory = true
authorship = "generated"
lifecycle = { states = ["draft", "in-progress", "complete"], edges = [{ from = "draft", to = "in-progress" }, { from = "in-progress", to = "complete" }] }

[[relationships]]
name = "changes"
inverse = "changed-by"
"#;

pub const TEST_FILE: &str = "tests/store_test.rs";
pub const TEST_SRC: &str = "#[test]\nfn reopen_keeps_prior() {}\n\n#[test]\nfn other_test() {}\n";

pub struct Run {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Run {
    pub fn json(&self) -> Value {
        serde_json::from_str(&self.stdout)
            .unwrap_or_else(|e| panic!("stdout is not json ({e}): {}", self.stdout))
    }

    pub fn findings(&self) -> Vec<Value> {
        self.json()["findings"]
            .as_array()
            .cloned()
            .unwrap_or_default()
    }

    pub fn messages(&self) -> Vec<String> {
        self.findings()
            .iter()
            .map(|f| f["message"].as_str().unwrap().to_string())
            .collect()
    }

    pub fn has_message(&self, needle: &str) -> bool {
        self.messages().iter().any(|m| m.contains(needle))
    }
}

pub struct Repo {
    dir: TempDir,
    fake_bin: TempDir,
    sub: &'static str,
}

pub fn install_fake_lazyspec(dir: &Path, version_output: &str) {
    use std::os::unix::fs::PermissionsExt;
    let script = dir.join("lazyspec");
    std::fs::write(&script, format!("#!/bin/sh\necho '{version_output}'\n")).unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
}

pub fn path_with_prepended(dir: &Path) -> std::ffi::OsString {
    let existing = std::env::var_os("PATH").unwrap_or_default();
    std::env::join_paths(std::iter::once(dir.to_path_buf()).chain(std::env::split_paths(&existing)))
        .unwrap()
}

impl Repo {
    pub fn new() -> Self {
        Self::in_subdir("")
    }

    pub fn in_subdir(sub: &'static str) -> Self {
        let fake_bin = tempfile::tempdir().unwrap();
        install_fake_lazyspec(fake_bin.path(), "lazyspec 0.15.0");
        let repo = Self {
            dir: tempfile::tempdir().unwrap(),
            fake_bin,
            sub,
        };
        std::fs::create_dir_all(repo.root()).unwrap();
        let init = Command::new("git")
            .arg("-C")
            .arg(repo.dir.path())
            .args(["init", "-q"])
            .output()
            .unwrap();
        assert!(init.status.success());
        repo.write(".lazyspec.toml", CONFIG);
        repo.write(TEST_FILE, TEST_SRC);
        repo.write("src/lib.rs", "pub fn a() {}\n");
        repo.commit("init");
        repo
    }

    pub fn root(&self) -> PathBuf {
        self.dir.path().join(self.sub)
    }

    pub fn path(&self, rel: &str) -> PathBuf {
        self.root().join(rel)
    }

    pub fn write(&self, rel: &str, content: &str) {
        let path = self.path(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    pub fn read(&self, rel: &str) -> String {
        std::fs::read_to_string(self.path(rel)).unwrap()
    }

    pub fn git(&self, args: &[&str]) -> String {
        let out = Command::new("git")
            .arg("-C")
            .arg(self.root())
            .args([
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@example.com",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap().trim().to_string()
    }

    pub fn commit(&self, message: &str) -> String {
        self.git(&["add", "-A"]);
        self.git(&["commit", "-q", "--allow-empty", "-m", message]);
        self.head()
    }

    pub fn head(&self) -> String {
        self.git(&["rev-parse", "--short", "HEAD"])
    }

    pub fn run(&self, args: &[&str], stdin: &str) -> Run {
        let mut child = Command::new(env!("CARGO_BIN_EXE_lemma"))
            .args(args)
            .env("PATH", path_with_prepended(self.fake_bin.path()))
            .current_dir(self.root())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(stdin.as_bytes())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        Run {
            code: out.status.code().unwrap(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        }
    }

    pub fn spec_file(&self, n: u32, status: &str, governs: &[&str], body: &str) {
        self.spec_file_with_tests(n, status, governs, &[], body);
    }

    pub fn spec_file_with_tests(
        &self,
        n: u32,
        status: &str,
        governs: &[&str],
        tests: &[&str],
        body: &str,
    ) {
        let globs = quote_all(governs);
        let tests_line = match tests.is_empty() {
            true => String::new(),
            false => format!("tests: [{}]\n", quote_all(tests)),
        };
        let content = format!(
            "---\ntitle: \"Spec {n}\"\ntype: spec\nstatus: {status}\nauthor: \"t\"\ndate: 2026-01-01\ntags: []\ngoverns: [{globs}]\n{tests_line}---\n\n{body}"
        );
        self.write(&format!("{}/index.md", spec_dir(n)), &content);
        self.spec_behaviour(n, "");
    }

    pub fn spec_behaviour(&self, n: u32, body: &str) {
        self.write(&spec_part_path(n), body);
    }

    pub fn delta_file(&self, n: u32, status: &str, changes: &[&str], body: &str) {
        let related: String = changes
            .iter()
            .map(|f| format!("- changes: {f}\n"))
            .collect();
        let related = if related.is_empty() {
            String::new()
        } else {
            format!("related:\n{related}")
        };
        let content = format!(
            "---\ntitle: \"Delta {n}\"\ntype: delta\nstatus: {status}\nauthor: \"t\"\ndate: 2026-01-01\ntags: []\n{related}---\n\n{body}"
        );
        self.write(&format!("{}/index.md", delta_dir(n)), &content);
    }

    pub fn delta_part(&self, n: u32, part: &str, body: &str) {
        self.write(&delta_part_path(n, part), body);
    }

    pub fn delta_claims(&self, n: u32, body: &str) {
        self.delta_part(n, "claims", body);
    }

    pub fn delta_tasks(&self, n: u32, body: &str) {
        self.delta_part(n, "tasks", body);
    }
}

fn quote_all(items: &[&str]) -> String {
    items
        .iter()
        .map(|g| format!("\"{g}\""))
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn spec_dir(n: u32) -> String {
    format!("docs/specs/SPEC-00{n}-s")
}

pub fn spec_part_path(n: u32) -> String {
    format!("{}/behaviour.md", spec_dir(n))
}

pub fn delta_dir(n: u32) -> String {
    format!("docs/deltas/DELTA-00{n}-d")
}

pub fn delta_part_path(n: u32, part: &str) -> String {
    format!("{}/{part}.md", delta_dir(n))
}

pub fn doc(
    id: &str,
    doc_type: &str,
    status: &str,
    governs: &[&str],
    changes: &[&str],
    body: &str,
) -> Value {
    let related: Vec<Value> = changes
        .iter()
        .map(|t| json!({"type": "changes", "target": t}))
        .collect();
    json!({
        "id": id,
        "path": format!("docs/{doc_type}s/{id}-x/index.md"),
        "title": id,
        "type": doc_type,
        "status": status,
        "governs": governs,
        "related": related,
        "body": body,
        "content_hash": format!("hash-{id}"),
    })
}

pub fn with_parts(mut doc: Value, parts: &[(&str, &str)]) -> Value {
    let dir = doc["path"]
        .as_str()
        .unwrap()
        .trim_end_matches("/index.md")
        .to_string();
    doc["parts"] = parts
        .iter()
        .map(|(name, body)| json!({"name": name, "path": format!("{dir}/{name}.md"), "body": body}))
        .collect();
    doc
}

pub fn with_tests(mut doc: Value, tests: &[&str]) -> Value {
    doc["attributes"]["tests"] = json!(tests);
    doc
}

pub fn validate_payload(docs: Vec<Value>) -> String {
    json!({"event": "validate", "hook": "lemma", "documents": docs}).to_string()
}

pub fn transition_payload(from: &str, to: &str, moved: Value, context: Vec<Value>) -> String {
    json!({
        "event": "pre-transition",
        "hook": "lemma",
        "transition": {"from": from, "to": to},
        "documents": [moved],
        "context": context,
    })
    .to_string()
}

pub const INTENT: &str = "## Intent\n\nBecause.\n";

pub const DICTUM_CONFIG: &str = r#"
[[types]]
name = "convention"
plural = "convention"
dir = "docs/convention"
prefix = "CONVENTION"
singleton = true
lifecycle = { states = ["draft", "accepted"], edges = [{ from = "draft", to = "accepted" }] }

[[types]]
name = "dictum"
plural = "dicta"
dir = "docs/convention"
prefix = "DICTUM"
parent_type = "convention"
lifecycle = { states = ["draft", "accepted"], edges = [{ from = "draft", to = "accepted" }] }
"#;

pub const DICTUM_PATH: &str = "docs/convention/DICTUM-001-testing.md";

impl Repo {
    pub fn with_dictums(&self) {
        self.write(".lazyspec.toml", &format!("{CONFIG}{DICTUM_CONFIG}"));
        self.write(
            "docs/convention/CONVENTION.md",
            "---\ntitle: \"Convention\"\ntype: convention\nstatus: accepted\nauthor: \"t\"\ndate: 2026-01-01\ntags: []\n---\n\nHouse rules.\n",
        );
    }

    pub fn dictum_file(&self, governs: &[&str], body: &str) {
        let globs = quote_all(governs);
        let content = format!(
            "---\ntitle: \"Testing\"\ntype: dictum\nstatus: accepted\nauthor: \"t\"\ndate: 2026-01-01\ntags: []\ngoverns: [{globs}]\n---\n\n{body}"
        );
        self.write(DICTUM_PATH, &content);
    }
}
