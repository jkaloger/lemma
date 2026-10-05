use crate::check::{self, Finding, Options, Span};
use crate::evidence::clean;
use crate::folded::{completed_since, spec_edits};
use crate::payload::Doc;
use crate::project;
use crate::session::say_once;
use crate::world::{in_flight, World};
use lazyspec::engine::store::Store;
use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const EDIT_TOOLS: [&str; 3] = ["Edit", "Write", "MultiEdit"];

#[derive(Debug, Deserialize, Default)]
pub struct Input {
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub tool_input: Option<ToolInput>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ToolInput {
    #[serde(default)]
    pub file_path: Option<String>,
}

struct Signal {
    key: String,
    text: String,
}

pub fn run(input: Input, fallback_dir: PathBuf) -> Option<String> {
    if !EDIT_TOOLS.contains(&input.tool_name.as_deref()?) {
        return None;
    }
    let session = input.session_id?;
    let start = input.cwd.map(PathBuf::from).unwrap_or(fallback_dir);
    let root = project::find_root(&start)?;
    let file = relative_to_root(&root, &start, Path::new(&input.tool_input?.file_path?))?;
    let store = project::load_store(&root).ok()?;
    let keyed = signals(&store, &root, &file, &session)
        .into_iter()
        .map(|signal| (signal.key, signal.text))
        .collect();
    let lines = say_once(&root, &session, keyed)?;
    if lines.is_empty() {
        return None;
    }
    Some(
        serde_json::json!({
            "hookSpecificOutput": {
                "hookEventName": "PostToolUse",
                "additionalContext": lines.join("\n"),
            }
        })
        .to_string(),
    )
}

fn relative_to_root(root: &Path, start: &Path, file: &Path) -> Option<String> {
    let file = std::fs::canonicalize(start.join(file)).ok()?;
    let root = std::fs::canonicalize(root).ok()?;
    let relative = file.strip_prefix(root).ok()?;
    Some(relative.to_string_lossy().into_owned())
}

fn signals(store: &Store, root: &Path, file: &str, session: &str) -> Vec<Signal> {
    let world = World::load(store, None);
    let governed = !store.governing(&root.join(file)).is_empty();
    let owner = owner_of(&world, file);
    let referenced = world
        .all_sites()
        .iter()
        .flat_map(|site| &site.claims)
        .flat_map(|claim| &claim.receipts)
        .any(|r| clean(&r.path) == file);
    let mut findings = Vec::new();
    if governed || referenced || owner.is_some() {
        let options = Options {
            session: Some(session),
            ..Options::default()
        };
        if let Ok(span) = check::resolve_span(root, &options, None) {
            let completed = completed_since(root, &world, &span.base, &span.files);
            if referenced {
                findings.extend(check::evidence_changes(
                    root,
                    &world,
                    &span,
                    &completed,
                    Some(file),
                ));
            }
            if governed {
                findings.extend(newly_uncovered(
                    store, root, &world, &span, &completed, file,
                ));
            }
            if let Some(spec) = owner.filter(|doc| doc.doc_type == "spec") {
                findings.extend(
                    spec_edits(root, &world, &span.base, &span.files)
                        .into_iter()
                        .filter(|f| f.id.as_deref() == Some(spec.id.as_str())),
                );
            }
        }
    }
    if let Some(doc) = owner {
        findings.extend(document_findings(root, &world, doc));
    }
    findings.sort();
    findings.dedup();
    context_pointers(store, root, file)
        .into_iter()
        .chain(findings.iter().map(|f| Signal {
            key: stable_key(f),
            text: f.render(),
        }))
        .collect()
}

fn stable_key(finding: &Finding) -> String {
    if let Some(key) = &finding.key {
        return key.clone();
    }
    format!(
        "check4:{}:{}:{}",
        finding.id.as_deref().unwrap_or_default(),
        finding.path.as_deref().unwrap_or_default(),
        finding.message
    )
}

fn context_pointers(store: &Store, root: &Path, file: &str) -> Option<Signal> {
    let governing = store.governing(&root.join(file));
    let of_type = |doc_type: &str| {
        governing
            .iter()
            .map(|(meta, _)| *meta)
            .filter(|meta| meta.doc_type.to_string() == doc_type)
            .collect::<Vec<_>>()
    };
    let specs = of_type("spec");
    let dictums = of_type("dictum");
    let decisions = specs.iter().flat_map(|spec| {
        store
            .reverse_links_for(&spec.path)
            .iter()
            .filter(|link| link.rel_type.as_str() == "affects")
            .filter_map(|link| store.get(&link.endpoint))
    });
    let mut seen = BTreeSet::new();
    let docs: Vec<_> = specs
        .iter()
        .copied()
        .chain(decisions)
        .chain(dictums)
        .filter(|meta| seen.insert(meta.id.clone()))
        .collect();
    if docs.is_empty() {
        return None;
    }
    let ids: Vec<&str> = docs.iter().map(|meta| meta.id.as_str()).collect();
    let pointers = docs
        .iter()
        .map(|meta| format!("{} {}", meta.id, meta.title))
        .collect::<Vec<_>>()
        .join(" · ");
    Some(Signal {
        key: format!("context:{}", ids.join(",")),
        text: format!("{file} → {pointers}"),
    })
}

fn newly_uncovered(
    store: &Store,
    root: &Path,
    world: &World,
    span: &Span,
    completed: &[&Doc],
    file: &str,
) -> Vec<Finding> {
    let covered = check::covered_specs(world, completed);
    let mut files = span.files.clone();
    files.retain(|f| f != file);
    let before: BTreeSet<Option<String>> = check::coverage(store, root, &covered, &files)
        .into_iter()
        .map(|f| f.id)
        .collect();
    files.push(file.to_string());
    check::coverage(store, root, &covered, &files)
        .into_iter()
        .filter(|f| !before.contains(&f.id))
        .collect()
}

fn owner_of<'a>(world: &'a World, file: &str) -> Option<&'a Doc> {
    world.docs().iter().find(|doc| {
        let applies = doc.doc_type == "spec" || in_flight(doc);
        let owns = doc.path == file || doc.parts.iter().any(|p| doc.part_path(&p.name) == file);
        applies && owns
    })
}

fn document_findings(root: &Path, world: &World, doc: &Doc) -> Vec<Finding> {
    let require = check::Require::default();
    let mut findings = check::integrity_findings(root, world, doc, require);
    findings.extend(check::evidence_findings(root, world, doc, require));
    findings.extend(check::double_strikes(world, &[doc]));
    findings.retain(|f| f.id.as_deref() == Some(doc.id.as_str()));
    findings
}
