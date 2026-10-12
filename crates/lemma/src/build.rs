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
use crate::{context, plan, project, reviews, runner, schema};
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
    Ticked,
    Reviewed(String),
}

struct Failure {
    detail: Vec<String>,
    ending: Ending,
    transcript: Option<String>,
}

impl Failure {
    fn at(self, transcript: Option<String>) -> Failure {
        Failure { transcript, ..self }
    }
}

fn unticked(detail: Vec<String>) -> Failure {
    Failure {
        detail,
        ending: Ending::Unticked,
        transcript: None,
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
    claims: String,
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
    let failed = |message: String| unticked(vec![format!("   {name}: {message}")]);
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
        failed(message).at(runner::transcript_line(
            root,
            reply.session.as_deref(),
            config.mode,
        ))
    };
    let parsed = serde_json::from_value(reply.json().map_err(after)?)
        .map_err(|e| after(format!("reply: {e}")))?;
    Ok((parsed, reply.session))
}

fn failed_call(name: &str, failed: runner::Failed) -> Failure {
    unticked(vec![format!("   {name}: {}", failed.message)]).at(failed.transcript)
}

fn accept(
    root: &Path,
    delta: &Doc,
    task: &Task,
    name: &str,
    parts: &[(String, String)],
    claims: &str,
    reply: &Reply,
) -> Result<(), Failure> {
    reject_part_edits(root, delta, parts)?;
    if let Verdict::Blocked = reply.outcome {
        return Err(unticked(vec![format!(
            "   {name} blocked: {}",
            reply.reason
        )]));
    }
    write_refs(root, delta, task, claims, &reply.refs)
}

