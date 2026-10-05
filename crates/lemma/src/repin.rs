use crate::base::{load_behaviour_parts, Loaded};
use crate::claims::{is_annotation, owned_lines, parse, Claim, Receipt};
use crate::outcome::Outcome;
use crate::payload::Finding;
use crate::receipt::{git_lines, git_output, is_dirty, Checker};
use anyhow::Result;
use lazyspec::engine::store::Store;
use std::path::Path;

pub fn run(
    store: &Store,
    root: &Path,
    id: &str,
    claims: &[String],
    force: bool,
) -> Result<Outcome> {
    let Ok(doc) = store.resolve_shorthand(id) else {
        return Ok(Outcome::refuse(vec![Finding::bare_error(format!(
            "no document matches {id}"
        ))]));
    };
    let loaded = load_behaviour_parts(store, &doc.parts)?;
    let checker = Checker::new(root);
    let Some(head) = checker.head_short() else {
        return Ok(Outcome::refuse(vec![Finding::error(
            &doc.id,
            None,
            "repository has no HEAD commit",
        )]));
    };

    let unknown = unknown_claims(&doc.id, &loaded, claims);
    if !unknown.is_empty() {
        return Ok(Outcome::refuse(unknown));
    }
    let broken: Vec<Finding> = loaded
        .iter()
        .flat_map(|l| unresolved_symbols(&doc.id, l, claims, &checker))
        .collect();
    if !broken.is_empty() {
        return Ok(Outcome::refuse(broken));
    }

    let dirty: Vec<Finding> = loaded
        .iter()
        .flat_map(|l| dirty_receipt_files(&doc.id, l, claims, root))
        .collect();
    if !dirty.is_empty() {
        return Ok(Outcome::refuse(dirty));
    }

    let drifted: Vec<Finding> = loaded
        .iter()
        .flat_map(|l| drifted_pins(&doc.id, l, claims, root, force))
        .collect();
    if !drifted.is_empty() && !force {
        return Ok(Outcome::refuse(drifted));
    }
    for l in &loaded {
        let receipts: Vec<&Receipt> = selected(l, claims).map(|(_, r)| r).collect();
        std::fs::write(
            root.join(&l.part.path),
            repinned_body(&l.body, &receipts, &head),
        )?;
    }
    Ok(Outcome::done(drifted))
}

fn unknown_claims(doc_id: &str, loaded: &[Loaded], wanted: &[String]) -> Vec<Finding> {
    wanted
        .iter()
        .filter(|id| !loaded.iter().any(|l| l.claims.iter().any(|c| &c.id == *id)))
        .map(|id| Finding::error(doc_id, None, format!("no claim {id} in {doc_id}")))
        .collect()
}

fn selected<'a>(
    loaded: &'a Loaded,
    wanted: &'a [String],
) -> impl Iterator<Item = (&'a Claim, &'a Receipt)> {
    loaded
        .claims
        .iter()
        .filter(|claim| wanted.is_empty() || wanted.contains(&claim.id))
        .flat_map(|claim| claim.receipts.iter().map(move |r| (claim, r)))
        .filter(|(_, receipt)| receipt.checked_by.is_none())
}

fn unresolved_symbols(
    doc_id: &str,
    loaded: &Loaded,
    wanted: &[String],
    checker: &Checker,
) -> Vec<Finding> {
    selected(loaded, wanted)
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

fn dirty_receipt_files(
    doc_id: &str,
    loaded: &Loaded,
    wanted: &[String],
    root: &Path,
) -> Vec<Finding> {
    selected(loaded, wanted)
        .filter(|(_, receipt)| is_dirty(root, receipt.path.trim_start_matches("./")))
        .map(|(claim, receipt)| {
            Finding::error(
                doc_id,
                Some(receipt.line),
                format!(
                    "{}: {} has uncommitted changes; commit before re-pinning",
                    claim.id, receipt.path
                ),
            )
            .in_part(&loaded.part.name)
        })
        .collect()
}

fn claim_text(body: Option<String>, id: &str) -> Option<Vec<String>> {
    let claim = parse(&body?).claims.into_iter().find(|c| c.id == id)?;
    Some(
        claim
            .block
            .iter()
            .filter(|l| !is_annotation(l))
            .map(|l| l.trim().to_string())
            .collect(),
    )
}

fn show(root: &Path, rev: &str, path: &str) -> Option<String> {
    git_output(root, &["show", &format!("{rev}:./{path}")])
}

fn commits_touching(root: &Path, sha: &str, file: &str) -> Vec<String> {
    let range = format!("{sha}..HEAD");
    git_lines(root, &["log", "--format=%H", &range, "--", file]).unwrap_or_default()
}

fn claim_moved_with_file(root: &Path, loaded: &Loaded, claim: &Claim, commits: &[String]) -> bool {
    let part = loaded.part.path.to_string_lossy();
    commits.iter().any(|c| {
        claim_text(show(root, &format!("{c}^"), &part), &claim.id)
            != claim_text(show(root, c, &part), &claim.id)
    })
}

fn claim_edited_in_worktree(root: &Path, loaded: &Loaded, claim: &Claim) -> bool {
    let part = loaded.part.path.to_string_lossy();
    claim_text(show(root, "HEAD", &part), &claim.id)
        != claim_text(Some(loaded.body.clone()), &claim.id)
}

fn drifted_pins(
    doc_id: &str,
    loaded: &Loaded,
    wanted: &[String],
    root: &Path,
    force: bool,
) -> Vec<Finding> {
    selected(loaded, wanted)
        .filter(|(claim, receipt)| pin_drifted(root, loaded, claim, receipt))
        .map(|(claim, receipt)| {
            let sha = receipt.sha.as_deref().unwrap_or_default();
            let what = format!(
                "{}: {} changed since {sha} but the claim did not",
                claim.id, receipt.path
            );
            let finding = if force {
                Finding::warning(
                    doc_id,
                    Some(receipt.line),
                    format!("{what}; re-pin is flagged for review"),
                )
            } else {
                Finding::error(
                    doc_id,
                    Some(receipt.line),
                    format!("{what}; re-pin refused (pass --force to override)"),
                )
            };
            finding.in_part(&loaded.part.name)
        })
        .collect()
}

fn pin_drifted(root: &Path, loaded: &Loaded, claim: &Claim, receipt: &Receipt) -> bool {
    let Some(sha) = receipt.sha.as_deref() else {
        return false;
    };
    if claim_edited_in_worktree(root, loaded, claim) {
        return false;
    }
    let file = receipt.path.trim_start_matches("./");
    let committed = commits_touching(root, sha, file);
    if committed.is_empty() {
        return false;
    }
    !claim_moved_with_file(root, loaded, claim, &committed)
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

fn repinned_body(body: &str, receipts: &[&Receipt], head: &str) -> String {
    let mut lines = owned_lines(body);
    for receipt in receipts {
        let idx = receipt.line - 1;
        lines[idx] = repinned_line(&lines[idx], head);
    }
    lines.join("\n")
}
