//! Terminal styling. Hand-rolled to keep the dependency tree at three crates.
//!
//! Colour is disabled when stdout is not a terminal, when `NO_COLOR` is set
//! (see no-color.org), or when `TERM=dumb`.
use std::io::IsTerminal;
use std::sync::OnceLock;

fn enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| {
        std::io::stdout().is_terminal()
            && std::env::var_os("NO_COLOR").is_none()
            && std::env::var("TERM").map(|t| t != "dumb").unwrap_or(true)
    })
}

fn wrap(code: &str, s: &str) -> String {
    if enabled() {
        format!("\x1b[{code}m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

pub fn bold(s: &str) -> String {
    wrap("1", s)
}
pub fn dim(s: &str) -> String {
    wrap("2", s)
}
pub fn green(s: &str) -> String {
    wrap("32", s)
}
pub fn yellow(s: &str) -> String {
    wrap("33", s)
}
pub fn cyan(s: &str) -> String {
    wrap("36", s)
}

/// Present.
pub fn ok() -> String {
    green("●")
}
/// Absent.
pub fn missing() -> String {
    dim("○")
}

/// Printable width, ignoring ANSI escapes — for aligning styled cells.
fn visible_len(s: &str) -> usize {
    let mut n = 0;
    let mut in_escape = false;
    for c in s.chars() {
        if in_escape {
            if c == 'm' {
                in_escape = false;
            }
        } else if c == '\x1b' {
            in_escape = true;
        } else {
            n += 1;
        }
    }
    n
}

/// Left-pad a possibly-styled string to `width`.
pub fn pad(s: &str, width: usize) -> String {
    let len = visible_len(s);
    if len >= width {
        s.to_string()
    } else {
        format!("{s}{}", " ".repeat(width - len))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visible_len_ignores_escape_sequences() {
        assert_eq!(visible_len("plain"), 5);
        assert_eq!(visible_len("\x1b[32m●\x1b[0m"), 1);
        assert_eq!(visible_len("\x1b[1mbold\x1b[0m text"), 9);
    }

    #[test]
    fn styling_is_inert_when_disabled() {
        // Tests run with stdout redirected, so colour must already be off.
        assert_eq!(bold("x"), "x", "piped output must carry no escape codes");
        assert_eq!(dim("x"), "x");
        assert!(!ok().contains('\x1b'));
    }

    #[test]
    fn pad_aligns_styled_and_plain_identically() {
        assert_eq!(pad("ab", 5).len() - 2, 3);
        let styled = pad("\x1b[32mab\x1b[0m", 5);
        assert_eq!(visible_len(&styled), 5);
    }
}
