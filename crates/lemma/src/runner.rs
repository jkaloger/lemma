use crate::activity::{follow, Tail};
use crate::output::Mode;
use serde::Deserialize;
use serde_json::Value;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

const DEFAULT_RUNNER: &str =
    "claude -p --session-id {session} --output-format json --json-schema {schema}";
const DEFAULT_RESUME: &str =
    "claude -p --resume {session} --output-format json --json-schema {schema}";
const DEFAULT_REVIEW_ROUNDS: u32 = 1;
const DEFAULT_TIMEOUT_SECS: u64 = 1800;
const SCHEMA: &str = "{schema}";
const SCHEMA_FILE: &str = "{schema_file}";
const SESSION: &str = "{session}";
const WATCH_FILTER: &str = r#"select(.type == "assistant") | .message.content[]? | select(.type == "tool_use") | "\(.name) \(.input | tostring | .[0:100])""#;
const STDERR_TAIL_LINES: usize = 5;
const POLL: Duration = Duration::from_millis(20);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub runner: String,
    pub reviewer: String,
    pub resume: String,
    pub review_rounds: u32,
    pub timeout: Duration,
    pub mode: Mode,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    runner: Option<String>,
    reviewer: Option<String>,
    resume: Option<String>,
    review_rounds: Option<u32>,
    timeout: Option<u64>,
}

