use crate::claims::{lines_of, parse, Claim};

pub fn fold(spec_body: &str, delta_claims: &[&Claim]) -> String {
    delta_claims
        .iter()
        .fold(spec_body.to_string(), |body, claim| apply(&body, claim))
}

fn apply(body: &str, claim: &Claim) -> String {
    let behaviour = parse(body);
    let existing = behaviour.claims.iter().find(|c| c.id == claim.id);
    match (existing, claim.struck) {
        (Some(old), true) => splice(body, old.start, old.end, &[]),
        (None, true) => body.to_string(),
        (Some(old), false) => splice(body, old.start, old.end, &claim.block),
        (None, false) => append(body, claim, &behaviour),
    }
}

fn splice(body: &str, start: usize, end: usize, replacement: &[String]) -> String {
    let lines = lines_of(body);
    let mut out: Vec<&str> = lines[..start].to_vec();
    out.extend(replacement.iter().map(String::as_str));
    out.extend(&lines[end..]);
    out.join("\n")
}

fn append(body: &str, claim: &Claim, behaviour: &crate::claims::Behaviour) -> String {
    let Some(section) = &behaviour.section else {
        return with_new_section(body, claim);
    };
    let lines = lines_of(body);
    if let Some(last) = behaviour.claims.last() {
        return splice(body, last.end, last.end, &claim.block);
    }
    let last_content = (section.heading..section.end)
        .rev()
        .find(|&i| !lines[i].trim().is_empty())
        .unwrap_or(section.heading);
    let mut insert = vec![String::new()];
    insert.extend(claim.block.iter().cloned());
    splice(body, last_content + 1, last_content + 1, &insert)
}

fn with_new_section(body: &str, claim: &Claim) -> String {
    let trimmed = body.trim_end_matches('\n');
    let separator = if trimmed.is_empty() { "" } else { "\n\n" };
    format!(
        "{trimmed}{separator}## Behaviour\n\n{}\n",
        claim.block.join("\n")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn delta_claims(body: &str) -> Vec<Claim> {
        parse(body).claims
    }

    fn run(spec: &str, delta: &str) -> String {
        let claims = delta_claims(delta);
        let refs: Vec<&Claim> = claims.iter().collect();
        fold(spec, &refs)
    }

    #[test]
    fn appends_new_claim_with_its_receipt_after_the_last_claim() {
        let spec =
            "## Intent\n\nx\n\n## Behaviour\n\n- [A-1] one\n  @ref a.rs#f@111\n\n## Notes\n\nn\n";
        let delta = "## Behaviour\n\n- [A-2] two\n  @ref a.rs#g@222\n";
        let out = run(spec, delta);
        assert_eq!(
            out,
            "## Intent\n\nx\n\n## Behaviour\n\n- [A-1] one\n  @ref a.rs#f@111\n- [A-2] two\n  @ref a.rs#g@222\n\n## Notes\n\nn\n"
        );
    }

    #[test]
    fn replaces_claim_with_same_id_and_moves_receipt() {
        let spec = "## Behaviour\n\n- [A-1] old\n  @ref a.rs#f@111\n- [A-2] keep\n";
        let delta = "## Behaviour\n\n- [A-1] new\n  @ref a.rs#f@999\n";
        let out = run(spec, delta);
        assert_eq!(
            out,
            "## Behaviour\n\n- [A-1] new\n  @ref a.rs#f@999\n- [A-2] keep\n"
        );
    }

    #[test]
    fn struck_claim_is_deleted_with_its_receipt() {
        let spec = "## Behaviour\n\n- [A-1] old\n  @ref a.rs#f@111\n- [A-2] keep\n";
        let delta = "## Behaviour\n\n- ~~[A-1]~~ old\n";
        assert_eq!(run(spec, delta), "## Behaviour\n\n- [A-2] keep\n");
    }

    #[test]
    fn strike_of_unknown_id_is_a_no_op() {
        let spec = "## Behaviour\n\n- [A-2] keep\n";
        assert_eq!(run(spec, "## Behaviour\n\n- ~~[A-9]~~ x\n"), spec);
    }

    #[test]
    fn creates_behaviour_section_when_absent() {
        let spec = "## Intent\n\nx\n";
        let out = run(spec, "## Behaviour\n\n- [A-1] one\n");
        assert_eq!(out, "## Intent\n\nx\n\n## Behaviour\n\n- [A-1] one\n");
    }

    #[test]
    fn fills_an_empty_behaviour_section() {
        let spec = "## Behaviour\n\n## Notes\n";
        let out = run(spec, "## Behaviour\n\n- [A-1] one\n");
        assert_eq!(out, "## Behaviour\n\n- [A-1] one\n\n## Notes\n");
    }

    #[test]
    fn folding_twice_is_idempotent() {
        let spec = "## Behaviour\n\n- [A-1] one\n";
        let delta = "## Behaviour\n\n- [A-1] one\n- [A-2] two\n";
        let once = run(spec, delta);
        assert_eq!(run(&once, delta), once);
    }

    #[test]
    fn appends_when_behaviour_is_last_without_trailing_newline() {
        let spec = "## Behaviour\n\n- [A-1] one";
        let out = run(spec, "## Behaviour\n\n- [A-2] two\n");
        assert_eq!(out, "## Behaviour\n\n- [A-1] one\n- [A-2] two");
    }

    #[test]
    fn fills_empty_last_behaviour_without_trailing_newline() {
        let out = run(
            "## Intent\n\nx\n\n## Behaviour",
            "## Behaviour\n\n- [A-1] one\n",
        );
        assert_eq!(out, "## Intent\n\nx\n\n## Behaviour\n\n- [A-1] one");
    }

    #[test]
    fn appends_before_the_next_section_after_a_multiline_claim() {
        let spec = "## Behaviour\n\n- [A-1] one\n  more\n  @ref a.rs#f@111\n## Notes\n\nn\n";
        let out = run(spec, "## Behaviour\n\n- [A-2] two\n");
        assert_eq!(
            out,
            "## Behaviour\n\n- [A-1] one\n  more\n  @ref a.rs#f@111\n- [A-2] two\n## Notes\n\nn\n"
        );
    }

    #[test]
    fn replaces_last_claim_when_behaviour_has_no_trailing_newline() {
        let out = run(
            "## Behaviour\n\n- [A-1] old",
            "## Behaviour\n\n- [A-1] new\n",
        );
        assert_eq!(out, "## Behaviour\n\n- [A-1] new");
    }
}
