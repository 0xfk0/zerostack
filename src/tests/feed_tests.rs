use crate::ui::feed::{BlockStyle, Feed, markdown_role};
use crate::ui::roles::{ROLES_GUARD, apply, default_color, reset};
use crossterm::style::Color;

#[test]
fn block_style_color_mapping() {
    assert_eq!(BlockStyle::User.color(), Color::Green);
    assert_eq!(BlockStyle::Agent.color(), Color::White);
    assert_eq!(BlockStyle::Reasoning.color(), Color::DarkMagenta);
    assert_eq!(BlockStyle::Tool.color(), Color::Yellow);
    assert_eq!(BlockStyle::ToolResult.color(), Color::DarkGrey);
    assert_eq!(BlockStyle::Error.color(), Color::Red);
    assert_eq!(BlockStyle::System.color(), Color::DarkGrey);
    assert_eq!(BlockStyle::Welcome.color(), Color::Cyan);
    assert_eq!(BlockStyle::Permission.color(), Color::Magenta);
    assert_eq!(BlockStyle::Plain.color(), Color::White);
    assert_eq!(BlockStyle::Code.color(), Color::DarkYellow);
    assert_eq!(BlockStyle::Link.color(), Color::DarkCyan);
}

/// Remapping the markdown palette onto roles must be a no-op until a theme
/// overrides one, i.e. every mapped role's *default* is the parser's color.
#[test]
fn markdown_palette_maps_to_matching_roles() {
    for (color, role) in [
        (Color::White, BlockStyle::Agent),
        (Color::DarkGrey, BlockStyle::System),
        (Color::Cyan, BlockStyle::Welcome),
        (Color::DarkYellow, BlockStyle::Code),
        (Color::DarkCyan, BlockStyle::Link),
    ] {
        assert_eq!(markdown_role(color), Some(role), "mapping for {color:?}");
        assert_eq!(
            default_color(role),
            color,
            "{role:?}'s default drifted from the parser color"
        );
    }

    // Anything the parser did not hardcode passes through untouched.
    assert_eq!(markdown_role(Color::Red), None);
    assert_eq!(markdown_role(Color::Reset), None);
}

/// Markdown text is laid out with the parser's hardcoded palette, so a theme's
/// role overrides only reach replies if the layout step remaps them.
#[test]
fn markdown_colors_follow_role_overrides() {
    let _guard = ROLES_GUARD.lock().unwrap_or_else(|e| e.into_inner());

    let dark = Color::Rgb {
        r: 0x11,
        g: 0x22,
        b: 0x33,
    };
    let roles: std::collections::HashMap<String, String> =
        ["agent", "system", "welcome", "code", "link"]
            .into_iter()
            .map(|name| (name.to_string(), "#112233".to_string()))
            .collect();
    apply(&roles);

    // One block per palette entry: heading (Cyan), body (White), quote
    // (DarkGrey), fenced code (DarkYellow), link text (DarkCyan).
    let mut feed = Feed::new();
    feed.push_line(
        BlockStyle::Agent,
        "# Head\n\nbody\n\n> quoted\n\n```\ncode\n```\n\n[link](https://example.com)\n",
    );
    let lines = feed.lines(40);

    assert!(!lines.is_empty());
    for line in &lines {
        assert_eq!(
            line.color, dark,
            "markdown color escaped the theme: {:?} -> {:?}",
            line.text, line.color
        );
    }

    reset();
}

#[test]
fn lines_wrap_plain_block() {
    let mut feed = Feed::new();
    feed.push_line(BlockStyle::Plain, "hello world");
    let lines = feed.lines(20);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].text, "hello world");
    assert_eq!(lines[0].color, Color::White);
}

#[test]
fn lines_wrap_narrow_width() {
    let mut feed = Feed::new();
    feed.push_line(BlockStyle::Plain, "hello world");
    let lines = feed.lines(5);
    assert!(lines.len() > 1);
    for line in &lines {
        assert!(line.text.chars().count() <= 5 || line.text == "hello" || line.text == "world");
    }
}

#[test]
fn empty_block_produces_empty_line() {
    let mut feed = Feed::new();
    feed.push_line(BlockStyle::Plain, "");
    let lines = feed.lines(80);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].text, "");
}

#[test]
fn agent_block_gets_prefix_and_markdown() {
    let mut feed = Feed::new();
    feed.push_block(BlockStyle::Agent, "hello **world**");
    let lines = feed.lines(80);
    assert!(!lines.is_empty());
    assert!(
        lines[0].text.starts_with("< "),
        "first agent line should start with '< ', got {:?}",
        lines[0].text
    );
    let joined: String = lines
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join("");
    assert!(
        joined.contains("hello "),
        "prose should be present: {}",
        joined
    );
    assert!(
        joined.contains("world"),
        "bold text should be present: {}",
        joined
    );
}

