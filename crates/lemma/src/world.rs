use crate::claims::{Claim, BEHAVIOUR_PART};
use crate::git::git_output;
use crate::payload::{Doc, Payload};
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
    pub governing: Option<&'a Doc>,
}

pub fn in_flight(doc: &Doc) -> bool {
    doc.doc_type == "delta" && matches!(doc.status.as_str(), "draft" | "in-progress")
}

pub fn folds_into(delta: &Doc, spec: &Doc) -> bool {
    delta.fold_targets().iter().any(|t| spec.is_named(t))
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

    pub fn sites<'a>(&'a self, doc: &'a Doc) -> Vec<Site<'a>> {
        if doc.doc_type == "dictum" {
            return vec![Site {
                doc,
                part: "",
                path: doc.path.clone(),
                claims: crate::claims::parse(&doc.body).claims,
                governing: Some(doc),
            }];
        }
        if doc.doc_type == "spec" {
            return doc
                .part(BEHAVIOUR_PART)
                .map(|part| Site {
                    doc,
                    part: BEHAVIOUR_PART,
                    path: doc.part_path(BEHAVIOUR_PART),
                    claims: crate::claims::parse(&part.body).claims,
                    governing: Some(doc),
                })
                .into_iter()
                .collect();
        }
        doc.behaviour_parts()
            .into_iter()
            .map(|part| Site {
                doc,
                part: part.name,
                path: doc.part_path(part.name),
                claims: part.claims,
                governing: part.target.and_then(|t| self.spec_named(t)),
            })
            .collect()
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
