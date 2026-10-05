use crate::check::{legacy_lines, Finding};
use crate::claims::{Claim, BEHAVIOUR_PART};
use crate::fold::fold;
use crate::payload::{Doc, Output, Payload, Update};
use crate::targets::Targets;
use crate::world::World;
use anyhow::{bail, Result};
use lazyspec::engine::store::Store;

struct Folded {
    update: Update,
    lines: Vec<String>,
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

fn claim_text(claim: &Claim) -> &str {
    let first = claim.block.first().map_or("", String::as_str);
    first
        .split_once(']')
        .map_or("", |(_, rest)| rest.trim_start_matches('~').trim())
}

fn summary(spec: &Doc, claims: &[&Claim]) -> Vec<String> {
    let existing = spec.spec_claims();
    claims
        .iter()
        .map(|claim| match claim.struck {
            true => {
                let text = existing
                    .iter()
                    .find(|e| e.id == claim.id)
                    .map_or_else(|| claim_text(claim), claim_text);
                format!("-{} {text}", claim.id)
            }
            false => format!("+{} {}", claim.id, claim_text(claim)),
        })
        .map(|line| format!("{}: {line}", spec.id))
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
                lines: summary(spec, &claims),
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

pub fn hook(store: &Store, payload: &Payload) -> Output {
    let Some(moved) = payload.documents.first() else {
        return Output::default();
    };
    let world = World::load(store, Some(payload));
    let Some(delta) = world.doc(&moved.id) else {
        return Output::default();
    };
    let plan = plan(&world, delta, true);
    let findings = plan.findings.iter().map(|f| f.for_hook(payload)).collect();
    if plan.failed() {
        return Output::findings(findings);
    }
    Output {
        findings,
        updates: plan.folded.into_iter().map(|f| f.update).collect(),
        ..Output::default()
    }
}

pub struct Preview {
    pub lines: Vec<String>,
    pub findings: Vec<Finding>,
}

pub fn preview(store: &Store, name: &str) -> Result<Preview> {
    let meta = store.resolve_shorthand(name)?;
    let world = World::load(store, None);
    let Some(delta) = world.doc(&meta.id).filter(|d| d.doc_type == "delta") else {
        bail!("{name} is not a delta");
    };
    let plan = plan(&world, delta, false);
    let lines = match plan.failed() {
        true => Vec::new(),
        false => plan.folded.into_iter().flat_map(|f| f.lines).collect(),
    };
    Ok(Preview {
        lines,
        findings: plan.findings,
    })
}
