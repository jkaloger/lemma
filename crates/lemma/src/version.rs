use std::process::Command;

const MINIMUM: (u32, u32) = (0, 15);
const INSTALL_HINT: &str = "install it with `cargo install lazyspec`";

fn parse(output: &str) -> Option<(u32, u32)> {
    let version = output
        .split_whitespace()
        .find(|word| word.starts_with(|c: char| c.is_ascii_digit()))?;
    let mut parts = version.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    Some((major, minor))
}

pub fn require_lazyspec() -> Result<(), String> {
    let (major, minor) = MINIMUM;
    let Ok(out) = Command::new("lazyspec").arg("--version").output() else {
        return Err(format!(
            "lazyspec not found on PATH; lemma needs lazyspec >= {major}.{minor}: {INSTALL_HINT}"
        ));
    };
    let stdout = String::from_utf8_lossy(&out.stdout);
    match parse(&stdout) {
        Some(found) if found >= MINIMUM => Ok(()),
        _ => Err(format!(
            "lemma needs lazyspec >= {major}.{minor}, found `{}`: {INSTALL_HINT}",
            stdout.trim()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_major_and_minor() {
        assert_eq!(parse("lazyspec 0.15.1\n"), Some((0, 15)));
        assert_eq!(parse("lazyspec 1.0.0"), Some((1, 0)));
        assert_eq!(parse("lazyspec 0.9"), Some((0, 9)));
        assert_eq!(parse("garbage"), None);
    }

    #[test]
    fn compares_against_the_minimum() {
        assert!(parse("lazyspec 0.14.9").unwrap() < MINIMUM);
        assert!(parse("lazyspec 0.15.0").unwrap() >= MINIMUM);
        assert!(parse("lazyspec 1.0.0").unwrap() >= MINIMUM);
    }
}
