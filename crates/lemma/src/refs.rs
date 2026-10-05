use crate::check::ref_name;
use crate::claims::Claim;
use crate::evidence::{resolve, RefStatus};
use crate::payload::{tests_or_default, Attributes, Doc};
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

pub fn run(store: &Store, root: &Path, claim_id: &str) -> Option<Listing> {
    let world = World::load(store, None);
    let found: Vec<(Claim, Option<&Doc>)> = world
        .all_sites()
        .into_iter()
        .flat_map(|site| {
            let governing = site.governing;
            site.claims
                .into_iter()
                .filter(|c| c.id == claim_id)
                .map(move |c| (c, governing))
        })
        .collect();
    if found.is_empty() {
        return None;
    }
    let mut lines = Vec::new();
    let mut all_ok = true;
    for (claim, governing) in &found {
        let tests = governing
            .map(Doc::tests_globs)
            .unwrap_or_else(|| tests_or_default(&Attributes::default()));
        for r in &claim.receipts {
            let status = resolve(root, &r.path, r.symbol.as_deref(), &tests);
            all_ok &= status == RefStatus::Ok;
            lines.push(format!(
                "{claim_id} {} {}",
                ref_name(&r.path, r.symbol.as_deref()),
                status_word(&status)
            ));
        }
    }
    Some(Listing { lines, all_ok })
}
