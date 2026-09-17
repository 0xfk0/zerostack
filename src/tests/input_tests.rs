use crate::ui::input::{InputEditor, swap_enter_and_newline};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

fn press_with(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

fn ctrl_j() -> KeyEvent {
    press_with(KeyCode::Char('j'), KeyModifiers::CONTROL)
}

fn type_str(editor: &mut InputEditor, s: &str) {
    for c in s.chars() {
        editor.handle_key(press(KeyCode::Char(c)));
    }
}

#[test]
fn typing_ascii_keeps_cursor_in_sync() {
    let mut editor = InputEditor::new();
    type_str(&mut editor, "hello");
    assert_eq!(editor.buffer.as_str(), "hello");
    assert_eq!(editor.cursor, 5);
}

#[test]
fn typing_multibyte_chars_does_not_panic() {
    // Regression for bug where `cursor += 1` (char step) was used with
    // `CompactString::insert(byte_idx, ch)` (byte boundary required).
    // Two Norwegian characters in a row were enough to trigger a panic.
    let mut editor = InputEditor::new();
    type_str(&mut editor, "på "); // used to panic on the space after 'å'
    assert_eq!(editor.buffer.as_str(), "på ");
    assert_eq!(editor.cursor, editor.buffer.len()); // cursor in bytes
}

#[test]
fn typing_mixed_ascii_and_multibyte() {
    let mut editor = InputEditor::new();
    type_str(&mut editor, "hei på deg så fin dag æøå");
    assert_eq!(editor.buffer.as_str(), "hei på deg så fin dag æøå");
    assert_eq!(editor.cursor, editor.buffer.len());
}

#[test]
fn backspace_after_multibyte_does_not_panic() {
    let mut editor = InputEditor::new();
    type_str(&mut editor, "å");
    editor.handle_key(press(KeyCode::Backspace));
    assert_eq!(editor.buffer.as_str(), "");
    assert_eq!(editor.cursor, 0);
}

#[test]
fn left_arrow_steps_one_char_not_one_byte() {
    let mut editor = InputEditor::new();
    type_str(&mut editor, "aåb");
    // cursor is after 'b', byte-idx 4 (a=1 + å=2 + b=1)
    assert_eq!(editor.cursor, 4);
    editor.handle_key(press(KeyCode::Left));
    // after 'å' → byte-idx 3
    assert_eq!(editor.cursor, 3);
    editor.handle_key(press(KeyCode::Left));
    // after 'a' → byte-idx 1 (skips the 2 bytes of 'å')
    assert_eq!(editor.cursor, 1);
}

#[test]
fn right_arrow_steps_one_char_not_one_byte() {
    let mut editor = InputEditor::new();
    type_str(&mut editor, "aåb");
    editor.cursor = 0;
    editor.handle_key(press(KeyCode::Right));
    assert_eq!(editor.cursor, 1); // after 'a'
    editor.handle_key(press(KeyCode::Right));
    assert_eq!(editor.cursor, 3); // after 'å' (skipped 2 bytes)
}

/// `Ctrl+D` deletes the character under the cursor (to its right); it is only
/// "quit" when the prompt is empty, which the app enforces before dispatch.
#[test]
fn ctrl_d_deletes_the_char_under_the_cursor() {
    let mut editor = InputEditor::new();
    type_str(&mut editor, "abc");
    editor.handle_key(press(KeyCode::Left));
    editor.handle_key(press(KeyCode::Left));
    assert_eq!(editor.cursor, 1); // between 'a' and 'b'
    editor.handle_key(press_with(KeyCode::Char('d'), KeyModifiers::CONTROL));
    assert_eq!(editor.buffer.as_str(), "ac");
    assert_eq!(editor.cursor, 1);
}

#[test]
fn ctrl_d_at_end_of_buffer_is_a_noop() {
    let mut editor = InputEditor::new();
    type_str(&mut editor, "ab");
    editor.handle_key(press_with(KeyCode::Char('d'), KeyModifiers::CONTROL));
    assert_eq!(editor.buffer.as_str(), "ab");
    assert_eq!(editor.cursor, 2);
}

#[test]
fn enter_returns_buffer_and_resets() {
    let mut editor = InputEditor::new();
    type_str(&mut editor, "hei på");
    let out = editor.handle_key(press(KeyCode::Enter)).unwrap();
    assert_eq!(out.as_str(), "hei på");
    assert_eq!(editor.cursor, 0);
    assert_eq!(editor.buffer.as_str(), "");
}

/// `Ctrl+J` is the portable newline key. It must insert `'\n'` rather than fall
/// through to the generic `Char` arm and type a literal `j`.
#[test]
fn ctrl_j_inserts_newline_instead_of_literal_j() {
    let mut editor = InputEditor::new();
    type_str(&mut editor, "one");
    assert!(
        editor.handle_key(ctrl_j()).is_none(),
        "Ctrl+J must not submit"
    );
    type_str(&mut editor, "two");
    assert_eq!(editor.buffer.as_str(), "one\ntwo");
    assert_eq!(editor.cursor, 7);
}

#[test]
fn enter_submits_a_multiline_buffer() {
    let mut editor = InputEditor::new();
    type_str(&mut editor, "a");
    editor.handle_key(ctrl_j());
    type_str(&mut editor, "b");
    let out = editor.handle_key(press(KeyCode::Enter)).unwrap();
    assert_eq!(out.as_str(), "a\nb");
    assert_eq!(editor.buffer.as_str(), "");
}

#[test]
fn swap_enter_and_newline_exchanges_the_two_keys() {
    let swapped = swap_enter_and_newline(press(KeyCode::Enter));
    assert_eq!(swapped.code, KeyCode::Char('j'));
    assert_eq!(swapped.modifiers, KeyModifiers::CONTROL);

    let swapped = swap_enter_and_newline(swapped);
    assert_eq!(swapped.code, KeyCode::Enter);
    assert_eq!(swapped.modifiers, KeyModifiers::NONE);
}

#[test]
fn swap_enter_and_newline_leaves_other_keys_alone() {
    for key in [
        press(KeyCode::Char('j')),                         // plain j
        press_with(KeyCode::Enter, KeyModifiers::SHIFT),   // Shift+Enter
        press_with(KeyCode::Enter, KeyModifiers::ALT),     // Alt+Enter
        press_with(KeyCode::Char('j'), KeyModifiers::ALT), // Alt+J
        press(KeyCode::Tab),
    ] {
        let swapped = swap_enter_and_newline(key);
        assert_eq!(swapped.code, key.code);
        assert_eq!(swapped.modifiers, key.modifiers);
    }
}

/// XTerm's `Shift+Insert` (and middle-click) converts every LF in the inserted
/// selection to CR, so a pasted selection arrives CR-separated. Those CRs are
/// literal characters to the editor — it splits rows on '\n' only and a raw
/// '\r' resets the terminal cursor — so a multi-line selection rendered as one
/// mostly-blank row. `handle_paste` must normalize them to LF.
#[test]
fn paste_normalizes_cr_line_endings() {
    let mut editor = InputEditor::new();
    editor.handle_paste("1\r2\r3\r".to_string());
    assert_eq!(editor.buffer.as_str(), "1\n2\n3\n");
    assert_eq!(
        editor.cursor,
        editor.buffer.len(),
        "cursor must land past the normalized text"
    );
}

#[test]
fn paste_collapses_crlf_without_inventing_blank_lines() {
    let mut editor = InputEditor::new();
    editor.handle_paste("a\r\nb\r\n".to_string());
    assert_eq!(editor.buffer.as_str(), "a\nb\n");
}

#[test]
fn paste_leaves_lf_untouched() {
    // The common case (every terminal with bracketed paste) must not gain a
    // copy or change a single byte.
    let mut editor = InputEditor::new();
    editor.handle_paste("a\nb".to_string());
    assert_eq!(editor.buffer.as_str(), "a\nb");
    assert_eq!(editor.cursor, 3);
}

#[test]
fn paste_inserts_at_cursor_and_normalizes() {
    let mut editor = InputEditor::new();
    type_str(&mut editor, "ac");
    editor.handle_key(press(KeyCode::Left));
    editor.handle_paste("b\r".to_string());
    assert_eq!(editor.buffer.as_str(), "ab\nc");
    assert_eq!(editor.cursor, 3);
}

/// With swapping on the app exchanges the keys before `handle_key`, so a real
/// `Enter` reaches the editor as `Ctrl+J` (newline) and a real `Ctrl+J` as
/// `Enter` (submit). Asserted through the swap helper, i.e. end to end on the
/// editor's side of the boundary.
#[test]
fn swapped_enter_inserts_newline_and_ctrl_j_submits() {
    let mut editor = InputEditor::new();
    type_str(&mut editor, "a");
    assert!(
        editor
            .handle_key(swap_enter_and_newline(press(KeyCode::Enter)))
            .is_none(),
        "swapped Enter must insert a newline, not submit"
    );
    type_str(&mut editor, "b");
    assert_eq!(editor.buffer.as_str(), "a\nb");

    let out = editor
        .handle_key(swap_enter_and_newline(ctrl_j()))
        .expect("swapped Ctrl+J must submit");
    assert_eq!(out.as_str(), "a\nb");
    assert_eq!(editor.buffer.as_str(), "");
}
