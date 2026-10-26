use crate::annotate::frontmatter_len;
use crate::claims::{self, heading_level, lines_of, live_lines};
use crate::gate::Finding;
use crate::payload::{Doc, CLAIMS_PART, INDEX_PART, TASKS_PART};
use crate::prompts::{self, Prompt};
use crate::review::part_file;
use crate::runner;
use crate::schema;
use crate::shape;
use crate::tasks::{self, Task, Tasks, FOLLOW_UPS};
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
    settled_by: BTreeMap<String, usize>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Replan {
    tasks: Vec<PlanTask>,
    settled_by: BTreeMap<String, usize>,
}

#[derive(Deserialize)]
struct PlanClaim {
    text: String,
    task: usize,
}

#[derive(Deserialize)]
struct PlanTask {
    text: String,
}

type Known = Vec<(String, String)>;

/// A rejected reply, or a runner that ran and failed, can be asked again; a failure (a runner
/// that can't start, an unreadable delta) cannot.
pub enum Error {
    Rejected(String),
    Failed(String),
}

impl Error {
    pub fn message(self) -> String {
        match self {
            Error::Rejected(message) | Error::Failed(message) => message,
        }
    }
}

impl From<String> for Error {
    fn from(message: String) -> Self {
        Error::Failed(message)
    }
}

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

