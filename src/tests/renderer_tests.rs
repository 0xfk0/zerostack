use crate::config::ClipboardSelection;
use crate::ui::renderer::{base64_encode, clipboard_strategy, copy_to_clipboard, is_safe_url};

#[test]
fn base64_encode_empty() {
    assert_eq!(base64_encode(b""), "");
}

#[test]
fn base64_encode_single_byte() {
    assert_eq!(base64_encode(b"f"), "Zg==");
}

#[test]
fn base64_encode_two_bytes() {
    assert_eq!(base64_encode(b"fo"), "Zm8=");
}

#[test]
fn base64_encode_three_bytes() {
    assert_eq!(base64_encode(b"foo"), "Zm9v");
}

#[test]
fn base64_encode_known_values() {
    assert_eq!(base64_encode(b"Hello"), "SGVsbG8=");
    assert_eq!(base64_encode(b"Hi!"), "SGkh");
    assert_eq!(base64_encode(b"ab"), "YWI=");
    assert_eq!(base64_encode(b"abc"), "YWJj");
    assert_eq!(base64_encode(b"Man"), "TWFu");
}

#[test]
fn base64_encode_long_input() {
    let input = "The quick brown fox jumps over the lazy dog. ".repeat(10);
    let encoded = base64_encode(input.as_bytes());
    assert!(encoded.len() > input.len());
    assert!(encoded.ends_with('=') || !encoded.contains('='));
}

#[test]
fn copy_to_clipboard_does_not_panic() {
    // Succeeds via an external tool or the OSC 52 fallback.
    copy_to_clipboard("test text", ClipboardSelection::Clipboard).expect("copy should succeed");
}

#[test]
fn copy_to_clipboard_empty_string() {
    copy_to_clipboard("", ClipboardSelection::Clipboard).expect("copy should succeed");
}

#[test]
fn copy_to_primary_selection_does_not_panic() {
    copy_to_clipboard("test text", ClipboardSelection::Primary).expect("copy should succeed");
}

#[test]
fn clipboard_strategy_includes_xsel_for_x11() {
    // Regression: X11 boxes without xclip must still reach the clipboard
    // through xsel. It was missing, so copy silently fell through to OSC 52.
    let (cmds, target) = clipboard_strategy(ClipboardSelection::Clipboard);
    assert_eq!(target, 'c');
    assert!(cmds.contains(&("xsel", &["--clipboard", "--input"][..])));
    assert!(cmds.contains(&("xclip", &["-selection", "clipboard"][..])));
}

#[test]
fn clipboard_strategy_primary_uses_only_x11_tools() {
    let (cmds, target) = clipboard_strategy(ClipboardSelection::Primary);
    assert_eq!(target, 'p');
    assert!(cmds.contains(&("xsel", &["--primary", "--input"][..])));
    assert!(cmds.contains(&("xclip", &["-selection", "primary"][..])));
    // Wayland/macOS/Windows tools target the clipboard, not PRIMARY.
    assert!(!cmds.iter().any(|(cmd, _)| *cmd == "wl-copy"));
    assert!(!cmds.iter().any(|(cmd, _)| *cmd == "pbcopy"));
}

#[test]
fn safe_url_accepts_http_and_https() {
    assert!(is_safe_url("https://example.com"));
    assert!(is_safe_url("http://example.com/path?q=1#frag"));
    assert!(is_safe_url("https://user@example.com:8080/x"));
}

#[test]
fn safe_url_rejects_non_http_schemes() {
    assert!(!is_safe_url("file:///etc/passwd"));
    assert!(!is_safe_url("javascript:alert(1)"));
    assert!(!is_safe_url("ftp://example.com"));
    assert!(!is_safe_url("example.com/no-scheme"));
    assert!(!is_safe_url(""));
}

#[test]
fn safe_url_rejects_missing_host() {
    assert!(!is_safe_url("https://"));
    assert!(!is_safe_url("http:///path"));
}

#[test]
fn safe_url_rejects_whitespace_and_control_chars() {
    assert!(!is_safe_url("https://example.com/a b"));
    assert!(!is_safe_url("https://example.com/\nevil"));
    assert!(!is_safe_url("https://example.com/\x07"));
}

#[test]
fn safe_url_rejects_overlong_urls() {
    let long = format!("https://example.com/{}", "a".repeat(2100));
    assert!(!is_safe_url(&long));
}

