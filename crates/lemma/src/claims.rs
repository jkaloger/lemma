use regex::Regex;
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    pub line: usize,
    pub path: String,
    pub symbol: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claim {
    pub id: String,
    pub line: usize,
    pub start: usize,
    pub end: usize,
    pub struck: bool,
    pub intent: bool,
    pub receipts: Vec<Receipt>,
    pub legacy: Vec<LegacyLine>,
    pub block: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyKind {
    PinnedRef,
    Base,
    Checked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyLine {
    pub line: usize,
    pub kind: LegacyKind,
}

impl LegacyLine {
    pub fn message(&self) -> &'static str {
        match self.kind {
            LegacyKind::PinnedRef => "pinned ref; run lemma migrate",
            LegacyKind::Base => "`@base` line; run lemma migrate",
            LegacyKind::Checked => "`@checked` line; run lemma migrate",
        }
    }
}

struct Section {
    heading: usize,
    end: usize,
}

pub const BEHAVIOUR_PART: &str = "behaviour";

pub fn is_behaviour_part(name: &str) -> bool {
    name == BEHAVIOUR_PART || target_of(name).is_some()
}

pub fn target_of(part_name: &str) -> Option<&str> {
    part_name
        .strip_prefix(BEHAVIOUR_PART)?
        .strip_prefix('.')
        .filter(|target| !target.is_empty())
}

pub fn part_for(target_id: &str) -> String {
    format!("{BEHAVIOUR_PART}.{target_id}")
}

#[derive(Debug, Default)]
pub struct Behaviour {
    pub claims: Vec<Claim>,
}

fn item_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^- (~~)?\[([A-Z][A-Z0-9]*(?:-[A-Z0-9]+)+)\](~~)?(?:\s|$)").unwrap()
    })
}

pub fn lines_of(body: &str) -> Vec<&str> {
    body.split('\n').collect()
}

fn fence_marker(line: &str) -> Option<char> {
    let trimmed = line.trim_start();
    [('`', "```"), ('~', "~~~")]
        .into_iter()
        .find(|(_, m)| trimmed.starts_with(m))
        .map(|(c, _)| c)
}

#[derive(Default)]
struct Masker {
    fence: Option<char>,
    in_comment: bool,
}

impl Masker {
    fn is_live(&mut self, line: &str) -> bool {
        if self.in_comment {
            self.in_comment = !line.contains("-->");
            return false;
        }
        if self.fence.is_none() && line.trim_start().starts_with("<!--") {
            self.in_comment = !line.contains("-->");
            return false;
        }
        match (self.fence, fence_marker(line)) {
            (None, Some(c)) => {
                self.fence = Some(c);
                false
            }
            (Some(open), Some(c)) if open == c => {
                self.fence = None;
                false
            }
            (Some(_), _) => false,
            (None, None) => true,
        }
    }
}

fn outside_fences(lines: &[&str]) -> Vec<bool> {
    let mut masker = Masker::default();
    lines.iter().map(|line| masker.is_live(line)).collect()
}

fn heading_level(line: &str) -> Option<(usize, &str)> {
    let hashes = line.chars().take_while(|c| *c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = line[hashes..].strip_prefix(' ')?;
    Some((hashes, rest.trim()))
}

fn find_section(lines: &[&str], live: &[bool], names: &[&str]) -> Option<Section> {
    let heading = (0..lines.len()).find(|&i| {
        live[i]
            && matches!(heading_level(lines[i]), Some((2, name))
                if names.iter().any(|n| name.eq_ignore_ascii_case(n)))
    })?;
    let end = (heading + 1..lines.len())
        .find(|&i| live[i] && matches!(heading_level(lines[i]), Some((level, _)) if level <= 2))
        .unwrap_or(lines.len());
    Some(Section { heading, end })
}

pub fn has_intent(body: &str) -> bool {
    let lines = lines_of(body);
    let live = outside_fences(&lines);
    let Some(section) = find_section(&lines, &live, &["Intent"]) else {
        return false;
    };
    (section.heading + 1..section.end).any(|i| live[i] && !lines[i].trim().is_empty())
}

fn claim_end(lines: &[&str], start: usize) -> usize {
    let mut end = start + 1;
    let mut cursor = start + 1;
    while cursor < lines.len() {
        let line = lines[cursor];
        let continues = line.starts_with(' ') || line.starts_with('\t');
        if line.trim().is_empty() {
            cursor += 1;
            continue;
        }
        if !continues {
            break;
        }
        cursor += 1;
        end = cursor;
    }
    end
}

pub const FORCED_MARKER: &str = "[forced]";
const BASE_MARKER: &str = "@base";

pub fn checked_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"@checked\s+([^@\s]+)@([0-9a-fA-F]+)").unwrap())
}

