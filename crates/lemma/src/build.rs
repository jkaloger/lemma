use crate::claims::{self, Claim};
use crate::evidence::{self, RefStatus};
use crate::git::git_output;
use crate::lint::{self, ref_name};
use crate::payload::{Doc, CLAIMS_PART, TASKS_PART};
use crate::prompts::{self, Prompt};
use crate::review::{part_file, PARTS};
use crate::tasks::{self, Task};
use crate::world::World;
use crate::{context, plan, project, runner, schema};
use serde::Deserialize;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

const LINT_CONTINUATION: &str = "         ";

pub enum Outcome {
    Complete,
    Stopped,
}

struct Failure {
    detail: Vec<String>,
    ticked: bool,
}

fn unticked(detail: Vec<String>) -> Failure {
    Failure {
        detail,
        ticked: false,
    }
}

fn load_delta(root: &Path, id: &str) -> Result<(lazyspec::engine::store::Store, Doc), String> {
    let store = project::load_store(root).map_err(|e| format!("{e:#}"))?;
    let delta = World::load(&store, None)
        .docs()
        .iter()
        .find(|d| d.doc_type == "delta" && d.is_named(id))
        .cloned()
        .ok_or(format!("unknown delta {id}"))?;
    Ok((store, delta))
}

fn changed_paths(root: &Path) -> usize {
    git_output(root, &["status", "--porcelain", "--untracked-files=all"])
        .unwrap_or_default()
        .lines()
        .count()
}

fn elapsed_text(elapsed: Duration) -> String {
    let secs = elapsed.as_secs();
    match secs >= 60 {
        true => format!("{}m{:02}s", secs / 60, secs % 60),
        false => format!("{secs}s"),
    }
}

