use crate::claims::{has_intent, parse, Claim, BEHAVIOUR_PART};
use crate::evidence::{changed, resolve, RefStatus};
use crate::fold::fold;
use crate::payload::{tests_or_default, Attributes, Doc, Finding as Wire, Payload};
use crate::range::{self, Inputs, Rule};
use crate::receipt::{git_output, glob_set, invalid_glob_findings, touched_files};
use crate::targets::Targets;
use crate::world::{in_flight, Site, World};
use lazyspec::engine::store::Store;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Debug, Default, Clone, Copy)]
pub struct Require {
    pub claims: bool,
    pub evidence: bool,
    pub clean: bool,
}

#[derive(Debug, Default)]
pub struct Options<'a> {
    pub base: Option<&'a str>,
    pub trunk: Option<&'a str>,
    pub require: Require,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Finding {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    pub severity: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part: Option<String>,
    pub message: String,
}

impl Finding {
    fn new(
        severity: &'static str,
        doc: &Doc,
        path: impl Into<String>,
        line: Option<usize>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            path: Some(path.into()),
            line,
            severity,
            id: Some(doc.id.clone()),
            part: None,
            message: message.into(),
        }
    }

    pub fn error(
        doc: &Doc,
        path: impl Into<String>,
        line: Option<usize>,
        m: impl Into<String>,
    ) -> Self {
        Self::new("error", doc, path, line, m)
    }

    pub fn warning(
        doc: &Doc,
        path: impl Into<String>,
        line: Option<usize>,
        m: impl Into<String>,
    ) -> Self {
        Self::new("warning", doc, path, line, m)
    }

    fn graded(
        is_error: bool,
        doc: &Doc,
        path: impl Into<String>,
        line: Option<usize>,
        m: impl Into<String>,
    ) -> Self {
        match is_error {
            true => Self::error(doc, path, line, m),
            false => Self::warning(doc, path, line, m),
        }
    }

    pub fn in_part(mut self, part: &str) -> Self {
        self.part = Some(part.to_string());
        self
    }

    pub fn bare_error(message: impl Into<String>) -> Self {
        Self {
            path: None,
            line: None,
            severity: "error",
            id: None,
            part: None,
            message: message.into(),
        }
    }

    pub fn is_error(&self) -> bool {
        self.severity == "error"
    }

    pub fn render(&self) -> String {
        let place = match (&self.path, self.line) {
            (Some(path), Some(line)) => format!("{path}:{line}"),
            (Some(path), None) => path.clone(),
            (None, _) => "lemma".to_string(),
        };
        format!("{place}: {}: {}", self.severity, self.message)
    }

    pub fn for_hook(&self, payload: &Payload) -> Wire {
        let in_payload = self
            .id
            .as_deref()
            .is_some_and(|id| payload.documents.iter().any(|d| d.id == id));
        let (id, message) = match (in_payload, &self.path) {
            (true, _) => (self.id.clone(), self.message.clone()),
            (false, Some(path)) => (None, format!("{path}: {}", self.message)),
            (false, None) => (None, self.message.clone()),
        };
        Wire {
            severity: self.severity,
            id,
            part: self.part.clone().filter(|_| in_payload),
            line: self.line.filter(|_| in_payload),
            message,
        }
    }
}

pub fn has_error(findings: &[Finding]) -> bool {
    findings.iter().any(Finding::is_error)
}

pub fn run(
    store: &Store,
    root: &Path,
    options: &Options,
    payload: Option<&Payload>,
) -> Vec<Finding> {
    let world = World::load(store, payload);
    let scope = scope_of(&world, payload);
    let mut findings = match resolve_base(root, options, payload) {
        Ok(base) => {
            let mut found = coverage(store, root, &world, &changed_files(root, &base));
            found.extend(evidence_changes(root, &world, &base));
            found
        }
        Err(message) => vec![Finding::bare_error(message)],
    };
    for doc in &scope {
        findings.extend(doc_findings(root, &world, doc, options.require));
    }
    findings.extend(double_strikes(&world, &scope));
    if options.require.clean {
        findings.extend(uncommitted(root, &world, &scope));
    }
    findings.sort();
    findings.dedup();
    findings
}

fn scope_of<'a>(world: &'a World, payload: Option<&Payload>) -> Vec<&'a Doc> {
    let applies = |d: &&Doc| d.doc_type == "spec" || in_flight(d);
    match payload {
        Some(payload) => payload
            .documents
            .iter()
            .filter_map(|d| world.doc(&d.id))
            .filter(applies)
            .collect(),
        None => world.docs().iter().filter(applies).collect(),
    }
}

