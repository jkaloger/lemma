use crate::claims::Claim;
use crate::payload::Doc;

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

    pub fn is_multi(&self) -> bool {
        self.declared.len() > 1
    }

    pub fn names(&self, partition: &str) -> bool {
        self.declared.contains(&partition) || self.resolved.iter().any(|s| s.is_named(partition))
    }

    pub fn spec_for(&self, claim: &Claim) -> Option<&'a Doc> {
        match &claim.partition {
            Some(name) => self.resolved.iter().copied().find(|s| s.is_named(name)),
            None if !self.is_multi() => self.resolved.first().copied(),
            None => None,
        }
    }

    pub fn claims_for<'c>(&self, spec: &Doc, claims: &'c [Claim]) -> Vec<&'c Claim> {
        claims
            .iter()
            .filter(|c| match &c.partition {
                Some(name) => spec.is_named(name),
                None => !self.is_multi(),
            })
            .collect()
    }
}

pub fn specs_in<'a>(docs: impl IntoIterator<Item = &'a Doc>) -> Vec<&'a Doc> {
    docs.into_iter().filter(|d| d.doc_type == "spec").collect()
}
