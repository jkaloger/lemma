use crate::annotate::annotate;
use crate::claims::review_lines;
use crate::payload::{Doc, CLAIMS_PART, DESIGN_PART, INDEX_PART, TASKS_PART};
use crate::plan;
use crate::reviews;
use crate::runner;
use crate::schema;
use crate::tasks;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const PARTS: [&str; 4] = [INDEX_PART, DESIGN_PART, CLAIMS_PART, TASKS_PART];
const ERROR_SEPARATOR: &str = "\nlemma: ";

pub enum Outcome {
    Clean(usize),
    Written(usize),
}

#[derive(Deserialize)]
struct Reply {
    findings: Vec<Finding>,
}

#[derive(Deserialize, Serialize, Clone, PartialEq)]
pub struct Finding {
    pub part: String,
    #[serde(default)]
    pub anchor: String,
    pub text: String,
}

struct Draft {
    part: String,
    original: String,
    current: String,
}

fn parse_findings(reply: serde_json::Value) -> Result<Vec<Finding>, String> {
    let mut reply: Reply =
        serde_json::from_value(reply).map_err(|e| format!("reviewer reply: {e}"))?;
    for finding in &mut reply.findings {
        if !PARTS.contains(&finding.part.as_str()) {
            return Err(format!("reviewer reply: unknown part `{}`", finding.part));
        }
        finding.anchor = finding.anchor.trim().to_string();
        finding.text = finding
            .text
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
    }
    Ok(reply.findings)
}

pub fn part_file(delta: &Doc, part: &str) -> String {
    match part {
        INDEX_PART => delta.path.clone(),
        part => delta.part_path(part),
    }
}

fn apply(root: &Path, delta: &Doc, findings: &[Finding]) -> Result<(Vec<Draft>, usize), String> {
    let mut drafts: Vec<Draft> = Vec::new();
    let mut unknown = Vec::new();
    let mut added = 0;
    for finding in findings {
        let at = match drafts.iter().position(|d| d.part == finding.part) {
            Some(at) => at,
            None => {
                let text = std::fs::read_to_string(root.join(part_file(delta, &finding.part)))
                    .unwrap_or_default();
                drafts.push(Draft {
                    part: finding.part.clone(),
                    original: text.clone(),
                    current: text,
                });
                drafts.len() - 1
            }
        };
        let draft = &mut drafts[at];
        match annotate(&draft.part, &draft.current, &finding.anchor, &finding.text) {
            None => unknown.push(format!("{}: no anchor `{}`", finding.part, finding.anchor)),
            Some(next) if next != draft.current => {
                draft.current = next;
                added += 1;
            }
            Some(_) => {}
        }
    }
    match unknown.is_empty() {
        true => Ok((drafts, added)),
        false => Err(unknown.join(ERROR_SEPARATOR)),
    }
}

pub fn remaining(delta: &Doc) -> usize {
    PARTS
        .iter()
        .map(|part| review_lines(delta.part_text(part)).len())
        .sum()
}

pub fn review_draft(
    root: &Path,
    delta: &Doc,
    prompt: &str,
    resolved: &[Finding],
    config: &runner::Config,
) -> Result<Outcome, String> {
    let env = [("LEMMA_DELTA", delta.id.as_str())];
    let schema = schema::review_doc(root, delta);
    let reply = runner::run(
        &config.reviewer,
        prompt,
        root,
        &env,
        config.timeout,
        Some(&schema),
    )?;
    let findings: Vec<Finding> = parse_findings(reply.json()?)?
        .into_iter()
        .filter(|finding| !resolved.contains(finding))
        .collect();
    if findings.is_empty() {
        return Ok(Outcome::Clean(remaining(delta)));
    }
    let (drafts, added) = apply(root, delta, &findings)?;
    for draft in drafts.iter().filter(|d| d.current != d.original) {
        let path = root.join(part_file(delta, &draft.part));
        std::fs::write(&path, &draft.current)
            .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    }
    reviews::record(root, &delta.id, &findings);
    Ok(Outcome::Written(added))
}

#[derive(Deserialize)]
struct DiffReply {
    findings: Vec<DiffFinding>,
}

#[derive(Deserialize)]
struct DiffFinding {
    text: String,
    #[serde(default)]
    settles: Vec<String>,
}

fn unique(settles: Vec<String>) -> Vec<String> {
    let mut seen = Vec::new();
    for id in settles {
        if !seen.contains(&id) {
            seen.push(id);
        }
    }
    seen
}

pub fn review_diff(
    root: &Path,
    delta: &Doc,
    prompt: &str,
    config: &runner::Config,
) -> Result<usize, String> {
    let env = [("LEMMA_DELTA", delta.id.as_str())];
    let schema = schema::review_diff(&plan::read(root, delta, CLAIMS_PART));
    let reply = runner::run(
        &config.reviewer,
        prompt,
        root,
        &env,
        config.timeout,
        Some(&schema),
    )?;
    let reply: DiffReply =
        serde_json::from_value(reply.json()?).map_err(|e| format!("reviewer reply: {e}"))?;
    let tasks_text = plan::read(root, delta, TASKS_PART);
    let parsed = tasks::parse(&tasks_text);
    let mut known: Vec<String> = parsed.tasks.iter().map(|t| t.text.clone()).collect();
    let mut number = plan::next_number(parsed.tasks.iter().map(|t| t.number));
    let mut lines = Vec::new();
    for finding in reply.findings {
        let text = plan::collapsed(&finding.text);
        if known.contains(&text) {
            continue;
        }
        lines.push(plan::task_entry(number, &text, &unique(finding.settles)));
        known.push(text);
        number += 1;
    }
    if lines.is_empty() {
        return Ok(0);
    }
    let path = root.join(part_file(delta, TASKS_PART));
    std::fs::write(&path, plan::append_follow_ups(&tasks_text, &lines))
        .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    Ok(lines.len())
}
