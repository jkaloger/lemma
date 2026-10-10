use crate::annotate::frontmatter_len;
use crate::claims::{self, heading_level, lines_of, live_lines};
use crate::payload::{Doc, CLAIMS_PART, INDEX_PART, TASKS_PART};
use crate::prompts::{self, Prompt};
use crate::review::part_file;
use crate::runner;
use crate::schema;
use crate::tasks::{self, Tasks, FOLLOW_UPS};
use serde::Deserialize;
use std::collections::{BTreeMap, HashSet};
use std::path::Path;

const ID_PREFIX: &str = "AC-";
const FRONTMATTER_ORDER: [&str; 7] = [
    "title", "type", "status", "author", "date", "tags", "related",
];
const HEADINGS: [(&str, &str); 2] = [("intent", "## Intent"), ("out of scope", "## Out of scope")];

#[derive(Deserialize)]
struct Reply {
    claims: Vec<PlanClaim>,
    tasks: Vec<PlanTask>,
}

#[derive(Deserialize)]
struct PlanClaim {
    text: String,
}

#[derive(Deserialize)]
struct PlanTask {
    text: String,
    #[serde(default)]
    settles: Vec<Settle>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Settle {
    Number(usize),
    Name(String),
}

impl std::fmt::Display for Settle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Settle::Number(n) => write!(f, "{n}"),
            Settle::Name(name) => write!(f, "{name}"),
        }
    }
}

type Known = Vec<(String, String)>;

#[derive(Default)]
pub struct Report {
    already_planned: bool,
    tagged: Vec<String>,
    proposed: usize,
    tasks: usize,
    index_tidied: bool,
}

impl Report {
    pub fn lines(&self) -> Vec<String> {
        let mut lines = match self.already_planned {
            true => vec!["review clean".to_string(), "already planned".to_string()],
            false => vec!["review clean, planning".to_string()],
        };
        lines.extend(self.claims_line());
        if self.tasks > 0 {
            let noun = if self.tasks == 1 { "task" } else { "tasks" };
            lines.push(format!("tasks.md: {} {noun} written", self.tasks));
        }
        if self.index_tidied {
            lines.push("index.md: frontmatter and headings tidied".to_string());
        }
        lines
    }

    fn claims_line(&self) -> Option<String> {
        let tagged = match self.tagged.as_slice() {
            [] => None,
            [only] => Some(format!("1 claim tagged {only}")),
            [first, .., last] => Some(format!(
                "{} claims tagged {first}..{last}",
                self.tagged.len()
            )),
        };
        let proposed = (self.proposed > 0).then(|| format!("{} [proposed]", self.proposed));
        let parts: Vec<String> = [tagged, proposed].into_iter().flatten().collect();
        (!parts.is_empty()).then(|| format!("claims.md: {}", parts.join(", ")))
    }
}

struct Files {
    index: String,
    claims: String,
    tasks: String,
}

struct Drafted {
    claims: String,
    tasks: String,
    report: Report,
}

pub fn read(root: &Path, delta: &Doc, part: &str) -> String {
    std::fs::read_to_string(root.join(part_file(delta, part))).unwrap_or_default()
}

fn ensure_newline(text: &mut String) {
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
}