pub fn is_base_line(line: &str) -> bool {
    line.trim_start()
        .strip_prefix(BASE_MARKER)
        .is_some_and(|rest| rest.starts_with(' '))
}

pub fn is_pinned(found: &lazyspec::engine::refs::Ref, line: &str) -> bool {
    found.commit_sha.is_some()
        || found.blob_hash.is_some()
        || line[found.span.1..].trim_start().starts_with(FORCED_MARKER)
}

fn scan(
    lines: &[&str],
    live: &[bool],
    start: usize,
    end: usize,
) -> (Vec<Receipt>, Vec<LegacyLine>) {
    let mut receipts = Vec::new();
    let mut legacy = Vec::new();
    for i in (start..end).filter(|&i| live[i]) {
        let line = lines[i];
        let mut found = |kind| legacy.push(LegacyLine { line: i + 1, kind });
        for r in lazyspec::engine::refs::parse_refs(line) {
            match is_pinned(&r, line) {
                true => found(LegacyKind::PinnedRef),
                false => receipts.push(Receipt {
                    line: i + 1,
                    path: r.path,
                    symbol: r.symbol,
                }),
            }
        }
        if checked_pattern().is_match(line) {
            found(LegacyKind::Checked);
        }
        if is_base_line(line) {
            found(LegacyKind::Base);
        }
    }
    legacy.dedup();
    (receipts, legacy)
}

fn marks_intent(lines: &[&str], live: &[bool], start: usize, end: usize) -> bool {
    (start..end).any(|i| live[i] && lines[i].contains("[intent]"))
}

