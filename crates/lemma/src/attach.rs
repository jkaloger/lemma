use crate::base::{
    base_hash_for, doc_tests, load_behaviour_parts, spec_claims, spec_tests, Loaded,
};
use crate::claims::{base_line, eol_of, owned_lines, target_of, Claim, Receipt};
use crate::outcome::Outcome;
use crate::payload::{Finding, Written};
use crate::receipt::{is_dirty, names_a_test, Checker};
use crate::repin::{pin_drifted, repinned_line};
use anyhow::Result;
use lazyspec::engine::document::{DocMeta, Part};
use lazyspec::engine::store::Store;
use std::path::Path;

struct Located<'a> {
    part: &'a Part,
    body: String,
    claim: Claim,
}

fn base_to_insert(store: &Store, part: &Part, claim: &Claim) -> Option<String> {
    if claim.base.is_some() {
        return None;
    }
    let spec = spec_claims(store, target_of(&part.name)?).ok()?;
    base_hash_for(&spec, &claim.id)
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
    if doc.doc_type.as_str() != "delta" {
        return refuse(format!(
            "{} is a {}; `lemma receipt` pins delta claims (spec receipts arrive by fold, humans write @checked)",
            doc.id, doc.doc_type
        ));
    }
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
    match owner_tests(store, doc, part) {
        Err(message) => return refuse(message),
        Ok((owner, globs)) if !names_a_test(path, &globs) => {
            return refuse(format!(
                "{path} is not a test file under {owner}'s `tests` globs; a production symbol is not a receipt"
            ))
        }
        Ok(_) => {}
    }
    let checker = Checker::new(root);
    if let Err(why) = checker.symbol_resolves(path, Some(symbol)) {
        return refuse(format!("cannot pin {claim_id}, {why}"));
    }
    if is_dirty(root, path.trim_start_matches("./")) {
        return refuse(format!(
            "{path} has uncommitted changes; commit before pinning so the pin does not lie"
        ));
    }
    let existing = existing_receipt(&claim, path, symbol);
    if let Some(receipt) = existing.filter(|r| pin_drifted(root, part, &body, &claim, r)) {
        let sha = receipt.sha.as_deref().unwrap_or_default();
        return refuse(format!(
            "{claim_id}: {path}#{symbol} changed since {sha} but the claim did not; \
             run `lemma re-pin {} {claim_id} --force` if the test still proves the claim",
            doc.id
        ));
    }
    let Some(head) = checker.head_short() else {
        return refuse("repository has no HEAD commit".to_string());
    };

    let mut lines = owned_lines(&body);
    let eol = eol_of(&lines[claim.start]);
    let mut wrote = Vec::new();
    let mut base_at = claim.start + 1;
    match existing {
        Some(receipt) => {
            let idx = receipt.line - 1;
            base_at = receipt.line;
            lines[idx] = repinned_line(&lines[idx], &head);
            wrote.push(Written::new(
                &claim.id,
                &part.name,
                receipt.line,
                &lines[idx],
            ));
        }
        None => {
            let text = format!("  @ref {path}#{symbol}@{head}{eol}");
            wrote.push(Written::new(&claim.id, &part.name, claim.start + 2, &text));
            lines.insert(claim.start + 1, text);
            base_at += 1;
        }
    }
    if let Some(hash) = base_to_insert(store, part, &claim) {
        let text = format!("{}{eol}", base_line(&hash));
        wrote.push(Written::new(&claim.id, &part.name, base_at + 1, &text));
        lines.insert(base_at, text);
    }
    std::fs::write(root.join(&part.path), lines.join("\n"))?;
    Ok(Outcome::done(Vec::new()).wrote(wrote))
}

fn owner_tests(store: &Store, doc: &DocMeta, part: &Part) -> Result<(String, Vec<String>), String> {
    match target_of(&part.name) {
        Some(target) => Ok((target.to_string(), spec_tests(store, target)?)),
        None => Ok((doc.id.clone(), doc_tests(doc)?)),
    }
}

fn locate<'a>(store: &Store, parts: &'a [Part], claim_id: &str) -> Result<Vec<Located<'a>>> {
    let found = load_behaviour_parts(store, parts)?
        .into_iter()
        .filter_map(|Loaded { part, body, claims }| {
            let claim = claims.into_iter().find(|c| c.id == claim_id && !c.struck)?;
            Some(Located { part, body, claim })
        })
        .collect();
    Ok(found)
}

fn existing_receipt<'a>(claim: &'a Claim, path: &str, symbol: &str) -> Option<&'a Receipt> {
    let path = path.trim_start_matches("./");
    claim
        .receipts
        .iter()
        .find(|r| r.path.trim_start_matches("./") == path && r.symbol.as_deref() == Some(symbol))
}
