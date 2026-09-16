//! Tests for the terminal suspend/resume sequences in `ui::terminal`.
//!
//! `write_suspend` / `write_resume` emit into a plain `Write`, so they can be
//! captured into a `Vec<u8>` and inspected without a real terminal. These lock
//! in the two fixes that matter for Ctrl-Z and Ctrl-G:
//!
//! * suspend must emit `Show` — the alternate screen restores only the cursor
//!   position, so a hidden caret otherwise leaks onto the shell prompt;
//! * suspend must undo the input modes the TUI enabled (bracketed paste, focus
//!   change, kitty keyboard protocol), which a child editor would otherwise
//!   render as garbage.

use crate::ui::terminal::{write_resume, write_suspend};

/// Run `f` against a fresh buffer and return the bytes it wrote.
fn capture(f: impl FnOnce(&mut Vec<u8>) -> std::io::Result<()>) -> Vec<u8> {
    let mut buf = Vec::new();
    f(&mut buf).expect("writing to a Vec never fails");
    buf
}

fn as_str(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn suspend_shows_cursor_and_disables_input_modes() {
    let out = as_str(&capture(|w| write_suspend(w, false)));

    assert!(
        out.contains("\x1b[?25h"),
        "suspend must show the cursor (fix for the invisible shell prompt): {out:?}"
    );
    assert!(
        out.contains("\x1b[?2004l"),
        "suspend must disable bracketed paste: {out:?}"
    );
    assert!(
        out.contains("\x1b[?1004l"),
        "suspend must disable focus-change reporting: {out:?}"
    );
    assert!(
        out.contains("\x1b[<1u"),
        "suspend must pop the kitty keyboard protocol: {out:?}"
    );
    assert!(
        out.contains("\x1b[?1049l"),
        "suspend must leave the alternate screen: {out:?}"
    );
}

#[test]
fn suspend_only_touches_mouse_when_capturing() {
    let off = capture(|w| write_suspend(w, false));
    let on = capture(|w| write_suspend(w, true));

    assert_ne!(
        off, on,
        "mouse_capture=true must additionally emit DisableMouseCapture"
    );
    let on = as_str(&on);
    assert!(
        on.contains("\x1b[?1000l"),
        "mouse capture must be disabled when enabled: {on:?}"
    );
}

#[test]
fn resume_enables_the_same_modes() {
    let out = as_str(&capture(|w| write_resume(w, false)));

    assert!(
        out.contains("\x1b[?1049h"),
        "resume must re-enter the alternate screen: {out:?}"
    );
    assert!(
        out.contains("\x1b[?2004h"),
        "resume must re-enable bracketed paste: {out:?}"
    );
    assert!(
        out.contains("\x1b[?1004h"),
        "resume must re-enable focus-change reporting: {out:?}"
    );
    assert!(
        out.contains("\x1b[>1u"),
        "resume must re-push the kitty keyboard protocol: {out:?}"
    );
}
