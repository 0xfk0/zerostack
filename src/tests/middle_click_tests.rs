//! The middle-button paste decision (see `MiddleClickPaste`). Kept pure so
//! every pairing of press / paste / release can be pinned without a terminal,
//! a clipboard, or the intermittent xterm behaviour this exists to survive.

use crate::ui::app::MiddleClickPaste;

#[test]
fn a_plain_press_and_release_pastes_primary() {
    let mut state = MiddleClickPaste::default();
    state.on_press();
    assert!(
        state.on_release(),
        "nothing pasted in between, so the app reads PRIMARY"
    );
}

#[test]
fn a_terminal_paste_inside_the_window_suppresses_the_read() {
    let mut state = MiddleClickPaste::default();
    state.on_press();
    state.on_paste();
    assert!(!state.on_release(), "the terminal pasted it already");
}

#[test]
fn a_release_without_a_press_never_pastes() {
    // With `mouse_capture` off the terminal pastes and the app sees neither
    // event; a stray release must not read PRIMARY on its own.
    let mut state = MiddleClickPaste::default();
    assert!(!state.on_release());

    state.on_paste();
    assert!(
        !state.on_release(),
        "an unarmed paste must not arm the next release"
    );
}

#[test]
fn a_paste_before_the_press_is_not_counted() {
    let mut state = MiddleClickPaste::default();
    state.on_paste(); // e.g. an unrelated Ctrl+V or paste-burst newline
    state.on_press();
    assert!(
        state.on_release(),
        "only a paste inside the window means the terminal handled the click"
    );
}

#[test]
fn each_gesture_starts_clean() {
    let mut state = MiddleClickPaste::default();
    state.on_press();
    state.on_paste();
    assert!(!state.on_release());

    state.on_press();
    assert!(state.on_release(), "the previous gesture must not leak");
}

#[test]
fn a_lost_release_does_not_leak_into_the_next_gesture() {
    let mut state = MiddleClickPaste::default();
    state.on_press();
    state.on_paste();
    // Release lost (focus change, terminal gone): the next press re-arms.
    state.on_press();
    assert!(state.on_release());
}
