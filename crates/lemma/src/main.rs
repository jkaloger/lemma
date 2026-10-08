mod annotate;
mod build;
mod claims;
mod context;
mod evidence;
mod git;
mod lint;
mod payload;
mod plan;
mod project;
mod prompts;
mod range;
mod refs;
mod review;
mod reviews;
mod runner;
mod schema;
mod session;
mod shape;
mod signal;
mod stop;
mod tasks;
#[cfg(test)]
mod testgit;
mod version;
mod world;

use clap::{Parser, Subcommand};
use lazyspec::engine::store::Store;
use payload::{Finding, Output, Payload};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use stop::StopInput;

#[derive(Parser)]
#[command(
    name = "lemma",
    version,
    about = "Hooks binary for the lemma lazyspec workflow pack"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Lint the working tree against the base; with a delta, its shape and evidence too
    Lint {
        /// The delta to lint, by id
        delta: Option<String>,
        /// Revision to diff against; overrides base resolution
        #[arg(long)]
        base: Option<String>,
        /// Trunk branch for the merge-base; default main, else master, else origin/HEAD
        #[arg(long)]
        trunk: Option<String>,
        /// Apply the rules of the transition into this status: in-progress or complete
        #[arg(long)]
        gate: Option<String>,
        #[arg(long, conflicts_with = "hook")]
        json: bool,
        /// Read a lazyspec payload on stdin, print findings JSON, exit 0
        #[arg(long)]
        hook: bool,
    },
    /// Review a delta: prose while draft, the diff while in progress
    Review {
        delta: String,
        /// Print the assembled prompt and exit
        #[arg(long)]
        dry_run: bool,
    },
    /// Plan a reviewed draft delta into claims and tasks
    Plan {
        delta: String,
        /// Print the assembled prompt and exit
        #[arg(long)]
        dry_run: bool,
    },
    /// Build an in-progress delta task by task
    Build {
        delta: String,
        /// Run only this task, as T<n>
        #[arg(long)]
        task: Option<String>,
        /// Print the assembled prompt and exit
        #[arg(long)]
        dry_run: bool,
    },
    /// List the refs of a claim and whether each resolves
    Refs { claim: String },
    /// Bound a session's changes (Claude Code Stop hook)
    Stop,
    /// Per-edit context and findings (Claude Code PostToolUse hook)
    Signal,
    /// Record the session's base commit (Claude Code SessionStart hook)
    SessionStart,
}

fn read_stdin() -> String {
    let mut input = String::new();
    let _ = std::io::stdin().read_to_string(&mut input);
    input
}

fn current_dir() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn print(output: &Output) {
    println!(
        "{}",
        serde_json::to_string(output).expect("output serialises")
    );
}

fn hook_with_store(run: impl FnOnce(&Store, &Path, &Payload) -> Output) -> ExitCode {
    let output = match serde_json::from_str::<Payload>(&read_stdin()) {
        Err(e) => Output::findings(vec![Finding::bare_error(format!(
            "lemma: invalid hook payload: {e}"
        ))]),
        Ok(payload) => match project::find_root(&current_dir()) {
            None => Output::findings(vec![Finding::bare_error("no .lazyspec.toml found")]),
            Some(root) => match project::load_store(&root) {
                Ok(store) => run(&store, &root, &payload),
                Err(e) => Output::findings(vec![Finding::bare_error(format!("{e:#}"))]),
            },
        },
    };
    print(&output);
    ExitCode::SUCCESS
}

fn cli_store() -> Result<(PathBuf, Store), String> {
    let root = project::find_root(&current_dir()).ok_or("no .lazyspec.toml found")?;
    version::require_lazyspec()?;
    let store = project::load_store(&root).map_err(|e| format!("{e:#}"))?;
    Ok((root, store))
}

fn fail(message: impl std::fmt::Display) -> ExitCode {
    eprintln!("lemma: {message}");
    ExitCode::from(2)
}

fn lint_command(
    delta: Option<String>,
    base: Option<String>,
    trunk: Option<String>,
    gate: Option<String>,
    json: bool,
    hook: bool,
) -> ExitCode {
    let gate = match gate.as_deref().map(shape::Gate::parse).transpose() {
        Ok(gate) => gate,
        Err(message) => return fail(message),
    };
    if gate.is_some() && delta.is_none() && !hook {
        return fail("--gate needs a delta: lemma lint <delta> --gate <status>");
    }
    let options = lint::Options {
        base: base.as_deref(),
        trunk: trunk.as_deref(),
        delta: delta.as_deref(),
        gate,
        ..lint::Options::default()
    };
    if hook {
        return hook_with_store(|store, root, payload| {
            let findings = lint::run(store, root, &options, Some(payload));
            Output::findings(findings.iter().map(|f| f.for_hook(payload)).collect())
        });
    }
    let (root, store) = match cli_store() {
        Ok(found) => found,
        Err(message) => return fail(message),
    };
    if let Some(name) = options.delta {
        if !world::World::load(&store, None)
            .docs()
            .iter()
            .any(|d| d.doc_type == "delta" && d.is_named(name))
        {
            return fail(format!("unknown delta {name}"));
        }
    }
    let findings = lint::run(&store, &root, &options, None);
    match json {
        true => println!("{}", serde_json::json!({ "findings": findings })),
        false => findings.iter().for_each(|f| println!("{}", f.render())),
    }
    match lint::has_error(&findings) {
        true => ExitCode::from(2),
        false => ExitCode::SUCCESS,
    }
}

