mod attach;
mod base;
mod claims;
mod fold;
mod outcome;
mod payload;
mod project;
mod receipt;
mod repin;
mod session;
mod stop;
mod targets;
mod transition;
mod validate;
mod version;

use anyhow::Result;
use clap::{Parser, Subcommand};
use lazyspec::engine::store::Store;
use outcome::Outcome;
use payload::{Finding, Output, Payload, StopInput};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

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
    /// Report claim and receipt problems (lazyspec validate hook)
    Validate,
    /// Gate a delta status transition and run the fold (lazyspec pre-transition hook)
    PreTransition,
    /// Bound a session's changes (Claude Code Stop hook)
    Stop,
    /// Record the session's base commit (Claude Code SessionStart hook)
    SessionStart,
    /// Update the receipt commits on one document, or on the named claims only
    RePin {
        id: String,
        claims: Vec<String>,
        /// Re-pin even when a receipt file changed and its claim did not
        #[arg(long)]
        force: bool,
    },
    /// Record the spec block a delta claim replaces or strikes (`@base <hash>`)
    Base { delta: String, claims: Vec<String> },
    /// Pin a test as a claim's receipt at the current HEAD
    Receipt {
        delta: String,
        claim: String,
        /// <path>#<symbol>
        target: String,
    },
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

fn with_payload(run: impl FnOnce(&Payload) -> Output) -> ExitCode {
    let output = match serde_json::from_str::<Payload>(&read_stdin()) {
        Ok(payload) => run(&payload),
        Err(e) => Output::findings(vec![Finding::bare_error(format!(
            "lemma: invalid hook payload: {e}"
        ))]),
    };
    print(&output);
    ExitCode::SUCCESS
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
    findings
        .iter()
        .for_each(|f| eprintln!("lemma: {}", f.message));
    ExitCode::from(2)
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

fn refuse_with(message: String) -> ExitCode {
    print(&Output::findings(vec![Finding::bare_error(message)]));
    ExitCode::from(2)
}

fn store_command(run: impl FnOnce(&Store, &Path) -> Result<Outcome>) -> ExitCode {
    let Some(root) = project::find_root(&current_dir()) else {
        return refuse_with("no .lazyspec.toml found".to_string());
    };
    if let Err(message) = version::require_lazyspec() {
        return refuse_with(message);
    }
    match project::load_store(&root).and_then(|store| run(&store, &root)) {
        Ok(outcome) => {
            print(&Output {
                wrote: outcome.wrote,
                ..Output::findings(outcome.findings)
            });
            if outcome.refused {
                ExitCode::from(2)
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(e) => refuse_with(format!("{e:#}")),
    }
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Validate => with_payload(|p| validate::run(p, &current_dir())),
        Command::PreTransition => with_payload(|p| transition::run(p, &current_dir())),
        Command::Stop => stop_command(),
        Command::SessionStart => session_start_command(),
        Command::RePin { id, claims, force } => {
            store_command(|store, root| repin::run(store, root, &id, &claims, force))
        }
        Command::Base { delta, claims } => {
            store_command(|store, root| base::run(store, root, &delta, &claims))
        }
        Command::Receipt {
            delta,
            claim,
            target,
        } => store_command(|store, root| attach::run(store, root, &delta, &claim, &target)),
    }
}
