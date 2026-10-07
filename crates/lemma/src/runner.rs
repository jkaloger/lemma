use serde::Deserialize;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const DEFAULT_RUNNER: &str = "claude -p --output-format json";
const DEFAULT_TIMEOUT_SECS: u64 = 1800;
const STDERR_TAIL_LINES: usize = 5;
const POLL: Duration = Duration::from_millis(20);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub runner: String,
    pub reviewer: String,
    pub timeout: Duration,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    runner: Option<String>,
    reviewer: Option<String>,
    timeout: Option<u64>,
}

pub fn load(root: &Path) -> Result<Config, String> {
    let path = root.join("lemma.toml");
    let raw: Raw = match std::fs::read_to_string(&path) {
        Err(_) => Raw {
            runner: None,
            reviewer: None,
            timeout: None,
        },
        Ok(text) => toml::from_str(&text).map_err(|e| format!("lemma.toml: {e}"))?,
    };
    let runner = raw.runner.unwrap_or_else(|| DEFAULT_RUNNER.to_string());
    Ok(Config {
        reviewer: raw.reviewer.unwrap_or_else(|| runner.clone()),
        runner,
        timeout: Duration::from_secs(raw.timeout.unwrap_or(DEFAULT_TIMEOUT_SECS)),
    })
}

pub fn split(command: &str) -> Result<Vec<String>, String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut in_word = false;
    let mut quote: Option<char> = None;
    for c in command.chars() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), c) => word.push(c),
            (None, '\'' | '"') => {
                quote = Some(c);
                in_word = true;
            }
            (None, c) if c.is_whitespace() => {
                if in_word {
                    words.push(std::mem::take(&mut word));
                    in_word = false;
                }
            }
            (None, c) => {
                word.push(c);
                in_word = true;
            }
        }
    }
    if quote.is_some() {
        return Err(format!("unterminated quote in command `{command}`"));
    }
    if in_word {
        words.push(word);
    }
    if words.is_empty() {
        return Err("empty runner command".to_string());
    }
    Ok(words)
}

fn drain(mut pipe: impl Read + Send + 'static) -> thread::JoinHandle<String> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = pipe.read_to_end(&mut bytes);
        String::from_utf8_lossy(&bytes).into_owned()
    })
}

fn tail(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(STDERR_TAIL_LINES)..].join("\n")
}

pub fn run(
    command: &str,
    prompt: &str,
    root: &Path,
    env: &[(&str, &str)],
    timeout: Duration,
) -> Result<String, String> {
    let argv = split(command)?;
    let mut child = Command::new(&argv[0])
        .args(&argv[1..])
        .current_dir(root)
        .envs(env.iter().copied())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot start `{}`: {e}", argv[0]))?;
    let mut stdin = child.stdin.take().expect("stdin is piped");
    let prompt = prompt.to_string();
    thread::spawn(move || {
        let _ = stdin.write_all(prompt.as_bytes());
    });
    let stdout = drain(child.stdout.take().expect("stdout is piped"));
    let stderr = drain(child.stderr.take().expect("stderr is piped"));
    let started = Instant::now();
    let status = loop {
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(status) => break status,
            None if started.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "`{}` timed out after {}s",
                    argv[0],
                    timeout.as_secs()
                ));
            }
            None => thread::sleep(POLL),
        }
    };
    let stdout = stdout.join().unwrap_or_default();
    let stderr = stderr.join().unwrap_or_default();
    if !status.success() {
        let code = status
            .code()
            .map_or("a signal".to_string(), |c| c.to_string());
        return Err(format!(
            "`{}` exited with {code}: {}",
            argv[0],
            tail(&stderr)
        ));
    }
    result_text(&stdout)
}

fn result_text(stdout: &str) -> Result<String, String> {
    let reply: serde_json::Value =
        serde_json::from_str(stdout).map_err(|e| format!("runner output is not JSON: {e}"))?;
    let text = reply["result"]
        .as_str()
        .ok_or("runner output has no `result` text")?;
    match reply["is_error"].as_bool() == Some(true) {
        true => Err(format!("runner reported an error: {text}")),
        false => Ok(text.to_string()),
    }
}