#[test]
fn agent_empty_block_no_lines() {
    let mut feed = Feed::new();
    feed.push_block(BlockStyle::Agent, "");
    let lines = feed.lines(80);
    assert!(lines.is_empty());
}

#[test]
fn line_count_matches_lines() {
    let mut feed = Feed::new();
    feed.push_line(BlockStyle::Plain, "one");
    feed.push_line(BlockStyle::Plain, "two");
    feed.push_line(BlockStyle::Plain, "three");
    assert_eq!(feed.line_count(80), 3);
}

#[test]
fn visible_range_bottom_aligned_when_short() {
    let mut feed = Feed::new();
    feed.push_line(BlockStyle::Plain, "one");
    feed.push_line(BlockStyle::Plain, "two");
    let (start, end) = feed.visible_range(80, 0, 10);
    assert_eq!(start, 0);
    assert_eq!(end, 2);
}

#[test]
fn visible_range_scrolled() {
    let mut feed = Feed::new();
    for i in 0..20 {
        feed.push_line(BlockStyle::Plain, format!("line {}", i));
    }
    let (start, end) = feed.visible_range(80, 5, 10);
    assert_eq!(end - start, 10);
    assert_eq!(start, 5);
}

#[test]
fn line_at_visual_row_bottom_pad() {
    let mut feed = Feed::new();
    feed.push_line(BlockStyle::Plain, "one");
    // viewport height 10, auto-scroll, content shorter than viewport -> padding
    assert_eq!(feed.line_at_visual_row(80, 0, 10, 0), None);
    assert_eq!(feed.line_at_visual_row(80, 0, 10, 9), Some(0));
}

#[test]
fn line_at_visual_row_scrolled() {
    let mut feed = Feed::new();
    for i in 0..20 {
        feed.push_line(BlockStyle::Plain, format!("line {}", i));
    }
    assert_eq!(feed.line_at_visual_row(80, 5, 10, 0), Some(5));
    assert_eq!(feed.line_at_visual_row(80, 5, 10, 9), Some(14));
}

#[test]
fn selected_text_extracts_lines() {
    let mut feed = Feed::new();
    feed.push_line(BlockStyle::Plain, "alpha");
    feed.push_line(BlockStyle::Plain, "beta");
    feed.push_line(BlockStyle::Plain, "gamma");
    let text = feed.selected_text(80, 0, 2);
    assert_eq!(text.as_deref(), Some("alpha\nbeta\ngamma"));
}

#[test]
fn selected_text_reversed_range() {
    let mut feed = Feed::new();
    feed.push_line(BlockStyle::Plain, "alpha");
    feed.push_line(BlockStyle::Plain, "beta");
    let text = feed.selected_text(80, 1, 0);
    assert_eq!(text.as_deref(), Some("alpha\nbeta"));
}

#[test]
fn append_to_last_extends_block() {
    let mut feed = Feed::new();
    feed.push_block(BlockStyle::Agent, "hello");
    assert!(feed.append_to_last(" world"));
    let lines = feed.lines(80);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].text.contains("hello world"));
}

#[test]
fn append_to_last_returns_false_when_empty() {
    let mut feed = Feed::new();
    assert!(!feed.append_to_last("orphan"));
}

#[test]
fn replace_last_updates_final_block() {
    let mut feed = Feed::new();
    feed.push_line(BlockStyle::Plain, "first");
    feed.push_line(BlockStyle::Plain, "second");
    feed.replace_last(BlockStyle::Agent, "replaced");
    let lines = feed.lines(80);
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0].text, "first");
    // The replacement opens an agent section, so the layout separates it.
    assert_eq!(lines[1].text, "");
    assert_eq!(lines[2].text, "< replaced");
}

#[test]
fn replace_last_pushes_when_empty() {
    let mut feed = Feed::new();
    feed.replace_last(BlockStyle::Agent, "only");
    let lines = feed.lines(80);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].text, "< only");
}

#[test]
fn truncate_blocks_keeps_prefix() {
    let mut feed = Feed::new();
    feed.push_line(BlockStyle::Plain, "first");
    feed.push_line(BlockStyle::Plain, "second");
    feed.push_line(BlockStyle::Plain, "third");
    feed.truncate_blocks(2);
    assert_eq!(feed.block_count(), 2);
    let lines = feed.lines(80);
    assert_eq!(lines.len(), 2);
}

// ── section spacing ─────────────────────────────────────────────────
//
// Agent replies and reasoning are separated from whatever precedes them by
// exactly one blank row, owned by the layout rather than written by callers.

