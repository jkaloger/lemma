use crate::claims::{lines_of, parse, Claim, Receipt};
use crate::outcome::Outcome;
use crate::payload::Finding;
use crate::receipt::{is_dirty, rewrite_body, Checker};
use crate::repin::repinned_line;
use anyhow::Result;
use lazyspec::engine::fs::RealFileSystem;
use lazyspec::engine::store::Store;
use std::path::Path;

pub fn run(store: &Store, root: &Path, id: &str, claim_id: &str, target: &str) -> Result<Outcome> {
    let Ok(doc) = store.resolve_shorthand(id) else {
        return Ok(Outcome::refuse(vec![Finding::bare_error(format!(
            "no document matches {id}"
        ))]));
    };
    let refuse = |message: String| {
        Ok(Outcome::refuse(vec![Finding::error(
            &doc.id, None, message,
        )]))
    };
    let Some((path, symbol)) = target
        .split_once('#')
        .filter(|(p, s)| !p.is_empty() && !s.is_empty())
    else {
        return refuse(format!("{target} is not <path>#<symbol>"));
    };
    let body = store.get_body_raw(&doc.path, &RealFileSystem)?;
    let Some(claim) = parse(&body).claims.into_iter().find(|c| c.id == claim_id) else {
        return refuse(format!("no claim {claim_id} in {}", doc.id));
    };
    let checker = Checker::new(root);
    if let Err(why) = checker.symbol_resolves(path, Some(symbol)) {
        return refuse(format!("cannot pin {claim_id}, {why}"));
    }
    if is_dirty(root, path.trim_start_matches("./")) {
        return refuse(format!(
            "{path} has uncommitted changes; commit before pinning so the pin does not lie"
        ));
    }
    let Some(head) = checker.head_short() else {
        return refuse("repository has no HEAD commit".to_string());
    };

    let mut lines: Vec<String> = lines_of(&body).iter().map(|l| l.to_string()).collect();
    match existing_receipt(&claim, path, symbol) {
        Some(receipt) => {
            let idx = receipt.line - 1;
            lines[idx] = repinned_line(&lines[idx], &head);
        }
        None => {
            let eol = if lines[claim.start].ends_with('\r') {
                "\r"
            } else {
                ""
            };
            lines.insert(
                claim.start + 1,
                format!("  @ref {path}#{symbol}@{head}{eol}"),
            );
        }
    }
    rewrite_body(root, &doc.path, &body, &lines.join("\n"))?;
    Ok(Outcome::done(Vec::new()))
}

fn existing_receipt<'a>(claim: &'a Claim, path: &str, symbol: &str) -> Option<&'a Receipt> {
    let path = path.trim_start_matches("./");
    claim
        .receipts
        .iter()
        .find(|r| r.path.trim_start_matches("./") == path && r.symbol.as_deref() == Some(symbol))
}
