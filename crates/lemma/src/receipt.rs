use crate::claims::{Claim, Receipt};
use crate::payload::{Doc, Finding};
use globset::{Glob, GlobSet, GlobSetBuilder};
use lazyspec::engine::symbols::{RustSymbolExtractor, SymbolExtractor, TypeScriptSymbolExtractor};
use regex::Regex;
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Fresh,
    Unresolved(String),
    Stale(String),
}

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

pub fn git_lines(root: &Path, args: &[&str]) -> Option<Vec<String>> {
    let out = git_output(root, args)?;
    Some(out.lines().map(str::to_string).collect())
}

fn git_nul_separated(root: &Path, args: &[&str]) -> Vec<String> {
    git_output(root, args)
        .unwrap_or_default()
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(str::to_string)
        .collect()
}

pub fn touched_files(root: &Path) -> Vec<String> {
    let tracked = git_nul_separated(root, &["diff", "--name-only", "--relative", "-z", "HEAD"]);
    let untracked = git_nul_separated(root, &["ls-files", "--others", "--exclude-standard", "-z"]);
    let all: BTreeSet<String> = tracked.into_iter().chain(untracked).collect();
    all.into_iter().collect()
}

pub fn is_dirty(root: &Path, file: &str) -> bool {
    git_output(root, &["status", "--porcelain", "--", file]).is_some_and(|s| !s.trim().is_empty())
}

fn normalised(pattern: &str) -> &str {
    pattern.trim_start_matches("./")
}

pub fn invalid_globs(patterns: &[String]) -> Vec<(String, String)> {
    patterns
        .iter()
        .filter_map(|p| {
            let err = Glob::new(normalised(p)).err()?;
            Some((p.clone(), err.to_string()))
        })
        .collect()
}

pub fn invalid_glob_findings(spec: &Doc) -> Vec<Finding> {
    let governs = invalid_globs(&spec.governs)
        .into_iter()
        .map(|g| ("governs", g));
    let tests = invalid_globs(&spec.tests_globs())
        .into_iter()
        .map(|g| ("tests", g));
    governs
        .chain(tests)
        .map(|(key, (glob, err))| {
            Finding::error(
                &spec.id,
                None,
                format!("{key} glob `{glob}` is invalid: {err}"),
            )
        })
        .collect()
}

pub fn glob_set(patterns: &[String]) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    patterns
        .iter()
        .filter_map(|p| Glob::new(normalised(p)).ok())
        .for_each(|g| {
            builder.add(g);
        });
    builder.build().unwrap_or_else(|_| GlobSet::empty())
}

fn text_has_word(source: &str, symbol: &str) -> bool {
    Regex::new(&format!(r"\b{}\b", regex::escape(symbol)))
        .map(|re| re.is_match(source))
        .unwrap_or(false)
}

fn symbol_in(path: &str, source: &str, symbol: &str) -> bool {
    let extension = Path::new(path).extension().and_then(|e| e.to_str());
    match extension {
        Some("rs") => RustSymbolExtractor::new().extract(source, symbol).is_some(),
        Some("ts") | Some("tsx") => TypeScriptSymbolExtractor::new()
            .extract(source, symbol)
            .is_some(),
        _ => text_has_word(source, symbol),
    }
}

pub fn names_a_test(path: &str, globs: &[String]) -> bool {
    glob_set(globs).is_match(path.trim_start_matches("./"))
}

pub fn non_test_receipts(doc_id: &str, part: &str, claim: &Claim, spec: &Doc) -> Vec<Finding> {
    let globs = spec.tests_globs();
    claim
        .receipts
        .iter()
        .filter(|r| r.checked_by.is_none() && !names_a_test(&r.path, &globs))
        .map(|r| {
            Finding::error(
                doc_id,
                Some(r.line),
                format!(
                    "{}: receipt {} is not a test file under {}'s `tests` globs",
                    claim.id, r.path, spec.id
                ),
            )
            .in_part(part)
        })
        .collect()
}

pub struct Checker {
    root: PathBuf,
    diffs: RefCell<HashMap<String, Option<Vec<String>>>>,
    dirty: RefCell<Option<Vec<String>>>,
}

