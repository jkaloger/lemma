use crate::claims::{Claim, Receipt};
use crate::evidence::clean;
use crate::git::git_output;
use crate::payload::{tests_or_default, Attributes, Doc, Payload, CLAIMS_PART};
use crate::project::docs_from_store;
use lazyspec::engine::doc_json::doc_to_json;
use lazyspec::engine::document::DocMeta;
use lazyspec::engine::store::Store;
use std::collections::BTreeMap;
use std::path::Path;

pub struct World {
    docs: Vec<Doc>,
}

/// One ref of a claim in a delta that had landed before the range.
pub struct Landed<'a> {
    pub delta: &'a Doc,
    pub claim: String,
    pub receipt: Receipt,
}

pub struct Site<'a> {
    pub doc: &'a Doc,
    pub part: &'a str,
    pub path: String,
    pub claims: Vec<Claim>,
    pub tests: Vec<String>,
}

pub fn in_flight(doc: &Doc) -> bool {
    doc.doc_type == "delta" && matches!(doc.status.as_str(), "draft" | "in-progress")
}

pub fn doc_at(root: &Path, rev: &str, path: &str) -> Option<Doc> {
    let text = git_output(root, &["show", &format!("{rev}:./{path}")])?;
    let meta = DocMeta::parse(&text).ok()?;
    serde_json::from_value(doc_to_json(&meta)).ok()
}

pub fn lints_on_edit(doc: &Doc) -> bool {
    matches!(doc.doc_type.as_str(), "spec" | "dictum") || in_flight(doc)
}

impl World {
    pub fn load(store: &Store, payload: Option<&Payload>) -> Self {
        let mut docs = docs_from_store(store);
        let Some(payload) = payload else {
            return Self { docs };
        };
        let moved = payload.documents.first().map(|d| d.id.as_str());
        for supplied in payload.documents.iter().chain(&payload.context) {
            if !matches!(supplied.doc_type.as_str(), "spec" | "delta" | "dictum") {
                continue;
            }
            let mut doc = supplied.clone();
            if let (Some(transition), true) = (&payload.transition, Some(doc.id.as_str()) == moved)
            {
                doc.status = transition.from.clone();
            }
            docs.retain(|d| d.id != doc.id);
            docs.push(doc);
        }
        Self { docs }
    }

    pub fn docs(&self) -> &[Doc] {
        &self.docs
    }

    pub fn in_flight(&self) -> Vec<&Doc> {
        self.docs.iter().filter(|d| in_flight(d)).collect()
    }

    pub fn doc(&self, id: &str) -> Option<&Doc> {
        self.docs.iter().find(|d| d.id == id)
    }

    pub fn spec_named(&self, name: &str) -> Option<&Doc> {
        self.docs
            .iter()
            .find(|d| d.doc_type == "spec" && d.is_named(name))
    }

    pub fn changed_specs(&self, delta: &Doc) -> Vec<&Doc> {
        delta
            .changed_specs()
            .into_iter()
            .filter_map(|t| self.spec_named(t))
            .collect()
    }

    /// The `tests` globs a delta's refs must match: those of every spec it changes.
    pub fn delta_tests(&self, delta: &Doc) -> Vec<String> {
        let mut globs: Vec<String> = self
            .changed_specs(delta)
            .into_iter()
            .flat_map(Doc::tests_globs)
            .collect();
        globs.dedup();
        match globs.is_empty() {
            true => tests_or_default(&Attributes::default()),
            false => globs,
        }
    }

    pub fn sites<'a>(&'a self, doc: &'a Doc) -> Vec<Site<'a>> {
        let site = |part: &'a str, body: &str, tests| Site {
            doc,
            part,
            path: match part {
                "" => doc.path.clone(),
                name => doc.part_path(name),
            },
            claims: crate::claims::parse(body).claims,
            tests,
        };
        match doc.doc_type.as_str() {
            "dictum" => vec![site("", &doc.body, doc.tests_globs())],
            "delta" => vec![site(
                CLAIMS_PART,
                doc.part_body(CLAIMS_PART),
                self.delta_tests(doc),
            )],
            _ => Vec::new(),
        }
    }

    /// The refs of every complete delta's live claims, by test file, leaving out deltas
    /// that completed within the range: their tests are the range's own work.
    pub fn landed_by_ref(&self, completed_in_range: &[&Doc]) -> BTreeMap<String, Vec<Landed<'_>>> {
        let mut by_file: BTreeMap<String, Vec<Landed>> = BTreeMap::new();
        let landed = self.docs.iter().filter(|d| {
            d.doc_type == "delta"
                && d.status == "complete"
                && completed_in_range.iter().all(|c| c.id != d.id)
        });
        for delta in landed {
            for site in self.sites(delta) {
                for claim in site.claims.into_iter().filter(|c| !c.struck) {
                    for receipt in claim.receipts {
                        by_file
                            .entry(clean(&receipt.path).to_string())
                            .or_default()
                            .push(Landed {
                                delta,
                                claim: claim.id.clone(),
                                receipt,
                            });
                    }
                }
            }
        }
        by_file
    }

    /// Deltas that are `complete` now, whose `index.md` changed in the range and was not
    /// `complete` at the base.
    pub fn completed_since(&self, root: &Path, base: &str, files: &[String]) -> Vec<&Doc> {
        let mut completed: Vec<&Doc> = self
            .docs
            .iter()
            .filter(|d| d.doc_type == "delta" && d.status == "complete")
            .filter(|d| files.contains(&d.path))
            .filter(|d| doc_at(root, base, &d.path).is_none_or(|was| was.status != "complete"))
            .collect();
        completed.sort_by(|a, b| a.id.cmp(&b.id));
        completed
    }
}