fn collapsed(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn id_number(id: &str) -> Option<u32> {
    id.strip_prefix(ID_PREFIX)?.parse().ok()
}

fn next_number(ids: impl Iterator<Item = u32>) -> u32 {
    ids.max().unwrap_or(0) + 1
}

fn refuse_ticked(parsed: &Tasks) -> Result<(), String> {
    let ticked: Vec<&str> = parsed
        .tasks
        .iter()
        .filter(|t| t.ticked)
        .map(|t| t.id.as_str())
        .collect();
    match ticked.is_empty() {
        true => Ok(()),
        false => Err(format!(
            "plan refuses: {} {} ticked",
            ticked.join(", "),
            if ticked.len() == 1 { "is" } else { "are" }
        )),
    }
}

fn is_planned(claims_text: &str, parsed: &Tasks) -> bool {
    let settled: HashSet<&str> = parsed
        .main()
        .flat_map(|t| t.settles.iter().map(String::as_str))
        .collect();
    parsed.main().next().is_some()
        && claims::bullets(claims_text).iter().all(|b| b.id.is_some())
        && claims::parse(claims_text)
            .claims
            .iter()
            .filter(|c| !c.struck)
            .all(|c| settled.contains(c.id.as_str()))
}

struct Tagged {
    claims_text: String,
    known: Known,
    tagged: Vec<String>,
}

fn tag_bullets(claims_text: &str) -> Tagged {
    let bullets = claims::bullets(claims_text);
    let mut next = next_number(bullets.iter().filter_map(|b| id_number(b.id.as_deref()?)));
    let mut lines: Vec<String> = lines_of(claims_text)
        .iter()
        .map(|l| l.to_string())
        .collect();
    let mut known = Known::new();
    let mut tagged = Vec::new();
    for bullet in bullets {
        if let Some(id) = bullet.id {
            known.push((id, bullet.text));
            continue;
        }
        let id = format!("{ID_PREFIX}{next}");
        next += 1;
        let line = &mut lines[bullet.line - 1];
        *line = format!("- [{id}] {}", &line[2..]);
        tagged.push(id.clone());
        known.push((id, bullet.text));
    }
    Tagged {
        claims_text: lines.join("\n"),
        known,
        tagged,
    }
}

fn add_proposed(
    text: &mut String,
    known: &mut Known,
    claims: &[PlanClaim],
) -> (Vec<String>, usize) {
    let mut next = next_number(known.iter().filter_map(|(id, _)| id_number(id)));
    let mut ids = Vec::new();
    let mut proposed = 0;
    for claim in claims {
        let matched = known
            .iter()
            .find(|(_, known_text)| known_text == claim.text.trim());
        if let Some((id, _)) = matched {
            ids.push(id.clone());
            continue;
        }
        let id = format!("{ID_PREFIX}{next}");
        next += 1;
        let claim_text = collapsed(&claim.text);
        ensure_newline(text);
        text.push_str(&format!("- [{id}] [proposed] {claim_text}\n"));
        known.push((id.clone(), claim_text));
        ids.push(id);
        proposed += 1;
    }
    (ids, proposed)
}

fn resolve(entry: &Settle, plan_ids: &[String], known: &Known) -> Option<String> {
    match entry {
        Settle::Number(n) => plan_ids.get(n.checked_sub(1)?).cloned(),
        Settle::Name(name) => known
            .iter()
            .find(|(id, _)| id == name)
            .map(|(id, _)| id.clone()),
    }
}

fn task_line(
    number: u32,
    task: &PlanTask,
    plan_ids: &[String],
    known: &Known,
) -> Result<String, String> {
    let mut settles: Vec<String> = Vec::new();
    for entry in &task.settles {
        let id = resolve(entry, plan_ids, known)
            .ok_or_else(|| format!("plan task `{}` settles unknown claim `{entry}`", task.text))?;
        if !settles.contains(&id) {
            settles.push(id);
        }
    }
    Ok(task_entry(number, &task.text, &settles))
}

fn task_entry(number: u32, text: &str, settles: &[String]) -> String {
    let tail = match settles.is_empty() {
        true => "chore".to_string(),
        false => format!("settles {}", settles.join(", ")),
    };
    format!("- [ ] T{number} {} ({tail})", collapsed(text))
}

fn follow_ups_line(tasks_text: &str) -> Option<usize> {
    live_lines(tasks_text)
        .into_iter()
        .find(|(_, line)| {
            matches!(heading_level(line), Some((_, name)) if name.eq_ignore_ascii_case(FOLLOW_UPS))
        })
        .map(|(n, _)| n - 1)
}

fn insert_tasks(tasks_text: &str, new: &[String]) -> String {
    let Some(heading) = follow_ups_line(tasks_text) else {
        let mut text = tasks_text.to_string();
        ensure_newline(&mut text);
        new.iter()
            .for_each(|line| text.push_str(&format!("{line}\n")));
        return text;
    };
    let lines = lines_of(tasks_text);
    let before = lines[..heading]
        .iter()
        .rposition(|l| !l.trim().is_empty())
        .map_or(0, |last| last + 1);
    let mut out: Vec<&str> = lines[..before].to_vec();
    out.extend(new.iter().map(String::as_str));
    out.push("");
    out.extend(&lines[heading..]);
    out.join("\n")
}

fn draft(
    files: &Files,
    parsed: &Tasks,
    tagged_claims: Tagged,
    reply: Reply,
) -> Result<Drafted, String> {
    let Tagged {
        mut claims_text,
        mut known,
        tagged,
    } = tagged_claims;
    let (plan_ids, proposed) = add_proposed(&mut claims_text, &mut known, &reply.claims);
    let first = next_number(parsed.tasks.iter().map(|t| t.number));
    let lines = (first..)
        .zip(&reply.tasks)
        .map(|(number, task)| task_line(number, task, &plan_ids, &known))
        .collect::<Result<Vec<_>, _>>()?;
    let tasks_text = match lines.is_empty() {
        true => files.tasks.clone(),
        false => insert_tasks(&files.tasks, &lines),
    };
    let report = Report {
        tagged,
        proposed,
        tasks: lines.len(),
        ..Report::default()
    };
    Ok(Drafted {
        claims: claims_text,
        tasks: tasks_text,
        report,
    })
}

fn is_continuation(line: &str) -> bool {
    line.starts_with(char::is_whitespace) || line.starts_with("- ")
}

fn canonical_frontmatter(front: &str) -> String {
    let lines: Vec<&str> = front.split_inclusive('\n').collect();
    let [open, inner @ .., close] = lines.as_slice() else {
        return front.to_string();
    };
    let mut chunks: Vec<(&str, String)> = Vec::new();
    for line in inner {
        match chunks.last_mut().filter(|_| is_continuation(line)) {
            Some((_, chunk)) => chunk.push_str(line),
            None => chunks.push((
                line.split(':').next().unwrap_or("").trim(),
                line.to_string(),
            )),
        }
    }
    chunks.sort_by_key(|(key, _)| {
        FRONTMATTER_ORDER
            .iter()
            .position(|k| k == key)
            .unwrap_or(FRONTMATTER_ORDER.len())
    });
    let body: String = chunks.into_iter().map(|(_, chunk)| chunk).collect();
    format!("{open}{body}{close}")
}

fn normalised_headings(body: &str) -> String {
    let mut lines: Vec<String> = lines_of(body).iter().map(|l| l.to_string()).collect();
    for (n, line) in live_lines(body) {
        let Some((_, name)) = heading_level(line) else {
            continue;
        };
        if let Some((_, canonical)) = HEADINGS.iter().find(|(h, _)| name.eq_ignore_ascii_case(h)) {
            lines[n - 1] = canonical.to_string();
        }
    }
    lines.join("\n")
}

fn tidy_index(index: &str) -> String {
    let (front, body) = index.split_at(frontmatter_len(index));
    format!(
        "{}{}",
        canonical_frontmatter(front),
        normalised_headings(body)
    )
}

fn write_changed(
    root: &Path,
    delta: &Doc,
    part: &str,
    before: &str,
    after: &str,
) -> Result<(), String> {
    if before == after {
        return Ok(());
    }
    let path = root.join(part_file(delta, part));
    std::fs::write(&path, after).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

fn ask_planner(
    root: &Path,
    delta: &Doc,
    values: &BTreeMap<&str, String>,
    files: &Files,
    parsed: &Tasks,
    config: &runner::Config,
) -> Result<Drafted, String> {
    let tagged = tag_bullets(&files.claims);
    let mut values = values.clone();
    values.insert("claims", tagged.claims_text.clone());
    let prompt = prompts::rendered(Prompt::Plan, root, &values)?;
    let known_ids: Vec<String> = tagged.known.iter().map(|(id, _)| id.clone()).collect();
    let env = [("LEMMA_DELTA", delta.id.as_str())];
    let schema = schema::plan(&known_ids);
    let reply = runner::tracked(
        "plan",
        &config.reviewer,
        &prompt,
        root,
        &env,
        config.timeout,
        Some(&schema),
        config.mode,
    )?;
    let reply = serde_json::from_value(reply.json()?).map_err(|e| format!("planner reply: {e}"))?;
    draft(files, parsed, tagged, reply)
}

pub fn plan(
    root: &Path,
    delta: &Doc,
    values: &BTreeMap<&str, String>,
    config: &runner::Config,
) -> Result<Report, String> {
    let files = Files {
        index: read(root, delta, INDEX_PART),
        claims: read(root, delta, CLAIMS_PART),
        tasks: read(root, delta, TASKS_PART),
    };
    let parsed = tasks::parse(&files.tasks);
    refuse_ticked(&parsed)?;
    let mut drafted = match is_planned(&files.claims, &parsed) {
        true => Drafted {
            claims: files.claims.clone(),
            tasks: files.tasks.clone(),
            report: Report {
                already_planned: true,
                ..Report::default()
            },
        },
        false => ask_planner(root, delta, values, &files, &parsed, config)?,
    };
    let index = tidy_index(&files.index);
    drafted.report.index_tidied = index != files.index;
    write_changed(root, delta, CLAIMS_PART, &files.claims, &drafted.claims)?;
    write_changed(root, delta, TASKS_PART, &files.tasks, &drafted.tasks)?;
    write_changed(root, delta, INDEX_PART, &files.index, &index)?;
    Ok(drafted.report)
}
