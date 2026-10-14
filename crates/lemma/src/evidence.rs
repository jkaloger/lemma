use crate::git::git_output;
use globset::{Glob, GlobSet, GlobSetBuilder};
use lazyspec::engine::symbols::{
    normalize_ast, RustSymbolExtractor, SymbolExtractor, TypeScriptSymbolExtractor,
};
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefStatus {
    Ok,
    Unresolved(String),
    NotATest,
}

pub fn clean(path: &str) -> &str {
    path.trim_start_matches("./")
}

fn is_script(extension: &str) -> bool {
    matches!(extension, "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs")
}

fn extension(path: &str) -> Option<&str> {
    Path::new(path).extension().and_then(|e| e.to_str())
}

pub fn invalid_globs(patterns: &[String]) -> Vec<(String, String)> {
    patterns
        .iter()
        .filter_map(|p| {
            let err = Glob::new(clean(p)).err()?;
            Some((p.clone(), err.to_string()))
        })
        .collect()
}

pub fn glob_set(patterns: &[String]) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    patterns
        .iter()
        .filter_map(|p| Glob::new(clean(p)).ok())
        .for_each(|g| {
            builder.add(g);
        });
    builder.build().unwrap_or_else(|_| GlobSet::empty())
}

fn names_a_test(path: &str, globs: &[String]) -> bool {
    glob_set(globs).is_match(clean(path))
}

fn word_offset(source: &str, symbol: &str) -> Option<usize> {
    Regex::new(&format!(r"\b{}\b", regex::escape(symbol)))
        .ok()?
        .find(source)
        .map(|m| m.start())
}

/// Byte offset where `symbol` starts in `source`. The lazyspec extractors return the
/// symbol's source text, not a span, so the text is located again in the file.
fn symbol_offset(path: &str, source: &str, symbol: &str) -> Option<usize> {
    match extension(path) {
        Some("rs") => source.find(&RustSymbolExtractor::new().extract(source, symbol)?),
        Some(ext) if is_script(ext) => {
            source.find(&TypeScriptSymbolExtractor::new().extract(source, symbol)?)
        }
        _ => word_offset(source, symbol),
    }
}

fn symbol_in(path: &str, source: &str, symbol: &str) -> bool {
    symbol_offset(path, source, symbol).is_some()
}

/// 1-based line where `symbol` starts in the file at `path`; line 1 with no symbol.
pub fn symbol_line(root: &Path, path: &str, symbol: Option<&str>) -> Option<usize> {
    let Some(symbol) = symbol else {
        return Some(1);
    };
    let source = std::fs::read_to_string(root.join(clean(path))).ok()?;
    let offset = symbol_offset(path, &source, symbol)?;
    Some(source[..offset].matches('\n').count() + 1)
}

pub fn resolve(root: &Path, path: &str, symbol: Option<&str>, tests: &[String]) -> RefStatus {
    let Ok(source) = std::fs::read_to_string(root.join(clean(path))) else {
        return RefStatus::Unresolved(format!("file {path} does not exist"));
    };
    if let Some(symbol) = symbol {
        if !symbol_in(path, &source, symbol) {
            return RefStatus::Unresolved(format!("symbol {symbol} not found in {path}"));
        }
    }
    match names_a_test(path, tests) {
        true => RefStatus::Ok,
        false => RefStatus::NotATest,
    }
}

fn region(path: &str, source: &str, symbol: Option<&str>) -> Option<String> {
    let extract = |extractor: &dyn SymbolExtractor| match symbol {
        Some(symbol) => extractor.extract(source, symbol),
        None => Some(source.to_string()),
    };
    match extension(path) {
        Some("rs") => Some(normalize_ast(
            &extract(&RustSymbolExtractor::new())?,
            tree_sitter_rust::LANGUAGE.into(),
        )),
        Some(ext) if is_script(ext) => Some(normalize_ast(
            &extract(&TypeScriptSymbolExtractor::new())?,
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        )),
        _ => Some(source.to_string()),
    }
}

fn source_now(root: &Path, path: &str) -> Option<String> {
    std::fs::read_to_string(root.join(clean(path))).ok()
}

fn source_at(root: &Path, rev: &str, path: &str) -> Option<String> {
    git_output(root, &["show", &format!("{rev}:./{}", clean(path))])
}

type Ref = (String, Option<String>);

pub struct Regions<'a> {
    root: &'a Path,
    base: &'a str,
    sources: HashMap<String, (Option<String>, Option<String>)>,
    results: HashMap<Ref, bool>,
}

impl<'a> Regions<'a> {
    pub fn new(root: &'a Path, base: &'a str) -> Self {
        Self {
            root,
            base,
            sources: HashMap::new(),
            results: HashMap::new(),
        }
    }

