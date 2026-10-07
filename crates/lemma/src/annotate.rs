use crate::claims::{self, heading_level, lines_of, live_lines, REVIEW_MARKER, REVIEW_QUOTE};
use crate::payload::{CLAIMS_PART, INDEX_PART, TASKS_PART};
use crate::tasks;

type Bounds = (usize, usize);

/// The file with a review line added at the anchor, or `None` when the anchor is unknown.
/// An identical line already under the anchor leaves the file unchanged.
pub fn annotate(part: &str, file: &str, anchor: &str, text: &str) -> Option<String> {
    let front = match part {
        INDEX_PART => frontmatter_len(file),
        _ => 0,
    };
    let (front, body) = file.split_at(front);
    Some(format!(
        "{front}{}",
        annotate_body(part, body, anchor.trim(), text)?
    ))
}

fn frontmatter_len(file: &str) -> usize {
    if !file.starts_with("---\n") {
        return 0;
    }
    let mut offset = 4;
    for line in file[4..].split_inclusive('\n') {
        offset += line.len();
        if line.trim_end() == "---" {
            return offset;
        }
    }
    0
}

fn annotate_body(part: &str, body: &str, anchor: &str, text: &str) -> Option<String> {
    let lines = lines_of(body);
    let block = match part {
        CLAIMS_PART => claim_block(body, &lines, anchor),
        TASKS_PART => task_block(body, &lines, anchor),
        _ => None,
    };
    match block {
        Some(block) => Some(add_to_block(&lines, block, text)),
        None => Some(add_to_section(
            &lines,
            section_bounds(body, &lines, part, anchor)?,
            text,
        )),
    }
}

fn claim_block(body: &str, lines: &[&str], anchor: &str) -> Option<Bounds> {
    if let Some(claim) = claims::parse(body).claims.iter().find(|c| c.id == anchor) {
        return Some((claim.start, claim.end));
    }
    let bullet = claims::bullets(body)
        .into_iter()
        .find(|b| b.id.is_none() && b.text == anchor)?;
    let start = bullet.line - 1;
    Some((start, claims::claim_end(lines, start)))
}

fn task_block(body: &str, lines: &[&str], anchor: &str) -> Option<Bounds> {
    let start = tasks::parse(body).named(anchor)?.line - 1;
    Some((start, claims::claim_end(lines, start)))
}

fn section_bounds(body: &str, lines: &[&str], part: &str, anchor: &str) -> Option<Bounds> {
    if anchor.is_empty() || anchor.eq_ignore_ascii_case(part) {
        return Some((0, lines.len()));
    }
    let live = live_lines(body);
    let (at, level) = live.iter().enumerate().find_map(|(at, (_, line))| {
        let (level, name) = heading_level(line)?;
        name.eq_ignore_ascii_case(anchor).then_some((at, level))
    })?;
    let closes = |line: &str| heading_level(line).is_some_and(|(next, _)| next <= level);
    let end = live[at + 1..]
        .iter()
        .find(|(_, line)| closes(line))
        .map_or(lines.len(), |(n, _)| n - 1);
    Some((live[at].0 - 1, end))
}

fn add_to_block(lines: &[&str], (start, end): Bounds, text: &str) -> String {
    let review = format!("  {REVIEW_MARKER} {text}");
    if lines[start + 1..end]
        .iter()
        .any(|l| l.trim() == review.trim())
    {
        return lines.join("\n");
    }
    splice(lines, end, &[review])
}

fn add_to_section(lines: &[&str], (start, end): Bounds, text: &str) -> String {
    let review = format!("{REVIEW_QUOTE} {text}");
    if lines[start..end].iter().any(|l| l.trim() == review) {
        return lines.join("\n");
    }
    let at = (start..end)
        .rev()
        .find(|&i| !lines[i].trim().is_empty())
        .map_or(start, |i| i + 1);
    match at {
        0 => splice(lines, at, &[review]),
        _ => splice(lines, at, &[String::new(), review]),
    }
}

