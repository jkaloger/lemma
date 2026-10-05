use crate::claims::{is_behaviour_part, lines_of, parse, Claim, Receipt};
use crate::outcome::Outcome;
use crate::payload::Finding;
use crate::receipt::{git_lines, git_output, is_dirty, Checker};
use anyhow::Result;
use lazyspec::engine::document::Part;
use lazyspec::engine::fs::RealFileSystem;
use lazyspec::engine::store::Store;
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

struct Loaded<'a> {
    part: &'a Part,
    body: String,
    claims: Vec<Claim>,
}

pub fn run(store: &Store, root: &Path, id: &str) -> Result<Outcome> {
    let Ok(doc) = store.resolve_shorthand(id) else {
        return Ok(Outcome::refuse(vec![Finding::bare_error(format!(
            "no document matches {id}"
        ))]));
    };
    let loaded = doc
        .parts
        .iter()
        .filter(|p| is_behaviour_part(&p.name))
        .map(|part| {
            let body = store.get_part_body_raw(&part.path, &RealFileSystem)?;
            let claims = parse(&body).claims;
            Ok(Loaded { part, body, claims })
        })
        .collect::<Result<Vec<_>>>()?;
    let checker = Checker::new(root);
    let Some(head) = checker.head_short() else {
        return Ok(Outcome::refuse(vec![Finding::error(
            &doc.id,
            None,
            "repository has no HEAD commit",
        )]));
    };

    let broken: Vec<Finding> = loaded
        .iter()
        .flat_map(|l| unresolved_symbols(&doc.id, l, &checker))
        .collect();
    if !broken.is_empty() {
        return Ok(Outcome::refuse(broken));
    }

    let flags = loaded
        .iter()
        .flat_map(|l| contract_flags(&doc.id, l, root))
        .collect();
    for l in &loaded {
        std::fs::write(
            root.join(&l.part.path),
            repinned_body(&l.body, &l.claims, &head),
        )?;
    }
    Ok(Outcome::done(flags))
}

fn receipts_of<'a>(loaded: &'a Loaded) -> impl Iterator<Item = (&'a Claim, &'a Receipt)> {
    loaded
        .claims
        .iter()
        .flat_map(|claim| claim.receipts.iter().map(move |r| (claim, r)))
}

fn unresolved_symbols(doc_id: &str, loaded: &Loaded, checker: &Checker) -> Vec<Finding> {
    receipts_of(loaded)
        .filter(|(_, receipt)| receipt.checked_by.is_none())
        .filter_map(|(claim, receipt)| {
            let err = checker
                .symbol_resolves(&receipt.path, receipt.symbol.as_deref())
                .err()?;
            Some(
                Finding::error(
                    doc_id,
                    Some(receipt.line),
                    format!("{}: cannot re-pin, {err}", claim.id),
                )
                .in_part(&loaded.part.name),
            )
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

fn contract_flags(doc_id: &str, loaded: &Loaded, root: &Path) -> Vec<Finding> {
    receipts_of(loaded)
        .filter(|(_, receipt)| receipt.checked_by.is_none())
        .filter_map(|(claim, receipt)| flag(doc_id, loaded.part, claim, receipt, root))
        .collect()
}

fn flag(
    doc_id: &str,
    part: &Part,
    claim: &Claim,
    receipt: &Receipt,
    root: &Path,
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
        .any(|c| diff_changes_claim(doc_diff_in_commit(root, c, &part.path)));
    if claim_moved_in_commit {
        return None;
    }
    if dirty && diff_changes_claim(doc_diff_in_worktree(root, &part.path)) {
        return None;
    }
    Some(
        Finding::warning(
            doc_id,
            Some(receipt.line),
            format!(
                "{}: {} changed since {sha} but the claim did not; re-pin is flagged for review",
                claim.id, receipt.path
            ),
        )
        .in_part(&part.name),
    )
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
