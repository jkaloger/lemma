use crate::claims::{has_intent, parse, Claim, BEHAVIOUR_PART};
use crate::evidence::{clean, glob_set, invalid_globs, resolve, RefStatus, Regions};
use crate::fold::fold;
use crate::folded::{completed_since, folds_into, spec_edits};
use crate::git::{changed_since, git_output, touched_files};
use crate::payload::{tests_or_default, Attributes, Doc, Finding as Wire, Payload};
use crate::range::{self, Inputs, Rule};
use crate::targets::Targets;
use crate::world::{in_flight, Site, World};
use lazyspec::engine::store::Store;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::Path;

#[derive(Debug, Default, Clone, Copy)]
pub struct Require {
    pub intent: bool,
    pub evidence: bool,
    pub clean: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Suite {
    #[default]
    Full,
    Session,
}

#[derive(Debug, Default)]
pub struct Options<'a> {
    pub base: Option<&'a str>,
    pub trunk: Option<&'a str>,
    pub session: Option<&'a str>,
    pub require: Require,
    pub suite: Suite,
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
    #[serde(skip)]
    pub key: Option<String>,
}

impl Finding {
    pub fn keyed(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

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
            key: None,
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
            key: None,
        }
    }

    pub fn bare_warning(message: impl Into<String>) -> Self {
        Self {
            severity: "warning",
            ..Self::bare_error(message)
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

pub struct Span {
    pub base: String,
    pub files: Vec<String>,
}

pub fn resolve_span(
    root: &Path,
    options: &Options,
    payload: Option<&Payload>,
) -> Result<Span, String> {
    let base = resolve_base(root, options, payload)?;
    let files = changed_since(root, &base);
    Ok(Span { base, files })
}

fn is_validate(payload: Option<&Payload>) -> bool {
    payload.is_some_and(|p| p.transition.is_none())
}

pub fn run(
    store: &Store,
    root: &Path,
    options: &Options,
    payload: Option<&Payload>,
) -> Vec<Finding> {
    let span = (!is_validate(payload)).then(|| resolve_span(root, options, payload));
    run_with(store, root, options, payload, span.as_ref())
}

pub fn run_with(
    store: &Store,
    root: &Path,
    options: &Options,
    payload: Option<&Payload>,
    span: Option<&Result<Span, String>>,
) -> Vec<Finding> {
    let world = World::load(store, payload);
    let mut scope = scope_of(&world, payload);
    let full = options.suite == Suite::Full;
    let mut findings = match span {
        None => Vec::new(),
        Some(Ok(span)) => {
            let found = range_findings(store, root, &world, span, full, payload);
            if !full {
                scope.retain(|doc| touched_by(doc, &span.files));
            }
            found
        }
        Some(Err(message)) => {
            if !full {
                scope.clear();
            }
            vec![Finding::bare_error(message)]
        }
    };
    for doc in &scope {
        if full {
            findings.extend(integrity_findings(root, &world, doc, options.require));
        }
        findings.extend(evidence_findings(root, &world, doc, options.require));
    }
    if full {
        findings.extend(double_strikes(&world, &scope));
    }
    if full && options.require.clean {
        findings.extend(uncommitted(root, &world, &scope));
    }
    findings.sort();
    findings.dedup();
    findings
}

fn range_findings(
    store: &Store,
    root: &Path,
    world: &World,
    span: &Span,
    full: bool,
    payload: Option<&Payload>,
) -> Vec<Finding> {
    let completed = completed_since(root, world, &span.base, &span.files);
    let covered = covered_specs(world, &completed);
    let mut found = coverage(store, root, &covered, &span.files);
    found.extend(evidence_changes(root, world, span, &completed, None));
    if full {
        found.extend(spec_edits(root, world, &span.base, &span.files));
    }
    if let Some(targets) = moved_targets(world, payload) {
        found.retain(|f| f.id.as_ref().is_some_and(|id| targets.contains(id)));
    }
    found
}

fn moved_targets(world: &World, payload: Option<&Payload>) -> Option<BTreeSet<String>> {
    let payload = payload.filter(|p| p.transition.is_some())?;
    let moved = world.doc(&payload.documents.first()?.id)?;
    match moved.doc_type.as_str() {
        "delta" => Some(
            moved
                .fold_targets()
                .into_iter()
                .filter_map(|t| world.spec_named(t))
                .map(|spec| spec.id.clone())
                .collect(),
        ),
        "spec" => Some(BTreeSet::from([moved.id.clone()])),
        _ => None,
    }
}

fn touched_by(doc: &Doc, files: &[String]) -> bool {
    let dir = Path::new(&doc.path).parent().unwrap_or(Path::new(""));
    let governed = glob_set(&doc.governs);
    let governs = doc.doc_type == "spec" && !doc.governs.is_empty();
    files
        .iter()
        .any(|f| Path::new(f).starts_with(dir) || (governs && governed.is_match(f.as_str())))
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

pub fn resolve_base(
    root: &Path,
    options: &Options,
    payload: Option<&Payload>,
) -> Result<String, String> {
    let delta_index = payload
        .filter(|p| p.transition.is_some())
        .and_then(|p| p.documents.first())
        .filter(|d| d.doc_type == "delta")
        .map(|d| Path::new(&d.path));
    let range = range::resolve(
        root,
        &Inputs {
            explicit: options.base,
            trunk: options.trunk,
            session: options.session,
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

fn within_one_area(per_file: &[BTreeSet<String>]) -> bool {
    let mut governed = per_file.iter().filter(|ids| !ids.is_empty());
    let Some(first) = governed.next() else {
        return true;
    };
    !governed
        .fold(first.clone(), |common, ids| &common & ids)
        .is_empty()
}

pub fn covered_specs(world: &World, completed: &[&Doc]) -> BTreeSet<String> {
    world
        .in_flight()
        .into_iter()
        .chain(completed.iter().copied())
        .flat_map(Doc::fold_targets)
        .filter_map(|target| world.spec_named(target))
        .map(|spec| spec.id.clone())
        .collect()
}

pub fn coverage(
    store: &Store,
    root: &Path,
    covered: &BTreeSet<String>,
    files: &[String],
) -> Vec<Finding> {
    let mut touched: BTreeMap<String, (String, Vec<&str>)> = BTreeMap::new();
    let mut per_file: Vec<BTreeSet<String>> = Vec::new();
    for file in files {
        let specs: Vec<_> = store
            .governing(&root.join(file))
            .into_iter()
            .filter(|(meta, _)| meta.doc_type.to_string() == "spec")
            .collect();
        per_file.push(specs.iter().map(|(meta, _)| meta.id.clone()).collect());
        for (meta, _) in specs {
            touched
                .entry(meta.id.clone())
                .or_insert_with(|| (meta.path.to_string_lossy().into_owned(), Vec::new()))
                .1
                .push(file);
        }
    }
    if within_one_area(&per_file) {
        return Vec::new();
    }
    let areas = touched.keys().cloned().collect::<Vec<_>>().join(", ");
    touched
        .iter()
        .filter(|(id, _)| !covered.contains(*id))
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
                key: Some(format!("check1:{id}")),
            }
        })
        .collect()
}

fn struck_for(world: &World, spec: &Doc, completed: &[&Doc]) -> BTreeSet<String> {
    let counts = |delta: &&Doc| {
        delta.doc_type == "delta"
            && (delta.status == "in-progress"
                || world.is_moving(delta)
                || completed.iter().any(|c| c.id == delta.id))
    };
    world
        .docs()
        .iter()
        .filter(counts)
        .filter(|delta| folds_into(delta, spec))
        .flat_map(|delta| world.sites(delta))
        .filter(|site| site.governing.is_some_and(|g| g.id == spec.id))
        .flat_map(|site| site.claims)
        .filter(|c| c.struck)
        .map(|c| c.id)
        .collect()
}

pub fn evidence_changes(
    root: &Path,
    world: &World,
    span: &Span,
    completed: &[&Doc],
    only: Option<&str>,
) -> Vec<Finding> {
    let base = span.base.as_str();
    let short = &base[..base.len().min(8)];
    let changed_files: HashSet<&str> = span.files.iter().map(String::as_str).collect();
    let mut regions = Regions::new(root, base);
    let mut findings = Vec::new();
    for spec in world.specs() {
        let Some(part) = spec.part(BEHAVIOUR_PART) else {
            continue;
        };
        if only.is_some_and(|file| !part.body.contains(file)) {
            continue;
        }
        let path = spec.part_path(BEHAVIOUR_PART);
        let Some(at_base) = git_output(root, &["show", &format!("{base}:./{path}")]) else {
            continue;
        };
        let struck = struck_for(world, spec, completed);
        let now = parse(&part.body).claims;
        for old in parse(&at_base).claims.iter().filter(|c| !c.struck) {
            let Some(live) = now.iter().find(|c| c.id == old.id && !c.struck) else {
                continue;
            };
            if struck.contains(&old.id) {
                continue;
            }
            for r in &old.receipts {
                if only.is_some_and(|file| clean(&r.path) != file) {
                    continue;
                }
                if !changed_files.contains(clean(&r.path)) {
                    continue;
                }
                if !regions.changed(&r.path, r.symbol.as_deref()) {
                    continue;
                }
                let line = live
                    .receipts
                    .iter()
                    .find(|n| n.path == r.path && n.symbol == r.symbol)
                    .map_or(live.line, |n| n.line);
                let name = ref_name(&r.path, r.symbol.as_deref());
                let message = match old.observed {
                    true => format!(
                        "{}: characterised behaviour changed; confirm the change is intended",
                        old.id
                    ),
                    false => format!(
                        "{}: evidence {name} changed since {short}; strike {} in a delta and add a new id, or revert the change",
                        old.id, old.id
                    ),
                };
                findings.push(
                    Finding::error(spec, &path, Some(line), message)
                        .in_part(BEHAVIOUR_PART)
                        .keyed(format!("check2:{}:{name}", old.id)),
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

pub fn integrity_findings(root: &Path, world: &World, doc: &Doc, require: Require) -> Vec<Finding> {
    let sites = world.sites(doc);
    let mut findings = Vec::new();
    findings.extend(index_claims(root, doc));
    if doc.doc_type == "spec" {
        findings.extend(assumed_in_spec(world, doc));
        findings.extend(
            invalid_glob_findings(doc)
                .into_iter()
                .map(|w| from_wire(doc, w)),
        );
    } else {
        findings.extend(delta_structure(world, doc, require));
        findings.extend(fold_findings(doc, &sites));
    }
    findings.extend(sites.iter().flat_map(legacy_lines));
    findings
}

pub fn evidence_findings(root: &Path, world: &World, doc: &Doc, require: Require) -> Vec<Finding> {
    world
        .sites(doc)
        .iter()
        .flat_map(|site| claims_have_evidence(root, site, require))
        .collect()
}

fn assumed_in_spec(world: &World, spec: &Doc) -> Vec<Finding> {
    world
        .sites(spec)
        .iter()
        .flat_map(|site| {
            site.claims
                .iter()
                .filter(|c| c.assumed && !c.struck)
                .map(|c| {
                    Finding::error(
                        spec,
                        &site.path,
                        Some(c.line),
                        format!(
                            "{}: [assumed] claim in a spec; a spec records what is known. Resolve it in a delta or move it to open questions",
                            c.id
                        ),
                    )
                    .in_part(site.part)
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn invalid_glob_findings(spec: &Doc) -> Vec<Wire> {
    let governs = invalid_globs(&spec.governs)
        .into_iter()
        .map(|g| ("governs", g));
    let tests = invalid_globs(&spec.tests_globs())
        .into_iter()
        .map(|g| ("tests", g));
    governs
        .chain(tests)
        .map(|(key, (glob, err))| {
            Wire::error(
                &spec.id,
                None,
                format!("{key} glob `{glob}` is invalid: {err}"),
            )
        })
        .collect()
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
        key: None,
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
    wires.extend(targets.missing_parts(delta, delta.status != "draft" || require.intent));
    wires.extend(targets.orphan_parts(&parts, delta));
    let mut findings: Vec<Finding> = wires.into_iter().map(|w| from_wire(delta, w)).collect();
    findings.extend(targets.resolved.iter().flat_map(|spec| {
        invalid_glob_findings(spec)
            .into_iter()
            .map(|w| from_wire(spec, w))
    }));
    if require.intent {
        findings.extend(readiness(delta));
    }
    findings
}

fn readiness(delta: &Doc) -> Vec<Finding> {
    if has_intent(&delta.body) {
        return Vec::new();
    }
    vec![Finding::error(
        delta,
        &delta.path,
        None,
        "missing or empty `## Intent` section; state why the change is needed",
    )]
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

pub const UNRESOLVED_ASSUMPTION: &str = "assumption unresolved; accept it as a claim, mark it [observed] with a test, delete it, or move it to the spec's open questions";

pub fn legacy_lines(site: &Site) -> Vec<Finding> {
    let legacy = site
        .claims
        .iter()
        .flat_map(|c| &c.legacy)
        .map(|l| Finding::error(site.doc, &site.path, Some(l.line), l.message()));
    let conflicts = site.claims.iter().filter(|c| c.marker_conflict).map(|c| {
        Finding::error(
            site.doc,
            &site.path,
            Some(c.line),
            format!(
                "{}: [intent], [observed] and [assumed] are mutually exclusive; keep one",
                c.id
            ),
        )
    });
    legacy
        .chain(conflicts)
        .map(|f| f.in_part(site.part))
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
        if claim.assumed && require.evidence && site.doc.doc_type != "spec" {
            findings.push(finding(
                true,
                claim.line,
                format!("{}: {UNRESOLVED_ASSUMPTION}", claim.id),
            ));
        }
        for r in &claim.receipts {
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
        let unevidenced =
            !claim.intent && !claim.assumed && claim.receipts.is_empty() && claim.legacy.is_empty();
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

pub fn double_strikes(world: &World, scope: &[&Doc]) -> Vec<Finding> {
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
                    c.receipts
                        .iter()
                        .map(|r| clean(&r.path).to_string())
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