#[test]
fn agent_after_tool_result_gets_exactly_one_blank() {
    let mut feed = Feed::new();
    feed.push_line(BlockStyle::ToolResult, "◈ result (2 chars):\nok");
    feed.push_line(BlockStyle::Agent, "hi");
    let lines = feed.lines(80);
    assert_eq!(
        lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(),
        ["◈ result (2 chars):", "ok", "", "< hi"]
    );
}

#[test]
fn reasoning_after_tool_result_gets_one_blank() {
    let mut feed = Feed::new();
    feed.push_line(BlockStyle::ToolResult, "◈ result (2 chars):\nok");
    feed.push_line(BlockStyle::Reasoning, "< thinking");
    let lines = feed.lines(80);
    assert_eq!(
        lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(),
        ["◈ result (2 chars):", "ok", "", "< thinking"]
    );
}

#[test]
fn no_leading_blank_before_first_section() {
    let mut feed = Feed::new();
    feed.push_line(BlockStyle::Agent, "hi");
    let lines = feed.lines(80);
    assert_eq!(lines[0].text, "< hi");
}

#[test]
fn callers_explicit_blank_is_not_doubled() {
    let mut feed = Feed::new();
    feed.push_line(BlockStyle::ToolResult, "ok");
    feed.push_line(BlockStyle::Plain, "");
    feed.push_line(BlockStyle::Agent, "hi");
    let lines = feed.lines(80);
    assert_eq!(
        lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(),
        ["ok", "", "< hi"]
    );
}

#[test]
fn consecutive_agent_blocks_are_separated() {
    let mut feed = Feed::new();
    feed.push_line(BlockStyle::Agent, "one");
    feed.push_line(BlockStyle::Agent, "two");
    let lines = feed.lines(80);
    assert_eq!(
        lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(),
        ["< one", "", "< two"]
    );
}

/// A tool result belongs to the call above it: it must not be split off as a
/// section of its own, nor the surrounding calls from each other.
#[test]
fn tool_call_and_result_stay_together() {
    let mut feed = Feed::new();
    feed.push_line(BlockStyle::Tool, "◈ read \"x\"");
    feed.push_line(BlockStyle::ToolResult, "◈ result (2 chars):\nok");
    feed.push_line(BlockStyle::Tool, "◈ read \"y\"");
    let lines = feed.lines(80);
    assert_eq!(
        lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(),
        ["◈ read \"x\"", "◈ result (2 chars):", "ok", "◈ read \"y\""]
    );
}

/// The separator is part of the laid-out rows, so scrolling and selection see
/// the same spacing the viewport draws.
#[test]
fn section_separator_is_visible_to_scroll_queries() {
    let mut feed = Feed::new();
    feed.push_line(BlockStyle::ToolResult, "ok");
    feed.push_line(BlockStyle::Agent, "hi");
    assert_eq!(feed.line_count(80), 3);
    assert_eq!(feed.selected_text(80, 0, 2).as_deref(), Some("ok\n\n< hi"));
}

#[test]
fn clear_empties_feed() {
    let mut feed = Feed::new();
    feed.push_line(BlockStyle::Plain, "hello");
    feed.clear();
    assert!(feed.is_empty());
    assert_eq!(feed.line_count(80), 0);
}

#[test]
fn generation_starts_at_zero() {
    let feed = Feed::new();
    assert_eq!(feed.generation(), 0);
}

#[test]
fn generation_bumps_on_each_mutator() {
    let mut feed = Feed::new();
    feed.push_block(BlockStyle::Plain, "one");
    assert_eq!(feed.generation(), 1);
    feed.push_line(BlockStyle::Plain, "two");
    assert_eq!(feed.generation(), 2);
    assert!(feed.append_to_last(" more"));
    assert_eq!(feed.generation(), 3);
    feed.replace_last(BlockStyle::Agent, "replaced");
    assert_eq!(feed.generation(), 4);
    feed.truncate_blocks(1);
    assert_eq!(feed.generation(), 5);
    feed.clear();
    assert_eq!(feed.generation(), 6);
}

#[test]
fn generation_not_bumped_by_failed_append() {
    let mut feed = Feed::new();
    assert!(!feed.append_to_last("orphan"));
    assert_eq!(feed.generation(), 0);
}

#[test]
fn generation_not_bumped_by_reads() {
    let mut feed = Feed::new();
    feed.push_line(BlockStyle::Plain, "one");
    let before = feed.generation();
    let _ = feed.lines(80);
    let _ = feed.line_count(80);
    let _ = feed.visible_range(80, 0, 10);
    let _ = feed.line_at_visual_row(80, 0, 10, 0);
    let _ = feed.selected_text(80, 0, 0);
    let _ = feed.is_empty();
    let _ = feed.block_count();
    assert_eq!(feed.generation(), before);
}

