use crate::claims::{eol_of, owned_lines, parse, target_of, Claim};
use crate::claims::{is_behaviour_part, BEHAVIOUR_PART};
use crate::legacy::{base_line, block_hash, without_base};
use crate::outcome::Outcome;
use crate::payload::{tests_or_default, Attributes, Finding, Written};
use anyhow::Result;
use lazyspec::engine::document::{DocMeta, Part};
use lazyspec::engine::fs::RealFileSystem;
use lazyspec::engine::store::Store;
use std::path::Path;

pub struct Loaded<'a> {
    pub part: &'a Part,
    pub body: String,
    pub claims: Vec<Claim>,
}

pub fn load_behaviour_parts<'a>(store: &Store, parts: &'a [Part]) -> Result<Vec<Loaded<'a>>> {
    parts
        .iter()
        .filter(|p| is_behaviour_part(&p.name))
        .map(|part| {
            let body = store.get_part_body_raw(&part.path, &RealFileSystem)?;
            let claims = parse(&body).claims;
            Ok(Loaded { part, body, claims })
        })
        .collect()
}

fn resolve_spec<'a>(store: &'a Store, target: &str) -> Result<&'a DocMeta, String> {
    store
        .resolve_shorthand(target)
        .map_err(|_| format!("fold target {target} does not resolve to a spec"))
}

pub fn spec_claims(store: &Store, target: &str) -> Result<Vec<Claim>, String> {
    let spec = resolve_spec(store, target)?;
    let part = spec
        .parts
        .iter()
        .find(|p| p.name == BEHAVIOUR_PART)
        .ok_or_else(|| format!("fold target {target} has no `{BEHAVIOUR_PART}` part"))?;
    let body = store
        .get_part_body_raw(&part.path, &RealFileSystem)
        .map_err(|e| format!("cannot read {target}: {e}"))?;
    Ok(parse(&body).claims)
}

pub fn base_hash_for(spec_claims: &[Claim], claim_id: &str) -> Option<String> {
    spec_claims
        .iter()
        .find(|c| c.id == claim_id)
        .map(block_hash)
}

pub fn run(store: &Store, root: &Path, delta: &str, claims: &[String]) -> Result<Outcome> {
    let Ok(doc) = store.resolve_shorthand(delta) else {
        return Ok(Outcome::refuse(vec![Finding::bare_error(format!(
            "no document matches {delta}"
        ))]));
    };
    let refuse = |message: String| {
        Ok(Outcome::refuse(vec![Finding::error(
            &doc.id, None, message,
        )]))
    };
    let loaded = load_behaviour_parts(store, &doc.parts)?;

    if let Some(unknown) = claims.iter().find(|id| !holds_claim(&loaded, id)) {
        return refuse(format!(
            "no claim {unknown} in any behaviour part of {}",
            doc.id
        ));
    }
    if let Some(untargeted) = claims.iter().find(|id| !holds_targeted_claim(&loaded, id)) {
        return refuse(format!(
            "{untargeted} is in the `{BEHAVIOUR_PART}` part of {}, which has no fold target; `lemma base` applies to delta claims",
            doc.id
        ));
    }

    let mut writes = Vec::new();
    let mut wrote = Vec::new();
    for Loaded { part, body, .. } in &loaded {
        let Some(target) = target_of(&part.name) else {
            continue;
        };
        let selected: Vec<Claim> = parse(body)
            .claims
            .into_iter()
            .filter(|c| claims.is_empty() || claims.contains(&c.id))
            .collect();
        if selected.is_empty() {
            continue;
        }
        let spec = match spec_claims(store, target) {
            Ok(spec) => spec,
            Err(message) => return refuse(message),
        };
        if let Err(message) = spec_tests(store, target) {
            return refuse(message);
        }
        let mut edits = Vec::new();
        for claim in &selected {
            match base_hash_for(&spec, &claim.id) {
                Some(hash) => edits.push((claim, hash)),
                None if claims.is_empty() => {}
                None => {
                    return refuse(format!(
                        "claim {} is new: {target} has no claim {} to base it on",
                        claim.id, claim.id
                    ))
                }
            }
        }
        if !edits.is_empty() {
            let (text, written) = write_bases(body, &edits);
            wrote.extend(
                written
                    .iter()
                    .map(|(id, line, text)| Written::new(id, &part.name, *line, text)),
            );
            writes.push((&part.path, text));
        }
    }
    for (path, text) in writes {
        std::fs::write(root.join(path), text)?;
    }
    Ok(Outcome::done(Vec::new()).wrote(wrote))
}

fn holds_claim(loaded: &[Loaded], claim_id: &str) -> bool {
    loaded
        .iter()
        .any(|l| l.claims.iter().any(|c| c.id == claim_id))
}

fn holds_targeted_claim(loaded: &[Loaded], claim_id: &str) -> bool {
    loaded
        .iter()
        .filter(|l| target_of(&l.part.name).is_some())
        .any(|l| l.claims.iter().any(|c| c.id == claim_id))
}

fn write_bases(body: &str, edits: &[(&Claim, String)]) -> (String, Vec<(String, usize, String)>) {
    let mut lines = owned_lines(body);
    let mut written = Vec::new();
    let mut shift = 0isize;
    for (claim, hash) in edits {
        let eol = eol_of(&claim.block[0]);
        let mut block = without_base(&claim.block);
        block.push(format!("{}{eol}", base_line(hash)));
        let at = (claim.start as isize + shift) as usize;
        let end = (claim.end as isize + shift) as usize;
        let text = block[block.len() - 1].clone();
        shift += block.len() as isize - (end - at) as isize;
        written.push((claim.id.clone(), at + block.len(), text));
        lines.splice(at..end, block);
    }
    (lines.join("\n"), written)
}

pub fn doc_tests(doc: &DocMeta) -> Result<Vec<String>, String> {
    let attributes = serde_json::to_value(&doc.attributes).unwrap_or_default();
    let attributes: Attributes = serde_json::from_value(attributes)
        .map_err(|_| format!("{}: `tests` must be a list of glob strings", doc.id))?;
    Ok(tests_or_default(&attributes))
}

pub fn spec_tests(store: &Store, target: &str) -> Result<Vec<String>, String> {
    doc_tests(resolve_spec(store, target)?)
}
