mod common;
#[allow(dead_code)]
#[path = "../src/spinners.rs"]
mod spinners;

use common::*;
use serde_json::json;
use spinners::{Style, STYLES};
use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;

/// A build of two tasks whose runner and reviewer each take long enough for the spinner to turn.
fn two_task_build() -> Repo {
    let repo = Repo::new();
    repo.git(&["config", "user.name", "t"]);
    repo.git(&["config", "user.email", "t@example.com"]);
    repo.git(&["config", "commit.gpgsign", "false"]);
    repo.with_dictums();
    repo.spec_file(1, "active", &["src/**"], "## Intent\n\nSpec one intent.\n");
    repo.delta_file(
        1,
        "in-progress",
        &["SPEC-001"],
        "## Intent\n\nDelta intent.\n",
    );
    repo.delta_part(1, "design", "Design prose.\n");
    repo.delta_claims(
        1,
        "- [AC-1] Reopen keeps the prior answer.\n- [AC-2] Close drops it.\n",
    );
    repo.delta_tasks(
        1,
        "- [ ] T1 reopen (settles AC-1)\n- [ ] T2 close (settles AC-2)\n",
    );
    repo.write(".gitignore", "replies/\n");
    repo.write(
        "tests/store_test.rs",
        "#[test]\nfn reopen_keeps_prior() {}\n#[test]\nfn close_drops_it() {}\n",
    );
    let done = |claim: &str, symbol: &str| {
        json!({ "structured_output": { "outcome": "done", "reason": "", "refs": [
            { "claim": claim, "path": "tests/store_test.rs", "symbol": symbol }
        ]}})
        .to_string()
    };
    let clean = json!({ "structured_output": { "stop": "", "findings": [] } }).to_string();
    repo.write("replies/T1.json", &done("AC-1", "reopen_keeps_prior"));
    repo.write("replies/T2.json", &done("AC-2", "close_drops_it"));
    repo.write("replies/review.json", &clean);
    repo.write(
        "agent.sh",
        "#!/bin/sh\ncat > /dev/null\nsleep 1\nif [ \"$1\" = review ]; then cat replies/review.json; exit 0; fi\nmkdir -p src && echo 'pub fn f() {}' > src/$LEMMA_TASK.rs\ncat replies/$LEMMA_TASK.json\n",
    );
    let agent = repo.path("agent.sh");
    std::fs::set_permissions(&agent, std::fs::Permissions::from_mode(0o755)).unwrap();
    repo.write(
        "lemma.toml",
        &format!(
            "runner = \"{0} build\"\nreviewer = \"{0} review\"\n",
            agent.display()
        ),
    );
    repo.commit("start");
    repo
}

/// Every spinner frame drawn, keyed by the task its label names.
fn frames_by_task(screen: &str) -> BTreeMap<String, Vec<String>> {
    let drawn = regex::Regex::new(r"^(\S+) (T\d+) (?:runner|review|fix)\b").unwrap();
    let screen = console::strip_ansi_codes(screen);
    let mut frames: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for line in screen.split(['\r', '\n']) {
        if let Some(caps) = drawn.captures(line.trim_start()) {
            frames
                .entry(caps[2].to_string())
                .or_default()
                .push(caps[1].to_string());
        }
    }
    frames
}

fn style_of(frame: &str) -> Option<&'static Style> {
    STYLES.iter().find(|s| s.frames.contains(&frame))
}

fn live_build_frames() -> BTreeMap<String, Vec<String>> {
    let repo = two_task_build();
    let run = repo.run_on_terminal(&["build", "DELTA-001"]);
    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert!(run.stdout.contains("all tasks complete"), "{}", run.stdout);
    let frames = frames_by_task(&run.stdout);
    assert_eq!(
        frames.keys().collect::<Vec<_>>(),
        ["T1", "T2"],
        "{}",
        run.stdout
    );
    frames
}

#[test]
fn every_live_spinner_frame_is_from_the_built_in_styles() {
    for (task, frames) in live_build_frames() {
        assert!(frames.len() > 1, "{task}: {frames:?}");
        for frame in &frames {
            assert!(style_of(frame).is_some(), "{task}: {frame} in {frames:?}");
        }
    }
}

#[test]
fn one_task_keeps_one_spinner_style() {
    for (task, frames) in live_build_frames() {
        let styles: Vec<&str> = frames
            .iter()
            .filter_map(|f| style_of(f).map(|s| s.name))
            .collect();
        assert_eq!(styles.len(), frames.len(), "{task}: {frames:?}");
        assert!(
            styles.iter().all(|s| *s == styles[0]),
            "{task}: {frames:?} spans {styles:?}"
        );
    }
}

#[test]
fn piped_build_output_has_no_colour_or_spinner_frames() {
    let repo = two_task_build();
    let run = repo.run_with_colour_allowed(&["build", "DELTA-001"], "");
    assert_eq!(run.code, 0, "{}{}", run.stdout, run.stderr);
    assert!(run.stdout.contains("all tasks complete"), "{}", run.stdout);
    for out in [&run.stdout, &run.stderr] {
        assert!(!out.contains('\x1b'), "{out:?}");
        assert!(!out.contains('\r'), "{out:?}");
        assert!(frames_by_task(out).is_empty(), "{out}");
        let glyphs: Vec<&str> = STYLES
            .iter()
            .flat_map(|s| s.frames)
            .filter(|f| !f.is_ascii() && out.contains(**f))
            .copied()
            .collect();
        assert!(glyphs.is_empty(), "{glyphs:?} in {out}");
    }
}

/// `text` wrapped in the SGR colour `code`, as the console crate writes it.
fn sgr(code: u8, text: &str) -> String {
    format!("\x1b[{code}m{text}\x1b[0m")
}

#[test]
fn live_build_colours_task_id_status_and_ref_lines_distinctly() {
    let (cyan, yellow, green, red, blue) = (36, 33, 32, 31, 34);

    let passed = two_task_build().run_on_terminal(&["build", "DELTA-001"]);
    assert_eq!(passed.code, 0, "{}", passed.stdout);
    let out = &passed.stdout;
    for shown in [
        sgr(cyan, "T1"),
        sgr(yellow, "runner"),
        sgr(yellow, "review"),
        sgr(green, "✓"),
        sgr(
            blue,
            "AC-1 @ref tests/store_test.rs#reopen_keeps_prior resolves",
        ),
    ] {
        assert!(out.contains(&shown), "{shown:?} not in {out:?}");
    }

    let repo = two_task_build();
    repo.write(
        "replies/T1.json",
        &json!({ "structured_output": { "outcome": "blocked", "reason": "no", "refs": [] } })
            .to_string(),
    );
    let failed = repo.run_on_terminal(&["build", "DELTA-001"]);
    assert_ne!(failed.code, 0, "{}", failed.stdout);
    let out = &failed.stdout;
    assert!(
        out.contains(&format!("\x1b[{red}m   stopped. T1")),
        "red stop not in {out:?}"
    );
}
