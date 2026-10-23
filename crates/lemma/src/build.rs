use crate::claims::{self, Claim};
use crate::evidence::{self, RefStatus};
use crate::gate;
use crate::git::{changed_since, git_output};
use crate::lint::{self, ref_name};
use crate::output::{counted, Block, Committed, Finish, Glyph, Look, Mode, ReviewOutcome, Tone};
use crate::payload::{Doc, CLAIMS_PART, TASKS_PART};
use crate::prompts::{self, Prompt};
use crate::review::{part_file, PARTS};
use crate::tasks::{self, Task};
use crate::world::World;
use crate::{builds, context, plan, project, runner, schema};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;
use std::time::Duration;

const LINT_CONTINUATION: &str = "         ";

pub enum Outcome {
    Complete,
    Stopped,
}

enum Ending {
    Unticked,
    /// Unticked by a check failure, which a fix round may clear.
    Checked,
    Ticked,
    Reviewed(String),
}

struct Failure {
    detail: Vec<String>,
    ending: Ending,
    findings: Vec<gate::Finding>,
    transcript: Option<String>,
    session: Option<String>,
}

impl Failure {
    fn at(self, transcript: Option<String>) -> Failure {
        Failure { transcript, ..self }
    }

    fn in_session(self, session: Option<&str>) -> Failure {
        Failure {
            session: self.session.or(session.map(str::to_string)),
            ..self
        }
    }

    fn record_findings(&self) -> Vec<gate::Finding> {
        if !self.findings.is_empty() {
            return self.findings.clone();
        }
        self.detail
            .iter()
            .map(|line| gate::Finding {
                text: line.trim().to_string(),
                basis: "task".to_string(),
            })
            .collect()
    }

    /// Findings a failure already carries stand; otherwise each diagnostic
    /// that names a claim rests on it, any other on the task.
    fn with_check_findings(self, claim_ids: &[String]) -> Failure {
        if !self.findings.is_empty() {
            return self;
        }
        let findings = self
            .detail
            .iter()
            .map(|line| {
                let text = line.trim().to_string();
                let basis = named_claim(&text, claim_ids).unwrap_or("task").to_string();
                gate::Finding { text, basis }
            })
            .collect();
        Failure { findings, ..self }
    }
}

fn named_claim<'a>(text: &str, claim_ids: &'a [String]) -> Option<&'a str> {
    text.split(|c: char| !c.is_ascii_alphanumeric() && c != '-')
        .find_map(|word| claim_ids.iter().find(|id| *id == word))
        .map(String::as_str)
}

fn unticked(detail: Vec<String>) -> Failure {
    Failure {
        detail,
        ending: Ending::Unticked,
        findings: Vec::new(),
        transcript: None,
        session: None,
    }
}

fn check_failed(detail: Vec<String>) -> Failure {
    Failure {
        ending: Ending::Checked,
        ..unticked(detail)
    }
}

