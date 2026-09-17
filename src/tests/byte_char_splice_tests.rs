//! Byte-vs-char splice regressions (see byte-char-bug.txt).
//!
//! `InputEditor.cursor` and the picker handlers' `*cursor` are BYTE offsets
//! into the buffer. Every test below uses at least one 2-byte string
//! (`café`) and one 3-byte string (`日本`) — a single width can hide
//! take/skip drift — plus an ASCII control case per family.

use crate::ui::input::{InputEditor, Picker};
use crate::ui::pickers::file::FilePicker;
use crate::ui::pickers::list::ListPicker;
use crate::ui::pickers::models::ModelsPicker;
use compact_str::CompactString;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::PathBuf;

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

fn ctrl(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
}

fn alt(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::ALT)
}

fn type_str(editor: &mut InputEditor, s: &str) {
    for c in s.chars() {
        editor.handle_key(press(KeyCode::Char(c)));
    }
}

fn list_picker_with(items: &[&str], query: &str) -> ListPicker {
    let mut picker = ListPicker::new();
    picker.set_items(items.iter().map(|s| s.to_string()).collect());
    picker.activate();
    for c in query.chars() {
        picker.char_input(c);
    }
    picker
}

// ---------------------------------------------------------------------------
// Editor: Ctrl+U / Ctrl+K / Ctrl+Y / Alt+Y (byte-cursor splices)
// ---------------------------------------------------------------------------

#[test]
fn ctrl_u_kills_multibyte_prefix_by_bytes() {
    let mut editor = InputEditor::new();
    type_str(&mut editor, "café 日本");
    editor.set_cursor(6); // after "café ", before 日
    editor.handle_key(ctrl('u'));
    assert_eq!(editor.buffer.as_str(), "日本");
    assert_eq!(editor.cursor, 0);
    assert_eq!(editor.last_kill(), Some("café "));

    // Ctrl+Y re-yanks at the byte cursor.
    editor.handle_key(ctrl('y'));
    assert_eq!(editor.buffer.as_str(), "café 日本");
    assert_eq!(editor.cursor, 6); // 6 = "café ".len()
}

#[test]
fn ctrl_u_ascii_unchanged() {
    let mut editor = InputEditor::new();
    type_str(&mut editor, "hello world");
    editor.set_cursor(6);
    editor.handle_key(ctrl('u'));
    assert_eq!(editor.buffer.as_str(), "world");
    assert_eq!(editor.cursor, 0);
    assert_eq!(editor.last_kill(), Some("hello "));
}

#[test]
fn ctrl_k_kills_multibyte_suffix_by_bytes() {
    let mut editor = InputEditor::new();
    type_str(&mut editor, "café 日本");
    editor.set_cursor(6); // after "café "
    editor.handle_key(ctrl('k'));
    assert_eq!(editor.buffer.as_str(), "café ");
    assert_eq!(editor.cursor, 6);
    assert_eq!(editor.last_kill(), Some("日本"));
}

#[test]
fn ctrl_y_yanks_into_multibyte_text_at_byte_cursor() {
    let mut editor = InputEditor::new();
    type_str(&mut editor, "café 日本");
    editor.handle_key(ctrl('u')); // kill "café 日本"
    type_str(&mut editor, "xx");
    editor.set_cursor(1); // between the two x's
    editor.handle_key(ctrl('y'));
    assert_eq!(editor.buffer.as_str(), "xcafé 日本x");
    assert_eq!(editor.cursor, 13); // 1 + "café 日本".len() = 1 + 12
}

#[test]
fn alt_y_rotates_kill_ring_in_multibyte_text() {
    let mut editor = InputEditor::new();
    type_str(&mut editor, "café ");
    editor.handle_key(ctrl('u')); // kill "café "
    type_str(&mut editor, "bär");
    editor.handle_key(ctrl('u')); // kill "bär" (now front)
    editor.handle_key(ctrl('y')); // yank "bär"
    assert_eq!(editor.buffer.as_str(), "bär");
    assert_eq!(editor.cursor, "bär".len());

    // Alt+Y drops the previous yank and inserts the rotated one.
    editor.handle_key(alt('y'));
    assert_eq!(editor.buffer.as_str(), "café ");
    assert_eq!(editor.cursor, "café ".len());
}