fn refs_command(claim: &str) -> ExitCode {
    let (root, store) = match cli_store() {
        Ok(found) => found,
        Err(message) => return fail(message),
    };
    let Some(listing) = refs::run(&store, &root, claim) else {
        return fail(format!("unknown claim {claim}"));
    };
    listing.lines.iter().for_each(|line| println!("{line}"));
    match listing.all_ok {
        true => ExitCode::SUCCESS,
        false => ExitCode::from(2),
    }
}

struct Startup {
    root: PathBuf,
    store: Store,
    delta: payload::Doc,
    config: runner::Config,
}

fn startup(id: &str) -> Result<Startup, String> {
    let (root, store) = cli_store()?;
    let delta = world::World::load(&store, None)
        .docs()
        .iter()
        .find(|d| d.doc_type == "delta" && d.is_named(id))
        .cloned()
        .ok_or(format!("unknown delta {id}"))?;
    prompts::Prompt::ALL
        .into_iter()
        .try_for_each(|prompt| prompts::load(&root, prompt).map(drop))?;
    let config = runner::load(&root)?;
    Ok(Startup {
        root,
        store,
        delta,
        config,
    })
}

fn print_prompt(
    prompt: prompts::Prompt,
    root: &Path,
    values: &std::collections::BTreeMap<&str, String>,
) -> ExitCode {
    match prompts::rendered(prompt, root, values) {
        Ok(text) => {
            println!("{text}");
            ExitCode::SUCCESS
        }
        Err(message) => fail(message),
    }
}

fn next(step: impl std::fmt::Display) {
    println!("{} {step}", console::style("next:").dim());
}

fn lines_remain(n: usize) -> String {
    match n {
        1 => "1 review line remains".to_string(),
        n => format!("{n} review lines remain"),
    }
}

fn review_draft_command(start: &Startup, prompt: &str, resolved: &[review::Finding]) -> ExitCode {
    let Startup {
        root,
        delta,
        config,
        ..
    } = start;
    let id = &delta.id;
    println!("{id} is draft: reviewing the document");
    match review::review_draft(root, delta, prompt, resolved, config) {
        Ok(review::Outcome::Clean(0)) => {
            println!("review clean");
            next(format!("lemma plan {id}"));
        }
        Ok(review::Outcome::Clean(n)) => {
            println!("review clean; {}", lines_remain(n));
            next(format!("resolve them, then lemma plan {id}"));
        }
        Ok(review::Outcome::Written(n)) => {
            match n {
                1 => println!("1 review line written"),
                n => println!("{n} review lines written"),
            }
            next(format!(
                "resolve each review line (edit, then delete the line) and run lemma review {id} again, or lemma plan {id} once none remain"
            ));
        }
        Err(message) => return fail(message),
    }
    ExitCode::SUCCESS
}

fn review_diff_command(
    start: &Startup,
    base: &str,
    values: &std::collections::BTreeMap<&str, String>,
) -> ExitCode {
    let Startup {
        root,
        delta,
        config,
        ..
    } = start;
    let prompt = match prompts::rendered(prompts::Prompt::ReviewDiff, root, values) {
        Ok(prompt) => prompt,
        Err(message) => return fail(message),
    };
    let short: String = base.chars().take(8).collect();
    println!(
        "{} is in-progress: reviewing the diff against {short}",
        delta.id
    );
    match review::review_diff(root, delta, &prompt, config) {
        Ok(n) => {
            match n {
                1 => println!("1 follow-up task written"),
                n => println!("{n} follow-up tasks written"),
            }
            match n {
                0 => next(format!("lazyspec advance {} to complete", delta.id)),
                _ => next(format!(
                    "delete the follow-ups you reject, then lemma build {}",
                    delta.id
                )),
            }
        }
        Err(message) => return fail(message),
    }
    ExitCode::SUCCESS
}

fn review_command(id: &str, dry_run: bool) -> ExitCode {
    let start = match startup(id) {
        Ok(start) => start,
        Err(message) => return fail(message),
    };
    let Startup {
        root, store, delta, ..
    } = &start;
    let mut values = context::assemble(store, root, delta);
    match (delta.status.as_str(), dry_run) {
        ("draft", dry_run) => {
            let resolved = reviews::resolved(root, delta);
            values.insert("resolved", reviews::render(&resolved));
            if dry_run {
                return print_prompt(prompts::Prompt::ReviewDoc, root, &values);
            }
            match prompts::rendered(prompts::Prompt::ReviewDoc, root, &values) {
                Ok(prompt) => review_draft_command(&start, &prompt, &resolved),
                Err(message) => fail(message),
            }
        }
        ("in-progress", dry_run) => {
            let world = world::World::load(store, None);
            let base = context::diff_base(root, delta);
            values.insert("diff", context::diff(root, &base, &world, delta));
            match dry_run {
                true => print_prompt(prompts::Prompt::ReviewDiff, root, &values),
                false => review_diff_command(&start, &base, &values),
            }
        }
        _ => fail("review needs a draft or in-progress delta"),
    }
}