#[test]
fn chat_margin_reduces_content_width() {
    let mut r = crate::ui::renderer::Renderer::new().unwrap();
    let full = r.line_width();
    r.set_chat_margin(4);
    assert_eq!(r.line_width(), full.saturating_sub(4));
    // Zero margin leaves the width unchanged.
    r.set_chat_margin(0);
    assert_eq!(r.line_width(), full);
}

mod dirty {
    use crate::ui::feed::BlockStyle;
    use crate::ui::renderer::{BottomRedrawPlan, BottomSnapshot, PromptSnapshot, Renderer};
    use crate::ui::statusline::StatusSpan;

    fn bottom_snapshot() -> BottomSnapshot {
        BottomSnapshot {
            cols: 80,
            rows: 24,
            statusline_height: 1,
            input: String::new(),
            cursor_pos: 0,
            is_running: false,
            spinner_frame: 0,
            input_vscroll_offset: 0,
            prompt: PromptSnapshot::Input,
            statusline: vec![vec![StatusSpan::Text {
                text: "model".to_string(),
                fg: None,
                bg: None,
            }]],
            scroll_indicator: false,
            monochrome: false,
            input_bg: None,
            status_bg: None,
        }
    }

    #[test]
    fn fresh_renderer_needs_chat_redraw() {
        let r = Renderer::new().unwrap();
        assert!(r.chat_needs_redraw());
    }

    #[test]
    fn chat_clean_after_mark_clean() {
        let mut r = Renderer::new().unwrap();
        r.mark_chat_clean();
        assert!(!r.chat_needs_redraw());
    }

    #[test]
    fn feed_mut_mutation_triggers_chat_redraw() {
        let mut r = Renderer::new().unwrap();
        r.mark_chat_clean();
        r.feed_mut().push_block(BlockStyle::Plain, "hello");
        assert!(r.chat_needs_redraw());
    }

    #[test]
    fn scroll_triggers_chat_redraw() {
        let mut r = Renderer::new().unwrap();
        // Enough lines to overflow the (fallback 80x24) viewport.
        for i in 0..40 {
            r.feed_mut()
                .push_line(BlockStyle::Plain, format!("line {i}"));
        }
        r.mark_chat_clean();
        assert!(!r.chat_needs_redraw());
        r.scroll_line_up();
        assert!(r.chat_needs_redraw());
    }

    #[test]
    fn resize_marks_chat_dirty() {
        let mut r = Renderer::new().unwrap();
        r.mark_chat_clean();
        r.resize();
        assert!(r.chat_needs_redraw());
    }

    #[test]
    fn selection_change_triggers_chat_redraw() {
        let mut r = Renderer::new().unwrap();
        r.feed_mut().push_line(BlockStyle::Plain, "selectable");
        r.mark_chat_clean();
        assert!(!r.chat_needs_redraw());
        // Selection fields are public and mutated directly by callers.
        r.selection_active = true;
        r.selection_start = Some(0);
        r.selection_end = Some(0);
        assert!(r.chat_needs_redraw());
        r.mark_chat_clean();
        r.clear_selection();
        assert!(r.chat_needs_redraw());
    }

    #[test]
    fn invalidate_marks_chat_dirty() {
        let mut r = Renderer::new().unwrap();
        r.mark_chat_clean();
        r.invalidate();
        assert!(r.chat_needs_redraw());
    }

    #[test]
    fn bottom_plan_full_when_no_previous() {
        let next = bottom_snapshot();
        assert_eq!(
            Renderer::bottom_redraw_plan(None, &next, false),
            BottomRedrawPlan::Full
        );
    }

    #[test]
    fn bottom_plan_skip_when_unchanged() {
        let prev = bottom_snapshot();
        let next = bottom_snapshot();
        assert_eq!(
            Renderer::bottom_redraw_plan(Some(&prev), &next, false),
            BottomRedrawPlan::Skip
        );
    }

    #[test]
    fn bottom_plan_force_full() {
        let prev = bottom_snapshot();
        let next = bottom_snapshot();
        assert_eq!(
            Renderer::bottom_redraw_plan(Some(&prev), &next, true),
            BottomRedrawPlan::Full
        );
    }

    #[test]
    fn bottom_plan_statusline_only_on_statusline_change() {
        let prev = bottom_snapshot();
        let mut next = bottom_snapshot();
        next.statusline = vec![vec![StatusSpan::Text {
            text: "other model".to_string(),
            fg: None,
            bg: None,
        }]];
        assert_eq!(
            Renderer::bottom_redraw_plan(Some(&prev), &next, false),
            BottomRedrawPlan::StatuslineOnly
        );
    }