fn resolve_base(
    root: &Path,
    options: &Options,
    payload: Option<&Payload>,
) -> Result<String, String> {
    let delta_index = payload
        .and_then(|p| p.documents.iter().find(|d| d.doc_type == "delta"))
        .map(|d| Path::new(&d.path));
    let range = range::resolve(
        root,
        &Inputs {
            explicit: options.base,
            trunk: options.trunk,
            session: None,
            delta_index,
        },
    );
    let spec = format!("{}^{{commit}}", range.rev);
    let resolves = git_output(root, &["rev-parse", "--verify", "--quiet", &spec]).is_some();
    match (resolves, range.rule) {
        (true, _) => Ok(range.rev),
        (false, Rule::Explicit) => Err(format!(
            "base {} does not resolve to a commit; pass --base <rev>",
            range.rev
        )),
        (false, _) => Ok(range.rev),
    }
}

fn nul_separated(root: &Path, args: &[&str]) -> Vec<String> {
    git_output(root, args)
        .unwrap_or_default()
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(str::to_string)
        .collect()
}

pub fn changed_files(root: &Path, base: &str) -> Vec<String> {
    let tracked = nul_separated(root, &["diff", "--name-only", "--relative", "-z", base]);
    let untracked = nul_separated(root, &["ls-files", "--others", "--exclude-standard", "-z"]);
    let all: BTreeSet<String> = tracked.into_iter().chain(untracked).collect();
    all.into_iter().collect()
}

fn coverage(store: &Store, root: &Path, world: &World, files: &[String]) -> Vec<Finding> {
    let mut touched: BTreeMap<String, (String, Vec<&str>)> = BTreeMap::new();
    for file in files {
        for (meta, _) in store.governing(&root.join(file)) {
            if meta.doc_type.to_string() != "spec" {
                continue;
            }
            touched
                .entry(meta.id.clone())
                .or_insert_with(|| (meta.path.to_string_lossy().into_owned(), Vec::new()))
                .1
                .push(file);
        }
    }
    if touched.len() < 2 {
        return Vec::new();
    }
    let covered: BTreeSet<&str> = world
        .in_flight()
        .into_iter()
        .flat_map(Doc::fold_targets)
        .filter_map(|target| world.spec_named(target))
        .map(|spec| spec.id.as_str())
        .collect();
    let areas = touched.keys().cloned().collect::<Vec<_>>().join(", ");
    touched
        .iter()
        .filter(|(id, _)| !covered.contains(id.as_str()))
        .map(|(id, (path, files))| {
            let sample: Vec<&str> = files.iter().take(3).copied().collect();
            let more = files.len().saturating_sub(sample.len());
            let tail = if more > 0 {
                format!(" and {more} more")
            } else {
                String::new()
            };
            Finding {
                path: Some(path.clone()),
                line: None,
                severity: "error",
                id: Some(id.clone()),
                part: None,
                message: format!(
                    "changes to {}{tail} touch more than one governed area ({areas}) and no in-flight delta folds into {id}; create a delta that folds into {id}",
                    sample.join(", ")
                ),
            }
        })
        .collect()
}

fn struck_by_in_flight(world: &World) -> BTreeSet<String> {
    world
        .in_flight()
        .into_iter()
        .flat_map(|d| world.sites(d))
        .flat_map(|site| site.claims)
        .filter(|c| c.struck)
        .map(|c| c.id)
        .collect()
}

fn evidence_changes(root: &Path, world: &World, base: &str) -> Vec<Finding> {
    let struck = struck_by_in_flight(world);
    let short = &base[..base.len().min(8)];
    let mut findings = Vec::new();
    for spec in world.specs() {
        let Some(part) = spec.part(BEHAVIOUR_PART) else {
            continue;
        };
        let path = spec.part_path(BEHAVIOUR_PART);
        let Some(at_base) = git_output(root, &["show", &format!("{base}:./{path}")]) else {
            continue;
        };
        let now = parse(&part.body).claims;
        for old in parse(&at_base).claims.iter().filter(|c| !c.struck) {
            let Some(live) = now.iter().find(|c| c.id == old.id && !c.struck) else {
                continue;
            };
            if struck.contains(&old.id) {
                continue;
            }
            for r in old
                .receipts
                .iter()
                .filter(|r| r.checked_by.is_none() && !r.path.is_empty())
            {
                if !changed(root, base, &r.path, r.symbol.as_deref()) {
                    continue;
                }
                let line = live
                    .receipts
                    .iter()
                    .find(|n| n.path == r.path && n.symbol == r.symbol)
                    .map_or(live.line, |n| n.line);
                findings.push(
                    Finding::error(
                        spec,
                        &path,
                        Some(line),
                        format!(
                            "{}: evidence {} changed since {short}; strike {} in a delta and add a new id, or revert the change",
                            old.id,
                            ref_name(&r.path, r.symbol.as_deref()),
                            old.id
                        ),
                    )
                    .in_part(BEHAVIOUR_PART),
                );
            }
        }
    }
    findings
}

