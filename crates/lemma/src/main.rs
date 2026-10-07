mod claims;
mod evidence;
mod git;
mod lint;
mod payload;
mod project;
mod range;
mod refs;
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
        Command::Refs { claim } => refs_command(&claim),
        Command::Stop => stop_command(),
        Command::SessionStart => session_start_command(),
        Command::Signal => signal_command(),
    }
}