    #[test]
    fn bottom_plan_statusline_only_on_scroll_indicator_change() {
        let prev = bottom_snapshot();
        let mut next = bottom_snapshot();
        next.scroll_indicator = true;
        assert_eq!(
            Renderer::bottom_redraw_plan(Some(&prev), &next, false),
            BottomRedrawPlan::StatuslineOnly
        );
    }

    #[test]
    fn bottom_plan_full_on_input_change() {
        let prev = bottom_snapshot();
        let mut next = bottom_snapshot();
        next.input = "typed".to_string();
        assert_eq!(
            Renderer::bottom_redraw_plan(Some(&prev), &next, false),
            BottomRedrawPlan::Full
        );
    }

    #[test]
    fn bottom_plan_full_on_cursor_change() {
        let prev = bottom_snapshot();
        let mut next = bottom_snapshot();
        next.cursor_pos = 3;
        assert_eq!(
            Renderer::bottom_redraw_plan(Some(&prev), &next, false),
            BottomRedrawPlan::Full
        );
    }

    #[test]
    fn bottom_plan_full_on_prompt_mode_change() {
        let prev = bottom_snapshot();
        let mut next = bottom_snapshot();
        next.prompt = PromptSnapshot::Chain {
            question: "continue?".into(),
            but_mode: false,
        };
        assert_eq!(
            Renderer::bottom_redraw_plan(Some(&prev), &next, false),
            BottomRedrawPlan::Full
        );
    }

    #[test]
    fn bottom_plan_full_on_geometry_change() {
        let prev = bottom_snapshot();
        let mut next = bottom_snapshot();
        next.rows = 40;
        assert_eq!(
            Renderer::bottom_redraw_plan(Some(&prev), &next, false),
            BottomRedrawPlan::Full
        );
    }

    #[test]
    fn bottom_plan_full_on_spinner_frame_change() {
        let prev = bottom_snapshot();
        let mut next = bottom_snapshot();
        next.is_running = true;
        next.spinner_frame = 1;
        assert_eq!(
            Renderer::bottom_redraw_plan(Some(&prev), &next, false),
            BottomRedrawPlan::Full
        );
    }

    #[test]
    fn bottom_plan_full_on_input_scroll_change() {
        let prev = bottom_snapshot();
        let mut next = bottom_snapshot();
        next.input_vscroll_offset = 1;
        assert_eq!(
            Renderer::bottom_redraw_plan(Some(&prev), &next, false),
            BottomRedrawPlan::Full
        );
    }

    #[test]
    fn bottom_plan_full_when_statusline_and_input_change() {
        let prev = bottom_snapshot();
        let mut next = bottom_snapshot();
        next.input = "typed".to_string();
        next.statusline = Vec::new();
        assert_eq!(
            Renderer::bottom_redraw_plan(Some(&prev), &next, false),
            BottomRedrawPlan::Full
        );
    }
}

/// Drive `draw_bottom` through a `FakeBackend` and read back the emitted
/// caret (row, col). Content-agnostic: works for any input string.
mod cursor_positioning {
    use crate::ui::renderer::{FakeBackend, Renderer};
    use regex::Regex;

    const COLS: u16 = 80;
    const PROMPT_WIDTH: usize = 2;
    const VISIBLE_WIDTH: usize = COLS as usize - PROMPT_WIDTH;

    fn emitted_cursor(input: &str, cursor: usize) -> (u16, u16) {
        let mut r = Renderer::with_backend(Box::new(FakeBackend::new(COLS, 24)));
        r.set_statusline_height(1);
        r.draw_bottom(input, cursor, &[], false).unwrap();
        let out = r.captured_output();
        // ANSI Cursor Position: ESC[row;colH (1-based), as emitted by crossterm's MoveTo.
        let re = Regex::new(r"\x1b\[(\d+);(\d+)H").unwrap();
        re.captures_iter(&out)
            .last()
            .map(|c| {
                (
                    c[1].parse::<u16>().unwrap() - 1,
                    c[2].parse::<u16>().unwrap() - 1,
                )
            })
            .expect("renderer must emit a cursor MoveTo")
    }

    #[test]
    fn empty_buffer_cursor_after_prompt() {
        assert_eq!(emitted_cursor("", 0).1, 2);
    }

    #[test]
    fn cursor_after_one_ascii_char() {
        assert_eq!(emitted_cursor("a", 1).1, 3);
    }

