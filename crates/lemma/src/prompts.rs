use regex::{Captures, Regex};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::OnceLock;

pub const KEYS: [&str; 14] = [
    "convention",
    "dictums",
    "spec_intent",
    "related",
    "delta",
    "design",
    "claims",
    "tasks",
    "task",
    "task_diff",
    "delta_diff",
    "resolved",
    "findings",
    "prompt",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prompt {
    ReviewDoc,
    Plan,
    Build,
    ReviewTask,
    Fix,
    Vibe,
    Revise,
    Replan,
}

impl Prompt {
    pub const ALL: [Prompt; 8] = [
        Prompt::ReviewDoc,
        Prompt::Plan,
        Prompt::Build,
        Prompt::ReviewTask,
        Prompt::Fix,
        Prompt::Vibe,
        Prompt::Revise,
        Prompt::Replan,
    ];

    pub fn file_name(self) -> &'static str {
        match self {
            Prompt::ReviewDoc => "review-doc.md",
            Prompt::Plan => "plan.md",
            Prompt::Build => "build.md",
            Prompt::ReviewTask => "review-task.md",
            Prompt::Fix => "fix.md",
            Prompt::Vibe => "vibe.md",
            Prompt::Revise => "revise.md",
            Prompt::Replan => "replan.md",
        }
    }

    pub fn pack_path(self) -> String {
        format!(".lazyspec/prompts/{}", self.file_name())
    }

    fn default_text(self) -> &'static str {
        match self {
            Prompt::ReviewDoc => include_str!("../../../.lazyspec/prompts/review-doc.md"),
            Prompt::Plan => include_str!("../../../.lazyspec/prompts/plan.md"),
            Prompt::Build => include_str!("../../../.lazyspec/prompts/build.md"),
            Prompt::ReviewTask => include_str!("../../../.lazyspec/prompts/review-task.md"),
            Prompt::Fix => include_str!("../../../.lazyspec/prompts/fix.md"),
            Prompt::Vibe => include_str!("../../../.lazyspec/prompts/vibe.md"),
            Prompt::Revise => include_str!("../../../.lazyspec/prompts/revise.md"),
            Prompt::Replan => include_str!("../../../.lazyspec/prompts/replan.md"),
        }
    }
}

fn placeholder() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\{\{\s*([^{}]*?)\s*\}\}").unwrap())
}

pub fn pack_text(root: &Path, prompt: Prompt) -> Option<String> {
    std::fs::read_to_string(root.join(prompt.pack_path())).ok()
}

pub fn unknown_keys(template: &str) -> Vec<(usize, String)> {
    template
        .lines()
        .enumerate()
        .flat_map(|(i, line)| {
            placeholder()
                .captures_iter(line)
                .map(|caps| caps[1].to_string())
                .filter(|key| !KEYS.contains(&key.as_str()))
                .map(move |key| (i + 1, key))
        })
        .collect()
}

pub fn unknown_key_message(key: &str) -> String {
    format!(
        "unknown prompt key `{{{{{key}}}}}`; use one of {}",
        KEYS.join(", ")
    )
}

pub fn load(root: &Path, prompt: Prompt) -> Result<String, String> {
    let text = pack_text(root, prompt).unwrap_or_else(|| prompt.default_text().to_string());
    match unknown_keys(&text).first() {
        Some((line, key)) => Err(format!(
            "{}:{line}: {}",
            prompt.pack_path(),
            unknown_key_message(key)
        )),
        None => Ok(text),
    }
}

pub fn rendered(
    prompt: Prompt,
    root: &Path,
    values: &BTreeMap<&str, String>,
) -> Result<String, String> {
    Ok(render(&load(root, prompt)?, values))
}

pub fn render(template: &str, values: &BTreeMap<&str, String>) -> String {
    placeholder()
        .replace_all(template, |caps: &Captures| {
            values.get(&caps[1]).cloned().unwrap_or_default()
        })
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_defaults_use_only_known_keys() {
        for prompt in Prompt::ALL {
            assert_eq!(unknown_keys(prompt.default_text()), vec![], "{prompt:?}");
        }
    }

    #[test]
    fn a_pack_file_wins_over_the_default() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join(".lazyspec/prompts");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("plan.md"), "my plan {{delta}}").unwrap();

        assert_eq!(
            load(root.path(), Prompt::Plan).unwrap(),
            "my plan {{delta}}"
        );
    }

    #[test]
    fn the_default_is_used_when_the_pack_has_no_file() {
        let root = tempfile::tempdir().unwrap();

        let text = load(root.path(), Prompt::Build).unwrap();

        assert_eq!(text, Prompt::Build.default_text());
    }

    #[test]
    fn an_unknown_key_fails_the_load_naming_file_and_line() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join(".lazyspec/prompts");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("plan.md"), "ok {{delta}}\n\n{{ nope }}\n").unwrap();

        let error = load(root.path(), Prompt::Plan).unwrap_err();

        assert!(
            error.starts_with(".lazyspec/prompts/plan.md:3: "),
            "{error}"
        );
        assert!(error.contains("`{{nope}}`"), "{error}");
    }

    #[test]
    fn render_fills_known_keys_and_empties_missing_values() {
        let values = BTreeMap::from([("delta", "D".to_string())]);

        assert_eq!(render("a {{delta}} b {{ claims }} c", &values), "a D b  c");
    }
}
