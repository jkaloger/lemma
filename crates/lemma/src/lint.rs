use crate::claims::{parse, Claim, BEHAVIOUR_PART};
use crate::evidence::{clean, glob_set, invalid_globs, resolve, RefStatus, Regions};
use crate::git::{changed_since, git_output};
use crate::payload::{Doc, Finding as Wire, Payload};
use crate::range::{self, Inputs, Rule};
use crate::shape::{self, Gate};
use crate::world::{changes, in_flight, Site, World};
use lazyspec::engine::store::Store;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::Path;

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
    pub delta: Option<&'a str>,
    pub gate: Option<Gate>,
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
        self.part = Some(part.to_string()).filter(|p| !p.is_empty());
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
    let mut scope = scope_of(&world, options, payload);
    let full = options.suite == Suite::Full;
    let mut findings = match span {
        None => Vec::new(),
        Some(Ok(span)) => {
            let found = range_findings(store, root, &world, span, payload);
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
            findings.extend(integrity_findings(root, &world, doc, options.gate));
        }
        findings.extend(evidence_findings(root, &world, doc, options.gate));
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
    payload: Option<&Payload>,
) -> Vec<Finding> {
    let completed = world.completed_since(root, &span.base, &span.files);
    let covered = covered_specs(world, &completed);
    let mut found = coverage(store, root, &covered, &span.files);
    found.extend(evidence_changes(root, world, span, &completed, None));
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
            world
                .changed_specs(moved)
                .into_iter()
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
    let governs = matches!(doc.doc_type.as_str(), "spec" | "dictum") && !doc.governs.is_empty();
    files
        .iter()
        .any(|f| Path::new(f).starts_with(dir) || (governs && governed.is_match(f.as_str())))
}

/// The documents linted one by one: every spec and dictum, plus the named delta, or every
/// in-flight delta for a session. A hook lints the payload's documents instead.
fn scope_of<'a>(world: &'a World, options: &Options, payload: Option<&Payload>) -> Vec<&'a Doc> {
    let linted = |d: &&Doc| matches!(d.doc_type.as_str(), "spec" | "dictum" | "delta");
    if let Some(payload) = payload {
        return payload
            .documents
            .iter()
            .filter_map(|d| world.doc(&d.id))
            .filter(linted)
            .collect();
    }
    let named = |d: &&Doc| options.delta.is_some_and(|name| d.is_named(name));
    let session = options.suite == Suite::Session;
    world
        .docs()
        .iter()
        .filter(|d| d.doc_type != "delta" || named(d) || (session && in_flight(d)))
        .filter(linted)
        .collect()
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
        .flat_map(Doc::changed_specs)
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
                    "changes to {}{tail} touch more than one governed area ({areas}) and no in-flight delta changes {id}; create a delta that changes {id}",
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
        .filter(|delta| changes(delta, spec))
        .flat_map(|delta| world.sites(delta))
        .flat_map(|site| site.claims)
        .filter(|c| c.struck)
        .map(|c| c.id)
        .collect()
}

struct ChangeSite<'a> {
    doc: &'a Doc,
    part: &'static str,
    path: String,
    now: Vec<Claim>,
    hand_edited: bool,
}

fn change_sites<'a>(root: &Path, world: &'a World) -> Vec<ChangeSite<'a>> {
    let specs = world.specs().into_iter().filter_map(|spec| {
        let part = spec.part(BEHAVIOUR_PART)?;
        Some(ChangeSite {
            doc: spec,
            part: BEHAVIOUR_PART,
            path: spec.part_path(BEHAVIOUR_PART),
            now: parse(&part.body).claims,
            hand_edited: false,
        })
    });
    let dictums = world
        .docs()
        .iter()
        .filter(|d| d.doc_type == "dictum")
        .map(|dictum| ChangeSite {
            doc: dictum,
            part: "",
            path: dictum.path.clone(),
            now: dictum.index_claims(root),
            hand_edited: true,
        });
    specs.chain(dictums).collect()
}