    #[test]
    fn cursor_after_three_ascii_chars() {
        assert_eq!(emitted_cursor("abc", 3).1, 5);
    }

    #[test]
    fn cursor_mid_line() {
        assert_eq!(emitted_cursor("abc", 1).1, 3);
    }

    fn repeat_ascii(n: usize) -> String {
        "a".repeat(n)
    }

    #[test]
    fn cursor_at_end_of_short_line() {
        // Line fits: caret is one past the last char.
        let line = repeat_ascii(5);
        assert_eq!(emitted_cursor(&line, 5).1, (PROMPT_WIDTH + 5) as u16);
    }

    #[test]
    fn cursor_at_end_of_full_width_line() {
        // Line exactly fills the row; it scrolls one column left so the caret
        // stays on-screen ONE PAST the last visible char — at the final column
        // (COLS-1), not on it (COLS-2) and not off-screen (COLS).
        let line = repeat_ascii(VISIBLE_WIDTH);
        assert_eq!(emitted_cursor(&line, VISIBLE_WIDTH).1, COLS - 1);
    }

    #[test]
    fn cursor_at_end_of_overflowing_line() {
        let line = repeat_ascii(VISIBLE_WIDTH + 12);
        assert_eq!(emitted_cursor(&line, VISIBLE_WIDTH + 12).1, COLS - 1);
    }
}

/// Streamed writes must not break a line just because a chunk boundary fell
/// inside it. Providers routinely batch a line's final token together with the
/// newline that ends it (".\n"), which used to strand the "." on its own row
/// under the line it belonged to.
mod streamed_writes {
    use crate::ui::renderer::{FakeBackend, Renderer};
    use crossterm::style::Color;

    const COLS: u16 = 80;

    fn renderer() -> Renderer {
        Renderer::with_backend(Box::new(FakeBackend::new(COLS, 24)))
    }

    fn rows(r: &Renderer) -> Vec<String> {
        r.feed()
            .lines(COLS as usize)
            .into_iter()
            .map(|l| l.text.to_string())
            .collect()
    }

    #[test]
    fn newline_chunk_does_not_split_the_line_it_ends() {
        let mut r = renderer();
        r.write(
            "commit_partial flushes the reasoning partial. OK",
            Color::DarkMagenta,
        )
        .unwrap();
        r.write(".\n\nSo:\n", Color::DarkMagenta).unwrap();
        assert_eq!(
            rows(&r),
            vec![
                "commit_partial flushes the reasoning partial. OK.",
                "",
                "So:"
            ]
        );
    }

    #[test]
    fn token_split_across_chunks_stays_on_one_row() {
        let mut r = renderer();
        r.write("let mut feed = Feed::new", Color::DarkMagenta)
            .unwrap();
        r.write("();\n", Color::DarkMagenta).unwrap();
        assert_eq!(rows(&r), vec!["let mut feed = Feed::new();"]);
    }

    #[test]
    fn blank_and_trailing_segments_still_become_rows() {
        let mut r = renderer();
        r.write("first\n\nsecond\n", Color::DarkMagenta).unwrap();
        assert_eq!(rows(&r), vec!["first", "", "second"]);
    }

    #[test]
    fn newline_only_chunk_yields_a_blank_row() {
        let mut r = renderer();
        r.write("\n", Color::DarkMagenta).unwrap();
        assert_eq!(rows(&r), vec![""]);
    }

    /// Providers stream reasoning in line-sized chunks, so the renderer ends up
    /// with one block per line. Those blocks are one continuous section: the
    /// layout must not treat each as a new section and insert a blank row
    /// between them.
    #[test]
    fn consecutive_reasoning_lines_are_not_double_spaced() {
        let mut r = renderer();
        r.write("line one\n", Color::DarkMagenta).unwrap();
        r.write("line two\n", Color::DarkMagenta).unwrap();
        assert_eq!(rows(&r), vec!["line one", "line two"]);
    }
}

/// Column-accurate drag selection: the copied text and the highlighted span are
/// bounded by mouse columns, not whole rows.
mod selection {
    use crate::ui::feed::BlockStyle;
    use crate::ui::renderer::{FakeBackend, Renderer, byte_index_for_col, word_bounds};

    fn renderer_with(lines: &[&str]) -> Renderer {
        let mut r = Renderer::with_backend(Box::new(FakeBackend::new(80, 24)));
        for line in lines {
            r.feed_mut().push_line(BlockStyle::Plain, *line);
        }
        r
    }

