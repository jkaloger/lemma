use crate::claims::{has_intent, Claim, BEHAVIOUR_PART};
use crate::fold::fold;
use crate::payload::{BehaviourPart, Doc, Finding, Output, Payload, Update};
use crate::project::specs_from_store;
use crate::receipt::{
    forced_warnings, glob_set, invalid_glob_findings, non_test_receipts, Checker, Verdict,
};
use crate::targets::{specs_in, Targets};
use std::collections::BTreeSet;
use std::path::Path;

const INDEX_FILE: &str = "index.md";

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
        ("draft", "in-progress") => Output::findings(start(delta, &specs)),
        ("in-progress", "complete") => complete_and_fold(delta, &specs, root),
        _ => Output::default(),
    }
}

fn by_hand(status: &str, delta: &Doc, specs: &[&Doc], root: &Path) -> Output {
    let (next, mut findings) = match status {
        "draft" => ("in-progress", start(delta, specs)),
        "in-progress" => {
            let mut findings = uncommitted_bundle_files(delta, root);
            findings.extend(complete_preview(delta, specs, root));
            ("complete", findings)
        }
        _ => return Output::default(),
    };
    if findings.iter().any(|f| f.severity == "error") {
        return Output::findings(findings);
    }
    findings.push(Finding::warning(
        &delta.id,
        None,
        format!("checked {status} → {next}: ready"),
    ));
    Output::findings(findings)
}

fn uncommitted_bundle_files(delta: &Doc, root: &Path) -> Vec<Finding> {
    let index = Path::new(&delta.path);
    let bundle = match index.file_name().is_some_and(|n| n == INDEX_FILE) {
        true => index.parent(),
        false => None,
    };
    Checker::new(root)
        .dirty_paths()
        .iter()
        .filter(|p| match bundle {
            Some(dir) => Path::new(p.as_str()).starts_with(dir),
            None => Path::new(p.as_str()) == index,
        })
        .map(|p| {
            Finding::warning(
                &delta.id,
                None,
                format!("{p} has uncommitted changes; commit the delta before asking for review"),
            )
        })
        .collect()
}

