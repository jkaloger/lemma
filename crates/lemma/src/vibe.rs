use crate::advance;
use crate::annotate::annotate;
use crate::annotate::frontmatter_len;
use crate::build;
use crate::builds;
use crate::claims::{accept_proposed, review_lines};
use crate::context;
use crate::git::untracked_files;
use crate::lint;
use crate::output::{self, Look, Mode, Tone};
use crate::payload::{Doc, CHANGES, CLAIMS_PART, DESIGN_PART, INDEX_PART, TASKS_PART};
use crate::plan;
use crate::prompts::{self, Prompt};
use crate::review::{self, part_file, Outcome, PARTS};
use crate::reviews;
use crate::runner;
use crate::schema;
use crate::tasks;
use crate::world::World;
use lazyspec::engine::store::Store;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

const LAZYSPEC: &str = "lazyspec";

#[derive(Deserialize)]
struct Draft {
    title: String,
    changes: Vec<String>,
    intent: String,
    out_of_scope: String,
    design: String,
    claims: Vec<String>,
}

#[derive(Deserialize)]
struct Revision {
    index: String,
    design: String,
    claims: String,
}

/// A run that stopped: the error, if any, and the command that picks it up by hand.
pub struct Stopped {
    pub message: Option<String>,
    pub next: Option<String>,
}

fn stopped(message: String, next: &str) -> Stopped {
    Stopped {
        message: Some(message),
        next: Some(next.to_string()),
    }
}

impl From<String> for Stopped {
    fn from(message: String) -> Self {
        Stopped {
            message: Some(message),
            next: None,
        }
    }
}

const STAGES: usize = 5;

/// What the live-mode summary reports; plain mode prints none of it.
struct Ledger {
    look: Look,
    stages: Vec<(&'static str, Duration)>,
    rounds: u32,
    accepted: usize,
    tasks: usize,
}

impl Ledger {
    fn stage<T, E>(
        &mut self,
        n: usize,
        name: &'static str,
        tone: Tone,
        body: impl FnOnce(&mut Ledger) -> Result<T, E>,
    ) -> Result<T, (&'static str, E)> {
        self.banner(n, name, tone);
        let started = Instant::now();
        let result = body(self);
        self.stages.push((name, started.elapsed()));
        result.map_err(|e| (name, e))
    }

    fn banner(&self, n: usize, name: &str, tone: Tone) {
        if self.look.mode != Mode::Live {
            return;
        }
        let (_, cols) = console::Term::stdout().size_checked().unwrap_or((0, 80));
        let head = format!("── ◆ [{n}/{STAGES}] {name} ");
        let fill = (cols as usize).saturating_sub(head.chars().count());
        let line = format!("{head}{}", "─".repeat(fill));
        println!("{}", self.look.tone(tone, &line));
    }

    fn summary(&self, id: Option<&str>, failed: Option<&str>) {
        if self.look.mode != Mode::Live {
            return;
        }
        let tone = match failed {
            Some(_) => Tone::Red,
            None => Tone::Green,
        };
        let title = match (id, failed) {
            (Some(id), Some(stage)) => format!("{id} stopped at {stage}"),
            (Some(id), None) => format!("{id} done"),
            (None, Some(stage)) => format!("stopped at {stage}"),
            (None, None) => "done".to_string(),
        };
        println!("{}", self.look.tone(tone, &format!("── summary: {title}")));
        for (name, elapsed) in &self.stages {
            println!("  {name:<8} {:.1}s", elapsed.as_secs_f64());
        }
        println!(
            "  {} · {} accepted · {} committed",
            output::counted(self.rounds as usize, "review round"),
            output::counted(self.accepted, "claim"),
            output::counted(self.tasks, "task"),
        );
    }

