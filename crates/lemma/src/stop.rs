use crate::check::{self, Finding, Options, Suite};
use crate::git::{git_output, touched_files};
use lazyspec::engine::document::DocMeta;
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
    let mut findings: Vec<Finding> = check::run(store, root, &options, None)
        .into_iter()
        .filter(Finding::is_error)
        .collect();
    findings.extend(unapproved_completions(store, root));
    findings
}

fn complete_at_head(root: &Path, doc_path: &Path) -> bool {
    let spec = format!("HEAD:./{}", doc_path.to_string_lossy());
    git_output(root, &["show", &spec])
        .and_then(|text| DocMeta::parse(&text).ok())
        .is_some_and(|meta| meta.status.to_string() == "complete")
}

fn unapproved_completions(store: &Store, root: &Path) -> Vec<Finding> {
    let touched = touched_files(root);
    store
        .all_docs()
        .into_iter()
        .filter(|d| d.doc_type.to_string() == "delta" && d.status.to_string() == "complete")
        .filter(|d| touched.iter().any(|f| Path::new(f) == d.path))
        .filter(|d| !complete_at_head(root, &d.path))
        .map(|d| Finding {
            path: Some(d.path.to_string_lossy().into_owned()),
            line: None,
            severity: "error",
            id: Some(d.id.clone()),
            part: None,
            message: format!(
                "{} is complete in the working tree but not at HEAD; if a human moved it, leave it and ask them to commit; never change the status back",
                d.id
            ),
        })
        .collect()
}