fn dictum_retires(old: &Claim, base: &[Claim], now: &[Claim]) -> bool {
    let struck = now.iter().any(|c| c.id == old.id && c.struck);
    let added = now.iter().any(|c| base.iter().all(|b| b.id != c.id));
    struck && added
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
    for site in change_sites(root, world) {
        let spec = site.doc;
        let path = &site.path;
        let mentions = |file: &str| {
            site.now
                .iter()
                .any(|c| c.receipts.iter().any(|r| clean(&r.path) == file))
        };
        if only.is_some_and(|file| !mentions(file)) {
            continue;
        }
        let Some(at_base) = git_output(root, &["show", &format!("{base}:./{path}")]) else {
            continue;
        };
        let struck = struck_for(world, spec, completed);
        let base_claims = parse(&at_base).claims;
        for old in base_claims.iter().filter(|c| !c.struck) {
            let live = site.now.iter().find(|c| c.id == old.id && !c.struck);
            let excused = match site.hand_edited {
                true => dictum_retires(old, &base_claims, &site.now),
                false => live.is_none() || struck.contains(&old.id),
            };
            if excused {
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
                    .and_then(|l| {
                        l.receipts
                            .iter()
                            .find(|n| n.path == r.path && n.symbol == r.symbol)
                            .map(|n| n.line)
                    })
                    .unwrap_or_else(|| live.map_or(old.line, |c| c.line));
                let name = ref_name(&r.path, r.symbol.as_deref());
                let how = match site.hand_edited {
                    true => format!("strike {} in the dictum and add a new id", old.id),
                    false => format!("strike {} in a delta and add a new id", old.id),
                };
                let message = format!(
                    "{}: evidence {name} changed since {short}; {how}, or revert the change",
                    old.id
                );
                let finding = Finding::error(spec, path, Some(line), message)
                    .in_part(site.part)
                    .keyed(format!("check2:{}:{name}", old.id));
                findings.push(finding);
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

fn located_sites<'a>(root: &Path, world: &'a World, doc: &'a Doc) -> Vec<Site<'a>> {
    let mut sites = world.sites(doc);
    if doc.doc_type == "dictum" {
        let offset = doc.body_line_offset(root);
        for claim in sites.iter_mut().flat_map(|s| s.claims.iter_mut()) {
            claim.line += offset;
        }
    }
    sites
}

pub fn integrity_findings(
    root: &Path,
    world: &World,
    doc: &Doc,
    gate: Option<Gate>,
) -> Vec<Finding> {
    match doc.doc_type.as_str() {
        "spec" | "dictum" => invalid_glob_findings(doc)
            .into_iter()
            .map(|w| from_wire(doc, w))
            .collect(),
        _ => world
            .changed_specs(doc)
            .into_iter()
            .flat_map(|spec| {
                invalid_glob_findings(spec)
                    .into_iter()
                    .map(|w| from_wire(spec, w))
            })
            .chain(shape::findings(root, doc, gate))
            .collect(),
    }
}

pub fn evidence_findings(
    root: &Path,
    world: &World,
    doc: &Doc,
    gate: Option<Gate>,
) -> Vec<Finding> {
    located_sites(root, world, doc)
        .iter()
        .flat_map(|site| claims_have_evidence(root, site, gate))
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

/// A ref that is written must resolve. A claim without one is an error on a delta at the
/// `complete` gate and a warning on a spec or dictum; on a delta short of the gate the
/// build loop has not reached it yet.
fn claims_have_evidence(root: &Path, site: &Site, gate: Option<Gate>) -> Vec<Finding> {
    let is_delta = site.doc.doc_type == "delta";
    let finding = |is_error, line, message: String| {
        Finding::graded(is_error, site.doc, &site.path, Some(line), message).in_part(site.part)
    };
    let mut findings = Vec::new();
    for claim in site.claims.iter().filter(|c| !c.struck) {
        for r in &claim.receipts {
            let name = ref_name(&r.path, r.symbol.as_deref());
            match resolve(root, &r.path, r.symbol.as_deref(), &site.tests) {
                RefStatus::Ok => {}
                RefStatus::Unresolved(why) => findings.push(finding(
                    true,
                    r.line,
                    format!(
                        "{}: ref {name} does not resolve: {why}; fix the ref (see lemma refs {})",
                        claim.id,
                        cite(site.doc, &claim.id)
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
        if claim.intent || !claim.receipts.is_empty() {
            continue;
        }
        let complete = gate == Some(Gate::Complete);
        if is_delta && !complete {
            continue;
        }
        findings.push(finding(
            complete,
            claim.line,
            format!(
                "{}: claim has no `@ref`; add `@ref <path>#<symbol>` to a test or mark it [intent]",
                claim.id
            ),
        ));
    }
    findings
}

/// How a claim is named outside its document: `DELTA-004/AC-2` for a delta, the bare id
/// for a dictum, whose ids are unique across the convention.
pub fn cite(doc: &Doc, claim: &str) -> String {
    match doc.doc_type.as_str() {
        "delta" => format!("{}/{claim}", doc.id),
        _ => claim.to_string(),
    }
}