    fn select(r: &mut Renderer, start: usize, start_col: usize, end: usize, end_col: usize) {
        r.selection_active = true;
        r.selection_start = Some(start);
        r.selection_start_col = Some(start_col);
        r.selection_end = Some(end);
        r.selection_end_col = Some(end_col);
    }

    #[test]
    fn byte_index_maps_columns_to_boundaries() {
        assert_eq!(byte_index_for_col("hello", 0), 0);
        assert_eq!(byte_index_for_col("hello", 3), 3);
        assert_eq!(byte_index_for_col("hello", 5), 5);
        assert_eq!(byte_index_for_col("hello", 99), 5);
        // Wide (CJK) chars: 日本語 = 6 columns = bytes 0..9.
        assert_eq!(byte_index_for_col("日本語abc", 0), 0);
        assert_eq!(byte_index_for_col("日本語abc", 5), 6);
        assert_eq!(byte_index_for_col("日本語abc", 6), 9);
    }

    #[test]
    fn one_word() {
        let mut r = renderer_with(&["hello world"]);
        select(&mut r, 0, 0, 0, 5);
        assert_eq!(r.selected_text().as_deref(), Some("hello"));
    }

    #[test]
    fn two_words_within_a_row() {
        let mut r = renderer_with(&["the quick brown fox"]);
        select(&mut r, 0, 4, 0, 15);
        assert_eq!(r.selected_text().as_deref(), Some("quick brown"));
    }

    #[test]
    fn spanning_rows_keeps_only_the_column_bounds() {
        let mut r = renderer_with(&["hello world", "second line"]);
        select(&mut r, 0, 6, 1, 6);
        assert_eq!(r.selected_text().as_deref(), Some("world\nsecond"));
    }

    #[test]
    fn whole_rows_when_columns_absent() {
        let mut r = renderer_with(&["alpha", "beta"]);
        r.selection_active = true;
        r.selection_start = Some(0);
        r.selection_end = Some(1);
        assert_eq!(r.selected_text().as_deref(), Some("alpha\nbeta"));
    }

    #[test]
    fn reversed_drag_is_normalized() {
        let mut r = renderer_with(&["one two three"]);
        // Released left of where the drag started: end col precedes start col.
        // Columns 4..7 are exactly "two".
        select(&mut r, 0, 7, 0, 4);
        assert_eq!(r.selected_text().as_deref(), Some("two"));
    }

    #[test]
    fn wide_chars_selection_snaps_to_glyph_boundaries() {
        let mut r = renderer_with(&["日本語abc"]);
        select(&mut r, 0, 0, 0, 6);
        assert_eq!(r.selected_text().as_deref(), Some("日本語"));
    }

    #[test]
    fn highlight_reverses_only_the_selected_span() {
        let mut r = renderer_with(&["the quick brown fox"]);
        select(&mut r, 0, 4, 0, 15);
        r.render_viewport().unwrap();
        let out = r.captured_output();
        // crossterm renders Reverse as ESC[7m and NoReverse as ESC[27m.
        assert!(
            out.contains("\u{1b}[7mquick brown\u{1b}[27m"),
            "selection span must be reversed in place: {out:?}"
        );
    }

    #[test]
    fn word_bounds_snaps_to_whole_words() {
        // Columns: 0..5 "hello", space at 5, 6..11 "world".
        assert_eq!(word_bounds("hello world", 2), (0, 5));
        assert_eq!(word_bounds("hello world", 8), (6, 11));
        // A click on the space selects that whitespace run.
        assert_eq!(word_bounds("hello world", 5), (5, 6));
        // Punctuation forms its own run, separate from adjacent word chars.
        assert_eq!(word_bounds("foo(bar)", 3), (3, 4));
        assert_eq!(word_bounds("foo(bar)", 4), (4, 7));
        assert_eq!(word_bounds("foo_bar baz", 2), (0, 7));
        // Past the end, the trailing word is still selected.
        assert_eq!(word_bounds("hello world", 99), (6, 11));
        assert_eq!(word_bounds("", 0), (0, 0));
    }

    #[test]
    fn double_click_selects_the_whole_word() {
        let mut r = renderer_with(&["the quick brown fox"]);
        // Click inside "quick" (columns 4..9).
        assert!(r.select_word(0, 6));
        assert_eq!(r.selected_text().as_deref(), Some("quick"));
    }

