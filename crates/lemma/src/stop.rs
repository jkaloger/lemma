use crate::builds;
use crate::git::changed_since;
use crate::lint::{self, Finding, Options, Span, Suite};
use crate::payload::TASKS_PART;
use crate::session::{recorded_base, say_once};
use crate::tasks;
use crate::world::World;
use lazyspec::engine::store::Store;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize, Default)]
pub struct StopInput {
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub stop_hook_active: bool,
}

pub fn run(store: &Store, root: &Path, session_id: Option<&str>) -> Vec<Finding> {
    let options = Options {
        session: session_id,
        suite: Suite::Session,
        ..Options::default()
    };
    let span = lint::resolve_span(root, &options, None);
    let stopped = resumable_deltas(store, root);
    let mut findings: Vec<Finding> = lint::run_with(store, root, &options, None, Some(&span))
        .into_iter()
        .filter(Finding::is_error)
        .map(|f| warn_if_stopped(f, &stopped))
        .collect();
    findings.extend(unapproved_completions(store, root, session_id, &span));
    findings
}

/// Deltas whose stopped build waits on the human: the session that meets
/// their findings cannot clear them.
fn resumable_deltas(store: &Store, root: &Path) -> Vec<String> {
    World::load(store, None)
        .docs()
        .iter()
        .filter(|d| d.doc_type == "delta")
        .filter(|d| {
            builds::read(root, &d.id)
                .is_some_and(|s| s.is_resumable(root, &tasks::parse(d.part_body(TASKS_PART))))
        })
        .map(|d| d.id.clone())
        .collect()
}

fn warn_if_stopped(finding: Finding, stopped: &[String]) -> Finding {
    let Some(id) = finding.id.clone().filter(|id| stopped.contains(id)) else {
        return finding;
    };
    Finding {
        severity: "warning",
        message: format!(
            "{}; {id} has a stopped build task, run `lemma build {id} --resume`",
            finding.message
        ),
        ..finding
    }
}

fn unapproved_completions(
    store: &Store,
    root: &Path,
    session_id: Option<&str>,
    span: &Result<Span, String>,
) -> Vec<Finding> {
    let world = World::load(store, None);
    let any_complete = world
        .docs()
        .iter()
        .any(|d| d.doc_type == "delta" && d.status == "complete");
    if !any_complete {
        return Vec::new();
    }
    let base = session_id
        .and_then(|id| recorded_base(root, id))
        .unwrap_or_else(|| "HEAD".to_string());
    let files = match span {
        Ok(span) if span.base == base => span.files.clone(),
        _ => changed_since(root, &base),
    };
    let found: Vec<Finding> = world.completed_since(root, &base, &files)
        .into_iter()
        .map(|d| {
            Finding::error(
                d,
                &d.path,
                None,
                format!(
                    "{} is complete but was not when the session started; completing a delta is the human's step after `lemma review`. If a human moved it, leave it and ask them to commit; never change the status back",
                    d.id
                ),
            )
            .keyed(format!("complete:{}", d.id))
        })
        .collect();
    let Some(session) = session_id else {
        return found;
    };
    let keyed = found
        .iter()
        .cloned()
        .map(|f| (f.key.clone().unwrap_or_default(), f))
        .collect();
    say_once(root, session, keyed).unwrap_or(found)
}
