use crate::claims::{is_behaviour_part, lines_of, parse, Claim, Receipt};
use crate::outcome::Outcome;
use crate::payload::Finding;
use crate::receipt::{is_dirty, Checker};
use crate::repin::repinned_line;
use anyhow::Result;
use lazyspec::engine::document::Part;
use lazyspec::engine::fs::RealFileSystem;
use lazyspec::engine::store::Store;
use std::path::Path;

struct Located<'a> {
    part: &'a Part,
    body: String,
    claim: Claim,
}

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
    let mut found = locate(store, &doc.parts, claim_id)?;
    if found.len() > 1 {
        let names: Vec<&str> = found.iter().map(|l| l.part.name.as_str()).collect();
        return refuse(format!(
            "claim {claim_id} is live in more than one part ({}); strike it in all but one",
            names.join(", ")
        ));
    }
    let Some(Located { part, body, claim }) = found.pop() else {
        return refuse(format!(
            "no live claim {claim_id} in any behaviour part of {}",
            doc.id
        ));
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
    std::fs::write(root.join(&part.path), lines.join("\n"))?;
    Ok(Outcome::done(Vec::new()))
}

fn locate<'a>(store: &Store, parts: &'a [Part], claim_id: &str) -> Result<Vec<Located<'a>>> {
    let mut found = Vec::new();
    for part in parts.iter().filter(|p| is_behaviour_part(&p.name)) {
        let body = store.get_part_body_raw(&part.path, &RealFileSystem)?;
        let live = parse(&body)
            .claims
            .into_iter()
            .find(|c| c.id == claim_id && !c.struck);
        if let Some(claim) = live {
            found.push(Located { part, body, claim });
        }
    }
    Ok(found)
}

fn existing_receipt<'a>(claim: &'a Claim, path: &str, symbol: &str) -> Option<&'a Receipt> {
    let path = path.trim_start_matches("./");
    claim
        .receipts
        .iter()
        .find(|r| r.path.trim_start_matches("./") == path && r.symbol.as_deref() == Some(symbol))
}
