// Release 1.x receipt machinery, kept compiling until the old commands are removed (W45).
use crate::claims::{parse, Claim};
use crate::fold::{append, splice};
use regex::Regex;
use std::sync::OnceLock;

pub const FORCED_MARKER: &str = "[forced]";

pub fn is_forced(after_ref: &str) -> bool {
    after_ref.trim_start().starts_with(FORCED_MARKER)
}

fn base_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\s*@base\s+([0-9a-f]+)").unwrap())
}

pub fn base_in(lines: &[&str], live: &[bool], start: usize, end: usize) -> Option<String> {
    (start..end)
        .filter(|&i| live[i] && is_base_line(lines[i]))
        .find_map(|i| base_pattern().captures(lines[i]))
        .map(|c| c[1].to_string())
}

const BASE_MARKER: &str = "@base";
const HASH_LEN: usize = 12;

pub fn is_base_line(line: &str) -> bool {
    line.trim_start()
        .strip_prefix(BASE_MARKER)
        .is_some_and(|rest| rest.starts_with(' '))
}

pub fn is_annotation(line: &str) -> bool {
    let line = line.trim_start();
    is_base_line(line) || line.starts_with("@ref ") || line.starts_with("@checked ")
}

pub fn without_base(block: &[String]) -> Vec<String> {
    block
        .iter()
        .filter(|line| !is_base_line(line))
        .cloned()
        .collect()
}

pub fn block_hash(claim: &Claim) -> String {
    let text = without_base(&claim.block)
        .iter()
        .enumerate()
        .map(|(i, line)| if i == 0 { line.trim() } else { line.trim_end() })
        .collect::<Vec<_>>()
        .join("\n");
    let mut hash = lazyspec::engine::hashing::sha256_hex(text.as_bytes());
    hash.truncate(HASH_LEN);
    hash
}

pub fn base_line(hash: &str) -> String {
    format!("  {BASE_MARKER} {hash}")
}

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

    #[test]
    fn base_line_is_parsed() {
        let b = parse("- [A-1] one\n  @ref a.rs#f@abc\n  @base 0123abcdef45\n- [A-2] two\n");
        assert_eq!(b.claims[0].base.as_deref(), Some("0123abcdef45"));
        assert_eq!(b.claims[1].base, None);
    }

    fn hash_of(body: &str) -> String {
        block_hash(&parse(body).claims[0])
    }

    #[test]
    fn hash_is_twelve_hex_chars_and_ignores_id_indentation_and_trailing_space() {
        let plain = hash_of("- [A-1] one\n  @ref a.rs#f@abc\n");
        assert_eq!(plain.len(), 12);
        assert!(plain.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(plain, hash_of("- [A-1] one   \n  @ref a.rs#f@abc\t\n"));
        let indented = block_hash(&Claim {
            block: vec!["  - [A-1] one".into(), "  @ref a.rs#f@abc".into()],
            ..parse("- [A-1] one\n").claims.remove(0)
        });
        assert_eq!(plain, indented);
    }

    #[test]
    fn hash_ignores_the_base_line_and_tracks_text_and_receipts() {
        let plain = hash_of("- [A-1] one\n  @ref a.rs#f@abc\n");
        assert_eq!(
            plain,
            hash_of("- [A-1] one\n  @ref a.rs#f@abc\n  @base 111111111111\n")
        );
        assert_ne!(plain, hash_of("- [A-1] uno\n  @ref a.rs#f@abc\n"));
        assert_ne!(plain, hash_of("- [A-1] one\n  @ref a.rs#f@abd\n"));
    }

    #[test]
    fn a_claim_that_mentions_base_in_its_text_keeps_its_id_line() {
        let body =
            "- [A-1] the fold strips `@base` lines\n  @ref a.rs#f@abc\n  @base 111111111111\n";
        let claim = &parse(body).claims[0];
        assert_eq!(
            without_base(&claim.block),
            vec!["- [A-1] the fold strips `@base` lines", "  @ref a.rs#f@abc"]
        );
        assert_eq!(claim.base.as_deref(), Some("111111111111"));
        assert_eq!(
            block_hash(claim),
            hash_of("- [A-1] the fold strips `@base` lines\n  @ref a.rs#f@abc\n")
        );
        assert_ne!(block_hash(claim), hash_of("- [A-1] \n  @ref a.rs#f@abc\n"));
    }

    #[test]
    fn annotation_lines_are_ref_checked_and_base() {
        assert!(is_annotation("  @ref a.rs#f@abc"));
        assert!(is_annotation("\t@checked jack@abc"));
        assert!(is_annotation("  @base 0123"));
        assert!(is_base_line("  @base 0123"));
        assert!(!is_base_line("  @ref a.rs#f@abc"));
        assert!(!is_annotation("- [A-1] strips `@base` lines"));
        assert!(!is_annotation("  prose"));
    }

    #[test]
    fn forced_follows_the_ref_it_marks() {
        let marked = parse("- [AC-1] x\n  @ref a.rs#f@abc [forced]\n");
        let trailing = parse("- [AC-1] x\n  @ref a.rs#f@abc trailing\n");

        assert_eq!(marked.claims[0].receipts[0].sha.as_deref(), Some("abc"));
        assert!(marked.claims[0].receipts[0].forced);
        assert_eq!(trailing.claims[0].receipts[0].sha.as_deref(), Some("abc"));
        assert!(!trailing.claims[0].receipts[0].forced);
    }
}
