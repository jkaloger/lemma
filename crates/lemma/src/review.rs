use crate::check::Finding;
use crate::claims::{parse, split_markers, Claim};
use crate::git::git_output;
use crate::payload::Doc;
use crate::range::{self, Inputs};
use crate::world::Site;
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::OnceLock;

const NO_PROVENANCE: &str = "provenance: none; accepting this delta establishes this behaviour";

pub fn base_of(root: &Path, delta: &Doc) -> String {
    range::resolve(
        root,
        &Inputs {
            delta_index: Some(Path::new(&delta.path)),
            ..Inputs::default()
        },
    )
    .rev
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Standing {
    Accepted,
    NotAccepted,
    Unverified,
}

impl Standing {
    fn label(self) -> &'static str {
        match self {
            Standing::Accepted => "accepted",
            Standing::NotAccepted => "not accepted at base",
            Standing::Unverified => "unverified",
        }
    }
}

fn id_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^([A-Z][A-Z0-9]*)-[0-9]+$").unwrap())
}

fn frontmatter_value(text: &str, key: &str) -> Option<String> {
    let mut lines = text.lines();
    (lines.next()? == "---").then_some(())?;
    lines
        .take_while(|line| *line != "---")
        .find_map(|line| line.strip_prefix(key)?.strip_prefix(':'))
        .map(|value| value.trim().trim_matches('"').to_string())
}

/// The repository as it was at the range's base.
struct Base<'a> {
    root: &'a Path,
    rev: &'a str,
    files: Vec<String>,
}

impl<'a> Base<'a> {
    fn new(root: &'a Path, rev: &'a str) -> Self {
        let files = git_output(root, &["ls-tree", "-r", "--name-only", rev])
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect();
        Self { root, rev, files }
    }

    fn read(&self, path: &str) -> Option<String> {
        git_output(self.root, &["show", &format!("{}:./{path}", self.rev)])
    }

    fn file_of(&self, id: &str) -> Option<&str> {
        let named = |name: &str| name == id || name.starts_with(&format!("{id}-"));
        self.files
            .iter()
            .map(String::as_str)
            .filter(|path| path.ends_with(".md"))
            .find(|path| {
                let path = Path::new(path);
                let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                let parent = path
                    .parent()
                    .and_then(|p| p.file_name())
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                match stem {
                    "index" => named(parent),
                    _ => named(stem),
                }
            })
    }

    fn standing(&self, citation: &str) -> Standing {
        let Some(token) = citation
            .split_whitespace()
            .next()
            .map(|t| t.trim_end_matches([':', ',', ';', '.']))
        else {
            return Standing::Unverified;
        };
        let Some(caps) = id_pattern().captures(token) else {
            return Standing::Unverified;
        };
        let at_base = self
            .file_of(token)
            .and_then(|path| self.read(path))
            .map(|text| {
                (
                    frontmatter_value(&text, "type"),
                    frontmatter_value(&text, "status"),
                )
            });
        match at_base {
            Some((Some(kind), status)) if kind == "decision" => match status.as_deref() {
                Some("accepted") => Standing::Accepted,
                _ => Standing::NotAccepted,
            },
            Some(_) => Standing::Unverified,
            None if &caps[1] == "DECISION" => Standing::NotAccepted,
            None => Standing::Unverified,
        }
    }

    fn provenance(&self, claim: &Claim) -> Vec<String> {
        if claim.per.is_empty() {
            return vec![NO_PROVENANCE.to_string()];
        }
        claim
            .per
            .iter()
            .map(|c| {
                format!(
                    "provenance: {} ({})",
                    c.text,
                    self.standing(&c.text).label()
                )
            })
            .collect()
    }

