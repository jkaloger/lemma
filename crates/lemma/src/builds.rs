use crate::gate::Finding;
use crate::git::{changed_since, git_output};
use crate::session::{is_safe_file_name, lemma_dir};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize)]
pub struct Record {
    pub task: String,
    pub ending: &'static str,
    pub base: String,
    pub head: String,
    pub session: Option<String>,
    pub findings: Vec<Finding>,
    pub tree: BTreeMap<String, Option<String>>,
    pub at: String,
}

/// The parts of a stop record that decide whether and where a build resumes.
#[derive(Deserialize)]
pub struct Stopped {
    pub task: String,
    pub ending: String,
    pub base: String,
    pub head: String,
    #[serde(default)]
    pub findings: Vec<Finding>,
    pub at: String,
}

impl Stopped {
    pub fn reason(&self) -> &str {
        self.findings
            .first()
            .map_or(self.ending.as_str(), |f| f.text.as_str())
    }
}

pub fn read(root: &Path, delta_id: &str) -> Option<Stopped> {
    let text = std::fs::read_to_string(file(root, delta_id)?).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn is_ancestor(root: &Path, sha: &str) -> bool {
    git_output(root, &["merge-base", "--is-ancestor", sha, "HEAD"]).is_some()
}

pub fn file(root: &Path, delta_id: &str) -> Option<PathBuf> {
    if !is_safe_file_name(delta_id) {
        return None;
    }
    Some(
        lemma_dir(root)?
            .join("builds")
            .join(format!("{delta_id}.json")),
    )
}

pub fn head(root: &Path) -> String {
    git_output(root, &["rev-parse", "HEAD"])
        .unwrap_or_default()
        .trim()
        .to_string()
}

/// Blob sha of each dirty path as it is on disk; a deleted path has none.
pub fn dirty_tree(root: &Path) -> BTreeMap<String, Option<String>> {
    changed_since(root, "HEAD")
        .into_iter()
        .map(|path| {
            let blob = root
                .join(&path)
                .is_file()
                .then(|| git_output(root, &["hash-object", "--", &path]))
                .flatten()
                .map(|sha| sha.trim().to_string());
            (path, blob)
        })
        .collect()
}

pub fn write(root: &Path, delta_id: &str, record: &Record) -> Option<()> {
    let path = file(root, delta_id)?;
    std::fs::create_dir_all(path.parent()?).ok()?;
    std::fs::write(path, serde_json::to_string_pretty(record).ok()?).ok()
}

pub fn remove(root: &Path, delta_id: &str) {
    if let Some(path) = file(root, delta_id) {
        let _ = std::fs::remove_file(path);
    }
}

pub fn now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default();
    rfc3339(secs)
}

/// Civil date from days since the epoch (Howard Hinnant's algorithm).
fn rfc3339(secs: u64) -> String {
    let (days, rem) = ((secs / 86_400) as i64, secs % 86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3_600,
        rem % 3_600 / 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use super::rfc3339;

    #[test]
    fn rfc3339_renders_utc_seconds() {
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(rfc3339(1_791_774_245), "2026-10-12T03:04:05Z");
    }
}
