use regex::Regex;
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    pub line: usize,
    pub path: String,
    pub symbol: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Citation {
    pub line: usize,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claim {
    pub id: String,
    pub line: usize,
    pub start: usize,
    pub end: usize,
    pub struck: bool,
    pub intent: bool,
    pub per: Vec<Citation>,
    pub receipts: Vec<Receipt>,
    pub block: Vec<String>,
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

pub const PER_MARKER: &str = "@per";

fn receipts(lines: &[&str], live: &[bool], start: usize, end: usize) -> Vec<Receipt> {
    (start..end)
        .filter(|&i| live[i])
        .flat_map(|i| {
            lazyspec::engine::refs::parse_refs(lines[i])
                .into_iter()
                .map(move |r| Receipt {
                    line: i + 1,
                    path: r.path,
                    symbol: r.symbol,
                })
        })
        .collect()
}

const INTENT_MARKER: &str = "[intent]";

fn leads_with_intent(first_line: &str) -> bool {
    let Some(caps) = item_pattern().captures(first_line) else {
        return false;
    };
    let rest = first_line[caps.get(0).map_or(0, |m| m.end())..].trim_start();
    rest.strip_prefix(INTENT_MARKER)
        .is_some_and(|after| after.is_empty() || after.starts_with(char::is_whitespace))
}

pub fn per_text(line: &str) -> Option<&str> {
    line.trim_start()
        .strip_prefix(PER_MARKER)?
        .strip_prefix(char::is_whitespace)
}

fn citations(lines: &[&str], live: &[bool], start: usize, end: usize) -> Vec<Citation> {
    (start..end)
        .filter(|&i| live[i])
        .filter_map(|i| {
            Some(Citation {
                line: i + 1,
                text: per_text(lines[i])?.trim().to_string(),
            })
        })
        .collect()
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
        claims.push(Claim {
            id: caps[2].to_string(),
            line: cursor + 1,
            start: cursor,
            end,
            struck: caps.get(1).is_some(),
            intent: leads_with_intent(line),
            per: citations(&lines, &live, cursor, end),
            receipts: receipts(&lines, &live, cursor, end),
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
    }

    #[test]
    fn intent_is_a_leading_token_only() {
        assert!(parse("- [A-1] [intent] x `[intent]`\n").claims[0].intent);
        assert!(parse("- ~~[A-1]~~ [intent] gone\n").claims[0].intent);
        assert!(!parse("- [A-1] text then [intent]\n").claims[0].intent);
        assert!(!parse("- [A-1] [intentional] x\n").claims[0].intent);
    }

    #[test]
    fn per_lines_parse_in_order_with_their_line_numbers() {
        let b = parse("- [A-1] x\n  @per DECISION-003\n  @per RFC 7231 section 4\n  @ref a.rs#f\n");
        let per: Vec<_> = b.claims[0]
            .per
            .iter()
            .map(|c| (c.line, c.text.as_str()))
            .collect();
        assert_eq!(per, vec![(2, "DECISION-003"), (3, "RFC 7231 section 4")]);
        assert_eq!(b.claims[0].receipts.len(), 1);
    }

    #[test]
    fn fenced_and_commented_per_lines_are_ignored() {
        let body = "- [A-1] x\n  ```\n  @per DECISION-001\n  ```\n  <!-- @per DECISION-002 -->\n";
        assert!(parse(body).claims[0].per.is_empty());
    }

    #[test]
    fn a_pinned_ref_is_read_as_its_path_and_symbol() {
        let claim = &parse("- [A-1] one\n  @ref tests/a.rs#f@3f9c2e1\n").claims[0];
        assert_eq!(claim.receipts[0].path, "tests/a.rs");
        assert_eq!(claim.receipts[0].symbol.as_deref(), Some("f"));
    }
}
