mod apply;
mod attach;
mod base;
mod check;
mod claims;
#[allow(dead_code)]
mod evidence;
mod fold;
mod legacy;
mod outcome;
mod payload;
mod project;
#[allow(dead_code)]
mod range;
mod receipt;
mod refs;
mod repin;
mod session;
mod stop;
mod targets;
#[cfg(test)]
mod testgit;
mod transition;
mod validate;
mod version;
mod world;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
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

#[derive(Clone, Copy, ValueEnum)]
enum RequireArg {
    Claims,
    Evidence,
    Clean,
}

#[derive(Subcommand)]
enum Command {
    /// Run the four checks over the working tree against the base
    Check {
        /// Revision to diff against; overrides base resolution
        #[arg(long)]
        base: Option<String>,
        /// Trunk branch for the merge-base
        #[arg(long, default_value = range::DEFAULT_TRUNK)]
        trunk: String,
        /// Turn a readiness condition into errors
        #[arg(long, value_enum)]
        require: Vec<RequireArg>,
        #[arg(long, conflicts_with = "hook")]
        json: bool,
        /// Read a lazyspec payload on stdin, print findings JSON, exit 0
        #[arg(long)]
        hook: bool,
    },
    /// Fold a delta into its specs: the pre-transition hook, or a preview with a delta
    Apply { delta: Option<String> },
    /// List the refs of a claim and whether each resolves
    Refs { claim: String },
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

fn check_command(
    base: Option<String>,
    trunk: String,
    require: &[RequireArg],
    json: bool,
    hook: bool,
) -> ExitCode {
    let options = check::Options {
        base: base.as_deref(),
        trunk: Some(&trunk),
        require: check::Require {
            claims: require.iter().any(|r| matches!(r, RequireArg::Claims)),
            evidence: require.iter().any(|r| matches!(r, RequireArg::Evidence)),
            clean: require.iter().any(|r| matches!(r, RequireArg::Clean)),
        },
    };
    if hook {
        return hook_with_store(|store, root, payload| {
            let findings = check::run(store, root, &options, Some(payload));
            Output::findings(findings.iter().map(|f| f.for_hook(payload)).collect())
        });
    }
    let (root, store) = match cli_store() {
        Ok(found) => found,
        Err(message) => return fail(message),
    };
    let findings = check::run(&store, &root, &options, None);
    match json {
        true => println!("{}", serde_json::json!({ "findings": findings })),
        false => findings.iter().for_each(|f| println!("{}", f.render())),
    }
    match check::has_error(&findings) {
        true => ExitCode::from(2),
        false => ExitCode::SUCCESS,
    }
}

fn apply_preview(name: &str) -> ExitCode {
    let (_, store) = match cli_store() {
        Ok(found) => found,
        Err(message) => return fail(message),
    };
    let preview = match apply::preview(&store, name) {
        Ok(preview) => preview,
        Err(e) => return fail(format!("{e:#}")),
    };
    preview.lines.iter().for_each(|line| println!("{line}"));
    preview
        .findings
        .iter()
        .for_each(|f| println!("{}", f.render()));
    match check::has_error(&preview.findings) {
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
        Command::Check {
            base,
            trunk,
            require,
            json,
            hook,
        } => check_command(base, trunk, &require, json, hook),
        Command::Apply { delta: None } => {
            hook_with_store(|store, _, payload| apply::hook(store, payload))
        }
        Command::Apply { delta: Some(name) } => apply_preview(&name),
        Command::Refs { claim } => refs_command(&claim),
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
