use crate::claims::Receipt;
use globset::{Glob, GlobSet, GlobSetBuilder};
use lazyspec::engine::symbols::{RustSymbolExtractor, SymbolExtractor, TypeScriptSymbolExtractor};
use regex::Regex;
use std::cell::RefCell;
use std::collections::HashMap;
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

fn glob_set(patterns: &[String]) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    patterns
        .iter()
        .filter_map(|p| Glob::new(p).ok())
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

pub struct Checker {
    root: PathBuf,
    diffs: RefCell<HashMap<String, Option<Vec<String>>>>,
}

impl Checker {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            diffs: RefCell::new(HashMap::new()),
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

    pub fn resolve(&self, receipt: &Receipt) -> Result<String, String> {
        let Some(sha) = receipt.sha.as_deref() else {
            return Err(format!("receipt for {} has no pinned commit", receipt.path));
        };
        if !self.commit_exists(sha) {
            return Err(format!("commit {sha} does not exist"));
        }
        self.symbol_resolves(&receipt.path, receipt.symbol.as_deref())?;
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
        if changed.iter().any(|f| f == test_path) {
            return Verdict::Stale(format!("{test_path} changed since {sha}"));
        }
        let governed = glob_set(governs);
        match changed.iter().find(|f| governed.is_match(f.as_str())) {
            Some(file) => Verdict::Stale(format!("governed file {file} changed since {sha}")),
            None => Verdict::Fresh,
        }
    }
}
