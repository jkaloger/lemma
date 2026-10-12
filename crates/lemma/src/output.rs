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

    pub fn continuation(&self, text: &str) -> String {
        match self.mode {
            Mode::Plain => text.to_string(),
            Mode::Live => format!("  {text}"),
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
}
