use crate::claims::{has_intent, parse, Claim};
use crate::fold::fold;
use crate::payload::{Doc, Finding, Output, Payload, Update};
use crate::project::specs_from_store;
use crate::receipt::{Checker, Verdict};
use crate::targets::{specs_in, Targets};
use std::path::Path;

pub fn run(payload: &Payload, root: &Path) -> Output {
    let Some(transition) = &payload.transition else {
        return Output::findings(vec![Finding::bare_error(
            "pre-transition payload has no transition",
        )]);
    };
    let Some(delta) = payload.documents.first() else {
        return Output::default();
    };
    let supplied = specs_in(payload.context.iter().chain(&payload.documents));
    let stored = specs_from_store(root, &Targets::of(delta, &supplied).unresolved());
    let specs: Vec<&Doc> = supplied.into_iter().chain(specs_in(&stored)).collect();
    if transition.from == transition.to {
        return by_hand(&transition.from, delta, &specs, root);
    }
    match (transition.from.as_str(), transition.to.as_str()) {
        ("draft", "in-progress") => Output::findings(start(delta, &specs, root)),
        ("in-progress", "complete") => Output::findings(complete(delta, &specs, root)),
        ("complete", "archived") => archive(delta, &specs, root),
        _ => Output::default(),
    }
}

fn by_hand(status: &str, delta: &Doc, specs: &[&Doc], root: &Path) -> Output {
    let (next, findings) = match status {
        "draft" => ("in-progress", start(delta, specs, root)),
        "in-progress" => ("complete", complete(delta, specs, root)),
        "complete" => ("archived", archive_preview(delta, specs, root)),
        _ => return Output::default(),
    };
    if findings.iter().any(|f| f.severity == "error") {
        return Output::findings(findings);
    }
    let mut findings = findings;
    findings.push(Finding::warning(
        &delta.id,
        None,
        format!("checked {status} → {next}: ready"),
    ));
    Output::findings(findings)
}

fn archive_preview(delta: &Doc, specs: &[&Doc], root: &Path) -> Vec<Finding> {
    let mut findings = archive_findings(delta, specs, root);
    let targets = Targets::of(delta, specs);
    let behaviour = delta.behaviour(root);
    findings.extend(targets.resolved.iter().filter_map(|spec| {
        let message = fold_summary(spec, &targets, &behaviour.claims)?;
        Some(Finding::warning(&delta.id, None, message))
    }));
    findings
}

fn unresolved_targets(delta: &Doc, targets: &Targets) -> Vec<Finding> {
    if targets.declared.is_empty() {
        return vec![Finding::error(
            &delta.id,
            None,
            "delta has no folds-into link",
        )];
    }
    targets
        .unresolved()
        .into_iter()
        .map(|t| {
            Finding::error(
                &delta.id,
                None,
                format!("fold target {t} does not resolve to a spec"),
            )
        })
        .collect()
}

fn dropped_claims(delta: &Doc, targets: &Targets, root: &Path) -> Vec<Finding> {
    let behaviour = delta.behaviour(root);
    let partitions = behaviour
        .partitions
        .iter()
        .filter(|p| !targets.names(&p.name))
        .map(|p| {
            Finding::error(
                &delta.id,
                Some(p.line),
                format!(
                    "partition {} is not one of this delta's folds-into targets; its claims would not fold",
                    p.name
                ),
            )
        });
    let claims = behaviour
        .claims
        .iter()
        .filter(|c| !c.struck && targets.spec_for(c).is_none())
        .filter(|c| !awaits_target(c, targets))
        .map(|c| {
            Finding::error(
                &delta.id,
                Some(c.line),
                format!(
                    "{}: claim would not fold into any spec (partition {})",
                    c.id,
                    c.partition.as_deref().unwrap_or("missing")
                ),
            )
        });
    partitions.chain(claims).collect()
}

fn awaits_target(claim: &Claim, targets: &Targets) -> bool {
    targets.declared.is_empty()
        || claim
            .partition
            .as_deref()
            .is_some_and(|p| targets.declared.contains(&p))
}

