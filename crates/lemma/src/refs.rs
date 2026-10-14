use crate::claims::{live_lines, parse, per_text, Claim};
use crate::evidence::{resolve, RefStatus};
use crate::lint::{cite, ref_diagnostic, ref_name};
use crate::payload::{tests_or_default, Attributes, Doc};
use crate::world::World;
use lazyspec::engine::refs::parse_refs;
use lazyspec::engine::store::Store;
use serde::Serialize;
use std::path::Path;

pub struct Listing {
    pub lines: Vec<String>,
    pub all_ok: bool,
}

fn status_word(status: &RefStatus) -> &'static str {
    match status {
        RefStatus::Ok => "ok",
        RefStatus::Unresolved(_) => "unresolved",
        RefStatus::NotATest => "not-a-test",
    }
}

/// `DELTA-004/AC-2` names one delta's claim; a bare id matches it in every document.
pub fn run(store: &Store, root: &Path, claim: &str) -> Option<Listing> {
    let (doc_id, claim_id) = match claim.split_once('/') {
        Some((doc, id)) => (Some(doc), id),
        None => (None, claim),
    };
    let world = World::load(store, None);
    let mut lines = Vec::new();
    let mut all_ok = true;
    let mut found = false;
    for site in world.docs().iter().flat_map(|doc| world.sites(doc)) {
        if doc_id.is_some_and(|id| !site.doc.is_named(id)) {
            continue;
        }
        for c in site.claims.iter().filter(|c| c.id == claim_id) {
            found = true;
            for r in &c.receipts {
                let status = resolve(root, &r.path, r.symbol.as_deref(), &site.tests);
                all_ok &= status == RefStatus::Ok;
                lines.push(format!(
                    "{} {} {}",
                    cite(site.doc, &c.id),
                    ref_name(&r.path, r.symbol.as_deref()),
                    status_word(&status)
                ));
            }
        }
    }
    found.then_some(Listing { lines, all_ok })
}

/// One directive in a doc. `line` is 1-based; `col` and `end_col` are 0-based byte
/// offsets in that line, end exclusive.
#[derive(Serialize)]
pub struct Entry {
    pub kind: &'static str,
    pub line: usize,
    pub col: usize,
    pub end_col: usize,
    pub text: String,
    pub status: String,
}

fn owner<'a>(world: &'a World, path: &str) -> Option<&'a Doc> {
    world
        .docs()
        .iter()
        .find(|d| d.path == path || d.parts.iter().any(|p| p.path == path))
}

fn tests_for(world: &World, doc: Option<&Doc>) -> Vec<String> {
    match doc {
        Some(doc) if doc.doc_type == "delta" => world.delta_tests(doc),
        Some(doc) => doc.tests_globs(),
        None => tests_or_default(&Attributes::default()),
    }
}

fn claim_at(claims: &[Claim], line: usize) -> Option<&Claim> {
    claims.iter().find(|c| c.start < line && line <= c.end)
}

fn per_entry(line: usize, text: &str) -> Option<Entry> {
    per_text(text)?;
    let col = text.len() - text.trim_start().len();
    let directive = text[col..].trim_end();
    Some(Entry {
        kind: "per",
        line,
        col,
        end_col: col + directive.len(),
        text: directive.to_string(),
        status: "ok".to_string(),
    })
}

/// Every `@ref` and `@per` in the doc at `path` (relative to `root`), in order. A ref's
/// status is the diagnostic lint gives it, or `ok`.
pub fn directives(store: &Store, root: &Path, path: &str, source: &str) -> Vec<Entry> {
    let world = World::load(store, None);
    let doc = owner(&world, path);
    let tests = tests_for(&world, doc);
    let claims = parse(source).claims;
    let mut entries = Vec::new();
    for (line, text) in live_lines(source) {
        for r in parse_refs(text) {
            let claim = claim_at(&claims, line).map(|c| {
                let cited = doc.map_or(c.id.clone(), |d| cite(d, &c.id));
                (c.id.as_str(), cited)
            });
            let status = resolve(root, &r.path, r.symbol.as_deref(), &tests);
            let name = ref_name(&r.path, r.symbol.as_deref());
            let diagnostic = ref_diagnostic(
                &status,
                &name,
                claim.as_ref().map(|(id, cited)| (*id, cited.as_str())),
            );
            entries.push(Entry {
                kind: "ref",
                line,
                col: r.span.0,
                end_col: r.span.1,
                text: text[r.span.0..r.span.1].to_string(),
                status: diagnostic.unwrap_or_else(|| "ok".to_string()),
            });
        }
        entries.extend(per_entry(line, text));
    }
    entries
}
