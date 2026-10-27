/// A spinner style for the live watch line; no frame is shared with another style.
#[derive(Debug, PartialEq, Eq)]
pub struct Style {
    pub name: &'static str,
    pub frames: &'static [&'static str],
}

pub const STYLES: &[Style] = &[
    Style {
        name: "dots",
        frames: &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
    },
    Style {
        name: "line",
        frames: &["-", "\\", "|", "/"],
    },
    Style {
        name: "arc",
        frames: &["◜", "◠", "◝", "◞", "◡", "◟"],
    },
    Style {
        name: "circle",
        frames: &["◐", "◓", "◑", "◒"],
    },
    Style {
        name: "arrows",
        frames: &["←", "↖", "↑", "↗", "→", "↘", "↓", "↙"],
    },
    Style {
        name: "squares",
        frames: &["▖", "▘", "▝", "▗"],
    },
];

/// The style `draw` lands on; `draw` is the randomness seam.
pub fn pick(draw: impl FnOnce() -> u128) -> &'static Style {
    &STYLES[(draw() % STYLES.len() as u128) as usize]
}

pub fn random() -> &'static Style {
    pick(|| uuid::Uuid::new_v4().as_u128())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn no_frame_is_shared_between_styles() {
        let mut seen = HashSet::new();
        for style in STYLES {
            for frame in style.frames {
                assert!(seen.insert(*frame), "{frame} repeats in {}", style.name);
            }
        }
    }

    #[test]
    fn every_style_animates() {
        for style in STYLES {
            assert!(style.frames.len() > 1, "{}", style.name);
        }
    }

    #[test]
    fn the_draw_pins_the_style() {
        assert_eq!(pick(|| 0), &STYLES[0]);
        assert_eq!(pick(|| 1), &STYLES[1]);
        assert_eq!(pick(|| STYLES.len() as u128 + 2), &STYLES[2]);
        assert_eq!(
            pick(|| u128::MAX),
            &STYLES[(u128::MAX % STYLES.len() as u128) as usize]
        );
    }

    #[test]
    fn a_random_pick_is_from_the_table() {
        assert!(STYLES.contains(random()));
    }
}