pub fn load(root: &Path) -> Result<Config, String> {
    let path = root.join("lemma.toml");
    let raw: Raw = match std::fs::read_to_string(&path) {
        Err(_) => Raw {
            runner: None,
            reviewer: None,
            resume: None,
            review_rounds: None,
            timeout: None,
        },
        Ok(text) => toml::from_str(&text).map_err(|e| format!("lemma.toml: {e}"))?,
    };
    let runner = raw.runner.unwrap_or_else(|| DEFAULT_RUNNER.to_string());
    Ok(Config {
        reviewer: raw.reviewer.unwrap_or_else(|| runner.clone()),
        runner,
        resume: raw.resume.unwrap_or_else(|| DEFAULT_RESUME.to_string()),
        review_rounds: raw.review_rounds.unwrap_or(DEFAULT_REVIEW_ROUNDS),
        timeout: Duration::from_secs(raw.timeout.unwrap_or(DEFAULT_TIMEOUT_SECS)),
        mode: Mode::Plain,
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

#[derive(Debug)]
pub struct Failed {
    pub message: String,
    pub transcript: Option<String>,
    pub session: Option<String>,
}

impl From<String> for Failed {
    fn from(message: String) -> Failed {
        Failed {
            message,
            transcript: None,
            session: None,
        }
    }
}

impl From<Failed> for String {
    fn from(failed: Failed) -> String {
        failed.message
    }
}

pub fn stop_lines(detail: &[String], stopped: String, transcript: Option<String>) -> Vec<String> {
    detail
        .iter()
        .cloned()
        .chain(std::iter::once(stopped))
        .chain(transcript)
        .collect()
}

#[derive(Debug)]
pub struct Reply {
    pub text: String,
    pub session: Option<String>,
    structured: Option<Value>,
}

impl Reply {
    pub fn json(&self) -> Result<Value, String> {
        match &self.structured {
            Some(value) => Ok(value.clone()),
            None => json_reply(&self.text),
        }
    }
}

struct Argv {
    words: Vec<String>,
    _schema_file: Option<tempfile::NamedTempFile>,
}

fn with_schema(words: Vec<String>, schema: Option<&Value>) -> Result<Argv, String> {
    let uses = |placeholder: &str| words.iter().any(|w| w.contains(placeholder));
    let (inline, file) = (uses(SCHEMA), uses(SCHEMA_FILE));
    if !inline && !file {
        return Ok(Argv {
            words,
            _schema_file: None,
        });
    }
    let schema =
        schema.ok_or("runner command uses a schema placeholder but no schema was given")?;
    let compact = schema.to_string();
    let schema_file = match file {
        true => {
            let mut handle = tempfile::NamedTempFile::new()
                .map_err(|e| format!("cannot write the schema file: {e}"))?;
            handle
                .write_all(compact.as_bytes())
                .map_err(|e| format!("cannot write the schema file: {e}"))?;
            Some(handle)
        }
        false => None,
    };
    let path = schema_file
        .as_ref()
        .map(|f| f.path().to_string_lossy().into_owned())
        .unwrap_or_default();
    let words = words
        .iter()
        .map(|w| w.replace(SCHEMA, &compact).replace(SCHEMA_FILE, &path))
        .collect();
    Ok(Argv {
        words,
        _schema_file: schema_file,
    })
}

fn encoded_root(root: &Path) -> String {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    root.to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

pub fn watch_line(root: &Path, session: &str) -> String {
    let encoded = encoded_root(root);
    format!(
        "   watch: tail -n +1 -F \"$HOME/.claude/projects/{encoded}/{session}.jsonl\" | jq -r '{WATCH_FILTER}'"
    )
}

/// The transcript the live spinner follows; plain mode and session-less commands have none.
fn transcript(root: &Path, session: Option<&str>, mode: Mode) -> Option<PathBuf> {
    let session = session.filter(|_| mode == Mode::Live)?;
    let home = std::env::var_os("HOME")?;
    Some(
        Path::new(&home)
            .join(".claude/projects")
            .join(encoded_root(root))
            .join(format!("{session}.jsonl")),
    )
}

/// The line a stopped task ends with so the human can read the failed call; plain mode prints the watch line instead.
pub fn transcript_line(root: &Path, session: Option<&str>, mode: Mode) -> Option<String> {
    let path = transcript(root, session, mode)?;
    Some(format!("   transcript: {}", path.display()))
}

fn with_session(command: &str) -> (String, Option<String>) {
    if !command.contains(SESSION) {
        return (command.to_string(), None);
    }
    let session = uuid::Uuid::new_v4().to_string();
    (command.replace(SESSION, &session), Some(session))
}

fn spinner(label: &str, following: bool) -> indicatif::ProgressBar {
    let bar =
        indicatif::ProgressBar::with_draw_target(None, indicatif::ProgressDrawTarget::stderr());
    let template = match following {
        true => "{spinner} {msg}",
        false => "{spinner} {msg} {elapsed}",
    };
    bar.set_style(
        indicatif::ProgressStyle::with_template(template).expect("the spinner template is valid"),
    );
    bar.set_message(label.to_string());
    bar.enable_steady_tick(Duration::from_millis(100));
    bar
}

#[allow(clippy::too_many_arguments)]
pub fn tracked(
    label: &str,
    command: &str,
    prompt: &str,
    root: &Path,
    env: &[(&str, &str)],
    timeout: Duration,
    schema: Option<&Value>,
    mode: Mode,
) -> Result<Reply, Failed> {
    let (command, session) = with_session(command);
    announced(
        &mut std::io::stdout(),
        label,
        &command,
        session,
        prompt,
        root,
        env,
        timeout,
        schema,
        mode,
    )
}

pub fn resumable(command: &str) -> bool {
    command.contains(SESSION)
}

/// Runs `command` in the session an earlier `tracked` call announced.
#[allow(clippy::too_many_arguments)]
pub fn resumed(
    label: &str,
    command: &str,
    session: &str,
    prompt: &str,
    root: &Path,
    env: &[(&str, &str)],
    timeout: Duration,
    schema: Option<&Value>,
    mode: Mode,
) -> Result<Reply, Failed> {
    announced(
        &mut std::io::stdout(),
        label,
        &command.replace(SESSION, session),
        Some(session.to_string()),
        prompt,
        root,
        env,
        timeout,
        schema,
        mode,
    )
}

#[allow(clippy::too_many_arguments)]
fn announced(
    out: &mut impl Write,
    label: &str,
    command: &str,
    session: Option<String>,
    prompt: &str,
    root: &Path,
    env: &[(&str, &str)],
    timeout: Duration,
    schema: Option<&Value>,
    mode: Mode,
) -> Result<Reply, Failed> {
    if let Some(session) = session.as_ref().filter(|_| mode == Mode::Plain) {
        let _ = writeln!(out, "{}", watch_line(root, session));
    }
    let failed_at = transcript_line(root, session.as_deref(), mode);
    let transcript = transcript(root, session.as_deref(), mode);
    let bar = spinner(label, transcript.is_some());
    let width = (console::Term::stderr().size().1 as usize).saturating_sub(2);
    let reply = watched(
        label,
        command,
        prompt,
        root,
        env,
        timeout,
        schema,
        transcript,
        width,
        |line| bar.set_message(line),
    );
    bar.finish_and_clear();
    match reply {
        Ok(reply) => Ok(Reply { session, ..reply }),
        Err(message) => Err(Failed {
            message,
            transcript: failed_at,
            session,
        }),
    }
}

/// Runs `command`, showing the transcript's activity while it waits. The reply never reads it.
#[allow(clippy::too_many_arguments)]
fn watched(
    label: &str,
    command: &str,
    prompt: &str,
    root: &Path,
    env: &[(&str, &str)],
    timeout: Duration,
    schema: Option<&Value>,
    transcript: Option<PathBuf>,
    width: usize,
    show: impl FnMut(String) + Send,
) -> Result<Reply, String> {
    let Some(transcript) = transcript else {
        return run(command, prompt, root, env, timeout, schema);
    };
    let (stop, stopped) = mpsc::channel::<()>();
    thread::scope(|scope| {
        scope.spawn(move || follow(Tail::new(transcript), root, label, width, &stopped, show));
        let reply = run(command, prompt, root, env, timeout, schema);
        drop(stop);
        reply
    })
}

pub fn run(
    command: &str,
    prompt: &str,
    root: &Path,
    env: &[(&str, &str)],
    timeout: Duration,
    schema: Option<&Value>,
) -> Result<Reply, String> {
    let Argv {
        words: argv,
        _schema_file,
    } = with_schema(split(command)?, schema)?;
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

/// Claude Code's `verbose` setting turns `--output-format json` into an array of every message.
fn result_message(output: Value) -> Result<Value, String> {
    let Value::Array(messages) = output else {
        return Ok(output);
    };
    messages
        .into_iter()
        .rfind(|m| m["type"] == "result")
        .ok_or_else(|| "runner output array has no `result` message".to_string())
}

fn result_text(stdout: &str) -> Result<Reply, String> {
    let output: Value =
        serde_json::from_str(stdout).map_err(|e| format!("runner output is not JSON: {e}"))?;
    let output = result_message(output)?;
    let text = output["result"].as_str();
    let structured = Some(&output["structured_output"])
        .filter(|v| !v.is_null())
        .cloned();
    let subtype = output["subtype"].as_str();
    let failed =
        output["is_error"].as_bool() == Some(true) || subtype.is_some_and(|s| s != "success");
    if failed {
        let detail = text
            .filter(|t| !t.is_empty())
            .or(subtype)
            .unwrap_or("no detail");
        return Err(format!("runner reported an error: {detail}"));
    }
    match (text, structured) {
        (None, None) => Err("runner output has no `result` text".to_string()),
        (text, structured) => Ok(Reply {
            text: text.unwrap_or_default().to_string(),
            session: None,
            structured,
        }),
    }
}

pub fn json_reply(result: &str) -> Result<serde_json::Value, String> {
    let (Some(start), Some(end)) = (result.find('{'), result.rfind('}')) else {
        return Err("runner reply has no JSON object".to_string());
    };
    serde_json::from_str(result.get(start..=end).unwrap_or_default())
        .map_err(|e| format!("runner reply is not valid JSON: {e}"))
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

    fn root_with_script_in(root: &Path, body: &str) -> String {
        let script = root.join(format!("fake-{}.sh", uuid::Uuid::new_v4()));
        std::fs::write(&script, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        script.to_string_lossy().into_owned()
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
            None,
        )
        .unwrap()
        .text;

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
            run(&command, &prompt, root.path(), &[], LONG, None)
                .unwrap()
                .text,
            "big"
        );
    }

    #[test]
    fn run_kills_a_runner_that_outlasts_the_timeout() {
        let (root, command) = root_with_script("sleep 5");
        let started = Instant::now();

        let error = run(&command, "", root.path(), &[], Duration::from_secs(1), None).unwrap_err();

        assert!(error.contains("timed out after 1s"), "{error}");
        assert!(started.elapsed() < Duration::from_secs(4));
    }

    #[test]
    fn run_reports_the_exit_code_and_the_stderr_tail() {
        let (root, command) =
            root_with_script("for i in 1 2 3 4 5 6 7; do echo \"line $i\" >&2; done; exit 3");

        let error = run(&command, "", root.path(), &[], LONG, None).unwrap_err();

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

        let error = run(&command, "", root.path(), &[], LONG, None).unwrap_err();

        assert!(error.contains("rate limited"), "{error}");
    }

    #[test]
    fn run_fails_on_output_that_is_not_json_or_has_no_result() {
        let (root, command) = root_with_script("echo hello");
        assert!(run(&command, "", root.path(), &[], LONG, None).is_err());
        let (root, command) = root_with_script(r#"printf '{"other": 1}'"#);
        assert!(run(&command, "", root.path(), &[], LONG, None).is_err());
    }

    #[test]
    fn run_reads_the_result_message_from_verbose_array_output() {
        let (root, command) = root_with_script(
            r#"printf '[{"type": "system"}, {"type": "assistant"}, {"type": "result", "subtype": "success", "result": "fine"}]'"#,
        );

        let reply = run(&command, "", root.path(), &[], LONG, None).unwrap();

        assert_eq!(reply.text, "fine");
    }

    #[test]
    fn run_fails_on_verbose_array_output_without_a_result_message() {
        let (root, command) = root_with_script(r#"printf '[{"type": "system"}]'"#);

        let error = run(&command, "", root.path(), &[], LONG, None).unwrap_err();

        assert!(error.contains("no `result` message"), "{error}");
    }

    #[test]
    fn run_prefers_structured_output_and_needs_no_result_text() {
        let (root, command) =
            root_with_script(r#"printf '{"structured_output": {"findings": []}}'"#);

        let reply = run(&command, "", root.path(), &[], LONG, None).unwrap();

        assert_eq!(reply.text, "");
        assert_eq!(reply.json().unwrap(), serde_json::json!({"findings": []}));
    }

    #[test]
    fn a_reply_without_structured_output_falls_back_to_the_json_in_the_text() {
        let (root, command) = root_with_script(
            r#"printf '{"result": "Here: {\\"clean\\": true}", "structured_output": null}'"#,
        );

        let reply = run(&command, "", root.path(), &[], LONG, None).unwrap();

        assert_eq!(reply.json().unwrap()["clean"], true);
    }

    #[test]
    fn run_fails_on_a_subtype_other_than_success() {
        let (root, command) = root_with_script(
            r#"printf '{"subtype": "error_max_structured_output_retries", "result": ""}'"#,
        );

        let error = run(&command, "", root.path(), &[], LONG, None).unwrap_err();

        assert!(
            error.contains("error_max_structured_output_retries"),
            "{error}"
        );
    }

    #[test]
    fn a_success_subtype_is_not_a_failure() {
        let (root, command) =
            root_with_script(r#"printf '{"subtype": "success", "result": "fine"}'"#);

        assert_eq!(
            run(&command, "", root.path(), &[], LONG, None)
                .unwrap()
                .text,
            "fine"
        );
    }

    #[test]
    fn schema_is_substituted_inline_into_an_argv_word() {
        let (root, script) =
            root_with_script(r#"printf '%s' "$1" > argv.txt; printf '{"result": "ok"}'"#);
        let schema = serde_json::json!({"type": "object"});

        run(
            &format!("{script} --schema={{schema}}"),
            "",
            root.path(),
            &[],
            LONG,
            Some(&schema),
        )
        .unwrap();

        assert_eq!(
            std::fs::read_to_string(root.path().join("argv.txt")).unwrap(),
            r#"--schema={"type":"object"}"#
        );
    }

    #[test]
    fn schema_file_names_a_file_holding_the_schema_for_the_run() {
        let (root, script) = root_with_script(
            r#"cat "$1" > schema-seen.txt; printf '%s' "$1" > path.txt; printf '{"result": "ok"}'"#,
        );
        let schema = serde_json::json!({"type": "object", "required": []});

        run(
            &format!("{script} {{schema_file}}"),
            "",
            root.path(),
            &[],
            LONG,
            Some(&schema),
        )
        .unwrap();

        assert_eq!(
            std::fs::read_to_string(root.path().join("schema-seen.txt")).unwrap(),
            r#"{"required":[],"type":"object"}"#
        );
        let path = std::fs::read_to_string(root.path().join("path.txt")).unwrap();
        assert!(!Path::new(&path).exists(), "{path} outlived the run");
    }

    #[test]
    fn a_placeholder_without_a_schema_is_an_error() {
        let (root, script) = root_with_script(r#"printf '{"result": "ok"}'"#);

        let error = run(
            &format!("{script} {{schema}}"),
            "",
            root.path(),
            &[],
            LONG,
            None,
        )
        .unwrap_err();

        assert!(
            error.contains("uses a schema placeholder but no schema"),
            "{error}"
        );
    }

    #[test]
    fn session_is_replaced_with_a_fresh_uuid_per_call() {
        let command = "claude --session-id {session} -p";

        let (first, one) = with_session(command);
        let (second, two) = with_session(command);

        let (one, two) = (one.unwrap(), two.unwrap());
        assert_eq!(first, format!("claude --session-id {one} -p"));
        assert_ne!(one, two);
        assert!(uuid::Uuid::parse_str(&one).is_ok(), "{one}");
        assert_ne!(first, second);
    }

    #[test]
    fn a_command_without_session_has_no_session() {
        let (command, session) = with_session("claude -p");

        assert_eq!(command, "claude -p");
        assert!(session.is_none());
    }

    #[test]
    fn tracked_hands_the_runner_the_session_it_announced() {
        let (root, command) = root_with_script(r#"cat >/dev/null; printf '{"result": "%s"}' "$1""#);
        let mut out = Vec::new();

        let (command, session) = with_session(&format!("{command} {{session}}"));
        let reply = announced(
            &mut out,
            "label",
            &command,
            session,
            "",
            root.path(),
            &[],
            LONG,
            None,
            Mode::Plain,
        )
        .unwrap();

        let printed = String::from_utf8(out).unwrap();
        assert!(uuid::Uuid::parse_str(&reply.text).is_ok(), "{}", reply.text);
        assert!(
            printed.contains(&format!("/{}.jsonl\"", reply.text)),
            "{printed}"
        );
    }

    fn watch_with(
        root: &Path,
        command: &str,
        transcript: Option<PathBuf>,
    ) -> (Result<Reply, String>, Vec<String>) {
        let shown = std::sync::Mutex::new(Vec::new());
        let reply = watched(
            "T1 runner",
            command,
            "",
            root,
            &[],
            LONG,
            None,
            transcript,
            80,
            |line| shown.lock().unwrap().push(line),
        );
        (reply, shown.into_inner().unwrap())
    }

    /// `T1 runner <elapsed>` then exactly `rest`; the elapsed depends on machine load.
    fn label_elapsed(line: &str, rest: &str) -> bool {
        regex::Regex::new(r"^T1 runner \d+s(.*)$")
            .unwrap()
            .captures(line)
            .is_some_and(|c| &c[1] == rest)
    }

    fn tool_line(name: &str, input: serde_json::Value) -> String {
        serde_json::json!({
            "type": "assistant",
            "message": { "content": [{ "type": "tool_use", "name": name, "input": input }]}
        })
        .to_string()
    }

    #[test]
    fn the_spinner_shows_the_latest_tool_call_in_the_transcript() {
        let root = tempfile::tempdir().unwrap();
        let transcript = root.path().join("session.jsonl");
        let read = tool_line(
            "Read",
            serde_json::json!({ "file_path": root.path().join("src/a.rs") }),
        );
        let bash = tool_line("Bash", serde_json::json!({ "command": "cargo test" }));
        std::fs::write(root.path().join("read.json"), read).unwrap();
        std::fs::write(root.path().join("bash.json"), bash).unwrap();
        let command = root_with_script_in(
            root.path(),
            r#"cat read.json > session.jsonl; echo >> session.jsonl; sleep 1; cat bash.json >> session.jsonl; echo >> session.jsonl; sleep 1; printf '{"result": "ok"}'"#,
        );

        let (reply, shown) = watch_with(root.path(), &command, Some(transcript));

        assert_eq!(reply.unwrap().text, "ok");
        let is_read = |l: &String| label_elapsed(l, " · Read src/a.rs");
        let is_bash = |l: &String| label_elapsed(l, " · Bash cargo test");
        let first_bash = shown
            .iter()
            .position(is_bash)
            .unwrap_or_else(|| panic!("{shown:?}"));
        assert!(shown[..first_bash].iter().any(is_read), "{shown:?}");
        assert!(!shown[first_bash..].iter().any(is_read), "{shown:?}");
        assert!(is_bash(shown.last().unwrap()), "{shown:?}");
    }

    #[test]
    fn a_missing_or_garbled_transcript_leaves_label_and_elapsed_and_the_outcome_unchanged() {
        let root = tempfile::tempdir().unwrap();
        let ok = root_with_script_in(root.path(), r#"sleep 1; printf '{"result": "ok"}'"#);
        let garbled = root_with_script_in(
            root.path(),
            r#"printf '{not json\n' > garbled.jsonl; sleep 1; printf '{"result": "ok"}'"#,
        );

        for (command, transcript) in [(&ok, "missing.jsonl"), (&garbled, "garbled.jsonl")] {
            let (reply, shown) =
                watch_with(root.path(), command, Some(root.path().join(transcript)));
            let (plain, _) = watch_with(root.path(), command, None);

            assert_eq!(reply.unwrap().text, plain.unwrap().text);
            assert!(!shown.is_empty());
            assert!(shown.iter().all(|l| label_elapsed(l, "")), "{shown:?}");
        }
    }

    #[test]
    fn a_runner_command_without_session_shows_no_activity() {
        let (root, command) = root_with_script(r#"printf '{"result": "ok"}'"#);
        let (command, session) = with_session(&command);

        let transcript = transcript(root.path(), session.as_deref(), Mode::Live);
        let (reply, shown) = watch_with(root.path(), &command, transcript);

        assert_eq!(reply.unwrap().text, "ok");
        assert!(shown.is_empty(), "{shown:?}");
        assert!(transcript_for_session_in(root.path(), Mode::Live).is_some());
        assert!(transcript_for_session_in(root.path(), Mode::Plain).is_none());
    }

    fn transcript_for_session_in(root: &Path, mode: Mode) -> Option<PathBuf> {
        transcript(root, Some("abc"), mode)
    }

    #[test]
    fn the_transcript_never_decides_the_reply_or_the_exit_status() {
        let root = tempfile::tempdir().unwrap();
        let decoy = r#"{"type":"result","subtype":"success","result":"{\"verdict\": \"done\"}"}"#;
        std::fs::write(root.path().join("decoy.jsonl"), format!("{decoy}\n")).unwrap();
        let failing = root_with_script_in(root.path(), "echo boom >&2; exit 3");
        let passing = root_with_script_in(root.path(), r#"printf '{"result": "real"}'"#);
        let transcript = Some(root.path().join("decoy.jsonl"));

        let (failed, _) = watch_with(root.path(), &failing, transcript.clone());
        let (passed, _) = watch_with(root.path(), &passing, transcript);

        assert_eq!(
            failed.unwrap_err(),
            watch_with(root.path(), &failing, None).0.unwrap_err()
        );
        assert_eq!(passed.unwrap().text, "real");
    }

    #[test]
    fn the_encoded_root_replaces_every_non_alphanumeric_character() {
        let root = tempfile::tempdir().unwrap();
        let nested = root.path().join("x.y_z");
        std::fs::create_dir(&nested).unwrap();
        let canonical = nested.canonicalize().unwrap();

        let expected: String = canonical
            .to_string_lossy()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();

        assert_eq!(encoded_root(&nested), expected);
        assert!(encoded_root(&nested).ends_with("-x-y-z"));
        assert_eq!(encoded_root(Path::new("/Users/a/x.y_z")), "-Users-a-x-y-z");
    }

    #[test]
    fn the_watch_line_follows_the_session_file_under_the_encoded_root() {
        let line = watch_line(Path::new("/Users/a/x.y_z"), "abc");

        assert!(
            line.starts_with(
                "   watch: tail -n +1 -F \"$HOME/.claude/projects/-Users-a-x-y-z/abc.jsonl\" | jq -r '"
            ),
            "{line}"
        );
    }

    #[test]
    fn live_mode_prints_no_watch_line() {
        let (root, command) = root_with_script(r#"cat >/dev/null; printf '{"result": "ok"}'"#);
        let (command, session) = with_session(&format!("{command} {{session}}"));
        let mut out = Vec::new();

        announced(
            &mut out,
            "label",
            &command,
            session,
            "",
            root.path(),
            &[],
            LONG,
            None,
            Mode::Live,
        )
        .unwrap();

        assert_eq!(String::from_utf8(out).unwrap(), "");
    }

    fn failed_call(mode: Mode) -> (Failed, String) {
        let (root, command) = root_with_script("cat >/dev/null; exit 3");
        let (command, session) = with_session(&format!("{command} {{session}}"));
        let session = session.unwrap();
        let failed = announced(
            &mut Vec::new(),
            "label",
            &command,
            Some(session.clone()),
            "",
            root.path(),
            &[],
            LONG,
            None,
            mode,
        )
        .unwrap_err();
        (failed, session)
    }

    #[test]
    fn a_failed_live_call_carries_the_transcript_of_its_own_session() {
        let (failed, session) = failed_call(Mode::Live);

        let line = failed.transcript.unwrap();
        assert!(line.starts_with("   transcript: "), "{line}");
        assert!(line.ends_with(&format!("/{session}.jsonl")), "{line}");
        assert!(
            failed.message.contains("exited with 3"),
            "{}",
            failed.message
        );
    }

    #[test]
    fn a_failed_plain_call_carries_no_transcript() {
        assert_eq!(failed_call(Mode::Plain).0.transcript, None);
    }

    #[test]
    fn a_stopped_live_task_ends_with_the_transcript_of_the_failed_call() {
        let detail = vec!["   runner: boom".to_string()];
        let stopped = "   stopped. T1 left unticked; working tree kept.".to_string();
        let (failed, _) = failed_call(Mode::Live);
        let line = failed.transcript.clone().unwrap();

        let live = stop_lines(&detail, stopped.clone(), failed.transcript);
        let plain = stop_lines(
            &detail,
            stopped.clone(),
            failed_call(Mode::Plain).0.transcript,
        );

        assert_eq!(live, [detail[0].clone(), stopped.clone(), line]);
        assert_eq!(plain, [detail[0].clone(), stopped]);
    }

    #[test]
    fn config_defaults_when_there_is_no_lemma_toml() {
        let root = tempfile::tempdir().unwrap();

        let config = load(root.path()).unwrap();

        assert_eq!(
            config.runner,
            "claude -p --session-id {session} --output-format json --json-schema {schema}"
        );
        assert_eq!(config.reviewer, config.runner);
        assert_eq!(
            config.resume,
            "claude -p --resume {session} --output-format json --json-schema {schema}"
        );
        assert_eq!(config.review_rounds, 1);
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
