use crate::check::{self, Finding, Options};
use crate::git::changed_since;
use crate::project;
use crate::session::{record_said, said_hashes};
use crate::version;
use crate::world::{in_flight, World};
use lazyspec::engine::hashing::sha256_hex;
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

impl Signal {
    fn hash(&self) -> String {
        sha256_hex(self.key.as_bytes())[..16].to_string()
    }
}

pub fn run(input: Input, fallback_dir: PathBuf) -> Option<String> {
    if !EDIT_TOOLS.contains(&input.tool_name.as_deref()?) {
        return None;
    }
    let session = input.session_id?;
    let start = input.cwd.map(PathBuf::from).unwrap_or(fallback_dir);
    let root = project::find_root(&start)?;
    let file = relative_to_root(&root, &start, Path::new(&input.tool_input?.file_path?))?;
    version::require_lazyspec().ok()?;
    let said = said_hashes(&root, &session)?;
    let store = project::load_store(&root).ok()?;
    let fresh: Vec<Signal> = signals(&store, &root, &file, &session)
        .into_iter()
        .filter(|signal| !said.contains(&signal.hash()))
        .collect();
    if fresh.is_empty() {
        return None;
    }
    let hashes: Vec<String> = fresh.iter().map(Signal::hash).collect();
    record_said(&root, &session, &hashes)?;
    let lines: Vec<&str> = fresh.iter().map(|s| s.text.as_str()).collect();
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
    let options = Options {
        session: Some(session),
        ..Options::default()
    };
    let mut findings = Vec::new();
    if let Ok(base) = check::resolve_base(root, &options, None) {
        findings.extend(check::evidence_changes(root, &world, &base, Some(file)));
        findings.extend(newly_uncovered(store, root, &world, &base, file));
    }
    findings.extend(document_findings(root, &world, file));
    findings.sort();
    findings.dedup();
    context_pointers(store, root, file)
        .into_iter()
        .chain(findings.iter().map(|f| {
            let text = f.render();
            Signal {
                key: text.clone(),
                text,
            }
        }))
        .collect()
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
    let pointers: Vec<String> = specs
        .iter()
        .copied()
        .chain(decisions)
        .chain(dictums)
        .filter(|meta| seen.insert(meta.id.clone()))
        .map(|meta| format!("{} {}", meta.id, meta.title))
        .collect();
    if pointers.is_empty() {
        return None;
    }
    let pointers = pointers.join(" · ");
    Some(Signal {
        key: format!("context {pointers}"),
        text: format!("{file} → {pointers}"),
    })
}

fn newly_uncovered(
    store: &Store,
    root: &Path,
    world: &World,
    base: &str,
    file: &str,
) -> Vec<Finding> {
    if store.governing(&root.join(file)).is_empty() {
        return Vec::new();
    }
    let mut files = changed_since(root, base);
    files.retain(|f| f != file);
    let before: BTreeSet<Option<String>> = check::coverage(store, root, world, &files)
        .into_iter()
        .map(|f| f.id)
        .collect();
    files.push(file.to_string());
    check::coverage(store, root, world, &files)
        .into_iter()
        .filter(|f| !before.contains(&f.id))
        .collect()
}

fn document_findings(root: &Path, world: &World, file: &str) -> Vec<Finding> {
    let owner = world.docs().iter().find(|doc| {
        let applies = doc.doc_type == "spec" || in_flight(doc);
        let owns = doc.path == file || doc.parts.iter().any(|p| doc.part_path(&p.name) == file);
        applies && owns
    });
    let Some(doc) = owner else {
        return Vec::new();
    };
    let require = check::Require::default();
    let mut findings = check::integrity_findings(root, world, doc, require);
    findings.extend(check::evidence_findings(root, world, doc, require));
    findings.extend(check::double_strikes(world, &[doc]));
    findings.retain(|f| f.id.as_deref() == Some(doc.id.as_str()));
    findings
}
