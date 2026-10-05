use crate::claims::{parse, Claim};
use crate::payload::{Doc, Finding, Output, Payload};
use crate::project::specs_from_store;
use crate::receipt::{Checker, Verdict};
use crate::targets::{specs_in, Targets};
use std::collections::BTreeMap;
use std::path::Path;

pub fn run(payload: &Payload, root: &Path) -> Output {
    let checker = Checker::new(root);
    let stored = specs_from_store(root, &targets_missing_from(&payload.documents));
    let specs = specs_in(payload.documents.iter().chain(&stored));
    let mut findings = Vec::new();
    for doc in &payload.documents {
        match doc.doc_type.as_str() {
            "delta" => findings.extend(check_delta(doc, &specs, &checker)),
            "spec" => findings.extend(check_spec(doc, &checker)),
            _ => {}
        }
    }
    findings.extend(overlapping_claims(&payload.documents));
    Output::findings(findings)
}

fn targets_missing_from(docs: &[Doc]) -> Vec<&str> {
    let specs = specs_in(docs);
    docs.iter()
        .filter(|d| d.doc_type == "delta")
        .flat_map(|d| Targets::of(d, &specs).unresolved())
        .collect()
}

fn receipts_apply(doc: &Doc) -> bool {
    match doc.doc_type.as_str() {
        "delta" => matches!(doc.status.as_str(), "in-progress" | "complete"),
        "spec" => matches!(doc.status.as_str(), "active" | "draft"),
        _ => false,
    }
}

fn receipt_findings(
    doc: &Doc,
    claim: &Claim,
    governs: &[String],
    checker: &Checker,
) -> Vec<Finding> {
    claim
        .receipts
        .iter()
        .filter_map(|receipt| match checker.check(receipt, governs) {
            Verdict::Fresh => None,
            Verdict::Unresolved(why) => Some(Finding::warning(
                &doc.id,
                Some(receipt.line),
                format!("{}: receipt does not resolve: {why}", claim.id),
            )),
            Verdict::Stale(why) => Some(Finding::warning(
                &doc.id,
                Some(receipt.line),
                format!("{}: receipt is stale: {why}", claim.id),
            )),
        })
        .collect()
}

fn check_spec(spec: &Doc, checker: &Checker) -> Vec<Finding> {
    if !receipts_apply(spec) {
        return Vec::new();
    }
    parse(&spec.body)
        .claims
        .iter()
        .flat_map(|claim| receipt_findings(spec, claim, &spec.governs, checker))
        .collect()
}

fn check_delta(delta: &Doc, specs: &[&Doc], checker: &Checker) -> Vec<Finding> {
    let behaviour = parse(&delta.body);
    let targets = Targets::of(delta, specs);
    let mut findings: Vec<Finding> = targets
        .unresolved()
        .into_iter()
        .map(|t| {
            Finding::warning(
                &delta.id,
                None,
                format!("fold target {t} does not resolve to a spec"),
            )
        })
        .collect();

    findings.extend(
        behaviour
            .partitions
            .iter()
            .filter(|p| !targets.names(&p.name))
            .map(|p| {
                Finding::warning(
                    &delta.id,
                    Some(p.line),
                    format!(
                        "partition {} is not in this delta's folds-into links",
                        p.name
                    ),
                )
            }),
    );

    if targets.is_multi() {
        findings.extend(
            behaviour
                .claims
                .iter()
                .filter(|c| c.partition.is_none())
                .map(|c| {
                    Finding::warning(
                        &delta.id,
                        Some(c.line),
                        format!(
                            "{}: claim is not under a spec partition in a multi-target delta",
                            c.id
                        ),
                    )
                }),
        );
    }

    for claim in behaviour.claims.iter().filter(|c| !c.struck) {
        if delta.status == "in-progress" && !claim.intent && claim.receipts.is_empty() {
            findings.push(Finding::warning(
                &delta.id,
                Some(claim.line),
                format!("{}: claim has no receipt", claim.id),
            ));
        }
        if receipts_apply(delta) {
            let governs = targets
                .spec_for(claim)
                .map(|s| s.governs.clone())
                .unwrap_or_default();
            findings.extend(receipt_findings(delta, claim, &governs, checker));
        }
    }
    findings
}

fn overlapping_claims(docs: &[Doc]) -> Vec<Finding> {
    let mut by_claim: BTreeMap<String, Vec<(&Doc, usize)>> = BTreeMap::new();
    for delta in docs
        .iter()
        .filter(|d| d.doc_type == "delta" && d.status == "in-progress")
    {
        for claim in parse(&delta.body).claims {
            let holders = by_claim.entry(claim.id).or_default();
            if holders.iter().any(|(d, _)| d.id == delta.id) {
                continue;
            }
            holders.push((delta, claim.line));
        }
    }
    by_claim
        .into_iter()
        .filter(|(_, holders)| holders.len() > 1)
        .flat_map(|(claim_id, holders)| {
            let ids: Vec<&str> = holders.iter().map(|(d, _)| d.id.as_str()).collect();
            holders
                .iter()
                .map(move |(delta, line)| {
                    Finding::warning(
                        &delta.id,
                        Some(*line),
                        format!(
                            "{claim_id} is touched by in-progress deltas {}",
                            ids.join(", ")
                        ),
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect()
}