/// Live claims no kept task settles: each needs a `settled_by` entry in the reply.
fn unsettled(claims_text: &str, parsed: &Tasks) -> Vec<String> {
    let settled: HashSet<&str> = parsed
        .main()
        .flat_map(|t| t.settles.iter().map(String::as_str))
        .collect();
    claims::parse(claims_text)
        .claims
        .into_iter()
        .filter(|c| !c.struck && !settled.contains(c.id.as_str()))
        .map(|c| c.id)
        .collect()
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

/// Appends the reply's new claims, each with the position of its settling task. A claim
/// echoing a known one is dropped: `settled_by` or a kept task already settles it.
fn add_proposed(
    text: &mut String,
    known: &mut Known,
    claims: &[PlanClaim],
) -> Vec<(String, usize)> {
    let mut next = next_number(known.iter().filter_map(|(id, _)| id_number(id)));
    let mut added = Vec::new();
    for claim in claims {
        let claim_text = collapsed(&claim.text);
        if known
            .iter()
            .any(|(_, known_text)| collapsed(known_text) == claim_text)
        {
            continue;
        }
        let id = format!("{ID_PREFIX}{next}");
        next += 1;
        ensure_newline(text);
        text.push_str(&format!("- [{id}] [proposed] {claim_text}\n"));
        known.push((id.clone(), claim_text));
        added.push((id, claim.task));
    }
    added
}

/// The first settling position outside the reply's tasks, which the schema cannot bound.
fn out_of_range(reply: &Reply) -> Option<String> {
    let tasks = reply.tasks.len();
    reply
        .settled_by
        .iter()
        .map(|(id, position)| (id.clone(), *position))
        .chain(
            reply
                .claims
                .iter()
                .map(|claim| (format!("new claim \"{}\"", claim.text), claim.task)),
        )
        .find_map(|(id, position)| task_index(&id, position, tasks).err())
}

fn task_index(id: &str, position: usize, tasks: usize) -> Result<usize, String> {
    match (1..=tasks).contains(&position) {
        true => Ok(position - 1),
        false => Err(format!(
            "plan settles {id} by task {position}, but the reply has {tasks} {}",
            if tasks == 1 { "task" } else { "tasks" }
        )),
    }
}

fn settles_by_task(
    reply: &Reply,
    known: &Known,
    added: &[(String, usize)],
) -> Result<Vec<Vec<String>>, String> {
    if let Some(unknown) = reply
        .settled_by
        .keys()
        .find(|key| !known.iter().any(|(id, _)| id == *key))
    {
        return Err(format!("plan settles unknown claim `{unknown}`"));
    }
    let mut settles = vec![Vec::new(); reply.tasks.len()];
    let settled = known
        .iter()
        .filter_map(|(id, _)| Some((id, *reply.settled_by.get(id)?)))
        .chain(added.iter().map(|(id, task)| (id, *task)));
    for (id, position) in settled {
        settles[task_index(id, position, reply.tasks.len())?].push(id.clone());
    }
    Ok(settles)
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
    let added = add_proposed(&mut claims_text, &mut known, &reply.claims);
    let settles = settles_by_task(&reply, &known, &added)?;
    let first = next_number(parsed.tasks.iter().map(|t| t.number));
    let lines: Vec<String> = (first..)
        .zip(reply.tasks.iter().zip(&settles))
        .map(|(number, (task, settles))| task_entry(number, &task.text, settles))
        .collect();
    let tasks_text = match lines.is_empty() {
        true => files.tasks.clone(),
        false => insert_tasks(&files.tasks, &lines),
    };
    let report = Report {
        tagged,
        proposed: added.len(),
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

#[allow(clippy::too_many_arguments)]
fn ask_planner(
    root: &Path,
    delta: &Doc,
    values: &BTreeMap<&str, String>,
    files: &Files,
    parsed: &Tasks,
    config: &runner::Config,
    rejected: Option<&str>,
    asked: &mut u32,
) -> Result<Drafted, Error> {
    let tagged = tag_bullets(&files.claims);
    let mut values = values.clone();
    values.insert("claims", tagged.claims_text.clone());
    let mut prompt = prompts::rendered(Prompt::Plan, root, &values)?;
    if let Some(rejection) = rejected {
        prompt.push_str(&format!(
            "\n\nYour last reply was rejected: {rejection}. Reply again.\n"
        ));
    }
    let env = [("LEMMA_DELTA", delta.id.as_str())];
    let schema = schema::plan(&unsettled(&tagged.claims_text, parsed));
    let mut ask = |prompt: &str| -> Result<Reply, Error> {
        *asked += 1;
        let reply = runner::tracked(
            "plan",
            &config.reviewer,
            prompt,
            root,
            &env,
            config.timeout,
            Some(&schema),
            config.mode,
        )
        .map_err(
            |failed| match failed.message.starts_with(runner::CANNOT_START) {
                true => Error::Failed(failed.message),
                false => Error::Rejected(failed.message),
            },
        )?;
        let json = reply.json().map_err(Error::Rejected)?;
        serde_json::from_value(json).map_err(|e| Error::Rejected(format!("planner reply: {e}")))
    };
    let mut reply = ask(&prompt)?;
    if let Some(error) = out_of_range(&reply) {
        reply = ask(&format!(
            "{prompt}\n\nYour last reply was rejected: {error}. Reply again with every task position inside `tasks`.\n"
        ))?;
    }
    let drafted = draft(files, parsed, tagged, reply).map_err(Error::Rejected)?;
    refuse_uncovered(&drafted).map_err(Error::Rejected)?;
    Ok(drafted)
}

fn refuse_uncovered(drafted: &Drafted) -> Result<(), String> {
    let claims = claims::parse(&drafted.claims).claims;
    let found = shape::uncovered(&claims, &tasks::parse(&drafted.tasks));
    match found.is_empty() {
        true => Ok(()),
        false => Err(format!(
            "plan refuses to write tasks the gate rejects: {}",
            found
                .into_iter()
                .map(|(_, message)| message)
                .collect::<Vec<_>>()
                .join("; ")
        )),
    }
}

/// `rejected` is the error from the last attempt, shown to the planner; `asked` counts its calls.
pub fn plan(
    root: &Path,
    delta: &Doc,
    values: &BTreeMap<&str, String>,
    config: &runner::Config,
    rejected: Option<&str>,
    asked: &mut u32,
) -> Result<Report, Error> {
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
        false => ask_planner(
            root, delta, values, &files, &parsed, config, rejected, asked,
        )?,
    };
    let index = tidy_index(&files.index);
    drafted.report.index_tidied = index != files.index;
    write_changed(root, delta, CLAIMS_PART, &files.claims, &drafted.claims)?;
    write_changed(root, delta, TASKS_PART, &files.tasks, &drafted.tasks)?;
    write_changed(root, delta, INDEX_PART, &files.index, &index)?;
    Ok(drafted.report)
}

fn replan_settles(stopped: &Task, reply: &Replan) -> Result<Vec<Vec<String>>, String> {
    if reply.tasks.is_empty() {
        return Err(format!("re-plan of {} has no tasks", stopped.id));
    }
    let wanted: HashSet<&str> = stopped.settles.iter().map(String::as_str).collect();
    let got: HashSet<&str> = reply.settled_by.keys().map(String::as_str).collect();
    if wanted != got {
        let mut keys: Vec<&str> = got.into_iter().collect();
        keys.sort();
        return Err(format!(
            "re-plan of {} must settle exactly {}, not {}",
            stopped.id,
            list_or_none(
                &stopped
                    .settles
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
            ),
            list_or_none(&keys)
        ));
    }
    let mut settles = vec![Vec::new(); reply.tasks.len()];
    for id in &stopped.settles {
        let position = reply.settled_by[id];
        settles[task_index(id, position, reply.tasks.len())?].push(id.clone());
    }
    Ok(settles)
}

fn list_or_none(ids: &[&str]) -> String {
    match ids.is_empty() {
        true => "no claims".to_string(),
        false => ids.join(", "),
    }
}

/// The stopped task's line swapped for the re-planned tasks, numbered after the highest id.
fn replaced(tasks_text: &str, stopped: &Task, reply: &Replan) -> Result<(String, usize), String> {
    let settles = replan_settles(stopped, reply)?;
    let first = next_number(tasks::parse(tasks_text).tasks.iter().map(|t| t.number));
    let new: Vec<String> = (first..)
        .zip(reply.tasks.iter().zip(&settles))
        .map(|(number, (task, settles))| task_entry(number, &task.text, settles))
        .collect();
    let lines = lines_of(tasks_text);
    let mut out: Vec<&str> = lines[..stopped.line - 1].to_vec();
    out.extend(new.iter().map(String::as_str));
    out.extend(&lines[stopped.line..]);
    Ok((out.join("\n"), new.len()))
}

fn findings_list(findings: &[Finding]) -> String {
    match findings.is_empty() {
        true => "None.".to_string(),
        false => findings
            .iter()
            .map(|f| format!("- [{}] {}", f.basis, f.text))
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

/// Asks the planner to replace one stopped task and writes its tasks in the stopped line's
/// place; returns how many it wrote. Unlike `plan`, this runs on an in-progress delta.
#[allow(dead_code)]
pub fn replan(
    root: &Path,
    delta: &Doc,
    values: &BTreeMap<&str, String>,
    config: &runner::Config,
    stopped: &Task,
    findings: &[Finding],
) -> Result<usize, Error> {
    let tasks_text = read(root, delta, TASKS_PART);
    let mut values = values.clone();
    values.insert(
        "task",
        task_entry(stopped.number, &stopped.text, &stopped.settles),
    );
    values.insert("findings", findings_list(findings));
    values.insert("tasks", tasks_text.clone());
    let prompt = prompts::rendered(Prompt::Replan, root, &values)?;
    let schema = schema::replan(&stopped.settles);
    let reply = runner::tracked(
        "re-plan",
        &config.reviewer,
        &prompt,
        root,
        &[("LEMMA_DELTA", delta.id.as_str())],
        config.timeout,
        Some(&schema),
        config.mode,
    )
    .map_err(
        |failed| match failed.message.starts_with(runner::CANNOT_START) {
            true => Error::Failed(failed.message),
            false => Error::Rejected(failed.message),
        },
    )?;
    let json = reply.json().map_err(Error::Rejected)?;
    let reply: Replan =
        serde_json::from_value(json).map_err(|e| Error::Rejected(format!("re-plan reply: {e}")))?;
    let (text, written) = replaced(&tasks_text, stopped, &reply).map_err(Error::Rejected)?;
    write_changed(root, delta, TASKS_PART, &tasks_text, &text)?;
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::Mode;
    use serde_json::json;
    use std::os::unix::fs::PermissionsExt;
    use std::time::Duration;

    const TASKS: &str = "\
- [x] T1 Add the type (settles AC-1)
- [ ] T2 Serve the balance (settles AC-2, AC-3)
- [ ] T3 Wire the route (chore)

## Follow-ups
- [ ] T7 Page the read (chore)
";

    struct Fixture {
        root: tempfile::TempDir,
        delta: Doc,
        config: runner::Config,
    }

    /// A delta whose planner replies `reply` to every call.
    fn fixture(reply: serde_json::Value) -> Fixture {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("d")).unwrap();
        std::fs::write(root.path().join("d/tasks.md"), TASKS).unwrap();
        let envelope = json!({"result": reply.to_string()}).to_string();
        std::fs::write(root.path().join("reply.json"), envelope).unwrap();
        let script = root.path().join("planner.sh");
        std::fs::write(&script, "#!/bin/sh\ncat > stdin.txt\ncat reply.json\n").unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let delta: Doc = serde_json::from_value(json!({
            "id": "DELTA-001", "path": "d/index.md", "title": "t", "type": "delta",
            "status": "in_progress", "body": "", "content_hash": "h",
        }))
        .unwrap();
        let script = script.to_string_lossy().into_owned();
        let config = runner::Config {
            runner: script.clone(),
            reviewer: script,
            resume: String::new(),
            review_rounds: 3,
            vibe_repairs: 3,
            timeout: Duration::from_secs(30),
            mode: Mode::Plain,
            check: None,
        };
        Fixture {
            root,
            delta,
            config,
        }
    }

    impl Fixture {
        fn replan(&self, id: &str) -> Result<usize, Error> {
            let parsed = tasks::parse(TASKS);
            let stopped = parsed.named(id).unwrap();
            let findings = [Finding {
                text: "too big".to_string(),
                basis: "task".to_string(),
            }];
            replan(
                self.root.path(),
                &self.delta,
                &BTreeMap::new(),
                &self.config,
                stopped,
                &findings,
            )
        }

        fn tasks(&self) -> String {
            std::fs::read_to_string(self.root.path().join("d/tasks.md")).unwrap()
        }

        fn prompt(&self) -> String {
            std::fs::read_to_string(self.root.path().join("stdin.txt")).unwrap()
        }
    }

    fn rejection(result: Result<usize, Error>) -> String {
        match result {
            Err(Error::Rejected(message)) => message,
            Err(Error::Failed(message)) => panic!("failed, not rejected: {message}"),
            Ok(written) => panic!("accepted {written} tasks"),
        }
    }

    #[test]
    fn replanned_tasks_are_numbered_after_the_highest_id_and_ticked_tasks_are_unchanged() {
        let fixture = fixture(json!({
            "tasks": [{"text": "Add the balance query"}, {"text": "Serve it over HTTP"}],
            "settled_by": {"AC-2": 1, "AC-3": 2},
        }));

        let written = fixture.replan("T2");

        assert_eq!(written.ok(), Some(2));
        assert_eq!(
            fixture.tasks(),
            "\
- [x] T1 Add the type (settles AC-1)
- [ ] T8 Add the balance query (settles AC-2)
- [ ] T9 Serve it over HTTP (settles AC-3)
- [ ] T3 Wire the route (chore)

## Follow-ups
- [ ] T7 Page the read (chore)
"
        );
        let prompt = fixture.prompt();
        assert!(prompt.contains("- [ ] T2 Serve the balance (settles AC-2, AC-3)"));
        assert!(prompt.contains("- [task] too big"));
    }

    #[test]
    fn a_replan_proposes_no_claims_and_settles_exactly_the_stopped_tasks_claims() {
        let schema = schema::replan(&["AC-2".to_string(), "AC-3".to_string()]);
        assert_eq!(schema["required"], json!(["settled_by", "tasks"]));
        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(
            schema["properties"]["settled_by"]["required"],
            json!(["AC-2", "AC-3"])
        );
        assert_eq!(
            schema["properties"]["settled_by"]["additionalProperties"],
            false
        );

        for settled_by in [
            json!({"AC-2": 1}),
            json!({"AC-2": 1, "AC-3": 1, "AC-1": 1}),
            json!({"AC-2": 1, "AC-4": 1}),
        ] {
            let fixture = fixture(json!({"tasks": [{"text": "Redo"}], "settled_by": settled_by}));

            let message = rejection(fixture.replan("T2"));

            assert!(
                message.contains("must settle exactly AC-2, AC-3"),
                "{message}"
            );
            assert_eq!(fixture.tasks(), TASKS);
        }
        let fixture = fixture(json!({
            "tasks": [{"text": "Redo"}],
            "settled_by": {"AC-2": 1, "AC-3": 1},
            "claims": [{"text": "A new behaviour.", "task": 1}],
        }));
        rejection(fixture.replan("T2"));
        assert_eq!(fixture.tasks(), TASKS);
    }

    #[test]
    fn a_replan_with_no_tasks_is_invalid_and_never_drops_the_stopped_task() {
        let schema = schema::replan(&[]);
        assert_eq!(schema["properties"]["tasks"]["minItems"], 1);

        for (id, settled_by) in [("T2", json!({"AC-2": 1, "AC-3": 1})), ("T3", json!({}))] {
            let fixture = fixture(json!({"tasks": [], "settled_by": settled_by}));

            let message = rejection(fixture.replan(id));

            assert!(message.contains("has no tasks"), "{message}");
            assert_eq!(fixture.tasks(), TASKS);
        }
    }
}