fn complete_preview(delta: &Doc, specs: &[&Doc], root: &Path) -> Vec<Finding> {
    let mut findings = complete(delta, specs, root, false);
    let targets = Targets::of(delta, specs);
    let parts = delta.behaviour_parts();
    findings.extend(targets.resolved.iter().filter_map(|spec| {
        let message = fold_summary(spec, &targets, &parts)?;
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
    targets.unresolved_findings(delta)
}

fn start(delta: &Doc, specs: &[&Doc]) -> Vec<Finding> {
    let mut findings = Vec::new();
    if !has_intent(&delta.body) {
        findings.push(Finding::error(
            &delta.id,
            None,
            "missing or empty `## Intent` section",
        ));
    }
    let parts = delta.behaviour_parts();
    if parts.iter().all(|p| p.claims.is_empty()) {
        findings.push(Finding::error(
            &delta.id,
            None,
            "no claims in the delta's behaviour parts",
        ));
    }
    let targets = Targets::of(delta, specs);
    findings.extend(unresolved_targets(delta, &targets));
    findings.extend(targets.missing_parts(delta, true));
    findings.extend(targets.orphan_parts(&parts, delta));
    findings.extend(base_findings(delta, &targets, &parts, true));
    findings
}

fn base_findings(
    delta: &Doc,
    targets: &Targets,
    parts: &[BehaviourPart],
    start_only: bool,
) -> Vec<Finding> {
    targets
        .base_issues(parts)
        .into_iter()
        .filter(|p| !start_only || p.issue.blocks_start())
        .map(|p| {
            Finding::error(
                &delta.id,
                Some(p.issue.line),
                p.issue.message(&delta.id, &p.spec),
            )
            .in_part(&p.part)
        })
        .collect()
}

fn complete(delta: &Doc, specs: &[&Doc], root: &Path, will_write: bool) -> Vec<Finding> {
    let checker = Checker::new(root);
    let targets = Targets::of(delta, specs);
    let parts = delta.behaviour_parts();
    let mut findings = unresolved_targets(delta, &targets);
    findings.extend(targets.orphan_parts(&parts, delta));
    for part in &parts {
        let governs = targets.governs_of(part);
        for claim in part.claims.iter().filter(|c| !c.struck && !c.intent) {
            findings.extend(unsettled(delta, part, claim, &governs, &checker));
            findings.extend(forced_warnings(&delta.id, part.name, claim));
            findings.extend(
                targets
                    .spec_of(part)
                    .into_iter()
                    .flat_map(|spec| non_test_receipts(&delta.id, part.name, claim, spec)),
            );
        }
    }
    findings.extend(
        targets
            .resolved
            .iter()
            .flat_map(|spec| invalid_glob_findings(spec)),
    );
    findings.extend(unfoldable_targets(delta, &targets, &parts, will_write));
    findings.extend(dirty_findings(delta, &targets, &parts, &checker));
    findings.extend(base_findings(delta, &targets, &parts, false));
    findings.extend(draft_specs(delta, &targets));
    findings
}

fn draft_specs(delta: &Doc, targets: &Targets) -> Vec<Finding> {
    targets
        .resolved
        .iter()
        .filter(|spec| spec.status == "draft")
        .map(|spec| {
            Finding::warning(
                &delta.id,
                None,
                format!(
                    "{} is draft; move it to active once this fold lands",
                    spec.id
                ),
            )
        })
        .collect()
}

fn dirty_findings(
    delta: &Doc,
    targets: &Targets,
    parts: &[BehaviourPart],
    checker: &Checker,
) -> Vec<Finding> {
    let governs: Vec<String> = targets
        .resolved
        .iter()
        .flat_map(|s| s.governs.clone())
        .collect();
    let governed = glob_set(&governs);
    let receipt_paths: BTreeSet<&str> = parts
        .iter()
        .flat_map(|p| &p.claims)
        .filter(|c| !c.struck && !c.intent)
        .flat_map(|c| &c.receipts)
        .filter(|r| r.checked_by.is_none())
        .map(|r| r.path.trim_start_matches("./"))
        .collect();
    checker
        .dirty_paths()
        .iter()
        .filter(|p| governed.is_match(p.as_str()) || receipt_paths.contains(p.as_str()))
        .map(|p| {
            Finding::error(
                &delta.id,
                None,
                format!("{p} has uncommitted changes; commit before completing"),
            )
        })
        .collect()
}

fn unsettled(
    delta: &Doc,
    part: &BehaviourPart,
    claim: &Claim,
    governs: &[String],
    checker: &Checker,
) -> Vec<Finding> {
    if claim.receipts.is_empty() {
        return vec![Finding::error(
            &delta.id,
            Some(claim.line),
            format!("{}: claim has no receipt", claim.id),
        )
        .in_part(part.name)];
    }
    claim
        .receipts
        .iter()
        .filter_map(|receipt| {
            let message = match checker.check(receipt, governs) {
                Verdict::Fresh => return None,
                Verdict::Unresolved(why) => {
                    format!("{}: receipt does not resolve: {why}", claim.id)
                }
                Verdict::Stale(why) => format!("{}: receipt is stale: {why}", claim.id),
            };
            Some(Finding::error(&delta.id, Some(receipt.line), message).in_part(part.name))
        })
        .collect()
}

fn unfoldable_targets(
    delta: &Doc,
    targets: &Targets,
    parts: &[BehaviourPart],
    will_write: bool,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    for spec in &targets.resolved {
        if targets.claims_for(spec, parts).is_empty() {
            continue;
        }
        if spec.part(BEHAVIOUR_PART).is_none() {
            findings.push(Finding::error(
                &delta.id,
                None,
                format!(
                    "fold target {} has no `{BEHAVIOUR_PART}` part to fold into",
                    spec.id
                ),
            ));
        }
        if will_write && spec.from_store {
            findings.push(Finding::error(
                &delta.id,
                None,
                format!(
                    "fold target {} was not in the hook payload, so its content hash is unknown and the fold cannot be written",
                    spec.id
                ),
            ));
        }
    }
    findings
}

fn complete_and_fold(delta: &Doc, specs: &[&Doc], root: &Path) -> Output {
    let findings = complete(delta, specs, root, true);
    if findings.iter().any(|f| f.severity == "error") {
        return Output::findings(findings);
    }
    let targets = Targets::of(delta, specs);
    let parts = delta.behaviour_parts();
    let updates = targets
        .resolved
        .iter()
        .filter_map(|spec| fold_into(spec, &targets, &parts))
        .collect();
    Output {
        findings,
        updates,
        ..Output::default()
    }
}

fn fold_summary(spec: &Doc, targets: &Targets, parts: &[BehaviourPart]) -> Option<String> {
    let mine = targets.claims_for(spec, parts);
    if mine.is_empty() {
        return None;
    }
    let existing = spec.spec_claims();
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

fn fold_into(spec: &Doc, targets: &Targets, parts: &[BehaviourPart]) -> Option<Update> {
    let mine = targets.claims_for(spec, parts);
    if mine.is_empty() {
        return None;
    }
    let behaviour = spec.part(BEHAVIOUR_PART)?;
    Some(Update {
        id: spec.id.clone(),
        part: Some(BEHAVIOUR_PART.to_string()),
        hash: spec.content_hash.clone(),
        body: fold(&behaviour.body, &mine),
    })
}
