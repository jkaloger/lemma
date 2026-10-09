use crate::build::git_run;
use crate::claims;
use crate::git::git_output;
use crate::payload::{Doc, CHANGES, CLAIMS_PART};
use crate::prompts::{self, Prompt};
use crate::tasks::Task;
use crate::{context, runner, schema};
use lazyspec::engine::store::Store;
use serde::Deserialize;
use std::path::Path;

const FIXED_BASES: [&str; 3] = ["task", "design", "convention"];

#[derive(Deserialize)]
pub struct Finding {
    pub text: String,
    pub basis: String,
}

#[derive(Deserialize)]
pub struct Verdict {
    pub stop: String,
    pub findings: Vec<Finding>,
}

fn related_ids(delta: &Doc) -> impl Iterator<Item = String> + '_ {
    delta
        .related
        .iter()
        .filter(|r| r.rel_type != CHANGES)
        .map(|r| r.target.clone())
}

fn bases(delta: &Doc, dictums: &str) -> Vec<String> {
    let claim_ids = claims::parse(delta.part_body(CLAIMS_PART))
        .claims
        .into_iter()
        .filter(|c| !c.struck)
        .map(|c| c.id);
    let rule_ids = claims::parse(dictums).claims.into_iter().map(|c| c.id);
    let mut found: Vec<String> = FIXED_BASES.iter().map(|b| b.to_string()).collect();
    for id in claim_ids.chain(rule_ids).chain(related_ids(delta)) {
        if !found.contains(&id) {
            found.push(id);
        }
    }
    found
}

fn staged_task_diff(root: &Path) -> Result<String, String> {
    git_run(root, &["add", "-A"])?;
    git_output(root, &["diff", "--cached", "HEAD"]).ok_or("cannot diff the staged task".to_string())
}

fn parse(reply: serde_json::Value, bases: &[String]) -> Result<Verdict, String> {
    let verdict: Verdict =
        serde_json::from_value(reply).map_err(|e| format!("reviewer reply: {e}"))?;
    match verdict.findings.iter().find(|f| !bases.contains(&f.basis)) {
        Some(f) => Err(format!("reviewer reply: unknown basis `{}`", f.basis)),
        None => Ok(verdict),
    }
}

pub fn review(
    root: &Path,
    store: &Store,
    delta: &Doc,
    task: &Task,
    config: &runner::Config,
) -> Result<Verdict, String> {
    let mut values = context::assemble(store, root, delta);
    let bases = bases(delta, &values["dictums"]);
    values.insert("task", context::task_text(delta, task));
    values.insert("task_diff", staged_task_diff(root)?);
    let base = context::diff_base(root, delta);
    values.insert(
        "delta_diff",
        git_output(root, &["diff", &base, "HEAD"]).unwrap_or_default(),
    );
    let prompt = prompts::rendered(Prompt::ReviewTask, root, &values)?;
    let env = [("LEMMA_DELTA", delta.id.as_str()), ("LEMMA_TASK", &task.id)];
    let schema = schema::review_task(&bases);
    let reply = runner::tracked(
        &format!("{} review", task.id),
        &config.reviewer,
        &prompt,
        root,
        &env,
        config.timeout,
        Some(&schema),
    )?;
    parse(reply.json()?, &bases)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn delta(claims: &str) -> Doc {
        serde_json::from_value(json!({
            "id": "DELTA-001", "path": "d/index.md", "title": "t", "type": "delta",
            "status": "in-progress", "body": "", "content_hash": "h",
            "related": [
                {"type": "changes", "target": "SPEC-001"},
                {"type": "affects", "target": "DECISION-002"},
            ],
            "parts": [{"name": "claims", "path": "d/claims.md", "body": claims}],
        }))
        .unwrap()
    }

    #[test]
    fn bases_are_the_fixed_three_then_claims_dictum_rules_and_related_docs() {
        let delta = delta("- [AC-1] one\n- ~~[AC-2]~~ struck\n");

        let found = bases(&delta, "## DICTUM-001 T\n\n- [TEST-1] [intent] x\n");

        assert_eq!(
            found,
            [
                "task",
                "design",
                "convention",
                "AC-1",
                "TEST-1",
                "DECISION-002"
            ]
        );
    }
}
