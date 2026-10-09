use crate::claims;
use crate::git::{git_output, repo_files};
use crate::payload::{Doc, CLAIMS_PART, DESIGN_PART, TASKS_PART};
use crate::range::{self, Inputs};
use crate::tasks::Task;
use crate::world::World;
use globset::Glob;
use lazyspec::engine::fs::RealFileSystem;
use lazyspec::engine::store::Store;
use std::collections::BTreeMap;
use std::path::Path;

pub const CHORE_LINE: &str = "This task is a chore: it settles no claim.";

fn section(id: &str, title: &str, body: &str) -> String {
    format!("## {id} {title}\n\n{body}")
}

fn rendered(docs: &[&Doc]) -> String {
    docs.iter()
        .map(|d| section(&d.id, &d.title, &d.body))
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn convention_body(store: &Store) -> String {
    store
        .all_docs()
        .into_iter()
        .find(|meta| meta.doc_type.to_string() == "convention")
        .and_then(|meta| store.get_body_raw(&meta.path, &RealFileSystem).ok())
        .unwrap_or_default()
}

fn literal_match(glob: &str, text: &str) -> bool {
    Glob::new(glob).is_ok_and(|g| g.compile_matcher().is_match(text))
}

fn globs_overlap(left: &[String], right: &[String], files: &[String]) -> bool {
    let matches_some_file = |globs: &[String]| {
        let set = crate::evidence::glob_set(globs);
        move |file: &String| set.is_match(file.as_str())
    };
    let (in_left, in_right) = (matches_some_file(left), matches_some_file(right));
    let literal = left.iter().any(|l| {
        right
            .iter()
            .any(|r| l == r || literal_match(l, r) || literal_match(r, l))
    });
    literal || files.iter().any(|file| in_left(file) && in_right(file))
}

fn governed_by_changed(
    root: &Path,
    dictum: &Doc,
    specs: &[&Doc],
    files: &mut Option<Vec<String>>,
) -> bool {
    if dictum.governs.is_empty() {
        return true;
    }
    let files = files.get_or_insert_with(|| repo_files(root));
    specs
        .iter()
        .any(|spec| globs_overlap(&dictum.governs, &spec.governs, files))
}

fn dictums_for(root: &Path, world: &World, specs: &[&Doc]) -> String {
    let all: Vec<&Doc> = world
        .docs()
        .iter()
        .filter(|d| d.doc_type == "dictum")
        .collect();
    let mut files = None;
    let selected: Vec<&Doc> = all
        .iter()
        .copied()
        .filter(|d| governed_by_changed(root, d, specs, &mut files))
        .collect();
    match selected.is_empty() {
        true => rendered(&all),
        false => rendered(&selected),
    }
}

fn related_sections(store: &Store, delta: &Doc) -> Vec<(String, String)> {
    let targets = delta
        .related
        .iter()
        .filter(|r| r.rel_type != crate::payload::CHANGES);
    targets
        .filter_map(|r| {
            let meta = store.all_docs().into_iter().find(|m| m.id == r.target)?;
            let body = store.get_body_raw(&meta.path, &RealFileSystem).ok()?;
            Some((meta.id.clone(), section(&meta.id, &meta.title, &body)))
        })
        .collect()
}

pub fn related_ids(store: &Store, delta: &Doc) -> Vec<String> {
    related_sections(store, delta)
        .into_iter()
        .map(|(id, _)| id)
        .collect()
}

fn related_bodies(store: &Store, delta: &Doc) -> String {
    related_sections(store, delta)
        .into_iter()
        .map(|(_, text)| text)
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub fn assemble(store: &Store, root: &Path, delta: &Doc) -> BTreeMap<&'static str, String> {
    let world = World::load(store, None);
    let specs = world.changed_specs(delta);
    BTreeMap::from([
        ("convention", convention_body(store)),
        ("dictums", dictums_for(root, &world, &specs)),
        ("spec_intent", rendered(&specs)),
        ("related", related_bodies(store, delta)),
        ("delta", delta.body.clone()),
        ("design", delta.part_body(DESIGN_PART).to_string()),
        ("claims", delta.part_body(CLAIMS_PART).to_string()),
        ("tasks", delta.part_body(TASKS_PART).to_string()),
    ])
}

pub fn diff_base(root: &Path, delta: &Doc) -> String {
    let inputs = Inputs {
        delta_index: Some(Path::new(&delta.path)),
        ..Inputs::default()
    };
    range::resolve(root, &inputs).rev
}

pub fn diff(root: &Path, base: &str, world: &World, delta: &Doc) -> String {
    let globs: Vec<String> = world
        .changed_specs(delta)
        .iter()
        .flat_map(|spec| spec.governs.clone())
        .chain(world.delta_tests(delta))
        .collect();
    if globs.is_empty() {
        return String::new();
    }
    let specs: Vec<String> = globs.iter().map(|g| format!(":(glob){g}")).collect();
    let mut args = vec!["diff", base, "--"];
    args.extend(specs.iter().map(String::as_str));
    git_output(root, &args).unwrap_or_default()
}

pub fn task_text(delta: &Doc, task: &Task) -> String {
    let line = delta
        .part_body(TASKS_PART)
        .split('\n')
        .nth(task.line - 1)
        .unwrap_or_default();
    if task.chore {
        return format!("{line}\n\n{CHORE_LINE}");
    }
    let parsed = claims::parse(delta.part_body(CLAIMS_PART));
    let blocks: Vec<String> = task
        .settles
        .iter()
        .filter_map(|id| parsed.claims.iter().find(|c| &c.id == id))
        .map(|c| c.block.join("\n").trim_end().to_string())
        .collect();
    format!("{line}\n\n{}", blocks.join("\n\n"))
}
