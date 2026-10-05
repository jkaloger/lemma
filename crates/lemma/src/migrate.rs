use crate::claims::{
    checked_pattern, is_pinned, lines_of, parse, Claim, LegacyKind, FORCED_MARKER,
};
use crate::world::{in_flight, Site, World};
use lazyspec::engine::refs::parse_refs;
use lazyspec::engine::store::Store;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

type Existing = BTreeMap<String, String>;

pub struct Replacement {
    old: String,
    new: String,
}

pub fn run(store: &Store, root: &Path) -> Vec<String> {
    let world = World::load(store, None);
    let mut sites: BTreeMap<String, Existing> = BTreeMap::new();
    for site in world.docs().iter().flat_map(|doc| world.sites(doc)) {
        let existing = replaceable(&site);
        sites.entry(site.path).or_default().extend(existing);
    }
    let mut assigned = BTreeSet::new();
    sites
        .into_iter()
        .filter_map(|(path, existing)| {
            let replaced = migrate_file(&root.join(&path), &existing, &mut assigned)?;
            let notes = replaced
                .iter()
                .map(|r| format!("  {} struck; replacement re-added as {}", r.old, r.new));
            Some(std::iter::once(path).chain(notes).collect::<Vec<_>>())
        })
        .flatten()
        .collect()
}

fn replaceable(site: &Site) -> Existing {
    if !in_flight(site.doc) {
        return Existing::new();
    }
    let Some(spec) = site.governing else {
        return Existing::new();
    };
    spec.spec_claims()
        .into_iter()
        .map(|c| (c.id.clone(), c.block[0].clone()))
        .collect()
}

fn migrate_file(
    file: &Path,
    existing: &Existing,
    assigned: &mut BTreeSet<String>,
) -> Option<Vec<Replacement>> {
    let text = std::fs::read_to_string(file).ok()?;
    let (migrated, replaced) = rewrite(&text, existing, assigned);
    if migrated == text {
        return None;
    }
    std::fs::write(file, migrated).ok()?;
    Some(replaced)
}

fn fresh_id(old: &str, taken: &BTreeSet<String>) -> String {
    let first = format!("{old}-R");
    if !taken.contains(&first) {
        return first;
    }
    (2..)
        .map(|n| format!("{first}{n}"))
        .find(|id| !taken.contains(id))
        .expect("an unbounded range has a free id")
}

fn struck_line(old_line: &str, id: &str) -> String {
    let rest = old_line.find(']').map_or("", |at| &old_line[at + 1..]);
    format!("- ~~[{id}]~~{rest}")
}

pub fn rewrite(
    text: &str,
    existing: &Existing,
    assigned: &mut BTreeSet<String>,
) -> (String, Vec<Replacement>) {
    let mut lines: Vec<String> = lines_of(text).into_iter().map(str::to_string).collect();
    let mut dropped = BTreeSet::new();
    let mut inserted: BTreeMap<usize, String> = BTreeMap::new();
    let mut replaced = Vec::new();
    let claims = parse(text).claims;
    let mut taken: BTreeSet<String> = existing.keys().cloned().collect();
    taken.extend(claims.iter().map(|c| c.id.clone()));
    taken.extend(assigned.iter().cloned());
    for claim in &claims {
        for legacy in &claim.legacy {
            let at = legacy.line - 1;
            match legacy.kind {
                LegacyKind::PinnedRef => lines[at] = unpin(&lines[at]),
                LegacyKind::Base => {
                    dropped.insert(at);
                }
                LegacyKind::Checked => {
                    lines[at] = checked_pattern().replace_all(&lines[at], "").into_owned();
                    if lines[at].trim().is_empty() {
                        dropped.insert(at);
                    }
                }
            }
        }
        if intent_only(claim) {
            lines[claim.start] = mark_intent(&lines[claim.start]);
        }
        let based = claim.legacy.iter().any(|l| l.kind == LegacyKind::Base);
        let Some(old_line) = existing.get(&claim.id).filter(|_| based && !claim.struck) else {
            continue;
        };
        let new = fresh_id(&claim.id, &taken);
        taken.insert(new.clone());
        assigned.insert(new.clone());
        lines[claim.start] =
            lines[claim.start].replacen(&format!("[{}]", claim.id), &format!("[{new}]"), 1);
        inserted.insert(claim.start, struck_line(old_line, &claim.id));
        replaced.push(Replacement {
            old: claim.id.clone(),
            new,
        });
    }
    let text = lines
        .into_iter()
        .enumerate()
        .filter(|(at, _)| !dropped.contains(at))
        .flat_map(|(at, line)| inserted.remove(&at).into_iter().chain([line]))
        .collect::<Vec<_>>()
        .join("\n");
    (text, replaced)
}

fn intent_only(claim: &Claim) -> bool {
    let kinds = |kind| claim.legacy.iter().any(|l| l.kind == kind);
    kinds(LegacyKind::Checked)
        && !kinds(LegacyKind::PinnedRef)
        && claim.receipts.is_empty()
        && !claim.intent
        && !claim.struck
}

fn mark_intent(line: &str) -> String {
    match line.find(']') {
        Some(at) => format!("{} [intent]{}", &line[..=at], &line[at + 1..]),
        None => line.to_string(),
    }
}

fn unpin(line: &str) -> String {
    let mut out = line.to_string();
    for found in parse_refs(line).iter().rev().filter(|r| is_pinned(r, line)) {
        let (start, mut end) = found.span;
        let after = &line[end..];
        let trimmed = after.trim_start();
        if trimmed.starts_with(FORCED_MARKER) {
            end += after.len() - trimmed.len() + FORCED_MARKER.len();
        }
        let plain = match &found.symbol {
            Some(symbol) => format!("@ref {}#{symbol}", found.path),
            None => format!("@ref {}", found.path),
        };
        out.replace_range(start..end, &plain);
    }
    out
}
