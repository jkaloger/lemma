use crate::claims::{Claim, BEHAVIOUR_PART};
use crate::git::git_output;
use crate::payload::{tests_or_default, Attributes, Doc, Payload, CLAIMS_PART};
use crate::project::docs_from_store;
use lazyspec::engine::doc_json::doc_to_json;
use lazyspec::engine::document::DocMeta;
use lazyspec::engine::store::Store;
use std::path::Path;

pub struct World {
    docs: Vec<Doc>,
    moving: Option<String>,
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

pub fn changes(delta: &Doc, spec: &Doc) -> bool {
    delta.changed_specs().iter().any(|t| spec.is_named(t))
}

pub fn doc_at(root: &Path, rev: &str, path: &str) -> Option<Doc> {
    let text = git_output(root, &["show", &format!("{rev}:./{path}")])?;
    let meta = DocMeta::parse(&text).ok()?;
    serde_json::from_value(doc_to_json(&meta)).ok()
}

pub fn carries_claims(doc: &Doc) -> bool {
    matches!(doc.doc_type.as_str(), "spec" | "dictum") || in_flight(doc)
}

impl World {
    pub fn load(store: &Store, payload: Option<&Payload>) -> Self {
        let mut docs = docs_from_store(store);
        let Some(payload) = payload else {
            return Self { docs, moving: None };
        };
        let moved = payload.documents.first().map(|d| d.id.as_str());
        let moving = payload.transition.as_ref().and(moved).map(str::to_string);
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
        Self { docs, moving }
    }

    pub fn is_moving(&self, doc: &Doc) -> bool {
        self.moving.as_deref() == Some(doc.id.as_str())
    }

    pub fn docs(&self) -> &[Doc] {
        &self.docs
    }

    pub fn specs(&self) -> Vec<&Doc> {
        self.docs.iter().filter(|d| d.doc_type == "spec").collect()
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
            "spec" => doc
                .part(BEHAVIOUR_PART)
                .map(|part| site(BEHAVIOUR_PART, &part.body, doc.tests_globs()))
                .into_iter()
                .collect(),
            "delta" => vec![site(
                CLAIMS_PART,
                doc.part_body(CLAIMS_PART),
                self.delta_tests(doc),
            )],
            _ => Vec::new(),
        }
    }

    pub fn all_sites(&self) -> Vec<Site<'_>> {
        self.docs
            .iter()
            .filter(|d| carries_claims(d))
            .flat_map(|d| self.sites(d))
            .collect()
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