// ---------------------------------------------------------------------------
// Editor: vertical movement (char-column vs byte-line-length)
// ---------------------------------------------------------------------------

#[test]
fn cursor_up_from_multibyte_line_clamps_to_line_end() {
    let mut editor = InputEditor::new();
    type_str(&mut editor, "日本語"); // 9 bytes
    editor.handle_key(ctrl('j')); // newline
    type_str(&mut editor, "café");
    assert_eq!(editor.buffer.as_str(), "日本語\ncafé");
    assert_eq!(editor.cursor, 15); // end of buffer

    // Up from end of the 4-char second line lands at end of the first line
    // (byte 9, before the '\n') — not past it.
    editor.handle_key(press(KeyCode::Up));
    assert_eq!(editor.cursor, 9);
}

// ---------------------------------------------------------------------------
// File picker: '@' splices (byte `rfind` fed to char counters before the fix)
// ---------------------------------------------------------------------------

fn file_picker_with(query: &str, cache: &[&str]) -> FilePicker {
    let mut picker = FilePicker::new();
    picker.activate();
    // `activate()` clears the cache, so seed it afterwards.
    picker.test_set_cache(cache.iter().map(PathBuf::from).collect());
    for c in query.chars() {
        picker.char_input(c);
    }
    picker
}

#[test]
fn file_picker_enter_splices_multibyte_buffer() {
    let mut buffer = CompactString::from("café @ma 日本");
    let mut cursor = buffer.len();
    let mut picker = file_picker_with("ma", &["main.rs"]);
    assert!(crate::ui::pickers::handlers::handle_file_key(
        &mut buffer,
        &mut cursor,
        &mut picker,
        press(KeyCode::Enter)
    ));
    assert_eq!(buffer.as_str(), "café main.rs 日本");
    assert_eq!(cursor, "café main.rs".len()); // before.len() + path.len()
}

#[test]
fn file_picker_enter_ascii_unchanged() {
    let mut buffer = CompactString::from("see @ma now");
    let mut cursor = buffer.len();
    let mut picker = file_picker_with("ma", &["main.rs"]);
    assert!(crate::ui::pickers::handlers::handle_file_key(
        &mut buffer,
        &mut cursor,
        &mut picker,
        press(KeyCode::Enter)
    ));
    assert_eq!(buffer.as_str(), "see main.rs now");
    assert_eq!(cursor, "see main.rs".len()); // before.len() + path.len()
}

#[test]
fn file_picker_backspace_empty_query_removes_only_the_at() {
    let mut buffer = CompactString::from("café @ma 日本");
    let mut cursor = buffer.len();
    let mut picker = file_picker_with("", &["main.rs"]);
    assert!(crate::ui::pickers::handlers::handle_file_key(
        &mut buffer,
        &mut cursor,
        &mut picker,
        press(KeyCode::Backspace)
    ));
    assert_eq!(buffer.as_str(), "café ma 日本");
    assert_eq!(cursor, "café ".len());
}

#[test]
fn file_picker_esc_drops_at_and_query_only() {
    let mut buffer = CompactString::from("café @ma 日本");
    let mut cursor = buffer.len();
    let mut picker = file_picker_with("ma", &["main.rs"]);
    assert!(crate::ui::pickers::handlers::handle_file_key(
        &mut buffer,
        &mut cursor,
        &mut picker,
        press(KeyCode::Esc)
    ));
    assert_eq!(buffer.as_str(), "café  日本");
    assert_eq!(cursor, "café ".len());
}

// ---------------------------------------------------------------------------
// Command picker: '/' splice with multibyte query and multibyte tail
// ---------------------------------------------------------------------------

#[test]
fn command_picker_esc_rewrites_multibyte_query_by_bytes() {
    let mut buffer = CompactString::from("/café 日本");
    let mut cursor = buffer.len();
    let mut picker = list_picker_with(&[], "café");
    let (handled, replacement) = crate::ui::pickers::handlers::handle_command_key(
        &mut buffer,
        &mut cursor,
        &crate::ui::pickers::handlers::CommandPickerCtx {
            prompt_names: &[],
            theme_names: &[],
            quick_model_names: &[],
            live_model_names: &[],
            provider_names: &[],
        },
        &mut picker,
        press(KeyCode::Esc),
    );
    assert!(handled);
    assert!(replacement.is_none());
    assert_eq!(buffer.as_str(), "/ 日本");
    assert_eq!(cursor, 1);
}