pub fn json_reply(result: &str) -> Result<serde_json::Value, String> {
    let (Some(start), Some(end)) = (result.find('{'), result.rfind('}')) else {
        return Err("reviewer reply has no JSON object".to_string());
    };
    serde_json::from_str(result.get(start..=end).unwrap_or_default())
        .map_err(|e| format!("reviewer reply is not valid JSON: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::os::unix::fs::PermissionsExt;
    use tempfile::TempDir;

    fn root_with_script(body: &str) -> (TempDir, String) {
        let root = tempfile::tempdir().unwrap();
        let script = root.path().join("fake.sh");
        std::fs::write(&script, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let command = script.to_string_lossy().into_owned();
        (root, command)
    }

    const LONG: Duration = Duration::from_secs(30);

    #[test]
    fn run_feeds_the_prompt_on_stdin_and_returns_the_result_text() {
        let (root, command) = root_with_script(
            r#"cat > stdin.txt
printf 'cwd=%s env=%s\n' "$(pwd -P)" "$LEMMA_DELTA" > seen.txt
printf '{"result": "ok", "usage": {}}'"#,
        );

        let text = run(
            &command,
            "the prompt\nline 2",
            root.path(),
            &[("LEMMA_DELTA", "DELTA-001")],
            LONG,
        )
        .unwrap();

        assert_eq!(text, "ok");
        assert_eq!(
            std::fs::read_to_string(root.path().join("stdin.txt")).unwrap(),
            "the prompt\nline 2"
        );
        let seen = std::fs::read_to_string(root.path().join("seen.txt")).unwrap();
        let cwd = root.path().canonicalize().unwrap();
        assert_eq!(seen, format!("cwd={} env=DELTA-001\n", cwd.display()));
    }

    #[test]
    fn run_survives_a_prompt_larger_than_the_pipe_buffer() {
        let (root, command) = root_with_script(r#"cat >/dev/null; printf '{"result": "big"}'"#);
        let prompt = "x".repeat(1 << 20);

        assert_eq!(
            run(&command, &prompt, root.path(), &[], LONG).unwrap(),
            "big"
        );
    }

    #[test]
    fn run_kills_a_runner_that_outlasts_the_timeout() {
        let (root, command) = root_with_script("sleep 5");
        let started = Instant::now();

        let error = run(&command, "", root.path(), &[], Duration::from_secs(1)).unwrap_err();

        assert!(error.contains("timed out after 1s"), "{error}");
        assert!(started.elapsed() < Duration::from_secs(4));
    }

    #[test]
    fn run_reports_the_exit_code_and_the_stderr_tail() {
        let (root, command) =
            root_with_script("for i in 1 2 3 4 5 6 7; do echo \"line $i\" >&2; done; exit 3");

        let error = run(&command, "", root.path(), &[], LONG).unwrap_err();

        assert!(error.contains("exited with 3"), "{error}");
        assert!(
            error.contains("line 7") && error.contains("line 3"),
            "{error}"
        );
        assert!(!error.contains("line 2"), "{error}");
    }

    #[test]
    fn run_fails_when_the_runner_flags_is_error() {
        let (root, command) =
            root_with_script(r#"printf '{"is_error": true, "result": "rate limited"}'"#);

        let error = run(&command, "", root.path(), &[], LONG).unwrap_err();

        assert!(error.contains("rate limited"), "{error}");
    }

    #[test]
    fn run_fails_on_output_that_is_not_json_or_has_no_result() {
        let (root, command) = root_with_script("echo hello");
        assert!(run(&command, "", root.path(), &[], LONG).is_err());
        let (root, command) = root_with_script(r#"printf '{"other": 1}'"#);
        assert!(run(&command, "", root.path(), &[], LONG).is_err());
    }

    #[test]
    fn config_defaults_when_there_is_no_lemma_toml() {
        let root = tempfile::tempdir().unwrap();

        let config = load(root.path()).unwrap();

        assert_eq!(config.runner, "claude -p --output-format json");
        assert_eq!(config.reviewer, config.runner);
        assert_eq!(config.timeout, Duration::from_secs(1800));
    }

    #[test]
    fn the_reviewer_defaults_to_a_configured_runner() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("lemma.toml"),
            "runner = \"codex exec\"\ntimeout = 60\n",
        )
        .unwrap();

        let config = load(root.path()).unwrap();

        assert_eq!(
            (config.runner.as_str(), config.reviewer.as_str()),
            ("codex exec", "codex exec")
        );
        assert_eq!(config.timeout, Duration::from_secs(60));
    }

    #[test]
    fn an_explicit_reviewer_is_kept() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("lemma.toml"), "reviewer = \"other\"\n").unwrap();

        assert_eq!(load(root.path()).unwrap().reviewer, "other");
    }

    #[test]
    fn an_unknown_lemma_toml_key_is_an_error_naming_it() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("lemma.toml"), "runer = \"x\"\n").unwrap();

        let error = load(root.path()).unwrap_err();

        assert!(error.contains("runer"), "{error}");
    }

    #[test]
    fn split_groups_quoted_words() {
        let argv = split(r#"claude -p "two words" 'a "b" c' x"y z"w"#).unwrap();
        assert_eq!(argv, ["claude", "-p", "two words", r#"a "b" c"#, "xy zw"]);
    }

    #[test]
    fn split_rejects_an_empty_command() {
        assert!(split("   ").is_err());
    }

    #[test]
    fn split_keeps_an_empty_quoted_word() {
        assert_eq!(split("a ''").unwrap(), vec!["a", ""]);
    }

    #[test]
    fn json_reply_reads_through_a_fence_and_prose() {
        let reply = "Here:\n```json\n{\"clean\": true}\n```\nDone.";
        assert_eq!(json_reply(reply).unwrap()["clean"], true);
    }

    #[test]
    fn json_reply_errors_without_an_object() {
        assert!(json_reply("all fine").is_err());
        assert!(json_reply("{ nope }").is_err());
    }
}
