use crate::claims::{block_hash, lines_of, parse, without_base, Behaviour, Claim};

pub fn fold(part_body: &str, delta_claims: &[&Claim]) -> String {
    delta_claims
        .iter()
        .fold(part_body.to_string(), |body, claim| apply(&body, claim))
}

#[derive(Debug, PartialEq, Eq)]
pub enum BaseKind {
    Missing,
    StrikeOfUnknown,
    Stale { spec: String, base: String },
    OrphanBase,
}

#[derive(Debug, PartialEq, Eq)]
pub struct BaseIssue {
    pub claim_id: String,
    pub line: usize,
    pub kind: BaseKind,
}

impl BaseIssue {
    pub fn message(&self, delta_id: &str, spec_id: &str) -> String {
        let id = &self.claim_id;
        match &self.kind {
            BaseKind::Missing => format!(
                "{id}: replaces/strikes a spec claim without `@base`; run `lemma base {delta_id} {id}`"
            ),
            BaseKind::StrikeOfUnknown => format!("{id}: strikes a claim {spec_id} does not have"),
            BaseKind::Stale { spec, base } => format!(
                "{id}: spec block changed since `@base` (spec {spec}, base {base}); re-read the spec claim and run `lemma base {delta_id} {id}`"
            ),
            BaseKind::OrphanBase => format!("{id}: has `@base` but {spec_id} has no claim {id}"),
        }
    }

    pub fn blocks_start(&self) -> bool {
        matches!(self.kind, BaseKind::Missing | BaseKind::StrikeOfUnknown)
    }
}

pub fn base_issues(spec_body: &str, delta_claims: &[Claim]) -> Vec<BaseIssue> {
    let spec = parse(spec_body);
    delta_claims
        .iter()
        .filter_map(|claim| {
            let existing = spec.claims.iter().find(|c| c.id == claim.id);
            let kind = match (existing, &claim.base) {
                (None, _) if claim.struck => BaseKind::StrikeOfUnknown,
                (Some(_), None) => BaseKind::Missing,
                (Some(old), Some(base)) if block_hash(old) != *base => BaseKind::Stale {
                    spec: block_hash(old),
                    base: base.clone(),
                },
                (None, Some(_)) => BaseKind::OrphanBase,
                _ => return None,
            };
            Some(BaseIssue {
                claim_id: claim.id.clone(),
                line: claim.line,
                kind,
            })
        })
        .collect()
}

fn apply(body: &str, claim: &Claim) -> String {
    let claim = &Claim {
        block: without_base(&claim.block),
        ..claim.clone()
    };
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
    fn base_lines_are_stripped_from_folded_claims() {
        let part = "- [A-1] old\n- [A-2] keep\n";
        let out = run(part, "- [A-1] new\n  @ref a.rs#f@999\n  @base 0123456789ab\n- [A-3] fresh\n  @base 0123456789ab\n");
        assert_eq!(
            out,
            "- [A-1] new\n  @ref a.rs#f@999\n- [A-2] keep\n- [A-3] fresh\n"
        );
    }

    fn issues(part: &str, delta_part: &str) -> Vec<BaseIssue> {
        base_issues(part, &parse(delta_part).claims)
    }

    fn base_for(part: &str) -> String {
        block_hash(&parse(part).claims[0])
    }

    #[test]
    fn replace_or_strike_without_base_is_missing() {
        let part = "- [A-1] old\n";
        assert_eq!(issues(part, "- [A-1] new\n")[0].kind, BaseKind::Missing);
        assert_eq!(issues(part, "- ~~[A-1]~~ old\n")[0].kind, BaseKind::Missing);
    }

    #[test]
    fn matching_base_and_new_ids_raise_nothing() {
        let part = "- [A-1] old\n";
        let delta = format!("- [A-1] new\n  @base {}\n- [A-2] fresh\n", base_for(part));
        assert!(issues(part, &delta).is_empty());
    }

    #[test]
    fn changed_spec_block_is_stale_naming_both_hashes() {
        let part = "- [A-1] changed\n";
        let issue = &issues(part, "- [A-1] new\n  @base 000000000000\n")[0];
        let spec = base_for(part);
        assert_eq!(
            issue.kind,
            BaseKind::Stale {
                spec: spec.clone(),
                base: "000000000000".into()
            }
        );
        let message = issue.message("DELTA-001", "SPEC-001");
        assert!(message.contains(&spec) && message.contains("000000000000"));
    }

    #[test]
    fn base_on_an_id_the_spec_lacks_is_orphan() {
        let issue = &issues("- [A-2] x\n", "- [A-1] new\n  @base 000000000000\n")[0];
        assert_eq!(issue.kind, BaseKind::OrphanBase);
    }

    #[test]
    fn struck_claim_with_base_for_an_id_the_spec_lacks_is_a_strike_of_unknown() {
        let issue = &issues("- [A-2] x\n", "- ~~[A-9]~~ gone\n  @base 000000000000\n")[0];
        assert_eq!(issue.kind, BaseKind::StrikeOfUnknown);
    }

    #[test]
    fn strike_of_an_id_the_spec_lacks_is_flagged() {
        let issue = &issues("- [A-2] x\n", "- ~~[A-9]~~ gone\n")[0];
        assert_eq!(issue.kind, BaseKind::StrikeOfUnknown);
        assert!(issue.blocks_start());
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