// ---------------------------------------------------------------------------
// Prefixed / models pickers: prefix + multibyte query + multibyte tail
// ---------------------------------------------------------------------------

#[test]
fn prefixed_picker_enter_replaces_multibyte_query_with_tail_intact() {
    let mut buffer = CompactString::from("/prompt café 日本");
    let mut cursor = buffer.len();
    let mut picker = list_picker_with(&["café-bis"], "café");
    assert!(crate::ui::pickers::handlers::handle_prefixed_key(
        &mut buffer,
        &mut cursor,
        &mut picker,
        "/prompt ",
        press(KeyCode::Enter)
    ));
    assert_eq!(buffer.as_str(), "/prompt café-bis 日本");
    assert_eq!(cursor, "/prompt café-bis".len());
}

#[test]
fn prefixed_picker_esc_drops_multibyte_query() {
    let mut buffer = CompactString::from("/prompt café 日本");
    let mut cursor = buffer.len();
    let mut picker = list_picker_with(&["café-bis"], "café");
    assert!(crate::ui::pickers::handlers::handle_prefixed_key(
        &mut buffer,
        &mut cursor,
        &mut picker,
        "/prompt ",
        press(KeyCode::Esc)
    ));
    assert_eq!(buffer.as_str(), "/prompt  日本");
    assert_eq!(cursor, "/prompt ".len());
}

#[test]
fn models_picker_enter_replaces_multibyte_query() {
    let mut buffer = CompactString::from("/models é");
    let mut cursor = buffer.len();
    let mut picker = ModelsPicker::new();
    picker.set_groups(Vec::new(), vec!["café".to_string()]);
    picker.activate();
    picker.char_input('é');
    assert_eq!(picker.matches, vec!["café".to_string()]);
    assert!(crate::ui::pickers::handlers::handle_models_key(
        &mut buffer,
        &mut cursor,
        &mut picker,
        press(KeyCode::Enter)
    ));
    assert_eq!(buffer.as_str(), "/models café");
    assert_eq!(cursor, "/models café".len());
}

// ---------------------------------------------------------------------------
// Editor: prefixed-picker activation on a multibyte first query char
// ---------------------------------------------------------------------------

#[test]
fn prefixed_picker_activates_on_multibyte_first_char() {
    let mut editor = InputEditor::new();
    editor.set_prompt_names(vec!["café".to_string()]);
    // Buffer already at "/prompt " with no active picker (e.g. after the
    // user deactivated one); the next typed char must arm the picker.
    editor.buffer = CompactString::from("/prompt ");
    editor.cursor = 8;
    editor.handle_key(press(KeyCode::Char('é')));
    match editor.picker {
        Some(Picker::Prefixed(ref p, prefix)) => {
            assert_eq!(prefix, "/prompt ");
            assert_eq!(p.query, "é");
        }
        _ => panic!("prefixed picker should be active with a multibyte first char"),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[test]
fn split_at_byte_clamps_to_char_boundary() {
    use crate::ui::utils::split_at_byte;
    let s = "café 日本"; // bytes: c a f é é ' ' 日日日 本本本
    let (before, after) = split_at_byte(s, 6);
    assert_eq!(before, "café ");
    assert_eq!(after, "日本");
    // Mid-char index clamps back to the boundary.
    let (before, after) = split_at_byte(s, 4); // inside 'é'
    assert_eq!(before, "caf");
    assert_eq!(after, "é 日本");
    // Out of range clamps to the end.
    let (before, after) = split_at_byte(s, 999);
    assert_eq!(before, s);
    assert_eq!(after, "");
}

#[test]
fn take_display_width_respects_columns_not_chars() {
    use crate::ui::utils::take_display_width;
    // CJK chars are 2 columns each: budget 5 fits "日本" (4) but not 本語.
    assert_eq!(take_display_width("日本語", 5), "日本");
    assert_eq!(take_display_width("日本語", 6), "日本語");
    // ASCII: one column per char.
    assert_eq!(take_display_width("hello", 3), "hel");
    // Zero budget yields the empty prefix.
    assert_eq!(take_display_width("hello", 0), "");
    // Mixed widths: 'é' (1 col) then CJK.
    assert_eq!(take_display_width("é日本", 4), "é日");
}