fn start(delta: &Doc, specs: &[&Doc], root: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();
    if !has_intent(&delta.body) {
        findings.push(Finding::error(
            &delta.id,
            None,
            "missing or empty `## Intent` section",
        ));
    }
    if delta.behaviour(root).claims.is_empty() {
        findings.push(Finding::error(
            &delta.id,
            None,
            "no claims under `## Behaviour`",
        ));
    }
    let targets = Targets::of(delta, specs);
    findings.extend(unresolved_targets(delta, &targets));
    findings.extend(dropped_claims(delta, &targets, root));
    findings
}

fn complete(delta: &Doc, specs: &[&Doc], root: &Path) -> Vec<Finding> {
    let checker = Checker::new(root);
    let targets = Targets::of(delta, specs);
    let mut findings = dropped_claims(delta, &targets, root);
    findings.extend(
        delta
            .behaviour(root)
            .claims
            .iter()
            .filter(|c| !c.struck && !c.intent)
            .flat_map(|claim| unsettled(delta, claim, &targets, &checker)),
    );
    findings
}

fn unsettled(delta: &Doc, claim: &Claim, targets: &Targets, checker: &Checker) -> Vec<Finding> {
    if claim.receipts.is_empty() {
        return vec![Finding::error(
            &delta.id,
            Some(claim.line),
            format!("{}: claim has no receipt", claim.id),
        )];
    }
    let governs = targets
        .spec_for(claim)
        .map(|s| s.governs.clone())
        .unwrap_or_default();
    claim
        .receipts
        .iter()
        .filter_map(|receipt| match checker.check(receipt, &governs) {
            Verdict::Fresh => None,
            Verdict::Unresolved(why) => Some(Finding::error(
                &delta.id,
                Some(receipt.line),
                format!("{}: receipt does not resolve: {why}", claim.id),
            )),
            Verdict::Stale(why) => Some(Finding::error(
                &delta.id,
                Some(receipt.line),
                format!("{}: receipt is stale: {why}", claim.id),
            )),
        })
        .collect()
}

fn archive_findings(delta: &Doc, specs: &[&Doc], root: &Path) -> Vec<Finding> {
    let targets = Targets::of(delta, specs);
    let mut findings = unresolved_targets(delta, &targets);
    findings.extend(dropped_claims(delta, &targets, root));
    findings
}

fn archive(delta: &Doc, specs: &[&Doc], root: &Path) -> Output {
    let findings = archive_findings(delta, specs, root);
    if !findings.is_empty() {
        return Output::findings(findings);
    }
    let targets = Targets::of(delta, specs);
    let behaviour = delta.behaviour(root);
    let updates = targets
        .resolved
        .iter()
        .filter_map(|spec| fold_into(spec, &targets, &behaviour.claims))
        .collect();
    Output {
        findings: Vec::new(),
        updates,
    }
}

fn fold_summary(spec: &Doc, targets: &Targets, claims: &[Claim]) -> Option<String> {
    let mine = targets.claims_for(spec, claims);
    if mine.is_empty() {
        return None;
    }
    let existing = parse(&spec.body).claims;
    let strikes = mine
        .iter()
        .filter(|c| c.struck && existing.iter().any(|e| e.id == c.id))
        .count();
    let folded = mine.iter().filter(|c| !c.struck).count();
    let strike_note = if strikes > 0 {
        format!(", strike {strikes}")
    } else {
        String::new()
    };
    Some(format!(
        "would fold {folded} claims into {}{strike_note}",
        spec.id
    ))
}

fn fold_into(spec: &Doc, targets: &Targets, claims: &[Claim]) -> Option<Update> {
    let mine = targets.claims_for(spec, claims);
    if mine.is_empty() {
        return None;
    }
    Some(Update {
        id: spec.id.clone(),
        hash: spec.content_hash.clone(),
        body: fold(&spec.body, &mine),
    })
}
