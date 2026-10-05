use crate::claims::{Claim, BEHAVIOUR_PART};
use crate::fold::BaseKind;
use crate::payload::{BehaviourPart, Doc, Finding, Output, Payload};
use crate::project::specs_from_store;
use crate::receipt::{invalid_glob_findings, non_test_receipts, Checker, Verdict};
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
            "delta" => {
                findings.extend(check_delta(doc, &specs, &checker));
                if doc.status != "complete" {
                    findings.extend(intent_count(doc, &delta_claims(doc)));
                }
            }
            "spec" => {
                findings.extend(invalid_glob_findings(doc));
                findings.extend(check_spec(doc, &checker));
                findings.extend(intent_count(doc, &doc.spec_claims()));
            }
            _ => continue,
        }
        findings.extend(claims_in_index(doc, root));
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
        "delta" => doc.status == "in-progress",
        "spec" => matches!(doc.status.as_str(), "active" | "draft"),
        _ => false,
    }
}

fn receipt_findings(
    doc: &Doc,
    part: &str,
    claim: &Claim,
    governs: &[String],
    checker: &Checker,
) -> Vec<Finding> {
    claim
        .receipts
        .iter()
        .filter_map(|receipt| match checker.check(receipt, governs) {
            Verdict::Fresh => None,
            Verdict::Unresolved(why) => Some(
                Finding::error(
                    &doc.id,
                    Some(receipt.line),
                    format!("{}: receipt does not resolve: {why}", claim.id),
                )
                .in_part(part),
            ),
            Verdict::Stale(why) => Some(
                Finding::warning(
                    &doc.id,
                    Some(receipt.line),
                    format!("{}: receipt is stale: {why}", claim.id),
                )
                .in_part(part),
            ),
        })
        .collect()
}

fn delta_claims(delta: &Doc) -> Vec<Claim> {
    delta
        .behaviour_parts()
        .into_iter()
        .flat_map(|p| p.claims)
        .collect()
}

fn intent_count(doc: &Doc, claims: &[Claim]) -> Option<Finding> {
    let live: Vec<&Claim> = claims.iter().filter(|c| !c.struck).collect();
    let intent = live.iter().filter(|c| c.intent).count();
    if intent == 0 {
        return None;
    }
    Some(Finding::warning(
        &doc.id,
        None,
        format!("{intent} of {} claims are [intent]", live.len()),
    ))
}

fn claims_in_index(doc: &Doc, root: &Path) -> Vec<Finding> {
    doc.index_claims(root)
        .into_iter()
        .map(|claim| {
            Finding::error(
                &doc.id,
                Some(claim.line),
                format!(
                    "{}: claim-shaped item is in index.md; claims live in a behaviour part",
                    claim.id
                ),
            )
        })
        .collect()
}

fn check_spec(spec: &Doc, checker: &Checker) -> Vec<Finding> {
    if !receipts_apply(spec) {
        return Vec::new();
    }
    spec.spec_claims()
        .iter()
        .flat_map(|claim| {
            let mut found = receipt_findings(spec, BEHAVIOUR_PART, claim, &spec.governs, checker);
            if !claim.intent {
                found.extend(non_test_receipts(&spec.id, BEHAVIOUR_PART, claim, spec));
            }
            found
        })
        .collect()
}

fn check_delta(delta: &Doc, specs: &[&Doc], checker: &Checker) -> Vec<Finding> {
    let parts = delta.behaviour_parts();
    let targets = Targets::of(delta, specs);
    let mut findings = targets.unresolved_findings(delta);
    findings.extend(targets.missing_parts(delta, delta.status != "draft"));
    findings.extend(targets.orphan_parts(&parts, delta));
    findings.extend(base_findings(delta, &targets, &parts));

    for part in &parts {
        let governs = targets.governs_of(part);
        for claim in part.claims.iter().filter(|c| !c.struck) {
            if delta.status == "in-progress" && !claim.intent && claim.receipts.is_empty() {
                findings.push(
                    Finding::error(
                        &delta.id,
                        Some(claim.line),
                        format!("{}: claim has no receipt", claim.id),
                    )
                    .in_part(part.name),
                );
            }
            if receipts_apply(delta) {
                findings.extend(receipt_findings(delta, part.name, claim, &governs, checker));
                if !claim.intent {
                    findings.extend(
                        targets
                            .spec_of(part)
                            .into_iter()
                            .flat_map(|spec| non_test_receipts(&delta.id, part.name, claim, spec)),
                    );
                }
            }
        }
    }
    findings
}

fn base_findings(delta: &Doc, targets: &Targets, parts: &[BehaviourPart]) -> Vec<Finding> {
    if delta.status == "complete" {
        return Vec::new();
    }
    targets
        .base_issues(parts)
        .into_iter()
        .map(|p| {
            let message = p.issue.message(&delta.id, &p.spec);
            let line = Some(p.issue.line);
            let finding = match p.issue.kind {
                BaseKind::Missing | BaseKind::StrikeOfUnknown => {
                    Finding::error(&delta.id, line, message)
                }
                BaseKind::Stale { .. } | BaseKind::OrphanBase => {
                    Finding::warning(&delta.id, line, message)
                }
            };
            finding.in_part(&p.part)
        })
        .collect()
}

fn overlapping_claims(docs: &[Doc]) -> Vec<Finding> {
    let mut by_claim: BTreeMap<String, Vec<(&Doc, String, usize)>> = BTreeMap::new();
    for delta in docs
        .iter()
        .filter(|d| d.doc_type == "delta" && d.status == "in-progress")
    {
        for part in delta.behaviour_parts() {
            for claim in part.claims {
                let holders = by_claim.entry(claim.id).or_default();
                if holders.iter().any(|(d, _, _)| d.id == delta.id) {
                    continue;
                }
                holders.push((delta, part.name.to_string(), claim.line));
            }
        }
    }
    by_claim
        .into_iter()
        .filter(|(_, holders)| holders.len() > 1)
        .flat_map(|(claim_id, holders)| {
            let ids: Vec<&str> = holders.iter().map(|(d, _, _)| d.id.as_str()).collect();
            holders
                .iter()
                .map(move |(delta, part, line)| {
                    Finding::warning(
                        &delta.id,
                        Some(*line),
                        format!(
                            "{claim_id} is touched by in-progress deltas {}",
                            ids.join(", ")
                        ),
                    )
                    .in_part(part)
                })
                .collect::<Vec<_>>()
        })
        .collect()
}
