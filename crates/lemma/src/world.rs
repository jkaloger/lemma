use crate::claims::{Claim, BEHAVIOUR_PART};
use crate::payload::{Doc, Payload};
use crate::project::docs_from_store;
use lazyspec::engine::store::Store;

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

impl World {
    pub fn load(store: &Store, payload: Option<&Payload>) -> Self {
        let mut docs = docs_from_store(store);
        let Some(payload) = payload else {
            return Self { docs, moving: None };
        };
        let moved = payload.documents.first().map(|d| d.id.as_str());
        let moving = payload.transition.as_ref().and(moved).map(str::to_string);
        for supplied in payload.documents.iter().chain(&payload.context) {
            if !matches!(supplied.doc_type.as_str(), "spec" | "delta") {
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
            .filter(|d| d.doc_type == "spec" || in_flight(d))
            .flat_map(|d| self.sites(d))
            .collect()
    }
}