    /// Parsed versions of a part across the range, oldest first: the version at the base
    /// (empty when the part did not exist yet), one per commit that touched it, then the
    /// working tree, which the gate reads before anything is committed.
    fn versions(&self, path: &str, live: &[Claim]) -> Vec<Vec<Claim>> {
        let at_base = self.read(path).map(|text| parse(&text).claims);
        let range = format!("{}..HEAD", self.rev);
        let commits = git_output(
            self.root,
            &["log", "--reverse", "--format=%H", &range, "--", path],
        )
        .unwrap_or_default();
        let at_commit = commits.lines().map(|commit| {
            git_output(self.root, &["show", &format!("{commit}:./{path}")])
                .map(|text| parse(&text).claims)
                .unwrap_or_default()
        });
        std::iter::once(at_base.unwrap_or_default())
            .chain(at_commit)
            .chain(std::iter::once(live.to_vec()))
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub id: String,
    pub text: String,
    pub part: String,
    pub path: String,
    pub line: Option<usize>,
    pub provenance: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Resolved {
    pub was: String,
    pub entry: Entry,
    pub replaced: bool,
    pub observed: bool,
}

#[derive(Debug, Clone)]
pub struct Discarded {
    pub id: String,
    pub text: String,
    pub part: String,
    pub path: String,
}

#[derive(Debug, Default)]
pub struct TargetReview {
    pub spec: String,
    pub commitments: Vec<Entry>,
    pub observed: Vec<Entry>,
    pub struck: Vec<Entry>,
    pub resolved: Vec<Resolved>,
    pub discarded: Vec<Discarded>,
    pub unresolved: Vec<Entry>,
}

fn claim_text(claim: &Claim) -> String {
    let first = claim.block.first().map_or("", String::as_str);
    let (_, bare) = split_markers(first);
    bare.split_once(']')
        .map_or("", |(_, rest)| rest.trim_start_matches('~').trim())
        .to_string()
}

fn is_assumed(claim: &Claim) -> bool {
    claim.assumed && !claim.struck
}

/// Pairs a vanished assumption with the id that replaced it, step by step through the
/// versions. Decision 64 has no markup for this, so the pairing is a guess and fails safe:
/// only a lone vanished assumed id against a lone new id with overlapping text pairs up;
/// anything else leaves the assumption discarded, never shown as resolved.
fn replacements(versions: &[Vec<Claim>]) -> BTreeMap<String, String> {
    let mut pairs = BTreeMap::new();
    for window in versions.windows(2) {
        let (before, after) = (&window[0], &window[1]);
        let removed: Vec<&Claim> = before
            .iter()
            .filter(|c| is_assumed(c) && !after.iter().any(|a| a.id == c.id))
            .collect();
        let added: Vec<&Claim> = after
            .iter()
            .filter(|c| !c.struck && !c.assumed && !before.iter().any(|b| b.id == c.id))
            .collect();
        if let ([old], [new]) = (removed.as_slice(), added.as_slice()) {
            if similarity(&claim_text(old), &claim_text(new)) >= REPLACEMENT_SIMILARITY {
                pairs.insert(old.id.clone(), new.id.clone());
            }
        }
    }
    pairs
}

/// Half the words shared is enough to call a reworded claim the same behaviour, while an
/// unrelated claim added in the same step shares almost none.
const REPLACEMENT_SIMILARITY: f64 = 0.5;

fn words(text: &str) -> BTreeSet<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect()
}

fn similarity(a: &str, b: &str) -> f64 {
    let (a, b) = (words(a), words(b));
    let union = a.union(&b).count();
    if union == 0 {
        return 0.0;
    }
    a.intersection(&b).count() as f64 / union as f64
}

fn assumed_ever(versions: &[Vec<Claim>]) -> Vec<(String, String)> {
    let mut seen: Vec<(String, String)> = Vec::new();
    for claim in versions.iter().flatten().filter(|c| is_assumed(c)) {
        match seen.iter_mut().find(|(id, _)| *id == claim.id) {
            Some(entry) => entry.1 = claim_text(claim),
            None => seen.push((claim.id.clone(), claim_text(claim))),
        }
    }
    seen
}

struct SiteView<'a> {
    site: &'a Site<'a>,
    base: &'a Base<'a>,
    spec: &'a Doc,
}

impl SiteView<'_> {
    fn entry(&self, claim: &Claim, provenance: Vec<String>) -> Entry {
        Entry {
            id: claim.id.clone(),
            text: claim_text(claim),
            part: self.site.part.to_string(),
            path: self.site.path.clone(),
            line: Some(claim.line),
            provenance,
        }
    }

    fn live(&self, id: &str) -> Option<&Claim> {
        self.site.claims.iter().find(|c| c.id == id && !c.struck)
    }

    fn settle(&self, review: &mut TargetReview) {
        let existing = self.spec.spec_claims();
        for claim in &self.site.claims {
            match (claim.struck, claim.assumed, claim.observed) {
                (true, _, _) => {
                    let text = existing
                        .iter()
                        .find(|e| e.id == claim.id)
                        .map_or_else(|| claim_text(claim), claim_text);
                    review.struck.push(Entry {
                        text,
                        ..self.entry(claim, Vec::new())
                    });
                }
                (false, true, _) => review.unresolved.push(self.entry(claim, Vec::new())),
                (false, false, true) => review.observed.push(self.entry(claim, Vec::new())),
                (false, false, false) => {}
            }
        }
        let versions = self.base.versions(&self.site.path, &self.site.claims);
        let moved_to = replacements(&versions);
        let mut resolved_ids = Vec::new();
        for (old, text) in assumed_ever(&versions) {
            if self.live(&old).is_some_and(|c| c.assumed) {
                continue;
            }
            let successor = match self.live(&old) {
                Some(claim) => Some((claim, false)),
                None => moved_to
                    .get(&old)
                    .and_then(|new| self.live(new))
                    .filter(|c| !c.assumed)
                    .map(|c| (c, true)),
            };
            let Some((claim, replaced)) = successor else {
                review.discarded.push(Discarded {
                    id: old,
                    text,
                    part: self.site.part.to_string(),
                    path: self.site.path.clone(),
                });
                continue;
            };
            let provenance = match claim.observed {
                true => Vec::new(),
                false => self.base.provenance(claim),
            };
            resolved_ids.push(claim.id.clone());
            review.resolved.push(Resolved {
                was: old,
                entry: self.entry(claim, provenance),
                replaced,
                observed: claim.observed,
            });
        }
        let is_resolution = |id: &str| resolved_ids.iter().any(|r| r == id);
        review.observed.retain(|e| !is_resolution(&e.id));
        for claim in self
            .site
            .claims
            .iter()
            .filter(|c| !c.struck && !c.assumed && !c.observed && !is_resolution(&c.id))
        {
            review
                .commitments
                .push(self.entry(claim, self.base.provenance(claim)));
        }
    }
}

