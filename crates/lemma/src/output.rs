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
