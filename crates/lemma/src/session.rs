use crate::receipt::git_output;
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Default)]
pub struct SessionStartInput {
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub cwd: Option<String>,
}

fn is_safe_file_name(name: &str) -> bool {
    !name.is_empty() && name != "." && !name.contains("..") && !name.contains(['/', '\\', '\0'])
}

fn sessions_dir(dir: &Path) -> Option<PathBuf> {
    let git_dir = PathBuf::from(git_output(dir, &["rev-parse", "--git-dir"])?.trim());
    Some(dir.join(git_dir).join("lemma").join("sessions"))
}

pub fn record_base(dir: &Path, session_id: &str) -> Option<()> {
    if !is_safe_file_name(session_id) {
        return None;
    }
    let sessions = sessions_dir(dir)?;
    let file = sessions.join(session_id);
    if file.exists() {
        return None;
    }
    let head = git_output(dir, &["rev-parse", "HEAD"])?;
    std::fs::create_dir_all(&sessions).ok()?;
    std::fs::write(file, head.trim()).ok()
}

pub fn recorded_base(root: &Path, session_id: &str) -> Option<String> {
    if !is_safe_file_name(session_id) {
        return None;
    }
    let text = std::fs::read_to_string(sessions_dir(root)?.join(session_id)).ok()?;
    let base = text.trim();
    if base.is_empty() || !base.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    git_output(root, &["merge-base", "--is-ancestor", base, "HEAD"])?;
    Some(base.to_string())
}
