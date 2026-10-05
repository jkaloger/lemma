use crate::git::git_output;
use crate::session::recorded_base;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rule {
    Explicit,
    MergeBase,
    SessionBase,
    DeltaOrigin,
    Head,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Range {
    pub rev: String,
    pub rule: Rule,
}

#[derive(Debug, Default)]
pub struct Inputs<'a> {
    pub explicit: Option<&'a str>,
    pub trunk: Option<&'a str>,
    pub session: Option<&'a str>,
    pub delta_index: Option<&'a Path>,
}

fn rev_parse(root: &Path, rev: &str) -> Option<String> {
    let spec = format!("{rev}^{{commit}}");
    let out = git_output(root, &["rev-parse", "--verify", "--quiet", &spec])?;
    Some(out.trim().to_string())
}

fn trunk_candidates(root: &Path) -> Vec<String> {
    let first_existing = |names: [&str; 2]| {
        names
            .into_iter()
            .find(|name| rev_parse(root, name).is_some())
            .map(str::to_string)
    };
    let remote = first_existing(["origin/main", "origin/master"]).or_else(|| {
        let target = git_output(
            root,
            &["symbolic-ref", "--short", "refs/remotes/origin/HEAD"],
        )?;
        let target = target.trim();
        rev_parse(root, target).map(|_| target.to_string())
    });
    let local = first_existing(["main", "master"]);
    remote.into_iter().chain(local).collect()
}

fn is_ancestor(root: &Path, ancestor: &str, descendant: &str) -> bool {
    git_output(root, &["merge-base", "--is-ancestor", ancestor, descendant]).is_some()
}

fn nearest_merge_base(root: &Path, trunks: &[String]) -> Option<String> {
    let bases = trunks
        .iter()
        .filter_map(|trunk| git_output(root, &["merge-base", "HEAD", trunk]));
    bases
        .map(|base| base.trim().to_string())
        .reduce(
            |best, base| match best != base && is_ancestor(root, &best, &base) {
                true => base,
                false => best,
            },
        )
}

fn delta_origin_parent(root: &Path, index: &Path) -> Option<String> {
    let index = index.to_string_lossy();
    let log = git_output(
        root,
        &["log", "--diff-filter=A", "--format=%H", "--", &index],
    )?;
    let added_in = log.lines().last()?;
    rev_parse(root, &format!("{added_in}^"))
}

