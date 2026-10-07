use crate::evidence::{resolve, RefStatus};
use crate::lint::{cite, ref_name};
use crate::world::World;
use lazyspec::engine::store::Store;
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
