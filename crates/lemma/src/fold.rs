use crate::claims::{lines_of, parse, Behaviour, Claim};

#[derive(Debug, PartialEq, Eq)]
pub struct FoldIssue {
    pub id: String,
    pub line: usize,
    pub message: String,
}

pub fn fold(part_body: &str, delta_claims: &[&Claim]) -> Result<String, Vec<FoldIssue>> {
    let spec = parse(part_body);
    let issues: Vec<FoldIssue> = delta_claims
        .iter()
        .filter_map(|claim| issue_for(&spec, claim))
        .collect();
    if !issues.is_empty() {
        return Err(issues);
    }
    Ok(delta_claims
        .iter()
        .fold(part_body.to_string(), |body, claim| apply(&body, claim)))
}

fn issue_for(spec: &Behaviour, claim: &Claim) -> Option<FoldIssue> {
    let exists = spec.claims.iter().any(|c| c.id == claim.id);
    let message = match (exists, claim.struck) {
        (true, false) => format!("claims are immutable; strike {} and add a new id", claim.id),
        (false, true) => format!("{} is struck but the spec has no such claim", claim.id),
        _ => return None,
    };
    Some(FoldIssue {
        id: claim.id.clone(),
        line: claim.line,
        message,
    })
}

fn apply(body: &str, claim: &Claim) -> String {
    let behaviour = parse(body);
    let existing = behaviour.claims.iter().find(|c| c.id == claim.id);
    match (existing, claim.struck) {
        (Some(old), true) => splice(body, old.start, old.end, &[]),
        (None, false) => append(body, claim, &behaviour),
        _ => body.to_string(),
    }
}

pub(crate) fn splice(body: &str, start: usize, end: usize, replacement: &[String]) -> String {
    let lines = lines_of(body);
    let mut out: Vec<&str> = lines[..start].to_vec();
    out.extend(replacement.iter().map(String::as_str));
    out.extend(&lines[end..]);
    out.join("\n")
}

pub(crate) fn append(body: &str, claim: &Claim, behaviour: &Behaviour) -> String {
    if let Some(last) = behaviour.claims.last() {
        return splice(body, last.end, last.end, &claim.block);
    }
    let trimmed = body.trim_end_matches('\n');
    let separator = if trimmed.is_empty() { "" } else { "\n\n" };
    format!("{trimmed}{separator}{}\n", claim.block.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(part: &str, delta_part: &str) -> Result<String, Vec<FoldIssue>> {
        let claims = parse(delta_part).claims;
        let refs: Vec<&Claim> = claims.iter().collect();
        fold(part, &refs)
    }

    fn messages(result: Result<String, Vec<FoldIssue>>) -> Vec<String> {
        result
            .unwrap_err()
            .into_iter()
            .map(|i| format!("{}: {}", i.id, i.message))
            .collect()
    }

    #[test]
    fn appends_new_claim_with_its_ref_after_the_last_claim() {
        let part = "- [A-1] one\n  @ref tests/a.rs#f\n";
        let out = run(part, "- [A-2] two\n  @ref tests/a.rs#g\n").unwrap();
        assert_eq!(
            out,
            "- [A-1] one\n  @ref tests/a.rs#f\n- [A-2] two\n  @ref tests/a.rs#g\n"
        );
    }

    #[test]
    fn struck_claim_is_deleted_with_its_ref() {
        let part = "- [A-1] old\n  @ref tests/a.rs#f\n- [A-2] keep\n";
        assert_eq!(run(part, "- ~~[A-1]~~ old\n").unwrap(), "- [A-2] keep\n");
    }

    #[test]
    fn strike_and_new_id_replace_a_claim() {
        let part = "- [A-1] old\n- [A-2] keep\n";
        let out = run(part, "- ~~[A-1]~~ old\n- [A-3] new\n").unwrap();
        assert_eq!(out, "- [A-2] keep\n- [A-3] new\n");
    }

    #[test]
    fn unstruck_claim_with_an_existing_id_is_immutable() {
        let issues = run("- [A-1] old\n", "- [A-1] new\n").unwrap_err();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].id, "A-1");
        assert_eq!(issues[0].line, 1);
        assert_eq!(
            issues[0].message,
            "claims are immutable; strike A-1 and add a new id"
        );
    }

    #[test]
    fn strike_of_an_id_the_spec_lacks_is_an_error() {
        let issues = messages(run("- [A-2] keep\n", "- ~~[A-9]~~ x\n"));
        assert_eq!(
            issues,
            vec!["A-9: A-9 is struck but the spec has no such claim"]
        );
    }

    #[test]
    fn every_issue_is_reported_and_nothing_is_merged() {
        let part = "- [A-1] old\n";
        let issues = messages(run(part, "- [A-1] new\n- ~~[A-9]~~ x\n- [A-2] fine\n"));
        assert_eq!(issues.len(), 2);
    }

    #[test]
    fn refolding_the_same_delta_is_an_error() {
        let delta = "- [A-2] two\n";
        let once = run("- [A-1] one\n", delta).unwrap();
        assert_eq!(messages(run(&once, delta)).len(), 1);
    }

    #[test]
    fn fills_an_empty_part() {
        assert_eq!(run("", "- [A-1] one\n").unwrap(), "- [A-1] one\n");
        assert_eq!(run("\n", "- [A-1] one\n").unwrap(), "- [A-1] one\n");
    }

    #[test]
    fn appends_after_prose_in_a_part_without_claims() {
        assert_eq!(
            run("Some notes.\n", "- [A-1] one\n").unwrap(),
            "Some notes.\n\n- [A-1] one\n"
        );
    }

    #[test]
    fn appends_when_the_part_has_no_trailing_newline() {
        assert_eq!(
            run("- [A-1] one", "- [A-2] two\n").unwrap(),
            "- [A-1] one\n- [A-2] two"
        );
    }

    #[test]
    fn appends_after_a_multiline_claim() {
        let part = "- [A-1] one\n  more\n  @ref tests/a.rs#f\n";
        assert_eq!(
            run(part, "- [A-2] two\n").unwrap(),
            "- [A-1] one\n  more\n  @ref tests/a.rs#f\n- [A-2] two\n"
        );
    }
}
