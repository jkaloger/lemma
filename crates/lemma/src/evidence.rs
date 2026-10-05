use crate::receipt::{git_output, names_a_test, symbol_in};
use lazyspec::engine::symbols::{
    normalize_ast, RustSymbolExtractor, SymbolExtractor, TypeScriptSymbolExtractor,
};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefStatus {
    Ok,
    Unresolved(String),
    NotATest,
}

fn clean(path: &str) -> &str {
    path.trim_start_matches("./")
}

fn extension(path: &str) -> Option<&str> {
    Path::new(path).extension().and_then(|e| e.to_str())
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
        Some("ts") | Some("tsx") => Some(normalize_ast(
            &extract(&TypeScriptSymbolExtractor::new())?,
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        )),
        _ => Some(source.to_string()),
    }
}

pub fn region_now(root: &Path, path: &str, symbol: Option<&str>) -> Option<String> {
    let source = std::fs::read_to_string(root.join(clean(path))).ok()?;
    region(path, &source, symbol)
}

pub fn region_at(root: &Path, rev: &str, path: &str, symbol: Option<&str>) -> Option<String> {
    let source = git_output(root, &["show", &format!("{rev}:./{}", clean(path))])?;
    region(path, &source, symbol)
}

pub fn changed(root: &Path, base_rev: &str, path: &str, symbol: Option<&str>) -> bool {
    let Some(before) = region_at(root, base_rev, path, symbol) else {
        return true;
    };
    region_now(root, path, symbol).is_none_or(|now| now != before)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testgit::Repo;

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
    fn a_region_absent_at_base_is_changed() {
        let (repo, base) = repo_with("tests/a.rs", TESTS);
        repo.write(
            "tests/a.rs",
            &format!("{TESTS}\n#[test]\nfn added() {{}}\n"),
        );
        repo.write("tests/new.rs", TESTS);
        assert!(changed(repo.root(), &base, "tests/a.rs", Some("added")));
        assert!(changed(repo.root(), &base, "tests/new.rs", Some("other")));
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
    fn typescript_regions_are_extracted_and_normalised() {
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