impl Checker {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            diffs: RefCell::new(HashMap::new()),
            dirty: RefCell::new(None),
        }
    }

    pub fn head_short(&self) -> Option<String> {
        let out = git_output(&self.root, &["rev-parse", "--short", "HEAD"])?;
        Some(out.trim().to_string())
    }

    pub fn symbol_resolves(&self, path: &str, symbol: Option<&str>) -> Result<(), String> {
        let file = self.root.join(path.trim_start_matches("./"));
        let source =
            std::fs::read_to_string(&file).map_err(|_| format!("file {path} does not exist"))?;
        let Some(symbol) = symbol else {
            return Ok(());
        };
        if symbol_in(path, &source, symbol) {
            return Ok(());
        }
        Err(format!("symbol {symbol} not found in {path}"))
    }

    fn commit_exists(&self, sha: &str) -> bool {
        let spec = format!("{sha}^{{commit}}");
        git_output(&self.root, &["rev-parse", "--verify", "--quiet", &spec]).is_some()
    }

    pub fn changed_since(&self, sha: &str) -> Option<Vec<String>> {
        if let Some(cached) = self.diffs.borrow().get(sha) {
            return cached.clone();
        }
        let diff = git_lines(
            &self.root,
            &["diff", "--name-only", "--relative", sha, "HEAD"],
        );
        self.diffs
            .borrow_mut()
            .insert(sha.to_string(), diff.clone());
        diff
    }

    pub fn dirty_paths(&self) -> Vec<String> {
        if let Some(cached) = self.dirty.borrow().as_ref() {
            return cached.clone();
        }
        let paths = touched_files(&self.root);
        *self.dirty.borrow_mut() = Some(paths.clone());
        paths
    }

    pub fn resolve(&self, receipt: &Receipt) -> Result<String, String> {
        let Some(sha) = receipt.sha.as_deref() else {
            return Err(format!("receipt for {} has no pinned commit", receipt.path));
        };
        if !self.commit_exists(sha) {
            return Err(format!("commit {sha} does not exist"));
        }
        if receipt.checked_by.is_none() {
            self.symbol_resolves(&receipt.path, receipt.symbol.as_deref())?;
        }
        Ok(sha.to_string())
    }

    pub fn check(&self, receipt: &Receipt, governs: &[String]) -> Verdict {
        let sha = match self.resolve(receipt) {
            Ok(sha) => sha,
            Err(reason) => return Verdict::Unresolved(reason),
        };
        if git_output(&self.root, &["merge-base", "--is-ancestor", &sha, "HEAD"]).is_none() {
            return Verdict::Unresolved(format!(
                "pin {sha} is not on this branch's history; re-pin after merge"
            ));
        }
        let Some(changed) = self.changed_since(&sha) else {
            return Verdict::Unresolved(format!("cannot diff {sha} against HEAD"));
        };
        let test_path = receipt.path.trim_start_matches("./");
        if receipt.checked_by.is_none() && changed.iter().any(|f| f == test_path) {
            return Verdict::Stale(format!("{test_path} changed since {sha}"));
        }
        let governed = glob_set(governs);
        match changed.iter().find(|f| governed.is_match(f.as_str())) {
            Some(file) => Verdict::Stale(format!("governed file {file} changed since {sha}")),
            None => Verdict::Fresh,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defaults() -> Vec<String> {
        crate::payload::tests_or_default(&crate::payload::Attributes::default())
    }

    #[test]
    fn default_globs_accept_test_paths() {
        for path in [
            "tests/foo.rs",
            "src/tests/x.rs",
            "foo.test.ts",
            "foo_test.go",
            "a/b.spec.tsx",
            "./tests/foo.rs",
        ] {
            assert!(names_a_test(path, &defaults()), "{path}");
        }
    }

    #[test]
    fn default_globs_reject_production_paths() {
        for path in ["src/store.rs", "src/lib.rs", "docs/specs/x.md"] {
            assert!(!names_a_test(path, &defaults()), "{path}");
        }
    }

    #[test]
    fn leading_dot_slash_is_ignored_in_globs() {
        let globs = vec!["./checks/**".to_string()];
        assert!(names_a_test("checks/foo.rs", &globs));
        assert!(invalid_globs(&globs).is_empty());
    }

    #[test]
    fn invalid_globs_report_pattern_and_error() {
        let globs = vec!["src/**".to_string(), "src/[".to_string()];
        let bad = invalid_globs(&globs);
        assert_eq!(bad.len(), 1);
        assert_eq!(bad[0].0, "src/[");
        assert!(!bad[0].1.is_empty());
    }

    #[test]
    fn explicit_globs_replace_the_defaults() {
        let globs = vec!["checks/**".to_string()];
        assert!(names_a_test("checks/foo.rs", &globs));
        assert!(!names_a_test("tests/foo.rs", &globs));
    }
}