fn heading(task: &Task) -> String {
    match task.chore {
        true => format!("{} {} (chore)", task.id, task.text),
        false => format!(
            "{} {} (settles {})",
            task.id,
            task.text,
            task.settles.join(", ")
        ),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum Verdict {
    Done,
    Blocked,
}

#[derive(Deserialize)]
struct Reply {
    outcome: Verdict,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    refs: Vec<RefEntry>,
}

#[derive(Deserialize)]
struct RefEntry {
    claim: String,
    path: String,
    symbol: String,
}

fn delta_parts(root: &Path, delta: &Doc) -> Vec<(String, String)> {
    PARTS
        .iter()
        .map(|part| (part_file(delta, part), plan::read(root, delta, part)))
        .collect()
}

fn reject_part_edits(root: &Path, delta: &Doc, before: &[(String, String)]) -> Result<(), Failure> {
    let after = delta_parts(root, delta);
    let Some(((path, _), _)) = before.iter().zip(&after).find(|(b, a)| b.1 != a.1) else {
        return Ok(());
    };
    Err(unticked(vec![format!(
        "   runner edited {path}; lemma writes delta parts"
    )]))
}

fn with_refs(text: &str, task: &Task, refs: &[RefEntry]) -> Result<String, String> {
    let mut text = text.to_string();
    for entry in refs {
        if !task.settles.contains(&entry.claim) {
            return Err(format!("   {} does not settle {}", task.id, entry.claim));
        }
        let parsed = claims::parse(&text);
        let claim = parsed
            .claims
            .iter()
            .find(|c| c.id == entry.claim && !c.struck)
            .ok_or_else(|| format!("   {} is not in claims.md", entry.claim))?;
        let written = claim
            .receipts
            .iter()
            .any(|r| r.path == entry.path && r.symbol.as_deref() == Some(&entry.symbol));
        if written {
            continue;
        }
        let line = format!("  @ref {}", ref_name(&entry.path, Some(&entry.symbol)));
        let mut lines = claims::lines_of(&text);
        lines.insert(claim.end, &line);
        text = lines.join("\n");
    }
    Ok(text)
}

fn write_refs(root: &Path, delta: &Doc, task: &Task, refs: &[RefEntry]) -> Result<(), Failure> {
    let before = plan::read(root, delta, CLAIMS_PART);
    let after = with_refs(&before, task, refs).map_err(|line| unticked(vec![line]))?;
    if after == before {
        return Ok(());
    }
    let path = root.join(delta.part_path(CLAIMS_PART));
    std::fs::write(&path, after)
        .map_err(|e| unticked(vec![format!("   cannot write {}: {e}", path.display())]))
}

fn runner_reply(
    root: &Path,
    delta: &Doc,
    task: &Task,
    prompt: &str,
    config: &runner::Config,
) -> Result<Reply, Failure> {
    let failed = |message: String| unticked(vec![format!("   runner: {message}")]);
    let env = [("LEMMA_DELTA", delta.id.as_str()), ("LEMMA_TASK", &task.id)];
    let schema = schema::build(task);
    let started = std::time::Instant::now();
    let reply = runner::tracked(
        &format!("{} runner", task.id),
        &config.runner,
        prompt,
        root,
        &env,
        config.timeout,
        Some(&schema),
    )
    .map_err(failed)?;
    println!(
        "   runner: {}, {} files",
        elapsed_text(started.elapsed()),
        changed_paths(root)
    );
    serde_json::from_value(reply.json().map_err(failed)?).map_err(|e| failed(format!("reply: {e}")))
}

fn run_runner(
    root: &Path,
    store: &lazyspec::engine::store::Store,
    delta: &Doc,
    task: &Task,
    config: &runner::Config,
) -> Result<(), Failure> {
    let mut values = context::assemble(store, root, delta);
    values.insert("task", context::task_text(delta, task));
    let prompt = prompts::rendered(Prompt::Build, root, &values)
        .map_err(|message| unticked(vec![format!("   runner: {message}")]))?;
    let parts = delta_parts(root, delta);
    let reply = runner_reply(root, delta, task, &prompt, config)?;
    reject_part_edits(root, delta, &parts)?;
    if let Verdict::Blocked = reply.outcome {
        return Err(unticked(vec![format!(
            "   runner blocked: {}",
            reply.reason
        )]));
    }
    write_refs(root, delta, task, &reply.refs)
}

fn check_lint(root: &Path, id: &str) -> Result<(), Failure> {
    let (store, _) = load_delta(root, id).map_err(|message| unticked(vec![message]))?;
    let options = lint::Options {
        delta: Some(id),
        ..lint::Options::default()
    };
    let findings = lint::run(&store, root, &options, None);
    let mut errors = findings.iter().filter(|f| f.is_error()).map(|f| f.render());
    let Some(first) = errors.next() else {
        println!("{}", console::style("   lint: clean").green());
        return Ok(());
    };
    let rest = errors.map(|line| format!("{LINT_CONTINUATION}{line}"));
    Err(unticked(
        std::iter::once(format!("   lint: {first}"))
            .chain(rest)
            .collect(),
    ))
}

fn claim_evidence(root: &Path, tests: &[String], claim: &Claim) -> Result<Vec<String>, String> {
    if claim.intent {
        return Ok(vec![format!("   {} [intent]", claim.id)]);
    }
    if claim.receipts.is_empty() {
        return Err(format!("   {} has no @ref", claim.id));
    }
    claim
        .receipts
        .iter()
        .map(|r| {
            let name = ref_name(&r.path, r.symbol.as_deref());
            match evidence::resolve(root, &r.path, r.symbol.as_deref(), tests) {
                RefStatus::Ok => Ok(format!("   {} @ref {name} resolves", claim.id)),
                RefStatus::Unresolved(why) => Err(format!(
                    "   {} @ref {name} does not resolve: {why}",
                    claim.id
                )),
                RefStatus::NotATest => Err(format!(
                    "   {} @ref {name} does not resolve: not in the governing spec's `tests` globs",
                    claim.id
                )),
            }
        })
        .collect()
}

fn check_settled(root: &Path, id: &str, task: &Task) -> Result<(), Failure> {
    if task.chore {
        return Ok(());
    }
    let (store, delta) = load_delta(root, id).map_err(|message| unticked(vec![message]))?;
    let tests = World::load(&store, None).delta_tests(&delta);
    let parsed = claims::parse(delta.part_body(CLAIMS_PART));
    for settled in &task.settles {
        let claim = parsed
            .claims
            .iter()
            .find(|c| &c.id == settled && !c.struck)
            .ok_or_else(|| unticked(vec![format!("   {settled} is not in claims.md")]))?;
        let lines = claim_evidence(root, &tests, claim).map_err(|line| unticked(vec![line]))?;
        lines.iter().for_each(|line| println!("{line}"));
    }
    Ok(())
}

fn ticked_text(text: &str, task: &Task) -> String {
    text.split_inclusive('\n')
        .enumerate()
        .map(|(i, line)| match i + 1 == task.line {
            true => line.replacen("- [ ]", "- [x]", 1),
            false => line.to_string(),
        })
        .collect()
}

fn tick(root: &Path, delta: &Doc, task: &Task) -> Result<(), Failure> {
    let path = root.join(delta.part_path(TASKS_PART));
    let text = plan::read(root, delta, TASKS_PART);
    std::fs::write(&path, ticked_text(&text, task))
        .map_err(|e| unticked(vec![format!("   cannot write {}: {e}", path.display())]))
}

fn git_run(root: &Path, args: &[&str]) -> Result<(), String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|e| format!("cannot start git: {e}"))?;
    match out.status.success() {
        true => Ok(()),
        false => Err(String::from_utf8_lossy(&out.stderr).trim().to_string()),
    }
}