fn splice(lines: &[&str], at: usize, added: &[String]) -> String {
    let mut out = lines[..at].to_vec();
    out.extend(added.iter().map(String::as_str));
    out.extend_from_slice(&lines[at..]);
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLAIMS: &str =
        "- [AC-1] First.\n  @ref a.rs#x\n\n- plain bullet\n  more\n- ~~[AC-2]~~ struck\n";

    #[test]
    fn a_claim_id_gets_the_line_after_its_continuation() {
        let got = annotate("claims", CLAIMS, "AC-1", "Cite it.").unwrap();

        assert_eq!(
            got,
            "- [AC-1] First.\n  @ref a.rs#x\n  @review Cite it.\n\n- plain bullet\n  more\n- ~~[AC-2]~~ struck\n"
        );
    }

    #[test]
    fn a_struck_claim_still_anchors() {
        let got = annotate("claims", CLAIMS, "AC-2", "Why struck?").unwrap();

        assert!(got.ends_with("- ~~[AC-2]~~ struck\n  @review Why struck?\n"));
    }

    #[test]
    fn a_plain_bullet_anchors_by_its_text() {
        let got = annotate("claims", CLAIMS, "plain bullet", "Give it an id.").unwrap();

        assert!(got.contains("- plain bullet\n  more\n  @review Give it an id.\n- ~~"));
    }

    #[test]
    fn a_task_id_gets_the_line_after_its_indented_lines() {
        let tasks = "- [ ] T1 one (settles AC-1)\n  detail\n- [ ] T2 two (chore)\n";

        let got = annotate("tasks", tasks, "T1", "Too big.").unwrap();

        assert_eq!(
            got,
            "- [ ] T1 one (settles AC-1)\n  detail\n  @review Too big.\n- [ ] T2 two (chore)\n"
        );
    }

    #[test]
    fn a_heading_anchor_appends_a_quote_at_the_end_of_its_section() {
        let body = "# Title\n\n## Intent\n\nBecause.\n\n### Detail\n\nMore.\n\n## Other\n\nX.\n";

        let got = annotate("design", body, "intent", "Say more.").unwrap();

        assert_eq!(
            got,
            "# Title\n\n## Intent\n\nBecause.\n\n### Detail\n\nMore.\n\n> review: Say more.\n\n## Other\n\nX.\n"
        );
    }

    #[test]
    fn a_deeper_heading_ends_at_the_next_heading_of_its_level_or_higher() {
        let body = "## A\n\n### B\n\nb.\n\n### C\n\nc.\n";

        let got = annotate("design", body, "B", "q").unwrap();

        assert_eq!(got, "## A\n\n### B\n\nb.\n\n> review: q\n\n### C\n\nc.\n");
    }

    #[test]
    fn the_part_name_or_an_empty_anchor_appends_at_the_end_of_the_part() {
        let body = "## A\n\nText.\n";
        let expected = "## A\n\nText.\n\n> review: q\n";

        assert_eq!(annotate("design", body, "design", "q").unwrap(), expected);
        assert_eq!(annotate("design", body, "", "q").unwrap(), expected);
    }

    #[test]
    fn an_empty_part_gets_just_the_quote() {
        assert_eq!(annotate("design", "", "", "q").unwrap(), "> review: q\n");
    }

    #[test]
    fn trailing_newline_presence_is_preserved() {
        assert_eq!(
            annotate("design", "Text.", "", "q").unwrap(),
            "Text.\n\n> review: q"
        );
    }

    #[test]
    fn headings_in_fences_and_comments_never_anchor() {
        let body = "```\n## Fake\n```\n<!--\n## Hidden\n-->\n";

        assert_eq!(annotate("design", body, "Fake", "q"), None);
        assert_eq!(annotate("design", body, "Hidden", "q"), None);
    }

    #[test]
    fn an_unknown_anchor_is_none() {
        assert_eq!(annotate("claims", CLAIMS, "AC-9", "q"), None);
        assert_eq!(annotate("tasks", "- [ ] T1 a (chore)\n", "T2", "q"), None);
    }

    #[test]
    fn a_claims_part_falls_back_to_prose_anchors() {
        let got = annotate("claims", "## Notes\n\nText.\n", "Notes", "q").unwrap();

        assert_eq!(got, "## Notes\n\nText.\n\n> review: q\n");
    }

    #[test]
    fn an_identical_line_under_the_anchor_is_not_repeated() {
        let once = annotate("claims", CLAIMS, "AC-1", "Cite it.").unwrap();
        assert_eq!(annotate("claims", &once, "AC-1", "Cite it.").unwrap(), once);

        let quoted = annotate("design", "## A\n\nx\n", "A", "q").unwrap();
        assert_eq!(annotate("design", &quoted, "A", "q").unwrap(), quoted);
    }

    #[test]
    fn index_frontmatter_is_kept_byte_for_byte() {
        let file = "---\ntitle: \"## Intent\"\n---\n\n## Intent\n\nBecause.\n";

        let got = annotate("index", file, "Intent", "q").unwrap();

        assert_eq!(
            got,
            "---\ntitle: \"## Intent\"\n---\n\n## Intent\n\nBecause.\n\n> review: q\n"
        );
    }
}
