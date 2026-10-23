use std::io::IsTerminal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Live,
    Plain,
}

impl Mode {
    pub fn pick() -> Mode {
        match std::io::stdout().is_terminal() && std::io::stderr().is_terminal() {
            true => Mode::Live,
            false => Mode::Plain,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    Pass,
    Fail,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Green,
    Red,
    Dim,
    Bold,
    Magenta,
    Cyan,
    Blue,
    Yellow,
}

impl Glyph {
    fn mark(self) -> &'static str {
        match self {
            Glyph::Pass => "✓",
            Glyph::Fail => "✗",
            Glyph::Info => "·",
        }
    }

    fn tone(self) -> Tone {
        match self {
            Glyph::Pass => Tone::Green,
            Glyph::Fail => Tone::Red,
            Glyph::Info => Tone::Dim,
        }
    }
}

fn styled(tone: Tone, text: &str) -> console::StyledObject<String> {
    let text = console::style(text.to_string());
    match tone {
        Tone::Green => text.green(),
        Tone::Red => text.red(),
        Tone::Dim => text.dim(),
        Tone::Bold => text.bold(),
        Tone::Magenta => text.magenta(),
        Tone::Cyan => text.cyan(),
        Tone::Blue => text.blue(),
        Tone::Yellow => text.yellow(),
    }
}

const STEP_INDENT: &str = "   ";

#[derive(Debug, Clone, Copy)]
pub struct Look {
    pub mode: Mode,
    colour: bool,
}

impl Look {
    pub fn new(mode: Mode, no_color: Option<&std::ffi::OsStr>) -> Look {
        let colour = no_color.is_none_or(|value| value.is_empty());
        Look { mode, colour }
    }

    pub fn current(mode: Mode) -> Look {
        Look::new(mode, std::env::var_os("NO_COLOR").as_deref())
    }

    pub fn tone(&self, tone: Tone, text: &str) -> String {
        match (self.mode, self.colour) {
            (Mode::Plain, _) => styled(tone, text).to_string(),
            (Mode::Live, true) => styled(tone, text).force_styling(true).to_string(),
            (Mode::Live, false) => text.to_string(),
        }
    }

    pub fn step(&self, glyph: Glyph, text: &str) -> String {
        self.step_toned(glyph, text, None)
    }

    pub fn step_toned(&self, glyph: Glyph, text: &str, plain: Option<Tone>) -> String {
        if self.mode == Mode::Plain {
            return match plain {
                Some(tone) => self.tone(tone, text),
                None => text.to_string(),
            };
        }
        let Some(body) = text.strip_prefix(STEP_INDENT) else {
            return text.to_string();
        };
        if body.starts_with(' ') {
            return text.to_string();
        }
        format!(
            "{STEP_INDENT}{} {body}",
            self.tone(glyph.tone(), glyph.mark())
        )
    }

    pub fn collapsed(&self, done: &Committed) -> String {
        format!(
            "{} {} {}  {} · {} · {}",
            self.tone(Glyph::Pass.tone(), Glyph::Pass.mark()),
            done.id,
            done.title,
            done.elapsed,
            done.review.summary(),
            done.sha
        )
    }

    pub fn continuation(&self, text: &str) -> String {
        match self.mode {
            Mode::Plain => text.to_string(),
            Mode::Live => format!("  {text}"),
        }
    }
}

pub(crate) fn counted(n: usize, noun: &str) -> String {
    match n {
        1 => format!("1 {noun}"),
        _ => format!("{n} {noun}s"),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReviewOutcome {
    pub findings: usize,
    pub rounds: u32,
}

impl ReviewOutcome {
    pub fn trailer(&self) -> String {
        match self.rounds {
            0 => "Lemma-Review: clean".to_string(),
            _ => format!(
                "Lemma-Review: {} fixed in {}",
                counted(self.findings, "finding"),
                counted(self.rounds as usize, "round")
            ),
        }
    }

    pub fn summary(&self) -> String {
        match self.rounds {
            0 => "review clean".to_string(),
            _ => format!("{} fixed", counted(self.findings, "finding")),
        }
    }
}

pub struct Committed<'a> {
    pub id: &'a str,
    pub title: &'a str,
    pub elapsed: &'a str,
    pub review: ReviewOutcome,
    pub sha: &'a str,
}

pub enum Finish<'a> {
    Committed(&'a Committed<'a>),
    Stopped,
}

pub struct Block {
    width: Option<usize>,
    rows: std::cell::Cell<usize>,
}

impl Block {
    pub fn new(width: Option<usize>) -> Block {
        Block {
            width: width.filter(|w| *w > 0),
            rows: std::cell::Cell::new(0),
        }
    }

    pub fn on_terminal() -> Block {
        let (_, cols) = console::Term::stdout().size_checked().unwrap_or((0, 0));
        Block::new(Some(cols as usize))
    }

    pub fn say(&self, line: &str) {
        println!("{line}");
        self.record(line);
    }

    fn record(&self, line: &str) {
        let Some(width) = self.width else {
            return;
        };
        let rows: usize = line
            .split('\n')
            .map(|part| console::measure_text_width(part).div_ceil(width).max(1))
            .sum();
        self.rows.set(self.rows.get() + rows);
    }