pub fn ref_name(path: &str, symbol: Option<&str>) -> String {
    match symbol {
        Some(symbol) => format!("{path}#{symbol}"),
        None => path.to_string(),
    }
}

fn doc_findings(root: &Path, world: &World, doc: &Doc, require: Require) -> Vec<Finding> {
    let sites = world.sites(doc);
    let mut findings = Vec::new();
    findings.extend(index_claims(root, doc));
    if doc.doc_type == "spec" {
        findings.extend(
            invalid_glob_findings(doc)
                .into_iter()
                .map(|w| from_wire(doc, w)),
        );
    } else {
        findings.extend(delta_structure(world, doc, require));
        findings.extend(fold_findings(doc, &sites));
    }
    for site in &sites {
        findings.extend(legacy_lines(site));
        findings.extend(claims_have_evidence(root, site, require));
    }
    findings.extend(intent_count(doc, &sites));
    findings
}

fn from_wire(doc: &Doc, wire: Wire) -> Finding {
    let path = match &wire.part {
        Some(part) => doc.part_path(part),
        None => doc.path.clone(),
    };
    Finding {
        path: Some(path),
        line: wire.line,
        severity: wire.severity,
        id: Some(doc.id.clone()),
        part: wire.part,
        message: wire.message,
    }
}

fn index_claims(root: &Path, doc: &Doc) -> Vec<Finding> {
    doc.index_claims(root)
        .into_iter()
        .map(|claim| {
            Finding::error(
                doc,
                &doc.path,
                Some(claim.line),
                format!(
                    "{}: claim-shaped item is in index.md; claims live in a behaviour part",
                    claim.id
                ),
            )
        })
        .collect()
}

fn delta_structure(world: &World, delta: &Doc, require: Require) -> Vec<Finding> {
    let specs = world.specs();
    let targets = Targets::of(delta, &specs);
    let parts = delta.behaviour_parts();
    let mut wires = Vec::new();
    if targets.declared.is_empty() {
        wires.push(Wire::error(
            &delta.id,
            None,
            "delta has no folds-into link; link the spec it folds into",
        ));
    }
    wires.extend(targets.unresolved_findings(delta));
    wires.extend(targets.missing_parts(delta, delta.status != "draft" || require.claims));
    wires.extend(targets.orphan_parts(&parts, delta));
    let mut findings: Vec<Finding> = wires.into_iter().map(|w| from_wire(delta, w)).collect();
    findings.extend(targets.resolved.iter().flat_map(|spec| {
        invalid_glob_findings(spec)
            .into_iter()
            .map(|w| from_wire(spec, w))
    }));
    if require.claims {
        findings.extend(readiness(delta, &parts));
    }
    findings
}

fn readiness(delta: &Doc, parts: &[crate::payload::BehaviourPart]) -> Vec<Finding> {
    let mut findings = Vec::new();
    if !has_intent(&delta.body) {
        findings.push(Finding::error(
            delta,
            &delta.path,
            None,
            "missing or empty `## Intent` section; state why the change is needed",
        ));
    }
    if parts.iter().all(|p| p.claims.is_empty()) {
        findings.push(Finding::error(
            delta,
            &delta.path,
            None,
            "no claims in the delta's behaviour parts; add `- [ID] text` items",
        ));
    }
    findings
}

fn fold_findings(delta: &Doc, sites: &[Site]) -> Vec<Finding> {
    sites
        .iter()
        .filter(|site| !site.claims.is_empty())
        .filter_map(|site| Some((site, site.governing?)))
        .flat_map(|(site, spec)| {
            let Some(behaviour) = spec.part(BEHAVIOUR_PART) else {
                return vec![Finding::error(
                    delta,
                    &site.path,
                    None,
                    format!(
                        "fold target {} has no `{BEHAVIOUR_PART}` part to fold into",
                        spec.id
                    ),
                )
                .in_part(site.part)];
            };
            let claims: Vec<&Claim> = site.claims.iter().collect();
            match fold(&behaviour.body, &claims) {
                Ok(_) => Vec::new(),
                Err(issues) => issues
                    .into_iter()
                    .map(|issue| {
                        Finding::error(delta, &site.path, Some(issue.line), issue.message)
                            .in_part(site.part)
                    })
                    .collect(),
            }
        })
        .collect()
}

