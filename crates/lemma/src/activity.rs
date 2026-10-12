#![allow(dead_code)]

use crate::build::elapsed_text;
use serde_json::Value;
use std::path::Path;
use std::time::Duration;

/// The `<tool> <target>` text for the last tool call in one transcript line, if it has one.
pub fn activity(line: &str, root: &Path) -> Option<String> {
    let entry: Value = serde_json::from_str(line).ok()?;
    if entry["type"] != "assistant" {
        return None;
    }
    let call = entry["message"]["content"]
        .as_array()?
        .iter()
        .rev()
        .find(|block| block["type"] == "tool_use")?;
    let name = call["name"].as_str()?;
    match target(name, &call["input"], root) {
        Some(target) => Some(format!("{name} {target}")),
        None => Some(name.to_string()),
    }
}

fn target(name: &str, input: &Value, root: &Path) -> Option<String> {
    match name {
        "Read" | "Edit" | "Write" | "MultiEdit" => {
            let path = Path::new(input["file_path"].as_str()?);
            let shown = path.strip_prefix(root).unwrap_or(path);
            Some(shown.display().to_string())
        }
        "Bash" => Some(input["command"].as_str()?.lines().next()?.to_string()),
        "Grep" | "Glob" => Some(input["pattern"].as_str()?.to_string()),
        _ => None,
    }
}

/// The spinner message `<label> <elapsed> · <activity>`, cut to `width` columns with `…`.
pub fn spinner_line(
    label: &str,
    elapsed: Duration,
    activity: Option<&str>,
    width: usize,
) -> String {
    let elapsed = elapsed_text(elapsed);
    let line = match activity {
        Some(activity) => format!("{label} {elapsed} · {activity}"),
        None => format!("{label} {elapsed}"),
    };
    console::truncate_str(&line, width, "…").into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn line(name: &str, input: Value) -> String {
        json!({
            "type": "assistant",
            "message": { "content": [
                { "type": "text", "text": "looking" },
                { "type": "tool_use", "name": name, "input": input }
            ]}
        })
        .to_string()
    }

    #[test]
    fn each_tool_shows_its_target() {
        let root = Path::new("/work/repo");
        let cases = [
            (
                "Read",
                json!({ "file_path": "/work/repo/src/main.rs" }),
                "Read src/main.rs",
            ),
            (
                "Edit",
                json!({ "file_path": "/work/repo/src/a.rs" }),
                "Edit src/a.rs",
            ),
            (
                "Write",
                json!({ "file_path": "/work/repo/b.md" }),
                "Write b.md",
            ),
            (
                "MultiEdit",
                json!({ "file_path": "/work/repo/c/d.rs" }),
                "MultiEdit c/d.rs",
            ),
            (
                "Bash",
                json!({ "command": "cargo test\necho done" }),
                "Bash cargo test",
            ),
            (
                "Grep",
                json!({ "pattern": "fn activity", "path": "src" }),
                "Grep fn activity",
            ),
            ("Glob", json!({ "pattern": "**/*.rs" }), "Glob **/*.rs"),
            (
                "WebFetch",
                json!({ "url": "https://example.com" }),
                "WebFetch",
            ),
        ];
        for (name, input, expected) in cases {
            assert_eq!(
                activity(&line(name, input), root).as_deref(),
                Some(expected)
            );
        }
    }

    #[test]
    fn the_last_tool_call_in_a_message_wins() {
        let text = json!({
            "type": "assistant",
            "message": { "content": [
                { "type": "tool_use", "name": "Read", "input": { "file_path": "/r/a.rs" } },
                { "type": "tool_use", "name": "Grep", "input": { "pattern": "x" } }
            ]}
        })
        .to_string();
        assert_eq!(activity(&text, Path::new("/r")).as_deref(), Some("Grep x"));
    }

    #[test]
    fn a_line_without_a_tool_call_has_no_activity() {
        let root = Path::new("/r");
        let user = json!({ "type": "user", "message": { "content": [
            { "type": "tool_result", "content": "ok" }
        ]}});
        let prose = json!({ "type": "assistant", "message": { "content": [
            { "type": "text", "text": "done" }
        ]}});
        assert_eq!(activity(&user.to_string(), root), None);
        assert_eq!(activity(&prose.to_string(), root), None);
        assert_eq!(activity("not json", root), None);
    }

    #[test]
    fn the_activity_line_is_cut_to_terminal_width() {
        let elapsed = Duration::from_secs(124);
        let bash = Some("Bash cargo test --workspace");
        assert_eq!(
            spinner_line("runner", elapsed, bash, 80),
            "runner 2m04s · Bash cargo test --workspace"
        );
        assert_eq!(
            spinner_line("runner", elapsed, bash, 24),
            "runner 2m04s · Bash car…"
        );
        assert_eq!(spinner_line("runner", elapsed, None, 80), "runner 2m04s");
        let wide = spinner_line("runner", elapsed, Some("Grep 日本語のパターン"), 20);
        assert_eq!(console::measure_text_width(&wide), 20);
    }
}
