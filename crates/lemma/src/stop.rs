use crate::payload::Finding;
use crate::receipt::{git_output, touched_files};
use crate::session::recorded_base;
use lazyspec::engine::document::DocMeta;
use lazyspec::engine::store::Store;
use regex::Regex;
use std::collections::BTreeSet;
use std::path::Path;

pub fn run(store: &Store, root: &Path, session_id: Option<&str>) -> Vec<Finding> {
    let base = session_id.and_then(|id| recorded_base(root, id));
    let touched = session_touched(root, base.as_deref());
    let mut findings = Vec::new();
    findings.extend(cross_governs(store, root, &touched, base.as_deref()));
    findings.extend(pending_tests(root, &touched));
    findings.extend(unapproved_completions(store, root, &touched));
    findings
}

fn session_touched(root: &Path, base: Option<&str>) -> Vec<String> {
    let current = touched_files(root);
    let Some(base) = base else {
        return current;
    };
    let committed = git_output(
        root,
        &["diff", "--name-only", "--relative", "-z", base, "HEAD"],
    )
    .unwrap_or_default();
    let all: BTreeSet<String> = committed
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(str::to_string)
        .chain(current)
        .collect();
    all.into_iter().collect()
}

fn status_at(root: &Path, rev: &str, doc_path: &Path) -> Option<String> {
    let spec = format!("{rev}:./{}", doc_path.to_string_lossy());
    let text = git_output(root, &["show", &spec])?;
    Some(DocMeta::parse(&text).ok()?.status.to_string())
}

fn was_complete_at(root: &Path, rev: &str, doc_path: &Path) -> bool {
    status_at(root, rev, doc_path).as_deref() == Some("complete")
}

fn unapproved_completions(store: &Store, root: &Path, touched: &[String]) -> Vec<Finding> {
    store
        .all_docs()
        .into_iter()
        .filter(|d| d.doc_type.to_string() == "delta" && d.status.to_string() == "complete")
        .filter(|d| touched.iter().any(|f| Path::new(f) == d.path))
        .filter(|d| !was_complete_at(root, "HEAD", &d.path))
        .map(|d| {
            Finding::bare_error(format!(
                "{} is complete in the working tree but not at HEAD. If a human moved it, leave it and ask them to commit; never change the status back",
                d.id
            ))
        })
        .collect()
}

fn governing_ids(store: &Store, root: &Path, file: &str) -> BTreeSet<String> {
    store
        .governing(&root.join(file))
        .into_iter()
        .map(|(doc, _)| doc.id.clone())
        .collect()
}

fn covered_specs(store: &Store, root: &Path, base: Option<&str>) -> BTreeSet<String> {
    let rev = base.unwrap_or("HEAD");
    store
        .all_docs()
        .into_iter()
        .filter(|d| d.doc_type.to_string() == "delta")
        .filter(|d| match d.status.to_string().as_str() {
            "in-progress" => true,
            "complete" => !was_complete_at(root, rev, &d.path),
            _ => false,
        })
        .flat_map(|d| {
            d.related
                .iter()
                .filter(|r| r.rel_type.as_str() == "folds-into")
        })
        .filter_map(|r| store.resolve_relation_target(&r.target))
        .map(|spec| spec.id.clone())
        .collect()
}

fn cross_governs(
    store: &Store,
    root: &Path,
    touched: &[String],
    base: Option<&str>,
) -> Option<Finding> {
    let governed: Vec<BTreeSet<String>> = touched
        .iter()
        .map(|f| governing_ids(store, root, f))
        .filter(|ids| !ids.is_empty())
        .collect();
    let first = governed.first()?;
    let common = governed.iter().fold(first.clone(), |acc, ids| &acc & ids);
    if !common.is_empty() {
        return None;
    }
    let covered = covered_specs(store, root, base);
    if governed.iter().all(|ids| !ids.is_disjoint(&covered)) {
        return None;
    }
    let areas: BTreeSet<&String> = governed.iter().flatten().collect();
    let names: Vec<&str> = areas.iter().map(|s| s.as_str()).collect();
    Some(Finding::bare_error(format!(
        "the session's changes span more than one governs area ({}) and no in-progress delta covers them; \
         narrow the change or create a delta that folds into those specs",
        names.join(", ")
    )))
}

fn pending_pattern(file: &str) -> Option<Regex> {
    let pattern = match Path::new(file).extension()?.to_str()? {
        "rs" => r"^\s*#\[ignore\s*\]",
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" => r"\b(it|test|describe)\.todo\(",
        _ => return None,
    };
    Regex::new(pattern).ok()
}

fn pending_tests(root: &Path, touched: &[String]) -> Vec<Finding> {
    touched
        .iter()
        .filter_map(|file| {
            let pattern = pending_pattern(file)?;
            Some((
                file,
                pattern,
                std::fs::read_to_string(root.join(file)).ok()?,
            ))
        })
        .flat_map(|(file, pattern, source)| {
            source
                .lines()
                .enumerate()
                .filter(|(_, line)| pattern.is_match(line))
                .map(|(i, _)| {
                    Finding::bare_error(format!(
                        "{file}:{}: pending test; give #[ignore] a reason or finish the test",
                        i + 1
                    ))
                })
                .collect::<Vec<_>>()
        })
        .collect()
}