fn commit(root: &Path, delta: &Doc, task: &Task) -> Result<(), Failure> {
    let message = format!("{} {}: {}", delta.id, task.id, task.text);
    let failed = |why: String| Failure {
        detail: vec![format!("   commit failed: {why}")],
        ticked: true,
    };
    git_run(root, &["add", "-A"]).map_err(failed)?;
    git_run(root, &["commit", "-q", "-m", &message]).map_err(failed)?;
    let sha = git_output(root, &["rev-parse", "--short", "HEAD"]).unwrap_or_default();
    println!(
        "   {} {} {message}",
        console::style("commit").green(),
        sha.trim()
    );
    Ok(())
}

fn build_task(
    root: &Path,
    store: &lazyspec::engine::store::Store,
    delta: &Doc,
    task: &Task,
    config: &runner::Config,
) -> Result<(), Failure> {
    println!("{}", console::style(heading(task)).bold());
    run_runner(root, store, delta, task, config)?;
    check_lint(root, &delta.id)?;
    check_settled(root, &delta.id, task)?;
    tick(root, delta, task)?;
    commit(root, delta, task)
}

fn report(failure: &Failure, task: &Task) {
    let red = |line: &str| println!("{}", console::style(line).red());
    failure.detail.iter().for_each(|line| red(line));
    match failure.ticked {
        true => red(&format!(
            "   stopped. {} ticked but not committed; working tree kept.",
            task.id
        )),
        false => red(&format!(
            "   stopped. {} left unticked; working tree kept.",
            task.id
        )),
    }
}

pub fn run(
    root: &Path,
    id: &str,
    only: Option<&str>,
    config: &runner::Config,
) -> Result<Outcome, String> {
    if !git_output(root, &["status", "--porcelain"])
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        return Err("build needs a clean working tree; commit or stash first".to_string());
    }
    let (_, delta) = load_delta(root, id)?;
    let parsed = tasks::parse(delta.part_body(TASKS_PART));
    if let Some(name) = only {
        let task = parsed.named(name).ok_or(format!("unknown task {name}"))?;
        if task.ticked {
            println!("{name} is already ticked");
            return Ok(Outcome::Complete);
        }
    }
    loop {
        let (store, delta) = load_delta(root, id)?;
        let parsed = tasks::parse(delta.part_body(TASKS_PART));
        let next = match only {
            Some(name) => parsed.named(name),
            None => parsed.first_unticked(),
        };
        let Some(task) = next.filter(|t| !t.ticked) else {
            println!("all tasks complete");
            return Ok(Outcome::Complete);
        };
        if let Err(failure) = build_task(root, &store, &delta, task, config) {
            report(&failure, task);
            return Ok(Outcome::Stopped);
        }
        if only.is_some() {
            return Ok(Outcome::Complete);
        }
    }
}