    pub fn finish(&self, look: &Look, ending: Finish) -> Option<String> {
        let Finish::Committed(done) = ending else {
            return None;
        };
        if look.mode == Mode::Plain {
            return None;
        }
        let summary = look.collapsed(done);
        match self.width {
            Some(_) => Some(format!("\x1b[{}A\x1b[J{summary}", self.rows.get())),
            None => Some(summary),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    fn live(no_color: Option<&str>) -> Look {
        Look::new(Mode::Live, no_color.map(OsStr::new))
    }

    #[test]
    fn live_step_lines_carry_a_glyph_per_outcome() {
        let look = live(Some("1"));
        assert_eq!(look.step(Glyph::Pass, "   lint: clean"), "   ✓ lint: clean");
        assert_eq!(
            look.step(Glyph::Fail, "   review: 1 finding"),
            "   ✗ review: 1 finding"
        );
        assert_eq!(
            look.step(Glyph::Info, "   AC-13 [intent]"),
            "   · AC-13 [intent]"
        );
    }

    #[test]
    fn plain_step_lines_are_the_text_unchanged() {
        let look = Look::new(Mode::Plain, None);
        for glyph in [Glyph::Pass, Glyph::Fail, Glyph::Info] {
            assert_eq!(look.step(glyph, "   lint: clean"), "   lint: clean");
        }
    }

    #[test]
    fn no_color_removes_colour_and_keeps_glyphs() {
        let plain_text = live(Some("1")).step(Glyph::Pass, "   lint: clean");
        assert_eq!(plain_text, "   ✓ lint: clean");
        assert!(!plain_text.contains('\u{1b}'));
        assert!(!live(Some("1"))
            .tone(Tone::Red, "stopped")
            .contains('\u{1b}'));

        let coloured = live(None).step(Glyph::Pass, "   lint: clean");
        assert!(coloured.contains('\u{1b}'));
        assert!(coloured.contains('✓'));
    }

    #[test]
    fn an_empty_no_color_leaves_colour_on() {
        assert!(live(Some("")).step(Glyph::Fail, "   x").contains('\u{1b}'));
    }

    #[test]
    fn continuation_lines_are_not_given_a_glyph() {
        let look = live(Some("1"));
        let line = "            lint error detail";
        assert_eq!(look.step(Glyph::Fail, line), line);
    }

    fn committed(review: ReviewOutcome) -> Committed<'static> {
        Committed {
            id: "T1",
            title: "add `Balance` type and the `SUM` query",
            elapsed: "2m52s",
            review,
            sha: "3f1c9a2",
        }
    }

    const ONE_FIXED: ReviewOutcome = ReviewOutcome {
        findings: 1,
        rounds: 1,
    };

    fn block_of(lines: &[&str], width: Option<usize>) -> Block {
        let block = Block::new(width);
        lines.iter().for_each(|line| block.record(line));
        block
    }

    #[test]
    fn a_committed_live_block_is_replaced_by_one_summary_line() {
        let block = block_of(
            &["T1 heading", "   ✓ runner 2s, 1 files", "   ✓ lint"],
            Some(80),
        );
        let out = block
            .finish(&live(Some("1")), Finish::Committed(&committed(ONE_FIXED)))
            .unwrap();
        assert_eq!(
            out,
            "\x1b[3A\x1b[J✓ T1 add `Balance` type and the `SUM` query  2m52s · 1 finding fixed · 3f1c9a2"
        );
    }

    #[test]
    fn a_wrapped_line_counts_every_row_it_occupies() {
        let block = block_of(&["short", &"x".repeat(25)], Some(10));
        let out = block
            .finish(&live(Some("1")), Finish::Committed(&committed(ONE_FIXED)))
            .unwrap();
        assert!(out.starts_with("\x1b[4A\x1b[J"));
    }

    #[test]
    fn an_unknown_line_count_prints_the_summary_under_the_block() {
        let block = block_of(&["T1 heading"], None);
        let out = block
            .finish(&live(Some("1")), Finish::Committed(&committed(ONE_FIXED)))
            .unwrap();
        assert!(out.starts_with("✓ T1 "));
        assert!(!out.contains('\u{1b}'));
    }

    #[test]
    fn a_stopped_task_is_never_collapsed() {
        let block = block_of(&["T1 heading", "   ✗ lint"], Some(80));
        assert_eq!(block.finish(&live(None), Finish::Stopped), None);
    }

    #[test]
    fn plain_mode_never_collapses() {
        let block = block_of(&["T1 heading"], Some(80));
        let plain = Look::new(Mode::Plain, None);
        assert_eq!(
            block.finish(&plain, Finish::Committed(&committed(ONE_FIXED))),
            None
        );
    }

    #[test]
    fn the_collapsed_review_outcome_matches_the_commit_trailer_counts() {
        let clean = ReviewOutcome {
            findings: 0,
            rounds: 0,
        };
        assert_eq!(clean.summary(), "review clean");
        assert_eq!(clean.trailer(), "Lemma-Review: clean");

        assert_eq!(ONE_FIXED.summary(), "1 finding fixed");
        assert_eq!(
            ONE_FIXED.trailer(),
            "Lemma-Review: 1 finding fixed in 1 round"
        );

        let many = ReviewOutcome {
            findings: 3,
            rounds: 2,
        };
        assert_eq!(many.summary(), "3 findings fixed");
        assert_eq!(many.trailer(), "Lemma-Review: 3 findings fixed in 2 rounds");
    }
}