    pub fn changed(&mut self, path: &str, symbol: Option<&str>) -> bool {
        let key = (clean(path).to_string(), symbol.map(str::to_string));
        if let Some(&known) = self.results.get(&key) {
            return known;
        }
        let (root, base) = (self.root, self.base);
        let (before, now) = self
            .sources
            .entry(key.0.clone())
            .or_insert_with(|| (source_at(root, base, path), source_now(root, path)));
        let result = differs(path, before.as_deref(), now.as_deref(), symbol);
        self.results.insert(key, result);
        result
    }
}

fn differs(path: &str, before: Option<&str>, now: Option<&str>, symbol: Option<&str>) -> bool {
    let Some(before) = before.and_then(|source| region(path, source, symbol)) else {
        return false;
    };
    now.and_then(|source| region(path, source, symbol))
        .is_none_or(|now| now != before)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testgit::Repo;

    fn region_now(root: &Path, path: &str, symbol: Option<&str>) -> Option<String> {
        region(path, &source_now(root, path)?, symbol)
    }

    fn region_at(root: &Path, rev: &str, path: &str, symbol: Option<&str>) -> Option<String> {
        region(path, &source_at(root, rev, path)?, symbol)
    }

    fn changed(root: &Path, base: &str, path: &str, symbol: Option<&str>) -> bool {
        Regions::new(root, base).changed(path, symbol)
    }

    const TESTS: &str =
        "#[test]\nfn reopen_keeps_prior() {\n    assert_eq!(1, 1);\n}\n\n#[test]\nfn other() {}\n";

    fn globs() -> Vec<String> {
        crate::payload::tests_or_default(&crate::payload::Attributes::default())
    }

    fn repo_with(path: &str, content: &str) -> (Repo, String) {
        let repo = Repo::new();
        repo.write(path, content);
        let base = repo.commit("base");
        (repo, base)
    }

    #[test]
    fn default_globs_accept_test_paths_and_reject_production_paths() {
        for path in [
            "tests/foo.rs",
            "src/tests/x.rs",
            "foo.test.ts",
            "foo_test.go",
            "a/b.spec.tsx",
            "./tests/foo.rs",
        ] {
            assert!(names_a_test(path, &globs()), "{path}");
        }
        for path in ["src/store.rs", "src/lib.rs", "docs/specs/x.md"] {
            assert!(!names_a_test(path, &globs()), "{path}");
        }
    }

    #[test]
    fn leading_dot_slash_is_ignored_and_invalid_globs_are_reported() {
        let globs = vec!["./checks/**".to_string(), "src/[".to_string()];
        assert!(names_a_test("checks/foo.rs", &globs));
        let bad = invalid_globs(&globs);
        assert_eq!(bad.len(), 1);
        assert_eq!(bad[0].0, "src/[");
    }

    #[test]
    fn a_symbol_in_a_test_file_resolves() {
        let (repo, _) = repo_with("tests/a.rs", TESTS);
        let status = resolve(
            repo.root(),
            "tests/a.rs",
            Some("reopen_keeps_prior"),
            &globs(),
        );
        assert_eq!(status, RefStatus::Ok);
    }

    #[test]
    fn a_missing_symbol_or_file_is_unresolved() {
        let (repo, _) = repo_with("tests/a.rs", TESTS);
        for (path, symbol) in [("tests/a.rs", "nope"), ("tests/missing.rs", "x")] {
            let status = resolve(repo.root(), path, Some(symbol), &globs());
            assert!(
                matches!(status, RefStatus::Unresolved(_)),
                "{path}#{symbol}"
            );
        }
    }

    #[test]
    fn a_symbol_outside_the_tests_globs_is_not_a_test() {
        let (repo, _) = repo_with("src/store.rs", "pub fn reopen() {}\n");
        let status = resolve(repo.root(), "src/store.rs", Some("reopen"), &globs());
        assert_eq!(status, RefStatus::NotATest);
    }

    #[test]
    fn unresolved_wins_over_not_a_test() {
        let (repo, _) = repo_with("src/store.rs", "pub fn reopen() {}\n");
        let status = resolve(repo.root(), "src/store.rs", Some("nope"), &globs());
        assert!(matches!(status, RefStatus::Unresolved(_)));
    }

    #[test]
    fn explicit_tests_globs_replace_the_defaults() {
        let (repo, _) = repo_with("checks/a.rs", TESTS);
        let custom = vec!["checks/**".to_string()];
        let status = resolve(repo.root(), "checks/a.rs", Some("other"), &custom);
        assert_eq!(status, RefStatus::Ok);
        assert_eq!(
            resolve(repo.root(), "checks/a.rs", Some("other"), &globs()),
            RefStatus::NotATest
        );
    }

    #[test]
    fn a_sibling_test_added_leaves_the_region_unchanged() {
        let (repo, base) = repo_with("tests/a.rs", TESTS);
        repo.write(
            "tests/a.rs",
            &format!("{TESTS}\n#[test]\nfn added() {{}}\n"),
        );
        assert!(!changed(
            repo.root(),
            &base,
            "tests/a.rs",
            Some("reopen_keeps_prior")
        ));
    }

    #[test]
    fn formatting_and_comments_do_not_change_the_region() {
        let (repo, base) = repo_with("tests/a.rs", TESTS);
        repo.write(
            "tests/a.rs",
            &TESTS.replace(
                "    assert_eq!(1, 1);",
                "    // note\n    assert_eq!(1,   1);",
            ),
        );
        assert!(!changed(
            repo.root(),
            &base,
            "tests/a.rs",
            Some("reopen_keeps_prior")
        ));
    }

    #[test]
    fn a_body_edit_changes_the_region() {
        let (repo, base) = repo_with("tests/a.rs", TESTS);
        repo.write(
            "tests/a.rs",
            &TESTS.replace("assert_eq!(1, 1)", "assert_eq!(1, 2)"),
        );
        assert!(changed(
            repo.root(),
            &base,
            "tests/a.rs",
            Some("reopen_keeps_prior")
        ));
        assert!(!changed(repo.root(), &base, "tests/a.rs", Some("other")));
    }

    #[test]
    fn a_region_absent_at_base_is_not_a_change() {
        let (repo, base) = repo_with("tests/a.rs", TESTS);
        repo.write(
            "tests/a.rs",
            &format!("{TESTS}\n#[test]\nfn added() {{}}\n"),
        );
        repo.write("tests/new.rs", TESTS);
        assert!(!changed(repo.root(), &base, "tests/a.rs", Some("added")));
        assert!(!changed(repo.root(), &base, "tests/new.rs", Some("other")));
    }

    #[test]
    fn a_region_present_at_base_and_gone_now_is_changed() {
        let (repo, base) = repo_with("tests/a.rs", TESTS);
        repo.write("tests/a.rs", "#[test]\nfn other() {}\n");
        assert!(changed(
            repo.root(),
            &base,
            "tests/a.rs",
            Some("reopen_keeps_prior")
        ));
        std::fs::remove_file(repo.path("tests/a.rs")).unwrap();
        assert!(changed(repo.root(), &base, "tests/a.rs", Some("other")));
    }

    #[test]
    fn javascript_files_use_the_typescript_extractor() {
        let src = "function a() { return 1; }\nfunction b() { return 2; }\n";
        for ext in ["js", "jsx", "mjs", "cjs"] {
            let path = format!("tests/a.{ext}");
            let (repo, base) = repo_with(&path, src);
            repo.write(&path, &format!("{src}function added() {{}}\n"));
            assert!(!changed(repo.root(), &base, &path, Some("a")), "{ext}");
            repo.write(&path, &src.replace("return 2", "return 3"));
            assert!(!changed(repo.root(), &base, &path, Some("a")), "{ext}");
            assert!(changed(repo.root(), &base, &path, Some("b")), "{ext}");
            assert_eq!(
                resolve(repo.root(), &path, Some("a"), &["tests/**".to_string()]),
                RefStatus::Ok,
                "{ext}"
            );
        }
    }

    #[test]
    fn a_file_without_an_extractor_compares_whole() {
        let (repo, base) = repo_with("tests/check.sh", "echo one\n# a\necho two\n");
        assert!(!changed(
            repo.root(),
            &base,
            "tests/check.sh",
            Some("anything")
        ));
        repo.write("tests/check.sh", "echo one\n# b\necho two\n");
        assert!(changed(
            repo.root(),
            &base,
            "tests/check.sh",
            Some("anything")
        ));
    }

    #[test]
    fn typescript_regions_are_extracted_and_clean() {
        let src = "function a() { return 1; }\nfunction b() { return 2; }\n";
        let (repo, base) = repo_with("tests/a.ts", src);
        repo.write("tests/a.ts", &src.replace("return 2", "return 3"));
        assert!(!changed(repo.root(), &base, "tests/a.ts", Some("a")));
        assert!(changed(repo.root(), &base, "tests/a.ts", Some("b")));
    }

    #[test]
    fn regions_are_read_relative_to_a_subdirectory_root() {
        let repo = Repo::new();
        repo.write("pkg/tests/a.rs", TESTS);
        let base = repo.commit("base");
        let root = repo.path("pkg");
        assert!(region_at(&root, &base, "tests/a.rs", Some("other")).is_some());
        assert!(!changed(&root, &base, "tests/a.rs", Some("other")));
    }

    #[test]
    fn region_text_is_available_at_a_rev_and_in_the_working_tree() {
        let (repo, base) = repo_with("tests/a.rs", TESTS);
        repo.write(
            "tests/a.rs",
            &TESTS.replace("fn other() {}", "fn other() { let x = 1; }"),
        );
        let before = region_at(repo.root(), &base, "tests/a.rs", Some("other")).unwrap();
        let now = region_now(repo.root(), "tests/a.rs", Some("other")).unwrap();
        assert!(!before.contains("let x"));
        assert!(now.contains("let x"));
    }
}
