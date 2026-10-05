mod common;
use common::*;

const SPEC_CLAIMS: &str =
    "- [AC-S-1] old one\n  @ref tests/store_test.rs#other_test@1111111\n- [AC-S-2] old two\n";

fn setup(repo: &Repo, delta_claims: &str) {
    repo.delta_file(1, "draft", &["SPEC-001"], INTENT);
    repo.delta_part(1, "SPEC-001", delta_claims);
    repo.spec_file(1, "active", &["src/**"], "## Intent\n\nA.\n");
    repo.spec_behaviour(1, SPEC_CLAIMS);
    repo.commit("fixtures");
}

fn delta_text(repo: &Repo) -> String {
    repo.read(&delta_part_path(1, "SPEC-001"))
}

fn refused(repo: &Repo, args: &[&str]) -> Run {
    let before = delta_text(repo);
    let run = repo.run(args, "");
    assert_eq!(run.code, 2, "{}", run.stdout);
    assert_eq!(delta_text(repo), before);
    run
}

#[test]
fn base_is_written_as_the_last_line_of_a_replacing_claim() {
    let repo = Repo::new();
    setup(
        &repo,
        "- [AC-S-1] new one\n  @ref a#b@abc\n  more\n- [AC-D1-1] fresh\n",
    );

    let run = repo.run(&["base", "DELTA-001", "AC-S-1"], "");

    assert_eq!(run.code, 0, "{}", run.stdout);
    assert_eq!(
        delta_text(&repo),
        format!(
            "- [AC-S-1] new one\n  @ref a#b@abc\n  more\n{}- [AC-D1-1] fresh\n",
            base_line_for(SPEC_CLAIMS, "AC-S-1")
        )
    );
}

#[test]
fn base_replaces_an_existing_base() {
    let repo = Repo::new();
    setup(&repo, "- ~~[AC-S-2]~~ old two\n  @base 000000000000\n");

    let run = repo.run(&["base", "DELTA-001", "AC-S-2"], "");

    assert_eq!(run.code, 0, "{}", run.stdout);
    assert_eq!(
        delta_text(&repo),
        format!(
            "- ~~[AC-S-2]~~ old two\n{}",
            base_line_for(SPEC_CLAIMS, "AC-S-2")
        )
    );
}

#[test]
fn base_refuses_an_unknown_claim() {
    let repo = Repo::new();
    setup(&repo, "- [AC-S-1] new one\n");

    let run = refused(&repo, &["base", "DELTA-001", "AC-S-9"]);

    assert!(run.has_message("no claim AC-S-9"));
}

#[test]
fn base_refuses_a_claim_the_spec_lacks() {
    let repo = Repo::new();
    setup(&repo, "- [AC-S-1] new one\n- [AC-D1-1] fresh\n");

    let run = refused(&repo, &["base", "DELTA-001", "AC-D1-1"]);

    assert!(run.has_message("SPEC-001 has no claim AC-D1-1"));
}

#[test]
fn base_refuses_when_nothing_is_written_for_a_valid_sibling() {
    let repo = Repo::new();
    setup(&repo, "- [AC-S-1] new one\n- [AC-D1-1] fresh\n");

    refused(&repo, &["base", "DELTA-001", "AC-S-1", "AC-D1-1"]);
}

#[test]
fn base_refuses_an_unresolved_target() {
    let repo = Repo::new();
    setup(&repo, "- [AC-S-1] new one\n");
    repo.delta_part(1, "SPEC-404", "- [AC-X-1] orphan\n");
    repo.commit("orphan part");

    let run = refused(&repo, &["base", "DELTA-001", "AC-X-1"]);

    assert_eq!(run.findings()[0]["severity"], "error");
}

#[test]
fn base_refuses_when_the_target_has_no_behaviour_part() {
    let repo = Repo::new();
    setup(&repo, "- [AC-S-1] new one\n");
    std::fs::remove_file(repo.path(&spec_part_path(1))).unwrap();
    repo.commit("drop part");

    let run = refused(&repo, &["base", "DELTA-001", "AC-S-1"]);

    assert!(run.has_message("no `behaviour` part"));
}

#[test]
fn without_arguments_every_colliding_claim_is_based_and_new_ids_are_left_alone() {
    let repo = Repo::new();
    setup(
        &repo,
        "- [AC-S-1] new one\n- ~~[AC-S-2]~~ old two\n- [AC-D1-1] fresh\n",
    );

    let run = repo.run(&["base", "DELTA-001"], "");

    assert_eq!(run.code, 0, "{}", run.stdout);
    assert_eq!(
        delta_text(&repo),
        format!(
            "- [AC-S-1] new one\n{}- ~~[AC-S-2]~~ old two\n{}- [AC-D1-1] fresh\n",
            base_line_for(SPEC_CLAIMS, "AC-S-1"),
            base_line_for(SPEC_CLAIMS, "AC-S-2")
        )
    );
}

#[test]
fn base_refuses_a_claim_that_lives_only_in_a_plain_behaviour_part() {
    let repo = Repo::new();
    setup(&repo, "- [AC-S-1] new one\n");
    repo.write(
        &format!("{}/behaviour.md", delta_dir(1)),
        "- [AC-D1-9] own\n",
    );
    repo.commit("plain part");

    let run = refused(&repo, &["base", "DELTA-001", "AC-D1-9"]);

    assert!(run.has_message(
        "AC-D1-9 is in the `behaviour` part of DELTA-001, which has no fold target; `lemma base` applies to delta claims"
    ));
}

#[test]
fn base_hashes_a_spec_claim_whose_text_mentions_base() {
    let repo = Repo::new();
    let spec_claims = "- [AC-S-1] the fold strips `@base` lines\n";
    setup(&repo, "- [AC-S-1] new one\n");
    repo.spec_behaviour(1, spec_claims);
    repo.commit("spec mentions base");

    let run = repo.run(&["base", "DELTA-001", "AC-S-1"], "");

    assert_eq!(run.code, 0, "{}", run.stdout);
    assert_eq!(
        delta_text(&repo),
        format!(
            "- [AC-S-1] new one\n{}",
            base_line_for(spec_claims, "AC-S-1")
        )
    );
}

#[test]
fn base_refuses_a_spec_whose_tests_attribute_is_not_a_list_of_strings() {
    let repo = Repo::new();
    setup(&repo, "- [AC-S-1] new one\n");
    let index = format!("{}/index.md", spec_dir(1));
    let text = repo
        .read(&index)
        .replace("governs:", "tests: \"checks/**\"\ngoverns:");
    repo.write(&index, &text);
    repo.commit("bad tests");

    let receipt = refused(
        &repo,
        &[
            "receipt",
            "DELTA-001",
            "AC-S-1",
            "tests/store_test.rs#other_test",
        ],
    );
    let base = refused(&repo, &["base", "DELTA-001"]);

    for run in [receipt, base] {
        assert!(run.has_message("SPEC-001: `tests` must be a list of glob strings"));
    }
}
