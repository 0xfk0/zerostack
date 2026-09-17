//! The transcript summary for a `read` tool call, which appends the line range
//! the model asked for (`src/ui/utils.rs`).

use crate::ui::utils::format_tool_call_summary;
use serde_json::json;

fn read_summary(args: serde_json::Value) -> String {
    format_tool_call_summary("read", &args)
}

#[test]
fn read_without_a_range_shows_only_the_path() {
    assert_eq!(
        read_summary(json!({ "path": "src/main.rs" })),
        r#"read "src/main.rs""#
    );
}

#[test]
fn read_with_offset_and_limit_shows_the_inclusive_range() {
    assert_eq!(
        read_summary(json!({ "path": "f.rs", "offset": 100, "limit": 101 })),
        r#"read "f.rs" 100-200"#
    );
}

#[test]
fn read_limit_counts_lines_from_the_offset() {
    assert_eq!(
        read_summary(json!({ "path": "f.rs", "offset": 10, "limit": 5 })),
        r#"read "f.rs" 10-14"#
    );
}

#[test]
fn read_with_offset_only_is_open_ended() {
    assert_eq!(
        read_summary(json!({ "path": "f.rs", "offset": 100 })),
        r#"read "f.rs" 100-..."#
    );
}

#[test]
fn read_with_limit_only_starts_at_line_one() {
    assert_eq!(
        read_summary(json!({ "path": "f.rs", "limit": 200 })),
        r#"read "f.rs" 1-200"#
    );
}

#[test]
fn read_offset_zero_is_the_first_line() {
    assert_eq!(
        read_summary(json!({ "path": "f.rs", "offset": 0 })),
        r#"read "f.rs" 1-..."#
    );
}

#[test]
fn read_zero_limit_is_not_an_end_line() {
    assert_eq!(
        read_summary(json!({ "path": "f.rs", "offset": 100, "limit": 0 })),
        r#"read "f.rs" 100-..."#
    );
}

#[test]
fn read_without_a_path_falls_back_to_the_tool_name() {
    assert_eq!(read_summary(json!({ "offset": 3 })), "read");
}

#[test]
fn other_tools_are_unaffected() {
    assert_eq!(
        format_tool_call_summary("write", &json!({ "path": "f.rs", "content": "x" })),
        r#"write "f.rs""#
    );
    assert_eq!(
        format_tool_call_summary("bash", &json!({ "command": "ls -la" })),
        "bash ls -la"
    );
}
