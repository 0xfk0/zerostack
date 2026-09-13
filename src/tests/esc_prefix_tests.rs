use crate::ui::{EscPrefix, EscVerdict};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn esc() -> KeyEvent {
    KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)
}

#[test]
fn disabled_passes_everything_through() {
    let mut m = EscPrefix::new(false);
    assert_eq!(m.on_key(esc()), EscVerdict::Key(esc()));
    assert_eq!(
        m.on_key(press(KeyCode::Char('b'))),
        EscVerdict::Key(press(KeyCode::Char('b')))
    );
    assert_eq!(m.on_timeout(), None);
    assert_eq!(m.flush(), None);
}

#[test]
fn lone_esc_is_held_not_emitted() {
    // A long window keeps the "not elapsed yet" assertion deterministic.
    let mut m = EscPrefix::with_window(true, Duration::from_secs(10));
    assert_eq!(m.on_key(esc()), EscVerdict::Pending);
    assert_eq!(m.on_timeout(), None);
}

#[test]
fn held_esc_is_released_after_the_window() {
    let mut m = EscPrefix::with_window(true, Duration::from_millis(5));
    assert_eq!(m.on_key(esc()), EscVerdict::Pending);
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(m.on_timeout(), Some(esc()));
    // Released once; a second timeout is a no-op.
    assert_eq!(m.on_timeout(), None);
}

#[test]
fn esc_then_char_becomes_alt_char() {
    let mut m = EscPrefix::new(true);
    assert_eq!(m.on_key(esc()), EscVerdict::Pending);
    assert_eq!(
        m.on_key(press(KeyCode::Char('b'))),
        EscVerdict::Key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::ALT))
    );
}

#[test]
fn esc_then_enter_becomes_alt_enter() {
    let mut m = EscPrefix::new(true);
    assert_eq!(m.on_key(esc()), EscVerdict::Pending);
    assert_eq!(
        m.on_key(press(KeyCode::Enter)),
        EscVerdict::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::ALT))
    );
}

#[test]
fn esc_then_modified_key_releases_esc_first() {
    let mut m = EscPrefix::new(true);
    assert_eq!(m.on_key(esc()), EscVerdict::Pending);
    let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert_eq!(m.on_key(ctrl_c), EscVerdict::EscThen(ctrl_c));
    assert_eq!(m.on_timeout(), None);
}

#[test]
fn esc_then_non_char_key_releases_esc_first() {
    let mut m = EscPrefix::new(true);
    assert_eq!(m.on_key(esc()), EscVerdict::Pending);
    assert_eq!(
        m.on_key(press(KeyCode::Tab)),
        EscVerdict::EscThen(press(KeyCode::Tab))
    );
}

#[test]
fn double_esc_collapses_to_one() {
    let mut m = EscPrefix::new(true);
    assert_eq!(m.on_key(esc()), EscVerdict::Pending);
    assert_eq!(m.on_key(esc()), EscVerdict::Key(esc()));
    assert_eq!(m.on_timeout(), None);
}

#[test]
fn alt_esc_from_a_single_read_collapses_to_one() {
    // `ESC ESC` in one read is reported by crossterm as Alt+Esc.
    let mut m = EscPrefix::new(true);
    let alt_esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::ALT);
    assert_eq!(m.on_key(alt_esc), EscVerdict::Key(esc()));
}

#[test]
fn expired_window_releases_esc_before_the_next_char() {
    let mut m = EscPrefix::with_window(true, Duration::from_millis(5));
    assert_eq!(m.on_key(esc()), EscVerdict::Pending);
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(
        m.on_key(press(KeyCode::Char('b'))),
        EscVerdict::EscThen(press(KeyCode::Char('b')))
    );
}

#[test]
fn flush_releases_a_held_esc_once() {
    let mut m = EscPrefix::new(true);
    assert_eq!(m.on_key(esc()), EscVerdict::Pending);
    assert_eq!(m.flush(), Some(esc()));
    assert_eq!(m.flush(), None);
}