pub fn review(root: &Path, base_rev: &str, spec: &Doc, sites: &[&Site]) -> TargetReview {
    let base = Base::new(root, base_rev);
    let mut review = TargetReview {
        spec: spec.id.clone(),
        ..TargetReview::default()
    };
    for site in sites {
        SiteView {
            site,
            base: &base,
            spec,
        }
        .settle(&mut review);
    }
    review
}

impl TargetReview {
    pub fn is_empty(&self) -> bool {
        self.commitments.is_empty()
            && self.observed.is_empty()
            && self.struck.is_empty()
            && self.resolved.is_empty()
            && self.discarded.is_empty()
            && self.unresolved.is_empty()
    }

    fn groups(&self, only_unresolved: bool) -> Vec<(&'static str, Vec<String>)> {
        let fold_line = |sign: char, e: &Entry| format!("{sign}{} {}", e.id, e.text);
        let with_provenance = |e: &Entry| {
            std::iter::once(fold_line('+', e))
                .chain(e.provenance.iter().map(|p| format!("  {p}")))
                .collect::<Vec<_>>()
        };
        let commitments: Vec<String> = self.commitments.iter().flat_map(with_provenance).collect();
        let observed: Vec<String> = self.observed.iter().map(|e| fold_line('+', e)).collect();
        let struck: Vec<String> = self.struck.iter().map(|e| fold_line('-', e)).collect();
        let resolved: Vec<String> = self
            .resolved
            .iter()
            .flat_map(|r| {
                let head = match (r.replaced, r.observed) {
                    (true, _) => format!("{} → {}", r.was, r.entry.id),
                    (false, true) => format!("{}: was assumed, now observed", r.was),
                    (false, false) => format!("{}: was assumed, now a commitment", r.was),
                };
                std::iter::once(head)
                    .chain(
                        with_provenance(&r.entry)
                            .into_iter()
                            .map(|l| format!("  {l}")),
                    )
                    .collect::<Vec<_>>()
            })
            .collect();
        let discarded: Vec<String> = self
            .discarded
            .iter()
            .map(|d| format!("{}: {}", d.id, d.text))
            .collect();
        let unresolved: Vec<String> = self
            .unresolved
            .iter()
            .map(|e| format!("{} {}", e.id, e.text))
            .collect();
        let all = vec![
            ("New commitments", commitments),
            ("Observed behaviour", observed),
            ("Struck", struck),
            ("Resolved assumptions", resolved),
            ("Discarded assumptions", discarded),
            ("Unresolved assumptions", unresolved),
        ];
        all.into_iter()
            .filter(|(name, lines)| {
                !lines.is_empty() && (!only_unresolved || *name == "Unresolved assumptions")
            })
            .collect()
    }

    /// Empty groups are omitted. When the fold is refused, only the unresolved
    /// assumptions are shown: nothing else would be written.
    pub fn render(&self, only_unresolved: bool) -> Vec<String> {
        let groups = self.groups(only_unresolved);
        if groups.is_empty() {
            return Vec::new();
        }
        std::iter::once(self.spec.clone())
            .chain(groups.into_iter().flat_map(|(name, lines)| {
                std::iter::once(format!("  {name}"))
                    .chain(lines.into_iter().map(|l| format!("    {l}")))
                    .collect::<Vec<_>>()
            }))
            .collect()
    }

    pub fn findings(&self, delta: &Doc) -> Vec<Finding> {
        let warn = |path: &str, part: &str, line: Option<usize>, message: String| {
            Finding::warning(delta, path, line, message).in_part(part)
        };
        let joined = |e: &Entry| e.provenance.join("; ");
        let mut found = Vec::new();
        for e in &self.commitments {
            let message = format!("{}: new commitment; {}", e.id, joined(e));
            found.push(warn(&e.path, &e.part, e.line, message));
        }
        for e in &self.observed {
            let message = format!("{}: observed behaviour", e.id);
            found.push(warn(&e.path, &e.part, e.line, message));
        }
        for r in &self.resolved {
            let who = match r.replaced {
                true => format!("{} → {}", r.was, r.entry.id),
                false => r.was.clone(),
            };
            let tail = match r.entry.provenance.is_empty() {
                true => String::new(),
                false => format!("; {}", joined(&r.entry)),
            };
            let message = format!("{who}: resolved assumption{tail}");
            found.push(warn(&r.entry.path, &r.entry.part, r.entry.line, message));
        }
        for d in &self.discarded {
            let message = format!("{}: discarded assumption", d.id);
            found.push(warn(&d.path, &d.part, None, message));
        }
        found
    }
}