pub fn legacy_lines(site: &Site) -> Vec<Finding> {
    site.claims
        .iter()
        .flat_map(|c| &c.legacy)
        .map(|l| Finding::error(site.doc, &site.path, Some(l.line), l.message()).in_part(site.part))
        .collect()
}

fn claims_have_evidence(root: &Path, site: &Site, require: Require) -> Vec<Finding> {
    let applies = site.doc.doc_type == "spec" || site.doc.status == "in-progress";
    if !applies {
        return Vec::new();
    }
    let tests = site
        .governing
        .map(Doc::tests_globs)
        .unwrap_or_else(|| tests_or_default(&Attributes::default()));
    let finding = |is_error, line, message: String| {
        Finding::graded(is_error, site.doc, &site.path, Some(line), message).in_part(site.part)
    };
    let mut findings = Vec::new();
    for claim in site.claims.iter().filter(|c| !c.struck) {
        for r in claim.refs() {
            let name = ref_name(&r.path, r.symbol.as_deref());
            match resolve(root, &r.path, r.symbol.as_deref(), &tests) {
                RefStatus::Ok => {}
                RefStatus::Unresolved(why) => findings.push(finding(
                    true,
                    r.line,
                    format!(
                        "{}: ref {name} does not resolve: {why}; fix the ref (see lemma refs {})",
                        claim.id, claim.id
                    ),
                )),
                RefStatus::NotATest => findings.push(finding(
                    true,
                    r.line,
                    format!(
                        "{}: ref {name} is not in the governing spec's `tests` globs; point it at a test or widen `tests`",
                        claim.id
                    ),
                )),
            }
        }
        let unevidenced = !claim.intent && claim.refs().next().is_none() && claim.legacy.is_empty();
        if unevidenced {
            findings.push(finding(
                require.evidence,
                claim.line,
                format!(
                    "{}: claim has no `@ref`; add `@ref <path>#<symbol>` to a test or mark it [intent]",
                    claim.id
                ),
            ));
        }
    }
    findings
}

fn intent_count(doc: &Doc, sites: &[Site]) -> Option<Finding> {
    let live: Vec<&Claim> = sites
        .iter()
        .flat_map(|s| &s.claims)
        .filter(|c| !c.struck)
        .collect();
    let intent = live.iter().filter(|c| c.intent).count();
    if intent == 0 {
        return None;
    }
    Some(Finding::warning(
        doc,
        &doc.path,
        None,
        format!("{intent} of {} claims are [intent]", live.len()),
    ))
}

fn double_strikes(world: &World, scope: &[&Doc]) -> Vec<Finding> {
    let mut strikes: BTreeMap<String, Vec<(&Doc, String, String, usize)>> = BTreeMap::new();
    for delta in world.in_flight() {
        for site in world.sites(delta) {
            for claim in site.claims.iter().filter(|c| c.struck) {
                strikes.entry(claim.id.clone()).or_default().push((
                    delta,
                    site.part.to_string(),
                    site.path.clone(),
                    claim.line,
                ));
            }
        }
    }
    strikes
        .into_iter()
        .filter(|(_, holders)| holders.len() > 1)
        .flat_map(|(id, holders)| {
            let ids: Vec<&str> = holders.iter().map(|(d, ..)| d.id.as_str()).collect();
            let names = ids.join(", ");
            holders
                .into_iter()
                .filter(|(d, ..)| scope.iter().any(|s| s.id == d.id))
                .map(|(delta, part, path, line)| {
                    Finding::warning(
                        delta,
                        path,
                        Some(line),
                        format!(
                            "{id} is struck by in-flight deltas {names}; the second fold will fail because the id is gone"
                        ),
                    )
                    .in_part(&part)
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn uncommitted(root: &Path, world: &World, scope: &[&Doc]) -> Vec<Finding> {
    let dirty = touched_files(root);
    let specs = world.specs();
    scope
        .iter()
        .filter(|d| in_flight(d))
        .flat_map(|delta| {
            let targets = Targets::of(delta, &specs);
            let governs: Vec<String> = targets
                .resolved
                .iter()
                .flat_map(|s| s.governs.clone())
                .collect();
            let governed = glob_set(&governs);
            let ref_paths: BTreeSet<String> = world
                .sites(delta)
                .into_iter()
                .flat_map(|s| s.claims)
                .filter(|c| !c.struck)
                .flat_map(|c| {
                    c.refs()
                        .map(|r| r.path.trim_start_matches("./").to_string())
                        .collect::<Vec<_>>()
                })
                .collect();
            dirty
                .iter()
                .filter(|p| governed.is_match(p.as_str()) || ref_paths.contains(p.as_str()))
                .map(|p| {
                    Finding::error(
                        delta,
                        p,
                        None,
                        format!("{p} has uncommitted changes; commit before completing"),
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect()
}