pub fn parse(body: &str) -> Behaviour {
    let lines = lines_of(body);
    let live = outside_fences(&lines);
    let mut claims = Vec::new();
    let mut cursor = 0;
    while cursor < lines.len() {
        let line = lines[cursor];
        let Some(caps) = live[cursor]
            .then(|| item_pattern().captures(line))
            .flatten()
        else {
            cursor += 1;
            continue;
        };
        let end = claim_end(&lines, cursor);
        let (receipts, legacy) = scan(&lines, &live, cursor, end);
        claims.push(Claim {
            id: caps[2].to_string(),
            line: cursor + 1,
            start: cursor,
            end,
            struck: caps.get(1).is_some(),
            intent: marks_intent(&lines, &live, cursor, end),
            receipts,
            legacy,
            block: lines[cursor..end].iter().map(|s| s.to_string()).collect(),
        });
        cursor = end;
    }
    Behaviour { claims }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PART: &str = "\
- [AC-C17-1] A reopened head keeps its prior answer.
  @ref src/store.rs#reopen_keeps_prior
- [AC-C17-2] [intent] Feels fast.
- ~~[AC-OLD-1]~~ gone
";

    #[test]
    fn parses_claims_with_receipts_and_lines() {
        let b = parse(PART);
        assert_eq!(b.claims.len(), 3);
        let first = &b.claims[0];
        assert_eq!(first.id, "AC-C17-1");
        assert_eq!(first.line, 1);
        assert_eq!(first.receipts.len(), 1);
        assert_eq!(first.receipts[0].path, "src/store.rs");
        assert_eq!(
            first.receipts[0].symbol.as_deref(),
            Some("reopen_keeps_prior")
        );
        assert_eq!(first.receipts[0].line, 2);
    }

    #[test]
    fn marks_intent_and_strike() {
        let b = parse(PART);
        assert!(b.claims[1].intent);
        assert!(!b.claims[0].intent);
        assert!(b.claims[2].struck);
        assert!(!b.claims[0].struck);
    }

    #[test]
    fn fenced_blocks_hide_claims_and_markers() {
        let body = "```\n- [AC-X-1] fenced\n```\n\n- [AC-X-2] real\n  ```\n  [intent]\n  @ref a.rs#f@abc\n  ```\n";
        let b = parse(body);
        assert_eq!(b.claims.len(), 1);
        assert_eq!(b.claims[0].id, "AC-X-2");
        assert!(!b.claims[0].intent);
        assert!(b.claims[0].receipts.is_empty());
        assert_eq!(b.claims[0].end, body.lines().count());
    }

    #[test]
    fn html_comments_hide_example_claims() {
        let body = "<!--\n- [AC-DELTA-017-1] example\n  @ref src/x.rs#t@abc1234\n-->\n\n<!-- - [AC-ONE-1] inline -->\n- [AC-REAL-1] real\n";
        let b = parse(body);
        assert_eq!(b.claims.len(), 1);
        assert_eq!(b.claims[0].id, "AC-REAL-1");
        assert!(b.claims[0].receipts.is_empty());
    }

    #[test]
    fn comment_inside_a_claim_does_not_contribute_receipts() {
        let b = parse("- [A-1] real\n  <!-- @ref a.rs#f@abc -->\n");
        assert!(b.claims[0].receipts.is_empty());
    }

    #[test]
    fn claim_block_excludes_trailing_blank_lines() {
        let b = parse("- [A-1] one\n  detail\n\n- [A-2] two\n");
        assert_eq!(b.claims[0].block, vec!["- [A-1] one", "  detail"]);
        assert_eq!(b.claims[0].end, 2);
    }

    #[test]
    fn intent_requires_non_empty_section() {
        assert!(has_intent("## Intent\n\ntext\n"));
        assert!(!has_intent("## Intent\n\n## Notes\n"));
        assert!(!has_intent("## Notes\n"));
        assert!(!has_intent("```\n## Intent\ntext\n```\n"));
    }

    #[test]
    fn template_intent_with_only_guidance_is_empty() {
        for template in [
            include_str!("../../../.lazyspec/templates/delta/index.md"),
            include_str!("../../../.lazyspec/templates/spec/index.md"),
        ] {
            assert!(!has_intent(template));
            let filled =
                template.replace("## Intent\n", "## Intent\nReopened heads keep answers.\n");
            assert!(has_intent(&filled));
        }
    }

    #[test]
    fn ids_must_have_a_dash_segment() {
        assert!(parse("- [AC] nope\n- [ab-1] nope\n").claims.is_empty());
    }

    #[test]
    fn behaviour_part_names() {
        assert!(is_behaviour_part("behaviour"));
        assert!(is_behaviour_part("behaviour.SPEC-003"));
        assert!(!is_behaviour_part("tasks"));
        assert!(!is_behaviour_part("behaviours"));
        assert_eq!(target_of("behaviour.SPEC-003"), Some("SPEC-003"));
        assert_eq!(target_of("behaviour"), None);
        assert_eq!(part_for("SPEC-003"), "behaviour.SPEC-003");
    }

    fn legacy_of(body: &str) -> Vec<(usize, &'static str)> {
        parse(body).claims[0]
            .legacy
            .iter()
            .map(|l| (l.line, l.message()))
            .collect()
    }

    #[test]
    fn unpinned_refs_are_the_only_clean_form() {
        let b = parse("- [A-1] one\n  @ref tests/a.rs#f\n  @ref tests/a.rs#g\n");
        let claim = &b.claims[0];
        assert!(claim.legacy.is_empty());
        assert_eq!(claim.receipts.len(), 2);
        assert_eq!(claim.receipts.first().unwrap().symbol.as_deref(), Some("f"));
    }

    #[test]
    fn commit_pinned_ref_is_a_legacy_line() {
        let body = "- [A-1] one\n  @ref tests/a.rs#f@3f9c2e1\n";
        assert_eq!(legacy_of(body), vec![(2, "pinned ref; run lemma migrate")]);
        assert_eq!(parse(body).claims[0].receipts.len(), 0);
    }

    #[test]
    fn blob_pinned_ref_is_a_legacy_line() {
        let body = "- [A-1] one\n  @ref tests/a.rs#f@{blob:3f9c2e1}\n";
        assert_eq!(legacy_of(body), vec![(2, "pinned ref; run lemma migrate")]);
        assert_eq!(parse(body).claims[0].receipts.len(), 0);
    }

    #[test]
    fn forced_marker_is_a_legacy_line() {
        let body = "- [A-1] one\n  @ref tests/a.rs#f@abc [forced]\n";
        assert_eq!(legacy_of(body), vec![(2, "pinned ref; run lemma migrate")]);
    }

    #[test]
    fn base_and_checked_lines_are_legacy_lines_in_line_order() {
        let body = "- [A-1] one\n  @checked jack@3f9c2e1\n  @base 0123abcdef45\n";
        assert_eq!(
            legacy_of(body),
            vec![
                (2, "`@checked` line; run lemma migrate"),
                (3, "`@base` line; run lemma migrate")
            ]
        );
    }

    #[test]
    fn legacy_markers_in_fences_comments_and_prose_are_ignored() {
        let body = "- [A-1] strips `@base` lines\n  ```\n  @base 0123abcdef45\n  @ref a.rs#f@abc\n  ```\n  <!-- @checked jack@abc -->\n";
        assert!(parse(body).claims[0].legacy.is_empty());
    }
}
