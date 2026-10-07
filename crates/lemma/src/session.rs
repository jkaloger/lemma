use crate::git::git_output;
use lazyspec::engine::hashing::sha256_hex;
use serde::Deserialize;
use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Default)]
pub struct SessionStartInput {
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub cwd: Option<String>,
}

pub fn is_safe_file_name(name: &str) -> bool {
    !name.is_empty() && name != "." && !name.contains("..") && !name.contains(['/', '\\', '\0'])
}

pub fn lemma_dir(dir: &Path) -> Option<PathBuf> {
    let git_dir = PathBuf::from(git_output(dir, &["rev-parse", "--git-dir"])?.trim());
    Some(dir.join(git_dir).join("lemma"))
}

fn sessions_dir(dir: &Path) -> Option<PathBuf> {
    Some(lemma_dir(dir)?.join("sessions"))
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

fn said_file(root: &Path, session_id: &str) -> Option<PathBuf> {
    if !is_safe_file_name(session_id) {
        return None;
    }
    Some(sessions_dir(root)?.join(format!("{session_id}.said")))
}

pub fn said_hashes(root: &Path, session_id: &str) -> Option<HashSet<String>> {
    let file = said_file(root, session_id)?;
    let text = std::fs::read_to_string(file).unwrap_or_default();
    Some(text.lines().map(str::to_string).collect())
}

pub fn record_said(root: &Path, session_id: &str, hashes: &[String]) -> Option<()> {
    let file = said_file(root, session_id)?;
    std::fs::create_dir_all(file.parent()?).ok()?;
    let mut out = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(file)
        .ok()?;
    hashes.iter().try_for_each(|h| writeln!(out, "{h}")).ok()
}

fn key_hash(key: &str) -> String {
    sha256_hex(key.as_bytes())[..16].to_string()
}

pub fn say_once<T>(root: &Path, session_id: &str, items: Vec<(String, T)>) -> Option<Vec<T>> {
    let mut said = said_hashes(root, session_id)?;
    let fresh: Vec<(String, T)> = items
        .into_iter()
        .map(|(key, item)| (key_hash(&key), item))
        .filter(|(hash, _)| said.insert(hash.clone()))
        .collect();
    if fresh.is_empty() {
        return Some(Vec::new());
    }
    let hashes: Vec<String> = fresh.iter().map(|(hash, _)| hash.clone()).collect();
    record_said(root, session_id, &hashes)?;
    Some(fresh.into_iter().map(|(_, item)| item).collect())
}
