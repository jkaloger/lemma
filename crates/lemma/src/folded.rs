use crate::check::{ref_name, Finding};
use crate::claims::{checked_pattern, is_base_line, parse, Claim, BEHAVIOUR_PART};
use crate::evidence::clean;
use crate::fold::fold;
use crate::git::git_output;
use crate::payload::Doc;
use crate::world::World;
use lazyspec::engine::doc_json::doc_to_json;
use lazyspec::engine::document::DocMeta;
use std::collections::BTreeSet;
use std::path::Path;

pub fn folds_into(delta: &Doc, spec: &Doc) -> bool {
    delta.fold_targets().iter().any(|t| spec.is_named(t))
}

fn at_base(root: &Path, base: &str, path: &str) -> Option<String> {
    git_output(root, &["show", &format!("{base}:./{path}")])
}

fn base_doc(root: &Path, base: &str, path: &str) -> Option<Doc> {
    let meta = DocMeta::parse(&at_base(root, base, path)?).ok()?;
    serde_json::from_value(doc_to_json(&meta)).ok()
}

pub fn completed_since<'a>(
    root: &Path,
    world: &'a World,
    base: &str,
    files: &[String],
) -> Vec<&'a Doc> {
    let mut completed: Vec<&Doc> = world
        .docs()
        .iter()
        .filter(|d| d.doc_type == "delta" && d.status == "complete")
        .filter(|d| files.contains(&d.path))
        .filter(|d| base_doc(root, base, &d.path).is_none_or(|was| was.status != "complete"))
        .collect();
    completed.sort_by(|a, b| a.id.cmp(&b.id));
    completed
}

fn behaviour_text(claim: &Claim) -> String {
    let lines = claim.block.iter().enumerate().filter_map(|(at, line)| {
        let line = checked_pattern().replace_all(line, "");
        let line = line.replace("[intent]", "");
        let trimmed = line.trim();
        let dropped = trimmed.is_empty() || is_base_line(trimmed) || trimmed.starts_with("@ref");
        (at == 0 || !dropped).then(|| trimmed.to_string())
    });
    lines
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn identity_of(claim: &Claim) -> (String, String) {
    (claim.id.clone(), behaviour_text(claim))
}

fn identity(claims: &[Claim]) -> BTreeSet<(String, String)> {
    claims.iter().map(identity_of).collect()
}

fn ref_names(claim: &Claim) -> BTreeSet<String> {
    claim
        .receipts
        .iter()
        .chain(&claim.pinned)
        .map(|r| ref_name(clean(&r.path), r.symbol.as_deref()))
        .collect()
}

fn folded_by(world: &World, spec: &Doc, completed: &[&Doc], base_body: &str) -> String {
    let mut pending: Vec<Vec<Claim>> = completed
        .iter()
        .flat_map(|delta| world.sites(delta))
        .filter(|site| site.governing.is_some_and(|g| g.id == spec.id))
        .map(|site| site.claims)
        .collect();
    let mut body = base_body.to_string();
    while !pending.is_empty() {
        let before = pending.len();
        pending.retain(|claims| {
            let refs: Vec<&Claim> = claims.iter().collect();
            match fold(&body, &refs) {
                Ok(next) => {
                    body = next;
                    false
                }
                Err(_) => true,
            }
        });
        if pending.len() == before {
            break;
        }
    }
    body
}

fn scope_changed(root: &Path, base: &str, spec: &Doc, files: &[String]) -> Option<Finding> {
    if !files.contains(&spec.path) {
        return None;
    }
    let was = base_doc(root, base, &spec.path)?;
    let same = was.governs == spec.governs && was.tests_globs() == spec.tests_globs();
    if same {
        return None;
    }
    Some(Finding::warning(
        spec,
        &spec.path,
        None,
        "governs/tests changed in this range; review",
    ))
}

fn text_edit(spec: &Doc, path: &str, now: &[Claim], expected: &[Claim]) -> Option<Finding> {
    let wanted = identity(expected);
    if identity(now) == wanted {
        return None;
    }
    let line = now
        .iter()
        .find(|c| !wanted.contains(&identity_of(c)))
        .map(|c| c.line);
    Some(
        Finding::error(
            spec,
            path,
            line,
            "spec behaviour edited outside a fold; change it through a delta",
        )
        .in_part(BEHAVIOUR_PART),
    )
}

fn removed_refs(spec: &Doc, path: &str, now: &[Claim], expected: &[Claim]) -> Vec<Finding> {
    expected
        .iter()
        .filter(|want| !want.struck)
        .filter_map(|want| Some((want, now.iter().find(|c| c.id == want.id && !c.struck)?)))
        .flat_map(|(want, live)| {
            let kept = ref_names(live);
            ref_names(want)
                .into_iter()
                .filter(|name| !kept.contains(name))
                .map(|name| {
                    Finding::error(
                        spec,
                        path,
                        Some(live.line),
                        format!(
                            "{}: ref {name} removed from the spec outside a fold; change it through a delta",
                            live.id
                        ),
                    )
                    .in_part(BEHAVIOUR_PART)
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn edited_outside_a_fold(
    root: &Path,
    world: &World,
    base: &str,
    files: &[String],
    spec: &Doc,
) -> Vec<Finding> {
    let Some(part) = spec.part(BEHAVIOUR_PART) else {
        return Vec::new();
    };
    let path = spec.part_path(BEHAVIOUR_PART);
    if !files.contains(&path) || at_base(root, base, &spec.path).is_none() {
        return Vec::new();
    }
    let base_body = at_base(root, base, &path).unwrap_or_default();
    let completed = completed_since(root, world, base, files);
    let now = parse(&part.body).claims;
    let expected = parse(&folded_by(world, spec, &completed, &base_body)).claims;
    text_edit(spec, &path, &now, &expected)
        .into_iter()
        .chain(removed_refs(spec, &path, &now, &expected))
        .collect()
}

pub fn spec_edits(root: &Path, world: &World, base: &str, files: &[String]) -> Vec<Finding> {
    world
        .specs()
        .into_iter()
        .flat_map(|spec| {
            let edits = edited_outside_a_fold(root, world, base, files, spec);
            edits
                .into_iter()
                .chain(scope_changed(root, base, spec, files))
        })
        .collect()
}
