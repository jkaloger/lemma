use crate::claims::{bullets, has_intent, live_lines, parse, review_lines, Claim};
use crate::lint::Finding;
use crate::payload::{Doc, CLAIMS_PART, DESIGN_PART, INDEX_PART, TASKS_PART};
use crate::tasks;
use regex::Regex;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Gate {
    InProgress,
    Complete,
}

impl Gate {
    pub fn parse(status: &str) -> Result<Self, String> {
        match status {
            "in-progress" => Ok(Gate::InProgress),
            "complete" => Ok(Gate::Complete),
            other => Err(format!(
                "unknown gate `{other}`; use `in-progress` or `complete`"
            )),
        }
    }
}

const PARTS: [&str; 4] = [INDEX_PART, DESIGN_PART, CLAIMS_PART, TASKS_PART];

fn claim_id() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^AC-[1-9][0-9]*$").unwrap())
}

struct Delta<'a> {
    doc: &'a Doc,
    index_offset: usize,
    claims: Vec<Claim>,
    tasks: tasks::Tasks,
}

impl<'a> Delta<'a> {
    fn path(&self, part: &str) -> String {
        match part {
            INDEX_PART => self.doc.path.clone(),
            name => self.doc.part_path(name),
        }
    }

    fn at(&self, part: &str, line: Option<usize>, message: String) -> Finding {
        let line = match part {
            INDEX_PART => line.map(|l| l + self.index_offset),
            _ => line,
        };
        let part_name = if part == INDEX_PART { "" } else { part };
        Finding::error(self.doc, self.path(part), line, message).in_part(part_name)
    }

    fn live_claims(&self) -> impl Iterator<Item = &Claim> {
        self.claims.iter().filter(|c| !c.struck)
    }
}

pub fn findings(root: &Path, doc: &Doc, gate: Option<Gate>) -> Vec<Finding> {
    let delta = Delta {
        doc,
        index_offset: doc.body_line_offset(root),
        claims: parse(doc.part_body(CLAIMS_PART)).claims,
        tasks: tasks::parse(doc.part_body(TASKS_PART)),
    };
    let mut found = always(&delta);
    if gate.is_some() {
        found.extend(at_any_gate(&delta));
    }
    if gate == Some(Gate::Complete) {
        found.extend(at_complete(&delta));
    }
    found
}

fn always(delta: &Delta) -> Vec<Finding> {
    let mut found = Vec::new();
    let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
    for claim in &delta.claims {
        if !claim_id().is_match(&claim.id) {
            found.push(delta.at(
                CLAIMS_PART,
                Some(claim.line),
                format!("claim id {} is not AC-<n>", claim.id),
            ));
        }
        if let Some(first) = seen.insert(&claim.id, claim.line) {
            found.push(delta.at(
                CLAIMS_PART,
                Some(claim.line),
                format!(
                    "{} is used twice (lines {first} and {})",
                    claim.id, claim.line
                ),
            ));
        }
        if claim.proposed && claim.intent {
            found.push(delta.at(
                CLAIMS_PART,
                Some(claim.line),
                format!(
                    "{}: [proposed] and [intent] are exclusive; keep one",
                    claim.id
                ),
            ));
        }
    }
    for bad in &delta.tasks.malformed {
        found.push(delta.at(
            TASKS_PART,
            Some(bad.line),
            "task is not `- [ ] T<n> <text> (settles AC-<n>, ..)` or `- [ ] T<n> <text> (chore)`"
                .to_string(),
        ));
    }
    let mut task_ids: BTreeMap<&str, usize> = BTreeMap::new();
    for task in &delta.tasks.tasks {
        if let Some(first) = task_ids.insert(&task.id, task.line) {
            found.push(delta.at(
                TASKS_PART,
                Some(task.line),
                format!(
                    "{} is used twice (lines {first} and {})",
                    task.id, task.line
                ),
            ));
        }
        for id in task
            .settles
            .iter()
            .filter(|id| !seen.contains_key(id.as_str()))
        {
            found.push(delta.at(
                TASKS_PART,
                Some(task.line),
                format!("{} settles {id}, which claims.md does not have", task.id),
            ));
        }
    }
    for (line, text) in live_lines(delta.doc.part_body(DESIGN_PART)) {
        if !lazyspec::engine::refs::parse_refs(text).is_empty() {
            found.push(delta.at(
                DESIGN_PART,
                Some(line),
                "design.md holds prose; an `@ref` belongs under a claim in claims.md".to_string(),
            ));
        }
    }
    found
}

fn at_any_gate(delta: &Delta) -> Vec<Finding> {
    let mut found = Vec::new();
    if !has_intent(&delta.doc.body) {
        found.push(delta.at(
            INDEX_PART,
            None,
            "missing or empty `## Intent` section; state why the change is needed".to_string(),
        ));
    }
    if delta.live_claims().next().is_none() {
        found.push(delta.at(
            CLAIMS_PART,
            None,
            "no claims; write the behaviour as bullets in claims.md".to_string(),
        ));
    }
    for bullet in bullets(delta.doc.part_body(CLAIMS_PART))
        .iter()
        .filter(|b| b.id.is_none())
    {
        found.push(delta.at(
            CLAIMS_PART,
            Some(bullet.line),
            "claim has no id; run `lemma review` to plan".to_string(),
        ));
    }
    for (line, message) in uncovered(&delta.claims, &delta.tasks) {
        found.push(delta.at(CLAIMS_PART, Some(line), message));
    }
    for part in PARTS {
        for line in review_lines(delta.doc.part_text(part)) {
            found.push(delta.at(
                part,
                Some(line),
                "unresolved review line; act on it, then delete the line".to_string(),
            ));
        }
    }
    for claim in delta.live_claims().filter(|c| c.proposed) {
        found.push(delta.at(
            CLAIMS_PART,
            Some(claim.line),
            format!(
                "{} is [proposed]; delete the tag to accept it or the claim to reject it",
                claim.id
            ),
        ));
    }
    found
}

/// The claim-coverage rule: every live claim is settled by exactly one main task.
pub fn uncovered(claims: &[Claim], tasks: &tasks::Tasks) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    for claim in claims.iter().filter(|c| !c.struck) {
        let settling: Vec<&str> = tasks
            .main()
            .filter(|t| t.settles.contains(&claim.id))
            .map(|t| t.id.as_str())
            .collect();
        let message = match settling.as_slice() {
            [_] => continue,
            [] => format!("{} is settled by no task", claim.id),
            many => format!(
                "{} is settled by {}; exactly one task settles a claim",
                claim.id,
                many.join(", ")
            ),
        };
        found.push((claim.line, message));
    }
    found
}

fn at_complete(delta: &Delta) -> Vec<Finding> {
    delta
        .tasks
        .tasks
        .iter()
        .filter(|t| !t.ticked)
        .map(|t| {
            delta.at(
                TASKS_PART,
                Some(t.line),
                format!("{} is not ticked; run `lemma build`", t.id),
            )
        })
        .collect()
}
