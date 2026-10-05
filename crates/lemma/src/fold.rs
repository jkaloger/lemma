use crate::claims::{lines_of, parse, Behaviour, Claim};

pub fn fold(part_body: &str, delta_claims: &[&Claim]) -> String {
    delta_claims
        .iter()
        .fold(part_body.to_string(), |body, claim| apply(&body, claim))
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

fn append(body: &str, claim: &Claim, behaviour: &Behaviour) -> String {
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

    fn run(part: &str, delta_part: &str) -> String {
        let claims = parse(delta_part).claims;
        let refs: Vec<&Claim> = claims.iter().collect();
        fold(part, &refs)
    }

    #[test]
    fn appends_new_claim_with_its_receipt_after_the_last_claim() {
        let part = "- [A-1] one\n  @ref a.rs#f@111\n";
        let out = run(part, "- [A-2] two\n  @ref a.rs#g@222\n");
        assert_eq!(
            out,
            "- [A-1] one\n  @ref a.rs#f@111\n- [A-2] two\n  @ref a.rs#g@222\n"
        );
    }

    #[test]
    fn replaces_claim_with_same_id_and_moves_receipt() {
        let part = "- [A-1] old\n  @ref a.rs#f@111\n- [A-2] keep\n";
        let out = run(part, "- [A-1] new\n  @ref a.rs#f@999\n");
        assert_eq!(out, "- [A-1] new\n  @ref a.rs#f@999\n- [A-2] keep\n");
    }

    #[test]
    fn struck_claim_is_deleted_with_its_receipt() {
        let part = "- [A-1] old\n  @ref a.rs#f@111\n- [A-2] keep\n";
        assert_eq!(run(part, "- ~~[A-1]~~ old\n"), "- [A-2] keep\n");
    }

    #[test]
    fn strike_of_unknown_id_is_a_no_op() {
        let part = "- [A-2] keep\n";
        assert_eq!(run(part, "- ~~[A-9]~~ x\n"), part);
    }

    #[test]
    fn fills_an_empty_part() {
        assert_eq!(run("", "- [A-1] one\n"), "- [A-1] one\n");
        assert_eq!(run("\n", "- [A-1] one\n"), "- [A-1] one\n");
    }

    #[test]
    fn appends_after_prose_in_a_part_without_claims() {
        assert_eq!(
            run("Some notes.\n", "- [A-1] one\n"),
            "Some notes.\n\n- [A-1] one\n"
        );
    }

    #[test]
    fn folding_twice_is_idempotent() {
        let part = "- [A-1] one\n";
        let delta = "- [A-1] one\n- [A-2] two\n";
        let once = run(part, delta);
        assert_eq!(run(&once, delta), once);
    }

    #[test]
    fn appends_when_the_part_has_no_trailing_newline() {
        assert_eq!(
            run("- [A-1] one", "- [A-2] two\n"),
            "- [A-1] one\n- [A-2] two"
        );
    }

    #[test]
    fn appends_after_a_multiline_claim() {
        let part = "- [A-1] one\n  more\n  @ref a.rs#f@111\n";
        assert_eq!(
            run(part, "- [A-2] two\n"),
            "- [A-1] one\n  more\n  @ref a.rs#f@111\n- [A-2] two\n"
        );
    }

    #[test]
    fn replaces_last_claim_when_the_part_has_no_trailing_newline() {
        assert_eq!(run("- [A-1] old", "- [A-1] new\n"), "- [A-1] new");
    }
}
