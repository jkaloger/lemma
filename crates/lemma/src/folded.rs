use crate::check::Finding;
use crate::claims::{parse, Claim, BEHAVIOUR_PART};
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

fn identity_of(claim: &Claim) -> (String, String) {
    let text = claim.block[0].replace("[intent]", "");
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    (claim.id.clone(), text)
}

fn identity(claims: &[Claim]) -> BTreeSet<(String, String)> {
    claims.iter().map(identity_of).collect()
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

fn edited_outside_a_fold(
    root: &Path,
    world: &World,
    base: &str,
    files: &[String],
    completed: &[&Doc],
    spec: &Doc,
) -> Option<Finding> {
    let part = spec.part(BEHAVIOUR_PART)?;
    let path = spec.part_path(BEHAVIOUR_PART);
    if !files.contains(&path) {
        return None;
    }
    at_base(root, base, &spec.path)?;
    let base_body = at_base(root, base, &path).unwrap_or_default();
    let now = parse(&part.body).claims;
    let expected = parse(&folded_by(world, spec, completed, &base_body)).claims;
    if identity(&now) == identity(&expected) {
        return None;
    }
    let wanted = identity(&expected);
    let line = now
        .iter()
        .find(|c| !wanted.contains(&identity_of(c)))
        .map(|c| c.line);
    Some(
        Finding::error(
            spec,
            &path,
            line,
            "spec behaviour edited outside a fold; change it through a delta",
        )
        .in_part(BEHAVIOUR_PART),
    )
}

pub fn spec_edits(
    root: &Path,
    world: &World,
    base: &str,
    files: &[String],
    completed: &[&Doc],
) -> Vec<Finding> {
    world
        .specs()
        .into_iter()
        .flat_map(|spec| {
            let edit = edited_outside_a_fold(root, world, base, files, completed, spec);
            edit.into_iter()
                .chain(scope_changed(root, base, spec, files))
        })
        .collect()
}
