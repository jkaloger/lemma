mod claims;
mod fold;
mod payload;
mod project;
mod receipt;
mod repin;
mod stop;
mod targets;
mod transition;
mod validate;

use clap::{Parser, Subcommand};
use payload::{Finding, Output, Payload, StopInput};
use std::io::Read;
use std::path::PathBuf;
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
    /// Update the receipt commits on one document
    RePin { id: String },
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
    let store = match project::load_store(&root) {
        Ok(store) => store,
        Err(e) => {
            eprintln!("lemma stop: {e:#}");
            return ExitCode::SUCCESS;
        }
    };
    let findings = stop::run(&store, &root);
    if findings.is_empty() {
        return ExitCode::SUCCESS;
    }
    findings
        .iter()
        .for_each(|f| eprintln!("lemma: {}", f.message));
    ExitCode::from(2)
}

fn repin_command(id: &str) -> ExitCode {
    let Some(root) = project::find_root(&current_dir()) else {
        print(&Output::findings(vec![Finding::bare_error(
            "no .lazyspec.toml found",
        )]));
        return ExitCode::from(2);
    };
    let outcome = project::load_store(&root).and_then(|store| repin::run(&store, &root, id));
    match outcome {
        Ok(outcome) => {
            print(&Output::findings(outcome.findings));
            if outcome.refused {
                ExitCode::from(2)
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            print(&Output::findings(vec![Finding::bare_error(format!(
                "{e:#}"
            ))]));
            ExitCode::from(2)
        }
    }
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Validate => with_payload(|p| validate::run(p, &current_dir())),
        Command::PreTransition => with_payload(|p| transition::run(p, &current_dir())),
        Command::Stop => stop_command(),
        Command::RePin { id } => repin_command(&id),
    }
}
