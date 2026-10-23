use crate::annotate::frontmatter_len;
use crate::build;
use crate::context;
use crate::payload::{Doc, CHANGES, CLAIMS_PART, DESIGN_PART, INDEX_PART};
use crate::plan;
use crate::prompts::{self, Prompt};
use crate::review::part_file;
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

pub fn run(
    root: &Path,
    store: &Store,
    prompt: &str,
    config: &runner::Config,
) -> Result<(), String> {
    build::require_clean(root)?;
    let delta = draft(root, store, prompt, config)?;
    println!("draft: {} {}", delta.id, delta.title);
    Ok(())
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
