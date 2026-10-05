use crate::claims::{part_for, Claim};
use crate::payload::{BehaviourPart, Doc, Finding};

pub struct Targets<'a> {
    pub declared: Vec<&'a str>,
    pub resolved: Vec<&'a Doc>,
}

impl<'a> Targets<'a> {
    pub fn of(delta: &'a Doc, specs: &[&'a Doc]) -> Self {
        let declared = delta.fold_targets();
        let resolved = declared
            .iter()
            .filter_map(|t| specs.iter().copied().find(|s| s.is_named(t)))
            .collect();
        Self { declared, resolved }
    }

    pub fn unresolved(&self) -> Vec<&'a str> {
        self.declared
            .iter()
            .copied()
            .filter(|t| !self.resolved.iter().any(|s| s.is_named(t)))
            .collect()
    }

    pub fn names(&self, target: &str) -> bool {
        self.declared.contains(&target) || self.resolved.iter().any(|s| s.is_named(target))
    }

    pub fn spec_named(&self, target: &str) -> Option<&'a Doc> {
        self.resolved.iter().copied().find(|s| s.is_named(target))
    }

    pub fn governs_of(&self, part: &BehaviourPart) -> Vec<String> {
        part.target
            .and_then(|t| self.spec_named(t))
            .map(|s| s.governs.clone())
            .unwrap_or_default()
    }

    pub fn claims_for<'c>(&self, spec: &Doc, parts: &'c [BehaviourPart]) -> Vec<&'c Claim> {
        parts
            .iter()
            .filter(|p| p.target.is_some_and(|t| spec.is_named(t)))
            .flat_map(|p| &p.claims)
            .collect()
    }

    pub fn missing_parts(&self, delta: &Doc, blocking: bool) -> Vec<Finding> {
        self.resolved
            .iter()
            .map(|spec| spec.id.as_str())
            .filter(|id| delta.part(&part_for(id)).is_none())
            .map(|id| {
                let message = format!("fold target {id} has no `{}` part", part_for(id));
                match blocking {
                    true => Finding::error(&delta.id, None, message),
                    false => Finding::warning(&delta.id, None, message),
                }
            })
            .collect()
    }

    pub fn orphan_parts(&self, parts: &[BehaviourPart], delta: &Doc) -> Vec<Finding> {
        parts
            .iter()
            .filter(|p| !p.target.is_some_and(|t| self.names(t)))
            .map(|p| {
                Finding::error(
                    &delta.id,
                    None,
                    format!(
                        "part {} does not name one of this delta's folds-into targets (expected behaviour.<target-id>); its claims would not fold",
                        p.name
                    ),
                )
                .in_part(p.name)
            })
            .collect()
    }
}

pub fn specs_in<'a>(docs: impl IntoIterator<Item = &'a Doc>) -> Vec<&'a Doc> {
    docs.into_iter().filter(|d| d.doc_type == "spec").collect()
}