#[test]
fn running_agent_block_renders_tail_as_plain_text() {
    let mut feed = Feed::new();
    feed.push_streaming_block(BlockStyle::Agent);
    assert!(feed.append_to_last("hello **wor"));
    let lines = feed.lines(80);
    assert_eq!(lines.len(), 1);
    // No markdown parsing while the line is unfinished: markers stay literal.
    assert_eq!(lines[0].text, "< hello **wor");
    assert_eq!(lines[0].color, Color::White);
}

#[test]
fn running_agent_block_parses_only_completed_lines() {
    let mut feed = Feed::new();
    feed.push_streaming_block(BlockStyle::Agent);
    assert!(feed.append_to_last("first **bold**\nsecond **par"));
    let lines = feed.lines(80);
    assert_eq!(lines.len(), 2);
    // The completed line is parsed as markdown: bold markers are gone.
    assert_eq!(lines[0].text, "< first bold");
    // The unfinished tail line stays plain: markers remain literal.
    assert_eq!(lines[1].text, "second **par");
}

#[test]
fn running_agent_block_appends_grow_tail() {
    let mut feed = Feed::new();
    feed.push_streaming_block(BlockStyle::Agent);
    assert!(feed.append_to_last("hello"));
    assert!(feed.append_to_last(" world"));
    let lines = feed.lines(80);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].text, "< hello world");
}

#[test]
fn finalize_last_parses_full_text() {
    let mut feed = Feed::new();
    feed.push_streaming_block(BlockStyle::Agent);
    assert!(feed.append_to_last("hello **world**"));
    feed.finalize_last();
    let lines = feed.lines(80);
    assert_eq!(lines.len(), 1);
    // After finalizing, the former tail line is parsed as markdown.
    assert_eq!(lines[0].text, "< hello world");
}

#[test]
fn finalize_last_bumps_generation_once() {
    let mut feed = Feed::new();
    feed.push_streaming_block(BlockStyle::Agent);
    let before = feed.generation();
    feed.finalize_last();
    assert_eq!(feed.generation(), before + 1);
    // Second call is a no-op: the block is no longer running.
    feed.finalize_last();
    assert_eq!(feed.generation(), before + 1);
}

#[test]
fn finalize_last_on_complete_block_is_noop() {
    let mut feed = Feed::new();
    feed.push_block(BlockStyle::Agent, "done");
    let before = feed.generation();
    feed.finalize_last();
    assert_eq!(feed.generation(), before);
}

#[test]
fn replace_last_invalidates_cached_layout() {
    let mut feed = Feed::new();
    feed.push_block(BlockStyle::Agent, "aaaa **old**");
    let _ = feed.lines(80); // populate the layout cache
    // Same length, different content: the cached layout must not leak through.
    feed.replace_last(BlockStyle::Agent, "bbbb **new**");
    let lines = feed.lines(80);
    let joined: String = lines
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join("");
    assert!(joined.contains("new"), "expected new content: {joined}");
    assert!(!joined.contains("old"), "stale cached content: {joined}");
}

#[test]
fn agent_layout_recomputes_on_width_change() {
    let mut feed = Feed::new();
    feed.push_block(
        BlockStyle::Agent,
        "one two three four five six seven eight nine ten eleven twelve",
    );
    let wide = feed.lines(120);
    let narrow = feed.lines(20);
    assert!(
        narrow.len() > wide.len(),
        "narrow width should wrap into more lines: {} vs {}",
        narrow.len(),
        wide.len()
    );
}

#[test]
fn scroll_and_selection_queries_reuse_prewrapped_rows() {
    let mut feed = Feed::new();
    feed.push_line(BlockStyle::Plain, "hello");
    let _ = feed.lines(80);
    let _ = feed.line_count(80);
    let _ = feed.visible_range(80, 0, 10);
    let _ = feed.selected_text(80, 0, 0);
    let _ = feed.line_at_visual_row(80, 0, 10, 9);
    assert_eq!(
        feed.layout_computes(),
        1,
        "scroll/selection queries should reuse the pre-wrapped rows"
    );

    feed.push_line(BlockStyle::Plain, "world");
    let _ = feed.lines(80);
    assert_eq!(feed.layout_computes(), 2, "mutation should invalidate");

    let _ = feed.lines(40);
    assert_eq!(feed.layout_computes(), 3, "resize should invalidate");

    // Alternating back to a previously seen width still re-lays out once
    // (single-slot cache), then reuses.
    let _ = feed.lines(80);
    let _ = feed.lines(80);
    assert_eq!(feed.layout_computes(), 4);
}
