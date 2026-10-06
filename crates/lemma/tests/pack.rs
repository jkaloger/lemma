mod common;
use common::*;

const PACK_CONFIG: &str = include_str!("../../../.lazyspec.toml");
const SPEC_TEMPLATE: &str = include_str!("../../../.lazyspec/templates/spec/index.md");
const DELTA_TEMPLATE: &str = include_str!("../../../.lazyspec/templates/delta/index.md");
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
    repo.spec_behaviour(
        1,
        "- [AC-S1-1] Reopen keeps the prior answer.\n  @ref tests/store_test.rs#reopen_keeps_prior\n",
    );
    repo.commit("adopt pack");
    repo
}

fn delta_from_template(repo: &Repo, intent: Option<&str>) {
    let mut body =
        render(DELTA_TEMPLATE, "Retry").replace("related: []", "related:\n- folds-into: SPEC-001");
    if let Some(text) = intent {
        body = body.replace("## Intent\n", &format!("## Intent\n{text}\n"));
    }
    repo.write(&format!("{}/index.md", delta_dir(1)), &body);
    repo.delta_part(1, "SPEC-001", "- [AC-DELTA-001-1] Retries once.\n");
}

#[test]
fn a_repo_adopted_from_the_pack_checks_clean() {
    let repo = adopted();

    let run = repo.run(&["check", "--require", "evidence"], "");

    assert_eq!(
        (run.code, run.stdout.as_str(), run.stderr.as_str()),
        (0, "", "")
    );
}

#[test]
fn a_delta_template_with_only_guidance_has_no_intent() {
    let repo = adopted();
    delta_from_template(&repo, None);

    let run = repo.run(&["check", "--json", "--require", "intent"], "");

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

    let run = repo.run(&["check", "--json", "--require", "intent"], "");

    assert!(!run.has_message("`## Intent`"), "{}", run.stdout);
}