pub fn resolve(root: &Path, inputs: &Inputs) -> Range {
    let found = |rev: String, rule| Range { rev, rule };
    if let Some(rev) = inputs.explicit {
        return found(rev.to_string(), Rule::Explicit);
    }
    let head = rev_parse(root, "HEAD");
    let head_rev = head.clone().unwrap_or_else(|| "HEAD".to_string());
    let trunks = match inputs.trunk {
        Some(trunk) => vec![trunk.to_string()],
        None => trunk_candidates(root),
    };
    if let Some(base) = nearest_merge_base(root, &trunks).filter(|base| Some(base) != head.as_ref())
    {
        return found(base, Rule::MergeBase);
    }
    if let Some(base) = inputs.session.and_then(|id| recorded_base(root, id)) {
        return found(base, Rule::SessionBase);
    }
    if let Some(parent) = inputs
        .delta_index
        .and_then(|index| delta_origin_parent(root, index))
    {
        return found(parent, Rule::DeltaOrigin);
    }
    found(head_rev, Rule::Head)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::record_base;
    use crate::testgit::Repo;

    const DELTA: &str = "docs/deltas/DELTA-001/index.md";

    fn trunk_with_history() -> (Repo, String) {
        let repo = Repo::new();
        repo.write("a.txt", "1");
        let first = repo.commit("first");
        (repo, first)
    }

    fn inputs<'a>() -> Inputs<'a> {
        Inputs::default()
    }

    #[test]
    fn a_branch_uses_the_merge_base_with_trunk() {
        let (repo, first) = trunk_with_history();
        repo.git(&["checkout", "-q", "-b", "feature"]);
        repo.write("b.txt", "2");
        repo.commit("work");
        repo.write("c.txt", "3");
        repo.commit("more work");
        let range = resolve(repo.root(), &inputs());
        assert_eq!(
            range,
            Range {
                rev: first,
                rule: Rule::MergeBase
            }
        );
    }

    #[test]
    fn a_branch_ignores_the_session_base_and_delta_origin() {
        let (repo, first) = trunk_with_history();
        repo.git(&["checkout", "-q", "-b", "feature"]);
        repo.write("b.txt", "2");
        repo.commit("work");
        record_base(repo.root(), "s1");
        repo.write(DELTA, "x");
        repo.commit("delta");
        let delta = Path::new(DELTA);
        let range = resolve(
            repo.root(),
            &Inputs {
                session: Some("s1"),
                delta_index: Some(delta),
                ..inputs()
            },
        );
        assert_eq!(
            range,
            Range {
                rev: first,
                rule: Rule::MergeBase
            }
        );
    }

    fn merge_base_range(repo: &Repo, first: String) {
        let range = resolve(repo.root(), &inputs());
        assert_eq!(
            range,
            Range {
                rev: first,
                rule: Rule::MergeBase
            }
        );
    }

    #[test]
    fn master_is_the_trunk_when_main_does_not_exist() {
        let (repo, first) = trunk_with_history();
        repo.git(&["branch", "-m", "master"]);
        repo.git(&["checkout", "-q", "-b", "feature"]);
        repo.write("b.txt", "2");
        repo.commit("work");
        merge_base_range(&repo, first);
    }

    #[test]
    fn origin_head_is_the_trunk_when_neither_main_nor_master_exists() {
        let (repo, first) = trunk_with_history();
        repo.git(&["branch", "-m", "develop"]);
        repo.git(&["update-ref", "refs/remotes/origin/develop", "HEAD"]);
        repo.git(&[
            "symbolic-ref",
            "refs/remotes/origin/HEAD",
            "refs/remotes/origin/develop",
        ]);
        repo.git(&["checkout", "-q", "-b", "feature"]);
        repo.write("b.txt", "2");
        repo.commit("work");
        merge_base_range(&repo, first);
    }

    #[test]
    fn a_remote_trunk_ahead_of_a_stale_local_main_is_preferred() {
        let (repo, _) = trunk_with_history();
        repo.git(&["checkout", "-q", "-b", "upstream"]);
        repo.write("u.txt", "u");
        let upstream = repo.commit("upstream work");
        repo.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
        repo.git(&["checkout", "-q", "-b", "feature"]);
        repo.write("b.txt", "2");
        repo.commit("work");
        merge_base_range(&repo, upstream);
    }

    #[test]
    fn a_local_main_ahead_of_the_remote_trunk_is_preferred() {
        let (repo, _) = trunk_with_history();
        repo.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
        repo.write("u.txt", "u");
        let local = repo.commit("unpushed trunk work");
        repo.git(&["checkout", "-q", "-b", "feature"]);
        repo.write("b.txt", "2");
        repo.commit("work");
        merge_base_range(&repo, local);
    }

    #[test]
    fn a_custom_trunk_name_is_honoured() {
        let (repo, first) = trunk_with_history();
        repo.git(&["branch", "-m", "develop"]);
        repo.git(&["checkout", "-q", "-b", "feature"]);
        repo.write("b.txt", "2");
        repo.commit("work");
        let range = resolve(
            repo.root(),
            &Inputs {
                trunk: Some("develop"),
                ..inputs()
            },
        );
        assert_eq!(
            range,
            Range {
                rev: first,
                rule: Rule::MergeBase
            }
        );
    }

    #[test]
    fn on_trunk_the_session_base_is_used() {
        let (repo, first) = trunk_with_history();
        record_base(repo.root(), "s1");
        repo.write("b.txt", "2");
        repo.commit("work");
        let range = resolve(
            repo.root(),
            &Inputs {
                session: Some("s1"),
                ..inputs()
            },
        );
        assert_eq!(
            range,
            Range {
                rev: first,
                rule: Rule::SessionBase
            }
        );
    }

    #[test]
    fn on_trunk_without_a_session_the_delta_origin_parent_is_used() {
        let (repo, first) = trunk_with_history();
        repo.write(DELTA, "draft");
        repo.commit("add delta");
        repo.write(DELTA, "edited");
        repo.commit("edit delta");
        let delta = Path::new(DELTA);
        let range = resolve(
            repo.root(),
            &Inputs {
                delta_index: Some(delta),
                ..inputs()
            },
        );
        assert_eq!(
            range,
            Range {
                rev: first,
                rule: Rule::DeltaOrigin
            }
        );
    }

    #[test]
    fn the_session_base_wins_over_the_delta_origin_on_trunk() {
        let (repo, first) = trunk_with_history();
        record_base(repo.root(), "s1");
        repo.write(DELTA, "draft");
        repo.commit("add delta");
        let delta = Path::new(DELTA);
        let range = resolve(
            repo.root(),
            &Inputs {
                session: Some("s1"),
                delta_index: Some(delta),
                ..inputs()
            },
        );
        assert_eq!(
            range,
            Range {
                rev: first,
                rule: Rule::SessionBase
            }
        );
    }

    #[test]
    fn an_uncommitted_delta_uses_head() {
        let (repo, _) = trunk_with_history();
        repo.write("b.txt", "2");
        let head = repo.commit("work");
        repo.write(DELTA, "draft");
        let delta = Path::new(DELTA);
        let range = resolve(
            repo.root(),
            &Inputs {
                delta_index: Some(delta),
                ..inputs()
            },
        );
        assert_eq!(
            range,
            Range {
                rev: head,
                rule: Rule::Head
            }
        );
    }

    #[test]
    fn on_trunk_with_nothing_else_uses_head() {
        let (repo, first) = trunk_with_history();
        let range = resolve(repo.root(), &inputs());
        assert_eq!(
            range,
            Range {
                rev: first,
                rule: Rule::Head
            }
        );
    }

    #[test]
    fn an_unrecorded_session_falls_through() {
        let (repo, first) = trunk_with_history();
        let range = resolve(
            repo.root(),
            &Inputs {
                session: Some("nope"),
                ..inputs()
            },
        );
        assert_eq!(
            range,
            Range {
                rev: first,
                rule: Rule::Head
            }
        );
    }

    #[test]
    fn a_session_base_off_head_history_falls_through_to_the_delta_origin() {
        let (repo, first) = trunk_with_history();
        repo.git(&["checkout", "-q", "-b", "other"]);
        repo.write("x.txt", "x");
        repo.commit("elsewhere");
        record_base(repo.root(), "s1");
        repo.git(&["checkout", "-q", "main"]);
        repo.write(DELTA, "draft");
        repo.commit("add delta");
        let delta = Path::new(DELTA);
        let range = resolve(
            repo.root(),
            &Inputs {
                session: Some("s1"),
                delta_index: Some(delta),
                ..inputs()
            },
        );
        assert_eq!(
            range,
            Range {
                rev: first,
                rule: Rule::DeltaOrigin
            }
        );
    }

    #[test]
    fn a_repo_without_a_trunk_branch_falls_through() {
        let (repo, _) = trunk_with_history();
        repo.git(&["branch", "-m", "trunkless"]);
        record_base(repo.root(), "s1");
        let base = repo.head();
        repo.write("b.txt", "2");
        repo.commit("work");
        let range = resolve(
            repo.root(),
            &Inputs {
                session: Some("s1"),
                ..inputs()
            },
        );
        assert_eq!(
            range,
            Range {
                rev: base,
                rule: Rule::SessionBase
            }
        );
        let head = repo.head();
        assert_eq!(
            resolve(repo.root(), &inputs()),
            Range {
                rev: head,
                rule: Rule::Head
            }
        );
    }

    #[test]
    fn explicit_base_wins_over_everything() {
        let (repo, _) = trunk_with_history();
        repo.git(&["checkout", "-q", "-b", "feature"]);
        repo.write("b.txt", "2");
        repo.commit("work");
        record_base(repo.root(), "s1");
        let range = resolve(
            repo.root(),
            &Inputs {
                explicit: Some("origin/main"),
                session: Some("s1"),
                ..inputs()
            },
        );
        assert_eq!(
            range,
            Range {
                rev: "origin/main".to_string(),
                rule: Rule::Explicit
            }
        );
    }
}
