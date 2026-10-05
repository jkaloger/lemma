use crate::claims::{
    checked_pattern, is_pinned, lines_of, parse, Claim, LegacyKind, FORCED_MARKER,
};
use crate::world::World;
use lazyspec::engine::refs::parse_refs;
use lazyspec::engine::store::Store;
use std::collections::BTreeSet;
use std::path::Path;

pub fn run(store: &Store, root: &Path) -> Vec<String> {
    let world = World::load(store, None);
    let paths: BTreeSet<String> = world
        .docs()
        .iter()
        .flat_map(|doc| world.sites(doc))
        .map(|site| site.path)
        .collect();
    paths
        .into_iter()
        .filter(|path| migrate_file(&root.join(path)))
        .collect()
}

fn migrate_file(file: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(file) else {
        return false;
    };
    let migrated = rewrite(&text);
    migrated != text && std::fs::write(file, migrated).is_ok()
}

pub fn rewrite(text: &str) -> String {
    let mut lines: Vec<String> = lines_of(text).into_iter().map(str::to_string).collect();
    let mut dropped = BTreeSet::new();
    for claim in &parse(text).claims {
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
    }
    lines
        .into_iter()
        .enumerate()
        .filter(|(at, _)| !dropped.contains(at))
        .map(|(_, line)| line)
        .collect::<Vec<_>>()
        .join("\n")
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