fn plan_command(id: &str, dry_run: bool) -> ExitCode {
    let start = match startup(id) {
        Ok(start) => start,
        Err(message) => return fail(message),
    };
    let Startup {
        root,
        store,
        delta,
        config,
    } = &start;
    if delta.status != "draft" {
        return fail(format!(
            "plan needs a draft delta; {} is {}",
            delta.id, delta.status
        ));
    }
    let remaining = review::remaining(delta);
    if remaining > 0 {
        return fail(format!(
            "{}; resolve them before planning",
            lines_remain(remaining)
        ));
    }
    let values = context::assemble(store, root, delta);
    if dry_run {
        return print_prompt(prompts::Prompt::Plan, root, &values);
    }
    match plan::plan(root, delta, &values, config) {
        Ok(report) => {
            report.lines().iter().for_each(|line| println!("{line}"));
            next(format!(
                "read the diff, accept or reject each [proposed] claim, then lazyspec advance {}",
                delta.id
            ));
            ExitCode::SUCCESS
        }
        Err(message) => fail(message),
    }
}

fn next_after_build(id: &str) {
    next(format!(
        "lemma review {id} to review the diff, or lazyspec advance {id} to complete"
    ));
}

fn build_command(id: &str, task: Option<&str>, dry_run: bool) -> ExitCode {
    let start = match startup(id) {
        Ok(start) => start,
        Err(message) => return fail(message),
    };
    if start.delta.status != "in-progress" {
        return fail(format!(
            "build needs an in-progress delta; {} is {}",
            start.delta.id, start.delta.status
        ));
    }
    if !dry_run {
        return match build::run(&start.root, &start.delta.id, task, &start.config) {
            Ok(build::Outcome::Complete) => {
                next_after_build(&start.delta.id);
                ExitCode::SUCCESS
            }
            Ok(build::Outcome::Stopped) => ExitCode::from(2),
            Err(message) => fail(message),
        };
    }
    let Startup {
        root, store, delta, ..
    } = start;
    let parsed = tasks::parse(delta.part_body(payload::TASKS_PART));
    let chosen = match task {
        Some(name) => match parsed.named(name) {
            Some(task) => task,
            None => return fail(format!("unknown task {name}")),
        },
        None => match parsed.first_unticked() {
            Some(task) => task,
            None => {
                println!("all tasks complete");
                next_after_build(&delta.id);
                return ExitCode::SUCCESS;
            }
        },
    };
    let mut values = context::assemble(&store, &root, &delta);
    values.insert("task", context::task_text(&delta, chosen));
    print_prompt(prompts::Prompt::Build, &root, &values)
}

fn stop_command() -> ExitCode {
    let input: StopInput = serde_json::from_str(&read_stdin()).unwrap_or_default();
    if input.stop_hook_active {
        return ExitCode::SUCCESS;
    }
    let start = input.cwd.map(PathBuf::from).unwrap_or_else(current_dir);
    let Some(root) = project::find_root(&start) else {
        return ExitCode::SUCCESS;
    };
    if let Err(message) = version::require_lazyspec() {
        eprintln!("lemma stop: {message}");
        return ExitCode::SUCCESS;
    }
    let store = match project::load_store(&root) {
        Ok(store) => store,
        Err(e) => {
            eprintln!("lemma stop: {e:#}");
            return ExitCode::SUCCESS;
        }
    };
    let findings = stop::run(&store, &root, input.session_id.as_deref());
    if findings.is_empty() {
        return ExitCode::SUCCESS;
    }
    findings.iter().for_each(|f| eprintln!("{}", f.render()));
    ExitCode::from(2)
}

fn signal_command() -> ExitCode {
    let input = serde_json::from_str(&read_stdin()).unwrap_or_default();
    if let Some(output) = signal::run(input, current_dir()) {
        println!("{output}");
    }
    ExitCode::SUCCESS
}

fn session_start_command() -> ExitCode {
    let input: session::SessionStartInput = serde_json::from_str(&read_stdin()).unwrap_or_default();
    let dir = input.cwd.map(PathBuf::from).unwrap_or_else(current_dir);
    if project::find_root(&dir).is_none() {
        return ExitCode::SUCCESS;
    }
    if let Some(id) = input.session_id {
        session::record_base(&dir, &id);
    }
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Lint {
            delta,
            base,
            trunk,
            gate,
            json,
            hook,
        } => lint_command(delta, base, trunk, gate, json, hook),
        Command::Review { delta, dry_run } => review_command(&delta, dry_run),
        Command::Plan { delta, dry_run } => plan_command(&delta, dry_run),
        Command::Build {
            delta,
            task,
            dry_run,
        } => build_command(&delta, task.as_deref(), dry_run),
        Command::Refs { claim } => refs_command(&claim),
        Command::Stop => stop_command(),
        Command::SessionStart => session_start_command(),
        Command::Signal => signal_command(),
    }
}
