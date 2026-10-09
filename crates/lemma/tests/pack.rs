mod common;
use common::*;

const PACK_CONFIG: &str = include_str!("../../../.lazyspec.toml");
const SPEC_TEMPLATE: &str = include_str!("../../../.lazyspec/templates/spec/index.md");
const DELTA_TEMPLATE: &str = include_str!("../../../.lazyspec/templates/delta/index.md");
const DELTA_PARTS: [(&str, &str); 3] = [
    (
        "design",
        include_str!("../../../.lazyspec/templates/delta/design.md"),
    ),
    (
        "claims",
        include_str!("../../../.lazyspec/templates/delta/claims.md"),
    ),
    (
        "tasks",
        include_str!("../../../.lazyspec/templates/delta/tasks.md"),
    ),
];
const SEEDS: [(&str, &str); 3] = [
    (
        "docs/convention/CONVENTION.md",
        include_str!("../../../docs/convention/CONVENTION.md"),
    ),
    (
        "docs/convention/DICTUM-001-testing.md",
        include_str!("../../../docs/convention/DICTUM-001-testing.md"),
    ),
    (
        "docs/convention/DICTUM-002-code-change.md",
        include_str!("../../../docs/convention/DICTUM-002-code-change.md"),
    ),
];

fn render(template: &str, title: &str) -> String {
    template
        .replace("{title}", title)
        .replace("{author}", "t")
        .replace("{date}", "2026-01-01")
}

fn adopted() -> Repo {
    let repo = Repo::new();
    repo.write(".lazyspec.toml", PACK_CONFIG);
    for (path, content) in SEEDS {
        repo.write(path, content);
    }
    let spec = render(SPEC_TEMPLATE, "Store")
        .replace("governs: []", "governs: [\"src/**\"]")
        .replace("## Intent\n", "## Intent\nReopened heads keep answers.\n");
    repo.write(&format!("{}/index.md", spec_dir(1)), &spec);
    repo.commit("adopt pack");
    repo
}

fn delta_from_template(repo: &Repo, intent: Option<&str>) {
    let mut body =
        render(DELTA_TEMPLATE, "Retry").replace("related: []", "related:\n- changes: SPEC-001");
    if let Some(text) = intent {
        body = body.replace("## Intent\n", &format!("## Intent\n{text}\n"));
    }
    repo.write(&format!("{}/index.md", delta_dir(1)), &body);
    for (part, template) in DELTA_PARTS {
        repo.delta_part(1, part, template);
    }
}

#[test]
fn a_repo_adopted_from_the_pack_checks_clean() {
    let repo = adopted();

    let run = repo.run(&["lint"], "");

    assert_eq!(
        (run.code, run.stdout.as_str(), run.stderr.as_str()),
        (0, "", "")
    );
}

#[test]
fn a_delta_template_with_only_guidance_has_no_intent() {
    let repo = adopted();
    delta_from_template(&repo, None);

    let run = repo.run(
        &["lint", "DELTA-001", "--json", "--gate", "in-progress"],
        "",
    );

    assert!(
        run.has_message("missing or empty `## Intent` section"),
        "{}",
        run.stdout
    );
}

#[test]
fn a_filled_delta_template_has_intent() {
    let repo = adopted();
    delta_from_template(&repo, Some("Flaky upstream."));

    let run = repo.run(
        &["lint", "DELTA-001", "--json", "--gate", "in-progress"],
        "",
    );

    assert!(!run.has_message("`## Intent`"), "{}", run.stdout);
}

#[test]
fn delta_templates_trip_no_shape_rule_until_a_gate() {
    let repo = adopted();
    delta_from_template(&repo, Some("Flaky upstream."));

    let ungated = repo.run(&["lint", "DELTA-001"], "");
    let gated = repo.run(
        &["lint", "DELTA-001", "--json", "--gate", "in-progress"],
        "",
    );

    assert_eq!((ungated.code, ungated.stdout.as_str()), (0, ""));
    assert_eq!(
        gated.messages(),
        vec!["no claims; write the behaviour as bullets in claims.md"]
    );
}

#[test]
fn the_hook_table_runs_lint_at_validate_and_each_gate() {
    let config: toml::Value = toml::from_str(PACK_CONFIG).unwrap();
    let hooks: Vec<(String, String)> = config["hooks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|hook| {
            let run: Vec<&str> = hook["run"]
                .as_array()
                .unwrap()
                .iter()
                .map(|word| word.as_str().unwrap())
                .collect();
            (hook["name"].as_str().unwrap().to_string(), run.join(" "))
        })
        .collect();

    assert_eq!(
        hooks,
        [
            ("lemma-validate", "lemma lint --hook"),
            ("lemma-intent-gate", "lemma lint --hook --gate in-progress"),
            ("lemma-check-gate", "lemma lint --hook --gate complete"),
        ]
        .map(|(name, run)| (name.to_string(), run.to_string()))
    );
}

#[test]
fn the_plugin_ships_hooks_only() {
    let plugin = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugin");

    assert!(!plugin.join("skills").exists());
    assert!(plugin.join("hooks/hooks.json").is_file());
    let manifest: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(plugin.join(".claude-plugin/plugin.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["version"], "3.2.0");
}