    fn ticked(&mut self, root: &Path, id: &str) {
        if let Ok((_, delta)) = build::load_delta(root, id) {
            self.tasks = tasks::parse(delta.part_body(TASKS_PART))
                .main()
                .filter(|t| t.ticked)
                .count();
        }
    }
}

/// Returns the id of the delta it drafted and built.
pub fn run(
    root: &Path,
    store: &Store,
    prompt: &str,
    rounds: u32,
    config: &runner::Config,
) -> Result<String, Stopped> {
    build::require_clean(root)?;
    let mut ledger = Ledger {
        look: Look::current(config.mode),
        stages: Vec::new(),
        rounds: 0,
        accepted: 0,
        tasks: 0,
    };
    let mut id = None;
    let result = stages(root, store, prompt, rounds, config, &mut ledger, &mut id);
    ledger.summary(
        id.as_deref(),
        result.as_ref().err().map(|(stage, _)| *stage),
    );
    result.map_err(|(_, stopped)| stopped)?;
    Ok(id.unwrap_or_default())
}

type StageResult = Result<(), (&'static str, Stopped)>;

fn stages(
    root: &Path,
    store: &Store,
    prompt: &str,
    rounds: u32,
    config: &runner::Config,
    ledger: &mut Ledger,
    drafted: &mut Option<String>,
) -> StageResult {
    let delta = ledger.stage(1, "draft", Tone::Magenta, |_| {
        draft(root, store, prompt, config).map_err(|m| stopped(m, "lemma vibe -m …"))
    })?;
    let id = &delta.id;
    *drafted = Some(id.clone());
    println!("draft: {id} {}", delta.title);
    let review = format!("lemma review {id}");
    ledger.stage(2, "review", Tone::Cyan, |ledger| {
        review_rounds(root, id, rounds, config, &mut ledger.rounds)
            .map_err(|m| stopped(m, &review))?;
        strip_review_lines(root, id).map_err(|m| stopped(m, &review))
    })?;
    ledger.stage(3, "plan", Tone::Blue, |_| {
        plan(root, id, config).map_err(|m| stopped(m, &format!("lemma plan {id}")))
    })?;
    let accepting = format!("accept or reject each [proposed] claim, then lemma build {id}");
    ledger.stage(4, "accept", Tone::Yellow, |ledger| {
        ledger.accepted = accept(root, id).map_err(|m| stopped(m, &accepting))?;
        Ok(())
    })?;
    let moving = format!("lazyspec update {id} --status in-progress");
    ledger.stage(5, "build", Tone::Green, |ledger| {
        in_progress(root, id, config).map_err(|m| stopped(m, &moving))?;
        let built = run_build(root, id, config);
        ledger.ticked(root, id);
        built
    })
}

fn spec_ids(store: &Store) -> Vec<String> {
    World::load(store, None)
        .docs()
        .iter()
        .filter(|d| d.doc_type == "spec")
        .map(|d| d.id.clone())
        .collect()
}

fn draft(root: &Path, store: &Store, prompt: &str, config: &runner::Config) -> Result<Doc, String> {
    let specs = spec_ids(store);
    let values = context::for_draft(store, prompt);
    let rendered = prompts::rendered(Prompt::Vibe, root, &values)?;
    let schema = schema::vibe(&specs);
    let ask = |prompt: &str| -> Result<Draft, String> {
        let reply = runner::tracked(
            "draft",
            &config.runner,
            prompt,
            root,
            &[],
            config.timeout,
            Some(&schema),
            config.mode,
            config.spinner,
        )?;
        serde_json::from_value(reply.json()?).map_err(|e| format!("draft reply: {e}"))
    };
    let mut reply = ask(&rendered)?;
    let mut repairs = 0;
    while let Err(rejection) = require_known_changes(&reply.changes, &specs) {
        if repairs == config.vibe_repairs {
            return Err(rejection);
        }
        repairs += 1;
        reply = ask(&format!(
            "{rendered}\n\nYour last reply was rejected: {rejection}. Reply again.\n"
        ))?;
    }
    let delta = create(root, &reply.title)?;
    if let Err(message) = fill(root, &delta, &reply) {
        delete(root, &delta);
        return Err(message);
    }
    Ok(delta)
}

fn require_known_changes(changes: &[String], specs: &[String]) -> Result<(), String> {
    if changes.is_empty() {
        return Err("draft reply changes no spec; a delta needs a `changes` link".to_string());
    }
    match changes.iter().find(|c| !specs.contains(c)) {
        Some(unknown) => Err(format!("draft reply changes unknown spec {unknown}")),
        None => Ok(()),
    }
}

fn lazyspec(root: &Path, args: &[&str]) -> Result<String, String> {
    let shown = format!("{LAZYSPEC} {}", args.join(" "));
    let out = Command::new(LAZYSPEC)
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|e| format!("cannot start `{shown}`: {e}"))?;
    match out.status.success() {
        true => Ok(String::from_utf8_lossy(&out.stdout).into_owned()),
        false => Err(format!(
            "`{shown}` failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )),
    }
}

fn create(root: &Path, title: &str) -> Result<Doc, String> {
    let out = lazyspec(root, &["create", "delta", title, "--json"])?;
    serde_json::from_str(&out).map_err(|e| format!("`lazyspec create` reply: {e}"))
}

fn write(root: &Path, delta: &Doc, part: &str, text: &str) -> Result<(), String> {
    let path = root.join(part_file(delta, part));
    std::fs::write(&path, text).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

fn index_text(created: &str, reply: &Draft) -> String {
    let front = &created[..frontmatter_len(created)];
    format!(
        "{front}\n## Intent\n\n{}\n\n## Out of scope\n\n{}\n",
        reply.intent.trim(),
        reply.out_of_scope.trim()
    )
}

fn claims_text(claims: &[String]) -> String {
    claims
        .iter()
        .map(|c| format!("- {}\n", c.split_whitespace().collect::<Vec<_>>().join(" ")))
        .collect()
}

fn fill(root: &Path, delta: &Doc, reply: &Draft) -> Result<(), String> {
    let index = index_text(&plan::read(root, delta, INDEX_PART), reply);
    write(root, delta, INDEX_PART, &index)?;
    write(
        root,
        delta,
        DESIGN_PART,
        &format!("{}\n", reply.design.trim()),
    )?;
    write(root, delta, CLAIMS_PART, &claims_text(&reply.claims))?;
    for spec in &reply.changes {
        lazyspec(root, &["link", &delta.id, CHANGES, spec])?;
    }
    Ok(())
}

/// `lazyspec delete` removes only the index, so the parts go with the directory.
fn delete(root: &Path, delta: &Doc) {
    let _ = lazyspec(root, &["delete", &delta.id]);
    if let Some(dir) = Path::new(&delta.path).parent() {
        let _ = std::fs::remove_dir_all(root.join(dir));
    }
}

/// Every revision is reviewed again; findings from the last call are left for the strip.
fn review_rounds(
    root: &Path,
    id: &str,
    rounds: u32,
    config: &runner::Config,
    used: &mut u32,
) -> Result<(), String> {
    for round in 1..=rounds {
        *used = round;
        let (store, delta) = build::load_delta(root, id)?;
        let mut values = context::assemble(&store, root, &delta);
        let resolved = reviews::resolved(root, &delta);
        values.insert("resolved", reviews::render(&resolved));
        let prompt = prompts::rendered(Prompt::ReviewDoc, root, &values)?;
        println!("review round {round}/{rounds}");
        match review::review_draft(root, &delta, &prompt, &resolved, config)? {
            Outcome::Clean(_) => {
                println!("review clean");
                return Ok(());
            }
            Outcome::Written(n) => println!("{n} review lines written"),
        }
        if round < rounds {
            revise(root, id, config)?;
        }
    }
    Ok(())
}

fn revise(root: &Path, id: &str, config: &runner::Config) -> Result<(), String> {
    let (store, delta) = build::load_delta(root, id)?;
    let values = context::assemble(&store, root, &delta);
    let prompt = prompts::rendered(Prompt::Revise, root, &values)?;
    let env = [("LEMMA_DELTA", id)];
    let reply = runner::tracked(
        "revise",
        &config.runner,
        &prompt,
        root,
        &env,
        config.timeout,
        Some(&schema::revise()),
        config.mode,
        config.spinner,
    )?;
    let reply: Revision =
        serde_json::from_value(reply.json()?).map_err(|e| format!("revise reply: {e}"))?;
    let index = plan::read(root, &delta, INDEX_PART);
    let front = &index[..frontmatter_len(&index)];
    write(
        root,
        &delta,
        INDEX_PART,
        &format!("{front}\n{}\n", reply.index.trim()),
    )?;
    write(
        root,
        &delta,
        DESIGN_PART,
        &format!("{}\n", reply.design.trim()),
    )?;
    write(
        root,
        &delta,
        CLAIMS_PART,
        &format!("{}\n", reply.claims.trim()),
    )
}

/// Proceeds anyway past findings the rounds left (DECISION-015), so plan's no-review-lines rule holds.
fn strip_review_lines(root: &Path, id: &str) -> Result<(), String> {
    let (_, delta) = build::load_delta(root, id)?;
    for part in PARTS {
        let file = plan::read(root, &delta, part);
        let front = match part {
            INDEX_PART => frontmatter_len(&file),
            _ => 0,
        };
        let (front, body) = file.split_at(front);
        let review = review_lines(body);
        if review.is_empty() {
            continue;
        }
        let mut kept = String::from(front);
        for (n, line) in body.split_inclusive('\n').enumerate() {
            match review.contains(&(n + 1)) {
                true => eprintln!("warning: {id} {part}: unresolved {}", line.trim()),
                false => kept.push_str(line),
            }
        }
        write(root, &delta, part, &kept)?;
    }
    Ok(())
}

fn plan(root: &Path, id: &str, config: &runner::Config) -> Result<(), String> {
    let (store, delta) = build::load_delta(root, id)?;
    let values = context::assemble(&store, root, &delta);
    let mut asked = 0;
    let mut rejected = None;
    loop {
        match plan::plan(
            root,
            &delta,
            &values,
            config,
            rejected.as_deref(),
            &mut asked,
        ) {
            Ok(report) => {
                report.lines().iter().for_each(|line| println!("{line}"));
                return Ok(());
            }
            Err(plan::Error::Failed(message)) => return Err(message),
            Err(plan::Error::Rejected(message)) if asked > config.vibe_repairs => {
                return Err(message)
            }
            Err(plan::Error::Rejected(message)) => rejected = Some(message),
        }
    }
}

fn accept(root: &Path, id: &str) -> Result<usize, String> {
    let (_, delta) = build::load_delta(root, id)?;
    let (claims, accepted) = accept_proposed(&plan::read(root, &delta, CLAIMS_PART));
    write(root, &delta, CLAIMS_PART, &claims)?;
    println!("accepted {accepted} [proposed] claims");
    Ok(accepted)
}

/// Gate errors go to revise as review lines until the gate passes or the repairs run out;
/// then the move reports whatever errors are left.
fn in_progress(root: &Path, id: &str, config: &runner::Config) -> Result<(), String> {
    let mut repairs = 0;
    loop {
        let (store, delta) = build::load_delta(root, id)?;
        let errors: Vec<_> = advance::gate_findings(&store, root, id)
            .into_iter()
            .filter(lint::Finding::is_error)
            .collect();
        if errors.is_empty() || repairs == config.vibe_repairs {
            return advance::unasked(root, &store, &delta);
        }
        repairs += 1;
        errors.iter().for_each(|f| eprintln!("{}", f.render()));
        annotate_gate_errors(root, &delta, &errors)?;
        revise(root, id, config)?;
        strip_review_lines(root, id)?;
    }
}

fn annotate_gate_errors(root: &Path, delta: &Doc, errors: &[lint::Finding]) -> Result<(), String> {
    for error in errors {
        let part = finding_part(delta, error);
        let file = plan::read(root, delta, part);
        let annotated = annotate(part, &file, "", &error.message)
            .ok_or_else(|| format!("cannot annotate {part}: {}", error.message))?;
        write(root, delta, part, &annotated)?;
    }
    Ok(())
}

/// Revise rewrites index, design and claims, so a finding it cannot place lands in the index.
fn finding_part(delta: &Doc, finding: &lint::Finding) -> &'static str {
    let named = |part: &&str| {
        finding.part.as_deref() == Some(*part)
            || finding
                .path
                .as_deref()
                .is_some_and(|path| path.ends_with(&part_file(delta, part)))
    };
    [DESIGN_PART, CLAIMS_PART]
        .into_iter()
        .find(|part| named(part))
        .unwrap_or(INDEX_PART)
}

/// A stopped task is reset and re-planned, and the build runs again, until a task at depth
/// `vibe_repairs` stops; depths live only for this run, keyed by task id.
fn run_build(root: &Path, id: &str, config: &runner::Config) -> Result<(), Stopped> {
    let mut depths: HashMap<String, u32> = HashMap::new();
    loop {
        let untracked = untracked_files(root);
        let (task, base) = match build::run(root, id, None, false, config) {
            Ok(build::Outcome::Complete) => return Ok(()),
            Ok(build::Outcome::Stopped { task, base, .. }) => (task, base),
            Err(message) => return Err(build_stopped(root, id, Some(message))),
        };
        let depth = depths.get(&task).copied().unwrap_or(0);
        if depth == config.vibe_repairs {
            return Err(build_stopped(root, id, None));
        }
        reset_task(root, &base, &untracked).map_err(|m| build_stopped(root, id, Some(m)))?;
        let replaced =
            replan(root, id, &task, config).map_err(|m| build_stopped(root, id, Some(m)))?;
        depths.extend(replaced.into_iter().map(|t| (t, depth + 1)));
    }
}

fn build_stopped(root: &Path, id: &str, message: Option<String>) -> Stopped {
    let next = match builds::read(root, id) {
        Some(_) => format!("lemma build {id} --resume"),
        None => format!("lemma build {id}"),
    };
    Stopped {
        message,
        next: Some(next),
    }
}

/// Discards the stopped task's diff: tracked files back to its base, and the untracked
/// files it created removed, keeping those that were `before` it.
fn reset_task(root: &Path, base: &str, before: &[String]) -> Result<(), String> {
    build::git_run(root, &["reset", "-q", "--hard", base])
        .map_err(|e| format!("cannot reset to {base}: {e}"))?;
    for path in untracked_files(root) {
        if before.contains(&path) {
            continue;
        }
        std::fs::remove_file(root.join(&path)).map_err(|e| format!("cannot remove {path}: {e}"))?;
    }
    Ok(())
}

/// Replaces the stopped task in `tasks.md`, commits it and returns the new task ids.
fn replan(
    root: &Path,
    id: &str,
    task: &str,
    config: &runner::Config,
) -> Result<Vec<String>, String> {
    let findings = builds::read(root, id)
        .map(|s| s.findings)
        .unwrap_or_default();
    builds::remove(root, id);
    let (store, delta) = build::load_delta(root, id)?;
    let before = tasks::parse(delta.part_body(TASKS_PART));
    let stopped = before
        .named(task)
        .ok_or(format!("{task} is not in {id} tasks.md"))?;
    let values = context::assemble(&store, root, &delta);
    let mut repairs = 0;
    let mut rejected = None;
    loop {
        match plan::replan(
            root,
            &delta,
            &values,
            config,
            stopped,
            &findings,
            rejected.as_deref(),
        ) {
            Ok(_) => break,
            Err(plan::Error::Failed(message)) => return Err(message),
            Err(plan::Error::Rejected(message)) if repairs == config.vibe_repairs => {
                return Err(message)
            }
            Err(plan::Error::Rejected(message)) => rejected = Some(message),
        }
        repairs += 1;
    }
    let (_, delta) = build::load_delta(root, id)?;
    let replaced: Vec<String> = tasks::parse(delta.part_body(TASKS_PART))
        .tasks
        .into_iter()
        .filter(|t| before.named(&t.id).is_none())
        .map(|t| t.id)
        .collect();
    println!("re-planned {task} as {}", replaced.join(", "));
    let path = delta.part_path(TASKS_PART);
    build::git_run(root, &["add", "--", &path])
        .and_then(|_| {
            build::git_run(
                root,
                &["commit", "-q", "-m", &format!("{id}: re-plan {task}")],
            )
        })
        .map_err(|e| format!("re-plan of {task} not committed: {e}"))?;
    Ok(replaced)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testgit::Repo;

    #[test]
    fn a_reset_keeps_untracked_files_from_before_the_task_and_removes_the_tasks_own() {
        let repo = Repo::new();
        repo.write("src/lib.rs", "pub fn a() {}\n");
        let base = repo.commit("init");
        repo.write("notes.txt", "mine\n");
        let before = untracked_files(repo.root());
        repo.write("src/lib.rs", "changed\n");
        repo.write("scratch/new.txt", "x\n");

        reset_task(repo.root(), &base, &before).unwrap();

        assert_eq!(
            std::fs::read_to_string(repo.path("src/lib.rs")).unwrap(),
            "pub fn a() {}\n"
        );
        assert!(!repo.path("scratch/new.txt").exists());
        assert_eq!(
            std::fs::read_to_string(repo.path("notes.txt")).unwrap(),
            "mine\n"
        );
    }
}