    #[test]
    fn double_click_on_whitespace_selects_the_run() {
        let mut r = renderer_with(&["the quick brown"]);
        // Column 3 is the space between "the" and "quick".
        assert!(r.select_word(0, 3));
        assert_eq!(r.selected_text().as_deref(), Some(" "));
    }

    #[test]
    fn select_word_outside_the_buffer_is_a_noop() {
        let mut r = renderer_with(&["only line"]);
        assert!(!r.select_word(9, 0));
        assert!(!r.selection_active);
    }

    #[test]
    fn word_drag_right_keeps_whole_word() {
        let mut r = renderer_with(&["the quick brown fox"]);
        assert!(r.select_word(0, 6)); // "quick" spans columns 4..9
        r.extend_selection(0, 15);
        assert_eq!(r.selected_text().as_deref(), Some("quick brown"));
    }

    #[test]
    fn word_drag_left_keeps_whole_word() {
        let mut r = renderer_with(&["the quick brown fox"]);
        assert!(r.select_word(0, 6));
        r.extend_selection(0, 1);
        assert_eq!(r.selected_text().as_deref(), Some("he quick"));
    }

    #[test]
    fn word_drag_up_keeps_whole_word() {
        let mut r = renderer_with(&["hello world", "second line"]);
        assert!(r.select_word(1, 2)); // "second" on row 1, columns 0..6
        r.extend_selection(0, 3);
        assert_eq!(r.selected_text().as_deref(), Some("lo world\nsecond"));
    }

    #[test]
    fn word_drag_back_inside_reselects_the_whole_word() {
        let mut r = renderer_with(&["the quick brown fox"]);
        assert!(r.select_word(0, 6));
        r.extend_selection(0, 1);
        r.extend_selection(0, 6);
        assert_eq!(r.selected_text().as_deref(), Some("quick"));
    }

    #[test]
    fn plain_drag_after_a_word_drops_the_word_anchor() {
        let mut r = renderer_with(&["the quick brown fox"]);
        assert!(r.select_word(0, 6));
        r.start_selection(0, 1);
        r.extend_selection(0, 9);
        assert_eq!(r.selected_text().as_deref(), Some("he quick"));
    }
}

/// Drag-to-edge auto-scroll: holding the pointer on the top or bottom row
/// scrolls the transcript one line per event, so a selection can reach text
/// that is off screen.
mod drag_scroll {
    use crate::ui::feed::BlockStyle;
    use crate::ui::renderer::{FakeBackend, Renderer};

    fn renderer_with(n: usize) -> Renderer {
        let mut r = Renderer::with_backend(Box::new(FakeBackend::new(80, 24)));
        for i in 0..n {
            r.feed_mut()
                .push_line(BlockStyle::Plain, format!("line {i}"));
        }
        r
    }

    /// Index of the line at the top of the viewport (observable proxy for the
    /// scroll offset, whose field is private to the renderer module).
    fn top_row_index(r: &Renderer) -> usize {
        r.buffer_line_at_row(0)
            .expect("a full transcript has a top row")
    }

    #[test]
    fn middle_rows_do_not_scroll() {
        let mut r = renderer_with(100);
        let mid = (r.visible_lines() / 2) as u16;
        assert!(!r.drag_scroll(mid));
        assert!(!r.is_scrolling());
    }

    #[test]
    fn top_edge_drag_scrolls_toward_earlier_lines() {
        let mut r = renderer_with(100);
        let before = top_row_index(&r);
        assert!(r.drag_scroll(0));
        assert_eq!(top_row_index(&r), before - 1);
        assert!(r.is_scrolling());
    }

    #[test]
    fn bottom_edge_drag_scrolls_back_toward_the_latest_lines() {
        let mut r = renderer_with(100);
        r.drag_scroll(0);
        r.drag_scroll(0);
        let before = top_row_index(&r);
        let bottom = (r.visible_lines() - 1) as u16;
        assert!(r.drag_scroll(bottom));
        assert_eq!(top_row_index(&r), before + 1);
    }

    #[test]
    fn bottom_edge_at_the_live_bottom_is_a_noop() {
        let mut r = renderer_with(100);
        let bottom = (r.visible_lines() - 1) as u16;
        assert!(!r.drag_scroll(bottom));
        assert!(!r.is_scrolling());
    }

    #[test]
    fn repeated_edge_events_keep_scrolling_past_one_screenful() {
        let mut r = renderer_with(100);
        let before = top_row_index(&r);
        for _ in 0..10 {
            r.drag_scroll(0);
        }
        assert_eq!(top_row_index(&r), before - 10);
    }
}
