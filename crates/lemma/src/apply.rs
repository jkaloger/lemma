use crate::check::{legacy_lines, Finding, UNRESOLVED_ASSUMPTION};
use crate::claims::{Claim, BEHAVIOUR_PART};
use crate::fold::fold;
use crate::payload::{Doc, Output, Payload, Update};
use crate::review::{base_of, review, TargetReview};
use crate::targets::Targets;
use crate::world::{Site, World};
use anyhow::{bail, Result};
use lazyspec::engine::store::Store;
use std::path::Path;

struct Folded {
    update: Update,
}

struct Plan {
    folded: Vec<Folded>,
    findings: Vec<Finding>,
}

impl Plan {
    fn failed(&self) -> bool {
        self.findings.iter().any(Finding::is_error)
    }
}

fn unresolved_assumptions(site: &Site) -> Vec<Finding> {
    site.claims
        .iter()
        .filter(|c| c.assumed && !c.struck)
        .map(|c| {
            Finding::error(
                site.doc,
                &site.path,
                Some(c.line),
                format!("{}: {UNRESOLVED_ASSUMPTION}", c.id),
            )
            .in_part(site.part)
        })
        .collect()
}

fn plan(world: &World, delta: &Doc, writable: bool) -> Plan {
    let specs = world.specs();
    let targets = Targets::of(delta, &specs);
    let sites = world.sites(delta);
    let mut findings = Vec::new();
    let mut folded = Vec::new();
    let at_delta = |message: &str| Finding::error(delta, &delta.path, None, message);
    if targets.declared.is_empty() {
        findings.push(at_delta(
            "delta has no folds-into link; link the spec it folds into",
        ));
    }
    for target in targets.unresolved() {
        findings.push(at_delta(&format!(
            "fold target {target} does not resolve to a spec"
        )));
    }
    for site in &sites {
        if site.governing.is_none() {
            findings.push(
                Finding::error(
                    delta,
                    &site.path,
                    None,
                    format!(
                        "part {} does not name one of this delta's folds-into targets (expected behaviour.<target-id>); its claims would not fold",
                        site.part
                    ),
                )
                .in_part(site.part),
            );
        }
        findings.extend(legacy_lines(site));
        findings.extend(unresolved_assumptions(site));
    }
    for spec in &targets.resolved {
        let mine: Vec<(&str, &str, &Claim)> = sites
            .iter()
            .filter(|s| s.governing.is_some_and(|g| g.id == spec.id))
            .flat_map(|s| s.claims.iter().map(move |c| (s.part, s.path.as_str(), c)))
            .collect();
        if mine.is_empty() {
            continue;
        }
        let (part, path, _) = mine[0];
        let in_part = |f: Finding| f.in_part(part);
        if spec.status == "draft" {
            findings.push(Finding::warning(
                delta,
                &delta.path,
                None,
                format!(
                    "{} is draft; move it to active once this fold lands",
                    spec.id
                ),
            ));
        }
        let Some(behaviour) = spec.part(BEHAVIOUR_PART) else {
            findings.push(in_part(Finding::error(
                delta,
                path,
                None,
                format!(
                    "fold target {} has no `{BEHAVIOUR_PART}` part to fold into; create it on the spec first",
                    spec.id
                ),
            )));
            continue;
        };
        if writable && spec.from_store {
            findings.push(at_delta(&format!(
                "fold target {} was not in the hook payload, so its content hash is unknown and the fold cannot be written",
                spec.id
            )));
            continue;
        }
        let claims: Vec<&Claim> = mine.iter().map(|(_, _, c)| *c).collect();
        match fold(&behaviour.body, &claims) {
            Ok(body) => folded.push(Folded {
                update: Update {
                    id: spec.id.clone(),
                    part: Some(BEHAVIOUR_PART.to_string()),
                    hash: spec.content_hash.clone(),
                    body,
                },
            }),
            Err(issues) => findings.extend(issues.into_iter().map(|issue| {
                in_part(Finding::error(delta, path, Some(issue.line), issue.message))
            })),
        }
    }
    Plan { folded, findings }
}

fn reviews(world: &World, delta: &Doc, root: &Path) -> Vec<TargetReview> {
    let specs = world.specs();
    let targets = Targets::of(delta, &specs);
    let sites = world.sites(delta);
    let base = base_of(root, delta);
    targets
        .resolved
        .iter()
        .filter_map(|spec| {
            let mine: Vec<&Site> = sites
                .iter()
                .filter(|s| s.governing.is_some_and(|g| g.id == spec.id))
                .collect();
            let review = review(root, &base, spec, &mine);
            (!review.is_empty()).then_some(review)
        })
        .collect()
}

fn at_complete_gate(payload: &Payload) -> bool {
    payload
        .transition
        .as_ref()
        .is_some_and(|t| t.from == "in-progress" && t.to == "complete")
}

pub fn hook(store: &Store, root: &Path, payload: &Payload) -> Output {
    let Some(moved) = payload.documents.first() else {
        return Output::default();
    };
    let world = World::load(store, Some(payload));
    let Some(delta) = world.doc(&moved.id) else {
        return Output::default();
    };
    let plan = plan(&world, delta, true);
    let mut findings: Vec<_> = plan.findings.iter().map(|f| f.for_hook(payload)).collect();
    if plan.failed() {
        return Output::findings(findings);
    }
    if at_complete_gate(payload) {
        let reviewed = reviews(&world, delta, root);
        let accountability = reviewed
            .iter()
            .flat_map(|r| r.findings(delta))
            .map(|f| f.for_hook(payload));
        findings.extend(accountability);
    }
    Output {
        findings,
        updates: plan.folded.into_iter().map(|f| f.update).collect(),
    }
}

pub struct Preview {
    pub lines: Vec<String>,
    pub findings: Vec<Finding>,
}

pub fn preview(store: &Store, root: &Path, name: &str) -> Result<Preview> {
    let meta = store.resolve_shorthand(name)?;
    let world = World::load(store, None);
    let Some(delta) = world.doc(&meta.id).filter(|d| d.doc_type == "delta") else {
        bail!("{name} is not a delta");
    };
    let plan = plan(&world, delta, false);
    let failed = plan.failed();
    let lines = reviews(&world, delta, root)
        .iter()
        .flat_map(|r| r.render(failed))
        .collect();
    Ok(Preview {
        lines,
        findings: plan.findings,
    })
}
