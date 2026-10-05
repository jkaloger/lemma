use regex::Regex;
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    pub line: usize,
    pub path: String,
    pub symbol: Option<String>,
    pub sha: Option<String>,
    pub checked_by: Option<String>,
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

fn checked_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"@checked\s+([^@\s]+)@([0-9a-fA-F]+)").unwrap())
}

fn receipts_in(lines: &[&str], live: &[bool], start: usize, end: usize) -> Vec<Receipt> {
    (start..end)
        .filter(|&i| live[i])
        .flat_map(|i| {
            let refs = lazyspec::engine::refs::parse_refs(lines[i])
                .into_iter()
                .map(move |r| Receipt {
                    line: i + 1,
                    path: r.path,
                    symbol: r.symbol,
                    sha: r.commit_sha,
                    checked_by: None,
                });
            let checks = checked_pattern()
                .captures_iter(lines[i])
                .map(move |c| Receipt {
                    line: i + 1,
                    path: String::new(),
                    symbol: None,
                    sha: Some(c[2].to_string()),
                    checked_by: Some(c[1].to_string()),
                });
            refs.chain(checks).collect::<Vec<_>>()
        })
        .collect()
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
        claims.push(Claim {
            id: caps[2].to_string(),
            line: cursor + 1,
            start: cursor,
            end,
            struck: caps.get(1).is_some(),
            intent: marks_intent(&lines, &live, cursor, end),
            receipts: receipts_in(&lines, &live, cursor, end),
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
  @ref src/store.rs#reopen_keeps_prior@3f9c2e1
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
        assert_eq!(first.receipts[0].sha.as_deref(), Some("3f9c2e1"));
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
    fn checked_lines_parse_as_receipts() {
        let b = parse("- [A-1] one\n  @checked jack@3f9c2e1\n");
        let r = &b.claims[0].receipts[0];
        assert_eq!(r.checked_by.as_deref(), Some("jack"));
        assert_eq!(r.sha.as_deref(), Some("3f9c2e1"));
        assert_eq!(r.line, 2);
        assert!(r.symbol.is_none());
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
}
