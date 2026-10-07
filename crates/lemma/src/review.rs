use crate::annotate::annotate;
use crate::claims::review_lines;
use crate::payload::{Doc, CLAIMS_PART, DESIGN_PART, INDEX_PART, TASKS_PART};
use crate::plan;
use crate::runner;
use crate::tasks;
use serde::Deserialize;
use std::path::Path;

const PARTS: [&str; 4] = [INDEX_PART, DESIGN_PART, CLAIMS_PART, TASKS_PART];
const ERROR_SEPARATOR: &str = "\nlemma: ";

pub enum Outcome {
    Remaining(usize),
    Planned(plan::Report),
    Written(usize),
}

#[derive(Deserialize)]
struct Reply {
    findings: Vec<Finding>,
}

#[derive(Deserialize)]
struct Finding {
    part: String,
    #[serde(default)]
    anchor: String,
    text: String,
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

pub fn review_draft(
    root: &Path,
    delta: &Doc,
    prompt: &str,
    plan_prompt: &str,
    config: &runner::Config,
) -> Result<Outcome, String> {
    let env = [("LEMMA_DELTA", delta.id.as_str())];
    let result = runner::run(&config.reviewer, prompt, root, &env, config.timeout)?;
    let findings = parse_findings(runner::json_reply(&result)?)?;
    if findings.is_empty() {
        return clean(root, delta, plan_prompt, config);
    }
    let (drafts, added) = apply(root, delta, &findings)?;
    for draft in drafts.iter().filter(|d| d.current != d.original) {
        let path = root.join(part_file(delta, &draft.part));
        std::fs::write(&path, &draft.current)
            .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    }
    Ok(Outcome::Written(added))
}

fn clean(
    root: &Path,
    delta: &Doc,
    plan_prompt: &str,
    config: &runner::Config,
) -> Result<Outcome, String> {
    let remaining: usize = PARTS
        .iter()
        .map(|part| review_lines(delta.part_text(part)).len())
        .sum();
    if remaining > 0 {
        return Ok(Outcome::Remaining(remaining));
    }
    plan::plan(root, delta, plan_prompt, config).map(Outcome::Planned)
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
    let result = runner::run(&config.reviewer, prompt, root, &env, config.timeout)?;
    let reply: DiffReply = serde_json::from_value(runner::json_reply(&result)?)
        .map_err(|e| format!("reviewer reply: {e}"))?;
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