/// Splits a check failure, which goes to a fix round, from a stop.
fn checked(result: Result<(), Failure>) -> Result<Option<Failure>, Failure> {
    match result {
        Ok(()) => Ok(None),
        Err(failure) if matches!(failure.ending, Ending::Checked) => Ok(Some(failure)),
        Err(failure) => Err(failure),
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

type TreeState = BTreeMap<String, Option<Vec<u8>>>;

fn tree_state(root: &Path) -> TreeState {
    changed_since(root, "HEAD")
        .into_iter()
        .map(|path| {
            let content = std::fs::read(root.join(&path)).ok();
            (path, content)
        })
        .collect()
}

fn changed_between(before: &TreeState, after: &TreeState) -> usize {
    before
        .keys()
        .chain(after.keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|path| before.get(*path) != after.get(*path))
        .count()
}

pub(crate) fn elapsed_text(elapsed: Duration) -> String {
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
    Err(check_failed(vec![format!(
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

fn write_refs(
    root: &Path,
    delta: &Doc,
    task: &Task,
    base: &str,
    refs: &[RefEntry],
) -> Result<(), Failure> {
    let after = with_refs(base, task, refs).map_err(|line| unticked(vec![line]))?;
    if after == plan::read(root, delta, CLAIMS_PART) {
        return Ok(());
    }
    let path = root.join(delta.part_path(CLAIMS_PART));
    std::fs::write(&path, after)
        .map_err(|e| unticked(vec![format!("   cannot write {}: {e}", path.display())]))
}

struct Built {
    prompt: String,
    session: Option<String>,
    diff_base: Option<String>,
    claims: String,
    parts: Vec<(String, String)>,
}

impl Built {
    fn claim_ids(&self) -> Vec<String> {
        claims::parse(&self.claims)
            .claims
            .into_iter()
            .filter(|c| !c.struck)
            .map(|c| c.id)
            .collect()
    }
}

struct Call<'a> {
    name: &'a str,
    prompt: &'a str,
    resume: Option<&'a str>,
}

fn runner_reply(
    root: &Path,
    delta: &Doc,
    task: &Task,
    call: Call,
    config: &runner::Config,
    block: &Block,
) -> Result<(Reply, Option<String>), Failure> {
    let Call {
        name,
        prompt,
        resume,
    } = call;
    let failed = |message: String| check_failed(vec![format!("   {name}: {message}")]);
    let ran = |failed: runner::Failed| failed_call(name, failed);
    let env = [("LEMMA_DELTA", delta.id.as_str()), ("LEMMA_TASK", &task.id)];
    let schema = schema::build(task);
    let label = format!("{} {name}", task.id);
    let started = std::time::Instant::now();
    let before = tree_state(root);
    let reply = match resume {
        Some(session) => runner::resumed(
            &label,
            &config.resume,
            session,
            prompt,
            root,
            &env,
            config.timeout,
            Some(&schema),
            config.mode,
        ),
        None => runner::tracked(
            &label,
            &config.runner,
            prompt,
            root,
            &env,
            config.timeout,
            Some(&schema),
            config.mode,
        ),
    }
    .map_err(ran)?;
    let look = Look::current(config.mode);
    block.say(&look.step(
        Glyph::Pass,
        &format!(
            "   {name}: {}, {} files",
            elapsed_text(started.elapsed()),
            changed_between(&before, &tree_state(root))
        ),
    ));
    let after = |message: String| {
        failed(message)
            .at(runner::transcript_line(
                root,
                reply.session.as_deref(),
                config.mode,
            ))
            .in_session(reply.session.as_deref())
    };
    let parsed = serde_json::from_value(reply.json().map_err(after)?)
        .map_err(|e| after(format!("reply: {e}")))?;
    Ok((parsed, reply.session))
}

fn failed_call(name: &str, failed: runner::Failed) -> Failure {
    unticked(vec![format!("   {name}: {}", failed.message)])
        .at(failed.transcript)
        .in_session(failed.session.as_deref())
}

fn accept(
    root: &Path,
    delta: &Doc,
    task: &Task,
    name: &str,
    built: &mut Built,
    reply: &Reply,
) -> Result<(), Failure> {
    if let Verdict::Blocked = reply.outcome {
        return Err(unticked(vec![format!(
            "   {name} blocked: {}",
            reply.reason
        )]));
    }
    reject_part_edits(root, delta, &built.parts)?;
    write_refs(root, delta, task, &built.claims, &reply.refs)?;
    built.parts = delta_parts(root, delta);
    Ok(())
}

/// Runs a runner call, accepts its reply and checks the work. A check failure
/// is returned for a fix round; anything else stops.
fn reply_and_check(
    root: &Path,
    delta: &Doc,
    task: &Task,
    built: &mut Built,
    call: Call,
    config: &runner::Config,
    block: &Block,
) -> Result<Option<Failure>, Failure> {
    let name = call.name;
    let (reply, session) = match runner_reply(root, delta, task, call, config, block) {
        Ok(replied) => replied,
        Err(failure) => {
            built.session = built.session.take().or(failure.session.clone());
            return checked(Err(failure));
        }
    };
    built.session = built.session.take().or(session.clone());
    let attach = |f: Failure| {
        f.at(runner::transcript_line(
            root,
            session.as_deref(),
            config.mode,
        ))
        .in_session(session.as_deref())
    };
    if let Some(failure) = checked(accept(root, delta, task, name, built, &reply).map_err(attach))?
    {
        return Ok(Some(failure));
    }
    checked(check_work(
        root,
        delta,
        task,
        config.check.as_deref().map(|check| (check, config.timeout)),
        &Look::current(config.mode),
        block,
    ))
}

fn build_prompt(
    root: &Path,
    store: &lazyspec::engine::store::Store,
    delta: &Doc,
    task: &Task,
) -> Result<String, Failure> {
    let mut values = context::assemble(store, root, delta);
    values.insert("task", context::task_text(delta, task));
    prompts::rendered(Prompt::Build, root, &values)
        .map_err(|message| unticked(vec![format!("   runner: {message}")]))
}

fn run_runner(
    root: &Path,
    store: &lazyspec::engine::store::Store,
    delta: &Doc,
    task: &Task,
    config: &runner::Config,
    block: &Block,
) -> Result<(Built, Option<Failure>), Failure> {
    let prompt = build_prompt(root, store, delta, task)?;
    let mut built = Built {
        prompt: prompt.clone(),
        session: None,
        diff_base: None,
        claims: plan::read(root, delta, CLAIMS_PART),
        parts: delta_parts(root, delta),
    };
    let call = Call {
        name: "runner",
        prompt: &prompt,
        resume: None,
    };
    let failed = reply_and_check(root, delta, task, &mut built, call, config, block)?;
    Ok((built, failed))
}

/// A resume checks the tree as the stop and the human left it; no runner runs
/// unless the checks send it to a fix round.
fn resume_checks(
    root: &Path,
    store: &lazyspec::engine::store::Store,
    delta: &Doc,
    task: &Task,
    stopped: &builds::Stopped,
    look: &Look,
    block: &Block,
) -> Result<(Built, Option<Failure>), Failure> {
    block.say(&look.step(
        Glyph::Info,
        &format!(
            "   resuming {} (stopped {}: {})",
            task.id,
            stopped.at,
            stopped.reason()
        ),
    ));
    for path in stopped.changed_paths(root) {
        block.say(&look.step(Glyph::Info, &format!("   changed since the stop: {path}")));
    }
    let built = Built {
        prompt: build_prompt(root, store, delta, task)?,
        session: stopped.session.clone(),
        diff_base: Some(stopped.base.clone()),
        claims: plan::read(root, delta, CLAIMS_PART),
        parts: delta_parts(root, delta),
    };
    let failed = checked(check_work(root, delta, task, None, look, block))?;
    Ok((built, failed))
}

fn findings_list(findings: &[gate::Finding]) -> String {
    findings
        .iter()
        .map(|f| format!("- [{}] {}", f.basis, f.text))
        .collect::<Vec<_>>()
        .join("\n")
}

fn run_fix(
    root: &Path,
    delta: &Doc,
    task: &Task,
    built: &mut Built,
    findings: &[gate::Finding],
    config: &runner::Config,
    block: &Block,
) -> Result<Option<Failure>, Failure> {
    let values = BTreeMap::from([("findings", findings_list(findings))]);
    let fix = prompts::rendered(Prompt::Fix, root, &values)
        .map_err(|message| unticked(vec![format!("   fix: {message}")]))?;
    let session = built
        .session
        .clone()
        .filter(|_| runner::resumable(&config.resume));
    let prompt = match session {
        Some(_) => fix,
        None => format!("{}\n\n{fix}", built.prompt),
    };
    let call = Call {
        name: "fix",
        prompt: &prompt,
        resume: session.as_deref(),
    };
    reply_and_check(root, delta, task, built, call, config, block)
}

fn check_lint(root: &Path, id: &str, look: &Look, block: &Block) -> Result<(), Failure> {
    let (store, _) = load_delta(root, id).map_err(|message| unticked(vec![message]))?;
    let options = lint::Options {
        delta: Some(id),
        ..lint::Options::default()
    };
    let findings = lint::run(&store, root, &options, None);
    let mut errors = findings.iter().filter(|f| f.is_error()).map(|f| f.render());
    let Some(first) = errors.next() else {
        block.say(&look.step_toned(Glyph::Pass, "   lint: clean", Some(Tone::Green)));
        return Ok(());
    };
    let continuation = look.continuation(LINT_CONTINUATION);
    let rest = errors.map(|line| format!("{continuation}{line}"));
    Err(check_failed(
        std::iter::once(format!("   lint: {first}"))
            .chain(rest)
            .collect(),
    ))
}

fn claim_evidence(
    root: &Path,
    tests: &[String],
    claim: &Claim,
) -> Result<Vec<(Glyph, String)>, String> {
    if claim.intent {
        return Ok(vec![(Glyph::Info, format!("   {} [intent]", claim.id))]);
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
                RefStatus::Ok => Ok((Glyph::Pass, format!("   {} @ref {name} resolves", claim.id))),
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

fn check_settled(
    root: &Path,
    id: &str,
    task: &Task,
    look: &Look,
    block: &Block,
) -> Result<(), Failure> {
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
        let lines = claim_evidence(root, &tests, claim).map_err(|line| check_failed(vec![line]))?;
        lines
            .iter()
            .for_each(|(glyph, line)| block.say(&look.step(*glyph, line)));
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

pub(crate) fn git_run(root: &Path, args: &[&str]) -> Result<(), String> {
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

fn commit(
    root: &Path,
    delta: &Doc,
    task: &Task,
    trailer: &str,
    look: &Look,
    block: &Block,
) -> Result<String, Failure> {
    let message = format!("{} {}: {}", delta.id, task.id, task.text);
    let failed = |why: String| Failure {
        detail: vec![format!("   commit failed: {why}")],
        ending: Ending::Ticked,
        findings: Vec::new(),
        transcript: None,
        session: None,
    };
    git_run(root, &["add", "-A"]).map_err(failed)?;
    git_run(root, &["commit", "-q", "-m", &message, "-m", trailer]).map_err(failed)?;
    let sha = git_output(root, &["rev-parse", "--short", "HEAD"]).unwrap_or_default();
    let sha = sha.trim().to_string();
    match look.mode {
        Mode::Plain => block.say(&format!(
            "   {} {sha} {message}",
            console::style("commit").green()
        )),
        Mode::Live => block.say(&look.step(Glyph::Pass, &format!("   commit {sha} {message}"))),
    }
    Ok(sha)
}

fn reviewed_stop(verdict: &gate::Verdict, detail: Vec<String>, ending: String) -> Failure {
    let stop = verdict.stop.trim();
    let stopped = (!stop.is_empty()).then(|| gate::Finding {
        text: stop.to_string(),
        basis: "task".to_string(),
    });
    Failure {
        detail,
        ending: Ending::Reviewed(ending),
        findings: stopped
            .into_iter()
            .chain(verdict.findings.clone())
            .collect(),
        transcript: None,
        session: None,
    }
}

fn check_work(
    root: &Path,
    delta: &Doc,
    task: &Task,
    check: Option<(&str, Duration)>,
    look: &Look,
    block: &Block,
) -> Result<(), Failure> {
    check_lint(root, &delta.id, look, block)?;
    check_settled(root, &delta.id, task, look, block)?;
    match check {
        Some((command, timeout)) => check_command(root, command, timeout, look, block),
        None => Ok(()),
    }
}

fn check_command(
    root: &Path,
    command: &str,
    timeout: Duration,
    look: &Look,
    block: &Block,
) -> Result<(), Failure> {
    let ran = run_check(root, command, timeout)
        .map_err(|message| unticked(vec![format!("   check: {message}")]))?;
    let header = match ran.status {
        None => format!("check: {command} timed out after {}s", timeout.as_secs()),
        Some(status) if status.success() => {
            block.say(&look.step_toned(Glyph::Pass, "   check: pass", Some(Tone::Green)));
            return Ok(());
        }
        Some(status) => {
            let code = status
                .code()
                .map_or("by signal".to_string(), |code| code.to_string());
            format!("check: {command} exited {code}")
        }
    };
    Err(command_failed(header, &ran.output))
}

struct CheckRun {
    /// `None` when the check outlasted the timeout and was killed.
    status: Option<std::process::ExitStatus>,
    output: String,
}

/// Output goes to files, not pipes, so a killed check's children holding them open can't hang the read.
/// An error is a check that could not run, which the runner can't fix, so it stops the build.
fn run_check(root: &Path, command: &str, timeout: Duration) -> Result<CheckRun, String> {
    let argv = runner::split(command)?;
    let output_file = || tempfile::tempfile().map_err(|e| format!("cannot capture output: {e}"));
    let mut stdout = output_file()?;
    let mut stderr = output_file()?;
    let cannot_start = |e: std::io::Error| format!("cannot start `{}`: {e}", argv[0]);
    let mut child = Command::new(&argv[0])
        .args(&argv[1..])
        .current_dir(root)
        .stdin(std::process::Stdio::null())
        .stdout(stdout.try_clone().map_err(cannot_start)?)
        .stderr(stderr.try_clone().map_err(cannot_start)?)
        .spawn()
        .map_err(cannot_start)?;
    let started = std::time::Instant::now();
    let status = loop {
        match child
            .try_wait()
            .map_err(|e| format!("cannot wait on `{}`: {e}", argv[0]))?
        {
            Some(status) => break Some(status),
            None if started.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            None => std::thread::sleep(Duration::from_millis(20)),
        }
    };
    let output = [read_back(&mut stdout), read_back(&mut stderr)].concat();
    Ok(CheckRun { status, output })
}

fn read_back(file: &mut std::fs::File) -> String {
    use std::io::{Read, Seek};
    let mut bytes = Vec::new();
    let _ = file.rewind().and_then(|()| file.read_to_end(&mut bytes));
    String::from_utf8_lossy(&bytes).into_owned()
}

const CHECK_TAIL: usize = 40;

/// One finding on the task, so a claim id in the output never pins it.
fn command_failed(header: String, output: &str) -> Failure {
    let lines: Vec<&str> = output.lines().collect();
    let tail = &lines[lines.len().saturating_sub(CHECK_TAIL)..];
    let text = std::iter::once(header.as_str())
        .chain(tail.iter().copied())
        .collect::<Vec<_>>()
        .join("\n");
    let detail = std::iter::once(format!("   {header}"))
        .chain(tail.iter().map(|line| format!("   {line}")))
        .collect();
    Failure {
        findings: vec![gate::Finding {
            text,
            basis: "task".to_string(),
        }],
        ..check_failed(detail)
    }
}

fn print_fix_round(look: &Look, block: &Block, source: &str, count: usize, round: u32) {
    let found = format!("   {source}: {}", counted(count, "finding"));
    match look.mode {
        Mode::Plain => block.say(&format!("{found}, fix round {round}")),
        Mode::Live => {
            block.say(&look.step(Glyph::Fail, &found));
            block.say(&look.step(Glyph::Info, &format!("   fix round {round}")));
        }
    }
}

/// One reviewer call: `None` when clean, the findings when another fix round
/// is left, a stop otherwise.
#[allow(clippy::too_many_arguments)]
fn review_once(
    root: &Path,
    store: &lazyspec::engine::store::Store,
    delta: &Doc,
    task: &Task,
    diff_base: Option<&str>,
    rounds: u32,
    config: &runner::Config,
    block: &Block,
) -> Result<Option<Vec<gate::Finding>>, Failure> {
    let look = &Look::current(config.mode);
    let (verdict, session) =
        gate::review(root, store, delta, task, diff_base, config).map_err(|failed| {
            let runner_session_only = runner::Failed {
                session: None,
                ..failed
            };
            failed_call("review", runner_session_only)
        })?;
    let transcript = runner::transcript_line(root, session.as_deref(), config.mode);
    if !verdict.stop.trim().is_empty() {
        let ending = format!("review: {}", verdict.stop.trim());
        return Err(reviewed_stop(&verdict, Vec::new(), ending).at(transcript));
    }
    let count = verdict.findings.len();
    if count == 0 {
        block.say(&look.step_toned(Glyph::Pass, "   review: clean", Some(Tone::Green)));
        return Ok(None);
    }
    if rounds < config.review_rounds {
        return Ok(Some(verdict.findings));
    }
    block.say(&look.step(
        Glyph::Fail,
        &format!("   review: {}", counted(count, "finding")),
    ));
    let detail = verdict
        .findings
        .iter()
        .map(|f| format!("   {}: {}", f.basis, f.text))
        .collect();
    let ending = format!(
        "left unticked after {}",
        counted(rounds as usize, "fix round")
    );
    Err(reviewed_stop(&verdict, detail, ending).at(transcript))
}

/// Check failures and reviewer findings share one `review_rounds` budget.
#[allow(clippy::too_many_arguments)]
fn review_gate(
    root: &Path,
    store: &lazyspec::engine::store::Store,
    delta: &Doc,
    task: &Task,
    built: &mut Built,
    failed: Option<Failure>,
    config: &runner::Config,
    block: &Block,
) -> Result<ReviewOutcome, Failure> {
    let look = &Look::current(config.mode);
    let (mut fixed, mut rounds) = (0, 0);
    let mut failed = failed;
    loop {
        let (source, findings) = match failed.take() {
            Some(failure) => {
                let failure = failure.with_check_findings(&built.claim_ids());
                if rounds == config.review_rounds {
                    return Err(failure);
                }
                failure
                    .detail
                    .iter()
                    .for_each(|line| block.say(&look.step(Glyph::Fail, line)));
                ("check", failure.findings)
            }
            None => match review_once(
                root,
                store,
                delta,
                task,
                built.diff_base.as_deref(),
                rounds,
                config,
                block,
            )? {
                None => {
                    return Ok(ReviewOutcome {
                        findings: fixed,
                        rounds,
                    })
                }
                Some(findings) => ("review", findings),
            },
        };
        rounds += 1;
        fixed += findings.len();
        print_fix_round(look, block, source, findings.len(), rounds);
        failed = run_fix(root, delta, task, built, &findings, config, block)?;
    }
}

fn build_task(
    root: &Path,
    store: &lazyspec::engine::store::Store,
    delta: &Doc,
    task: &Task,
    stopped: Option<&builds::Stopped>,
    config: &runner::Config,
    block: &Block,
) -> Result<(), Failure> {
    let look = &Look::current(config.mode);
    let started = std::time::Instant::now();
    block.say(&look.tone(Tone::Bold, &heading(task)));
    let (mut built, failed) = match stopped {
        Some(stopped) => resume_checks(root, store, delta, task, stopped, look, block)
            .map_err(|f| f.in_session(stopped.session.as_deref()))?,
        None => run_runner(root, store, delta, task, config, block)?,
    };
    let review = review_gate(root, store, delta, task, &mut built, failed, config, block);
    let in_built = |f: Failure| f.in_session(built.session.as_deref());
    let review = review.map_err(in_built)?;
    tick(root, delta, task).map_err(in_built)?;
    let sha = commit(root, delta, task, &review.trailer(), look, block).map_err(in_built)?;
    builds::remove(root, &delta.id);
    let elapsed = elapsed_text(started.elapsed());
    let committed = Committed {
        id: &task.id,
        title: &task.text,
        elapsed: &elapsed,
        review,
        sha: &sha,
    };
    if let Some(collapsed) = block.finish(look, Finish::Committed(&committed)) {
        println!("{collapsed}");
    }
    Ok(())
}

/// A `ticked` stop already passed the runner and the review gate; only its
/// commit is left.
fn retry_commit(
    root: &Path,
    delta: &Doc,
    task: &Task,
    config: &runner::Config,
    block: &Block,
) -> Result<(), Failure> {
    let look = &Look::current(config.mode);
    block.say(&look.tone(Tone::Bold, &heading(task)));
    block.say(&look.step(Glyph::Info, "   retrying the commit"));
    commit(root, delta, task, RETRIED_TRAILER, look, block)?;
    builds::remove(root, &delta.id);
    Ok(())
}

const RETRIED_TRAILER: &str = "Lemma-Review: passed before a failed commit";

fn record_stop(root: &Path, delta: &Doc, task: &Task, base: String, failure: &Failure) {
    let ending = match failure.ending {
        Ending::Unticked | Ending::Checked => "unticked",
        Ending::Ticked => "ticked",
        Ending::Reviewed(_) => "reviewed",
    };
    let record = builds::Record {
        task: task.id.clone(),
        ending,
        base,
        head: builds::head(root),
        session: failure.session.clone(),
        findings: failure.record_findings(),
        tree: builds::dirty_tree(root),
        at: builds::now(),
    };
    builds::write(root, &delta.id, &record);
}

fn report(failure: &Failure, task: &Task, look: &Look) {
    let ending = match &failure.ending {
        Ending::Ticked => "ticked but not committed".to_string(),
        Ending::Unticked | Ending::Checked => "left unticked".to_string(),
        Ending::Reviewed(why) => why.clone(),
    };
    let stopped = format!("   stopped. {} {ending}; working tree kept.", task.id);
    let detail: Vec<String> = failure
        .detail
        .iter()
        .map(|line| look.step_toned(Glyph::Fail, line, Some(Tone::Red)))
        .collect();
    let stopped = look.tone(Tone::Red, &stopped);
    let transcript = failure
        .transcript
        .as_deref()
        .map(|t| look.tone(Tone::Red, t));
    runner::stop_lines(&detail, stopped, transcript)
        .iter()
        .for_each(|line| println!("{line}"));
}

pub(crate) const DIRTY_TREE: &str = "build needs a clean working tree; commit or stash first";

pub(crate) fn require_clean(root: &Path) -> Result<(), String> {
    match git_output(root, &["status", "--porcelain"])
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        true => Ok(()),
        false => Err(DIRTY_TREE.to_string()),
    }
}

fn stopped_message(stopped: &builds::Stopped) -> String {
    let task = &stopped.task;
    format!(
        "{task} stopped {}: {}; rerun with --resume, or clean the tree to start {task} fresh",
        stopped.at,
        stopped.reason()
    )
}

/// The task a valid `--resume` continues; a `ticked` stop names a task that
/// is no longer the first unticked one.
fn resumed_task(
    root: &Path,
    parsed: &tasks::Tasks,
    only: Option<&str>,
    stopped: &builds::Stopped,
) -> Result<String, String> {
    let ticked_stop = stopped.ending == "ticked"
        && only.is_none()
        && parsed.named(&stopped.task).is_some_and(|t| t.ticked);
    let expected = match only {
        Some(name) => Some(name.to_string()),
        None => parsed.first_unticked().map(|t| t.id.clone()),
    };
    if !ticked_stop && expected.as_deref() != Some(stopped.task.as_str()) {
        let against = match (only, expected) {
            (Some(name), _) => format!("--task {name}"),
            (None, Some(next)) => format!("the next unticked task {next}"),
            (None, None) => "no unticked task".to_string(),
        };
        return Err(format!(
            "cannot resume: the stop record is for {}, not {against}",
            stopped.task
        ));
    }
    if !builds::is_ancestor(root, &stopped.head) {
        return Err(format!(
            "cannot resume {}: it stopped at {}, which is not an ancestor of HEAD {}; history changed under the task",
            stopped.task,
            stopped.head,
            builds::head(root)
        ));
    }
    Ok(stopped.task.clone())
}

struct Resumed {
    task: String,
    stopped: builds::Stopped,
}

/// Decides how a build enters: `Some` resumes a stopped task.
fn enter(
    root: &Path,
    id: &str,
    parsed: &tasks::Tasks,
    only: Option<&str>,
    resume: bool,
) -> Result<Option<Resumed>, String> {
    let stopped = builds::read(root, id);
    if resume {
        let stopped = stopped.ok_or(format!("nothing to resume: {id} has no stopped task"))?;
        let task = resumed_task(root, parsed, only, &stopped)?;
        return Ok(Some(Resumed { task, stopped }));
    }
    let Some(stopped) = stopped else {
        require_clean(root)?;
        return Ok(None);
    };
    require_clean(root).map_err(|_| stopped_message(&stopped))?;
    builds::remove(root, id);
    Ok(None)
}

pub fn run(
    root: &Path,
    id: &str,
    only: Option<&str>,
    resume: bool,
    config: &runner::Config,
) -> Result<Outcome, String> {
    let (_, delta) = load_delta(root, id)?;
    let parsed = tasks::parse(delta.part_body(TASKS_PART));
    if let Some(name) = only {
        parsed.named(name).ok_or(format!("unknown task {name}"))?;
    }
    let mut resumed = enter(root, &delta.id, &parsed, only, resume)?;
    if let (Some(name), None) = (only, &resumed) {
        if parsed.named(name).is_some_and(|t| t.ticked) {
            println!("{name} is already ticked");
            return Ok(Outcome::Complete);
        }
    }
    loop {
        let (store, delta) = load_delta(root, id)?;
        let parsed = tasks::parse(delta.part_body(TASKS_PART));
        let (next, base, stopped) = match resumed.take() {
            Some(r) => (
                parsed.named(&r.task),
                r.stopped.base.clone(),
                Some(r.stopped),
            ),
            None => {
                let next = match only {
                    Some(name) => parsed.named(name),
                    None => parsed.first_unticked(),
                };
                (next.filter(|t| !t.ticked), builds::head(root), None)
            }
        };
        let Some(task) = next else {
            println!("all tasks complete");
            return Ok(Outcome::Complete);
        };
        let block = Block::on_terminal();
        let built = match &stopped {
            Some(s) if s.ending == "ticked" => retry_commit(root, &delta, task, config, &block),
            _ => build_task(root, &store, &delta, task, stopped.as_ref(), config, &block),
        };
        if let Err(failure) = built {
            record_stop(root, &delta, task, base, &failure);
            let look = Look::current(config.mode);
            if let Some(collapsed) = block.finish(&look, Finish::Stopped) {
                println!("{collapsed}");
            }
            report(&failure, task, &look);
            return Ok(Outcome::Stopped);
        }
        if only.is_some() {
            return Ok(Outcome::Complete);
        }
    }
}
