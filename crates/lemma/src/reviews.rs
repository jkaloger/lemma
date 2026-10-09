use crate::review::Finding;
use crate::session::{is_safe_file_name, lemma_dir};
use std::path::{Path, PathBuf};

const NONE: &str = "(none)";

pub fn file(root: &Path, delta_id: &str) -> Option<PathBuf> {
    if !is_safe_file_name(delta_id) {
        return None;
    }
    Some(
        lemma_dir(root)?
            .join("reviews")
            .join(format!("{delta_id}.json")),
    )
}

fn load(root: &Path, delta_id: &str) -> Vec<Finding> {
    let Some(text) = file(root, delta_id).and_then(|f| std::fs::read_to_string(f).ok()) else {
        return Vec::new();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

pub fn record(root: &Path, delta_id: &str, findings: &[Finding]) -> Option<()> {
    let path = file(root, delta_id)?;
    let mut all = load(root, delta_id);
    for finding in findings {
        if !all.contains(finding) {
            all.push(finding.clone());
        }
    }
    std::fs::create_dir_all(path.parent()?).ok()?;
    std::fs::write(path, serde_json::to_string_pretty(&all).ok()?).ok()
}

pub fn resolved(root: &Path, delta: &crate::payload::Doc) -> Vec<Finding> {
    load(root, &delta.id)
        .into_iter()
        .filter(|finding| !still_present(delta, finding))
        .collect()
}

fn still_present(delta: &crate::payload::Doc, finding: &Finding) -> bool {
    let text = delta.part_text(&finding.part);
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .contains(&finding.text)
}

pub fn render(resolved: &[Finding]) -> String {
    if resolved.is_empty() {
        return NONE.to_string();
    }
    resolved
        .iter()
        .map(|f| format!("- [{} / {}] {}", f.part, f.anchor, f.text))
        .collect::<Vec<_>>()
        .join("\n")
}