fn run_runner(
    root: &Path,
    store: &lazyspec::engine::store::Store,
    delta: &Doc,
    task: &Task,
    config: &runner::Config,
    block: &Block,
) -> Result<Built, Failure> {
    let mut values = context::assemble(store, root, delta);
    values.insert("task", context::task_text(delta, task));
    let prompt = prompts::rendered(Prompt::Build, root, &values)
        .map_err(|message| unticked(vec![format!("   runner: {message}")]))?;
    let claims = plan::read(root, delta, CLAIMS_PART);
    let parts = delta_parts(root, delta);
    let (reply, session) = runner_reply(
        root,
        delta,
        task,
        Call {
            name: "runner",
            prompt: &prompt,
            resume: None,
        },
        config,
        block,
    )?;
    accept(root, delta, task, "runner", &parts, &claims, &reply).map_err(|f| {
        f.at(runner::transcript_line(
            root,
            session.as_deref(),
            config.mode,
        ))
    })?;
    Ok(Built {
        prompt,
        session,
        claims,
    })
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
    built: &Built,
    findings: &[gate::Finding],
    config: &runner::Config,
    block: &Block,
) -> Result<(), Failure> {
    let values = BTreeMap::from([("findings", findings_list(findings))]);
    let fix = prompts::rendered(Prompt::Fix, root, &values)
        .map_err(|message| unticked(vec![format!("   fix: {message}")]))?;
    let session = built
        .session
        .as_deref()
        .filter(|_| runner::resumable(&config.resume));
    let prompt = match session {
        Some(_) => fix,
        None => format!("{}\n\n{fix}", built.prompt),
    };
    let parts = delta_parts(root, delta);
    let (reply, session) = runner_reply(
        root,
        delta,
        task,
        Call {
            name: "fix",
            prompt: &prompt,
            resume: session,
        },
        config,
        block,
    )?;
    accept(root, delta, task, "fix", &parts, &built.claims, &reply).map_err(|f| {
        f.at(runner::transcript_line(
            root,
            session.as_deref(),
            config.mode,
        ))
    })
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
    Err(unticked(
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
        let lines = claim_evidence(root, &tests, claim).map_err(|line| unticked(vec![line]))?;
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
        transcript: None,
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

fn record_review(root: &Path, delta: &Doc, task: &Task, verdict: &gate::Verdict) -> Option<()> {
    let file = reviews::file(root, &format!("{}-{}", delta.id, task.id))?;
    std::fs::create_dir_all(file.parent()?).ok()?;
    std::fs::write(file, serde_json::to_string_pretty(verdict).ok()?).ok()
}

fn reviewed_stop(
    root: &Path,
    delta: &Doc,
    task: &Task,
    verdict: &gate::Verdict,
    detail: Vec<String>,
    ending: String,
) -> Failure {
    record_review(root, delta, task, verdict);
    Failure {
        detail,
        ending: Ending::Reviewed(ending),
        transcript: None,
    }
}

fn check_work(
    root: &Path,
    delta: &Doc,
    task: &Task,
    look: &Look,
    block: &Block,
) -> Result<(), Failure> {
    check_lint(root, &delta.id, look, block)?;
    check_settled(root, &delta.id, task, look, block)
}

fn print_fix_round(look: &Look, block: &Block, count: usize, round: u32) {
    let found = format!("   review: {}", counted(count, "finding"));
    match look.mode {
        Mode::Plain => block.say(&format!("{found}, fix round {round}")),
        Mode::Live => {
            block.say(&look.step(Glyph::Fail, &found));
            block.say(&look.step(Glyph::Info, &format!("   fix round {round}")));
        }
    }
}

fn review_gate(
    root: &Path,
    store: &lazyspec::engine::store::Store,
    delta: &Doc,
    task: &Task,
    built: &Built,
    config: &runner::Config,
    block: &Block,
) -> Result<ReviewOutcome, Failure> {
    let look = &Look::current(config.mode);
    let (mut fixed, mut rounds) = (0, 0);
    loop {
        let (verdict, session) = gate::review(root, store, delta, task, config)
            .map_err(|failed| failed_call("review", failed))?;
        let transcript = runner::transcript_line(root, session.as_deref(), config.mode);
        if !verdict.stop.trim().is_empty() {
            let ending = format!("review: {}", verdict.stop.trim());
            return Err(
                reviewed_stop(root, delta, task, &verdict, Vec::new(), ending).at(transcript),
            );
        }
        let count = verdict.findings.len();
        if count == 0 {
            block.say(&look.step_toned(Glyph::Pass, "   review: clean", Some(Tone::Green)));
            return Ok(ReviewOutcome {
                findings: fixed,
                rounds,
            });
        }
        if rounds == config.review_rounds {
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
            return Err(reviewed_stop(root, delta, task, &verdict, detail, ending).at(transcript));
        }
        rounds += 1;
        fixed += count;
        print_fix_round(look, block, count, rounds);
        run_fix(root, delta, task, built, &verdict.findings, config, block)
            .and_then(|()| check_work(root, delta, task, look, block))
            .inspect_err(|_| {
                record_review(root, delta, task, &verdict);
            })?;
    }
}

fn build_task(
    root: &Path,
    store: &lazyspec::engine::store::Store,
    delta: &Doc,
    task: &Task,
    config: &runner::Config,
    block: &Block,
) -> Result<(), Failure> {
    let look = &Look::current(config.mode);
    let started = std::time::Instant::now();
    block.say(&look.tone(Tone::Bold, &heading(task)));
    let built = run_runner(root, store, delta, task, config, block)?;
    check_work(root, delta, task, look, block)?;
    let review = review_gate(root, store, delta, task, &built, config, block)?;
    tick(root, delta, task)?;
    let sha = commit(root, delta, task, &review.trailer(), look, block)?;
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

fn report(failure: &Failure, task: &Task, look: &Look) {
    let ending = match &failure.ending {
        Ending::Ticked => "ticked but not committed".to_string(),
        Ending::Unticked => "left unticked".to_string(),
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

pub fn run(
    root: &Path,
    id: &str,
    only: Option<&str>,
    config: &runner::Config,
) -> Result<Outcome, String> {
    require_clean(root)?;
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
        let block = Block::on_terminal();
        if let Err(failure) = build_task(root, &store, &delta, task, config, &block) {
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
