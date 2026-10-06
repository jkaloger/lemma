mod apply;
mod check;
mod claims;
mod evidence;
mod fold;
mod folded;
mod git;
mod migrate;
mod payload;
mod project;
mod range;
mod refs;
mod session;
mod signal;
mod stop;
mod targets;
#[cfg(test)]
mod testgit;
mod version;
mod world;

use clap::{Parser, Subcommand, ValueEnum};
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

#[derive(Clone, Copy, ValueEnum)]
enum RequireArg {
    Intent,
    /// Deprecated alias for intent
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
        /// Trunk branch for the merge-base; default main, else master, else origin/HEAD
        #[arg(long)]
        trunk: Option<String>,
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
    /// Bound a session's changes (Claude Code Stop hook)
    Stop,
    /// Per-edit context and findings (Claude Code PostToolUse hook)
    Signal,
    /// Record the session's base commit (Claude Code SessionStart hook)
    SessionStart,
    /// Rewrite 1.x specs and deltas to the 2.0 claim format
    Migrate,
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

const CLAIMS_DEPRECATED: &str =
    "`--require claims` is deprecated and will be removed; use `--require intent`";

fn check_command(
    base: Option<String>,
    trunk: Option<String>,
    require: &[RequireArg],
    json: bool,
    hook: bool,
) -> ExitCode {
    let options = check::Options {
        base: base.as_deref(),
        trunk: trunk.as_deref(),
        require: check::Require {
            intent: require
                .iter()
                .any(|r| matches!(r, RequireArg::Intent | RequireArg::Claims)),
            evidence: require.iter().any(|r| matches!(r, RequireArg::Evidence)),
            clean: require.iter().any(|r| matches!(r, RequireArg::Clean)),
        },
        ..check::Options::default()
    };
    let deprecated = require.iter().any(|r| matches!(r, RequireArg::Claims));
    let hint = deprecated.then(|| check::Finding::bare_warning(CLAIMS_DEPRECATED));
    if hook {
        return hook_with_store(|store, root, payload| {
            let findings = check::run(store, root, &options, Some(payload));
            let findings: Vec<_> = hint.iter().cloned().chain(findings).collect();
            Output::findings(findings.iter().map(|f| f.for_hook(payload)).collect())
        });
    }
    let (root, store) = match cli_store() {
        Ok(found) => found,
        Err(message) => return fail(message),
    };
    let findings = check::run(&store, &root, &options, None);
    let findings: Vec<_> = match (json, hint) {
        (true, Some(hint)) => std::iter::once(hint).chain(findings).collect(),
        (false, Some(hint)) => {
            eprintln!("lemma: {}", hint.message);
            findings
        }
        (_, None) => findings,
    };
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

fn migrate_command() -> ExitCode {
    let (root, store) = match cli_store() {
        Ok(found) => found,
        Err(message) => return fail(message),
    };
    migrate::run(&store, &root)
        .iter()
        .for_each(|path| println!("{path}"));
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
        Command::Stop => stop_command(),
        Command::SessionStart => session_start_command(),
        Command::Signal => signal_command(),
        Command::Migrate => migrate_command(),
    }
}
