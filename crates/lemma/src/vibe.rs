use crate::advance;
use crate::annotate::frontmatter_len;
use crate::build;
use crate::builds;
use crate::claims::{accept_proposed, review_lines};
use crate::context;
use crate::payload::{Doc, CHANGES, CLAIMS_PART, DESIGN_PART, INDEX_PART};
use crate::plan;
use crate::prompts::{self, Prompt};
use crate::review::{self, part_file, Outcome, PARTS};
use crate::reviews;
use crate::runner;
use crate::schema;
use crate::world::World;
use lazyspec::engine::store::Store;
use serde::Deserialize;
use std::path::Path;
use std::process::Command;

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

/// Returns the id of the delta it drafted and built.
pub fn run(
    root: &Path,
    store: &Store,
    prompt: &str,
    rounds: u32,
    config: &runner::Config,
) -> Result<String, Stopped> {
    build::require_clean(root)?;
    let delta = draft(root, store, prompt, config).map_err(|m| stopped(m, "lemma vibe -m …"))?;
    let id = &delta.id;
    println!("draft: {id} {}", delta.title);
    let review = format!("lemma review {id}");
    review_rounds(root, id, rounds, config).map_err(|m| stopped(m, &review))?;
    strip_review_lines(root, id).map_err(|m| stopped(m, &review))?;
    plan(root, id, config).map_err(|m| stopped(m, &format!("lemma plan {id}")))?;
    let accepting = format!("accept or reject each [proposed] claim, then lemma build {id}");
    accept(root, id).map_err(|m| stopped(m, &accepting))?;
    let moving = format!("lazyspec update {id} --status in-progress");
    in_progress(root, id).map_err(|m| stopped(m, &moving))?;
    run_build(root, id, config)?;
    Ok(delta.id)
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
    let reply = runner::tracked(
        "draft",
        &config.runner,
        &rendered,
        root,
        &[],
        config.timeout,
        Some(&schema::vibe(&specs)),
        config.mode,
    )?;
    let reply: Draft =
        serde_json::from_value(reply.json()?).map_err(|e| format!("draft reply: {e}"))?;
    require_known_changes(&reply.changes, &specs)?;
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
) -> Result<(), String> {
    for round in 1..=rounds {
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
    let report = plan::plan(root, &delta, &values, config)?;
    report.lines().iter().for_each(|line| println!("{line}"));
    Ok(())
}

fn accept(root: &Path, id: &str) -> Result<(), String> {
    let (_, delta) = build::load_delta(root, id)?;
    let (claims, accepted) = accept_proposed(&plan::read(root, &delta, CLAIMS_PART));
    write(root, &delta, CLAIMS_PART, &claims)?;
    println!("accepted {accepted} [proposed] claims");
    Ok(())
}

fn in_progress(root: &Path, id: &str) -> Result<(), String> {
    let (store, delta) = build::load_delta(root, id)?;
    advance::unasked(root, &store, &delta)
}

fn run_build(root: &Path, id: &str, config: &runner::Config) -> Result<(), Stopped> {
    let message = match build::run(root, id, None, false, config) {
        Ok(build::Outcome::Complete) => return Ok(()),
        Ok(build::Outcome::Stopped) => None,
        Err(message) => Some(message),
    };
    let next = match builds::read(root, id) {
        Some(_) => format!("lemma build {id} --resume"),
        None => format!("lemma build {id}"),
    };
    Err(Stopped {
        message,
        next: Some(next),
    })
}
