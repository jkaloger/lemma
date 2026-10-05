use crate::claims::{lines_of, parse, Claim, Receipt};
use crate::outcome::Outcome;
use crate::payload::Finding;
use crate::receipt::{body_line_offset, git_lines, git_output, is_dirty, rewrite_body, Checker};
use anyhow::Result;
use lazyspec::engine::fs::RealFileSystem;
use lazyspec::engine::store::Store;
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

pub fn run(store: &Store, root: &Path, id: &str) -> Result<Outcome> {
    let Ok(doc) = store.resolve_shorthand(id) else {
        return Ok(Outcome::refuse(vec![Finding::bare_error(format!(
            "no document matches {id}"
        ))]));
    };
    let body = store.get_body_raw(&doc.path, &RealFileSystem)?;
    let claims = parse(&body).claims;
    let checker = Checker::new(root);
    let Some(head) = checker.head_short() else {
        return Ok(Outcome::refuse(vec![Finding::error(
            &doc.id,
            None,
            "repository has no HEAD commit",
        )]));
    };

    let offset = body_line_offset(root, &doc.path, &body)?;
    let broken = unresolved_symbols(&doc.id, &claims, &checker, offset);
    if !broken.is_empty() {
        return Ok(Outcome::refuse(broken));
    }

    let flags = contract_flags(&doc.id, &doc.path, &claims, root, offset);
    rewrite_body(
        root,
        &doc.path,
        &body,
        &repinned_body(&body, &claims, &head),
    )?;
    Ok(Outcome::done(flags))
}

fn unresolved_symbols(
    doc_id: &str,
    claims: &[Claim],
    checker: &Checker,
    offset: usize,
) -> Vec<Finding> {
    claims
        .iter()
        .flat_map(|claim| claim.receipts.iter().map(move |r| (claim, r)))
        .filter_map(|(claim, receipt)| {
            let err = checker
                .symbol_resolves(&receipt.path, receipt.symbol.as_deref())
                .err()?;
            Some(Finding::error(
                doc_id,
                Some(receipt.line + offset),
                format!("{}: cannot re-pin, {err}", claim.id),
            ))
        })
        .collect()
}

fn claim_line_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[+-]\s*- (~~)?\[[A-Z]").unwrap())
}

fn diff_changes_claim(diff: Option<String>) -> bool {
    diff.is_some_and(|d| d.lines().any(|l| claim_line_pattern().is_match(l)))
}

fn doc_diff_in_commit(root: &Path, commit: &str, doc_path: &Path) -> Option<String> {
    let path = doc_path.to_string_lossy();
    git_output(
        root,
        &["show", "-m", "--format=", "-U0", commit, "--", &path],
    )
}

fn doc_diff_in_worktree(root: &Path, doc_path: &Path) -> Option<String> {
    let path = doc_path.to_string_lossy();
    git_output(root, &["diff", "-U0", "HEAD", "--", &path])
}

fn commits_touching(root: &Path, sha: &str, file: &str) -> Vec<String> {
    let range = format!("{sha}..HEAD");
    git_lines(root, &["log", "--format=%H", &range, "--", file]).unwrap_or_default()
}

fn contract_flags(
    doc_id: &str,
    doc_path: &Path,
    claims: &[Claim],
    root: &Path,
    offset: usize,
) -> Vec<Finding> {
    claims
        .iter()
        .flat_map(|claim| claim.receipts.iter().map(move |r| (claim, r)))
        .filter_map(|(claim, receipt)| flag(doc_id, doc_path, claim, receipt, root, offset))
        .collect()
}

fn flag(
    doc_id: &str,
    doc_path: &Path,
    claim: &Claim,
    receipt: &Receipt,
    root: &Path,
    offset: usize,
) -> Option<Finding> {
    let sha = receipt.sha.as_deref()?;
    let file = receipt.path.trim_start_matches("./");
    let committed = commits_touching(root, sha, file);
    let dirty = is_dirty(root, file);
    if committed.is_empty() && !dirty {
        return None;
    }
    let claim_moved_in_commit = committed
        .iter()
        .any(|c| diff_changes_claim(doc_diff_in_commit(root, c, doc_path)));
    if claim_moved_in_commit {
        return None;
    }
    if dirty && diff_changes_claim(doc_diff_in_worktree(root, doc_path)) {
        return None;
    }
    Some(Finding::warning(
        doc_id,
        Some(receipt.line + offset),
        format!(
            "{}: {} changed since {sha} but the claim did not; re-pin is flagged for review",
            claim.id, receipt.path
        ),
    ))
}

pub fn repinned_line(line: &str, head: &str) -> String {
    let mut out = line.to_string();
    let mut refs = lazyspec::engine::refs::parse_refs(line);
    refs.sort_by_key(|r| std::cmp::Reverse(r.span.0));
    for r in refs.iter().filter(|r| r.blob_hash.is_none()) {
        let symbol = r
            .symbol
            .as_ref()
            .map(|s| format!("#{s}"))
            .unwrap_or_default();
        out.replace_range(
            r.span.0..r.span.1,
            &format!("@ref {}{symbol}@{head}", r.path),
        );
    }
    out
}

fn repinned_body(body: &str, claims: &[Claim], head: &str) -> String {
    let mut lines: Vec<String> = lines_of(body).iter().map(|l| l.to_string()).collect();
    for receipt in claims.iter().flat_map(|c| &c.receipts) {
        let idx = receipt.line - 1;
        lines[idx] = repinned_line(&lines[idx], head);
    }
    lines.join("\n")
}
