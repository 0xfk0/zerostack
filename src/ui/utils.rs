use crossterm::style::Color;
use unicode_width::UnicodeWidthStr;

use crate::extras::truncate::truncate_cjk;

const TOOL_SUMMARY_MAX: usize = 200;

fn display_value(val: &str) -> String {
    if val.len() <= TOOL_SUMMARY_MAX {
        format!("\"{}\"", val)
    } else {
        format!("\"{}\"", truncate_cjk(val, TOOL_SUMMARY_MAX, "..."))
    }
}

/// Returns the display width of a string in terminal columns.
/// CJK characters typically occupy 2 columns; ASCII occupies 1.
#[inline]
pub(crate) fn display_width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

/// Returns the display width of a single character.
#[inline]
pub(crate) fn char_display_width(c: char) -> usize {
    unicode_width::UnicodeWidthChar::width(c).unwrap_or(0)
}

/// Splits `s` at byte index `byte`, clamping back to the nearest char
/// boundary at or before it. Cursors in the input editor and pickers are
/// byte offsets kept on char boundaries by construction, so the clamp is
/// only defense for offsets derived elsewhere.
pub(crate) fn split_at_byte(s: &str, byte: usize) -> (&str, &str) {
    let mut i = byte.min(s.len());
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    (&s[..i], &s[i..])
}

/// Longest prefix of `s` that fits in `max_cols` terminal columns (a wide
/// char that would overflow is not included).
pub(crate) fn take_display_width(s: &str, max_cols: usize) -> &str {
    let mut cols = 0usize;
    for (i, c) in s.char_indices() {
        let w = char_display_width(c);
        if cols + w > max_cols {
            return &s[..i];
        }
        cols += w;
    }
    s
}

/// Resolves a color based on monochrome mode.
#[inline]
pub(crate) fn resolve_color(color: Color, monochrome: bool) -> Color {
    if monochrome {
        let _ = color;
        Color::Reset
    } else {
        color
    }
}

/// Converts an RGB color to the nearest ANSI 256 color index (16-255).
fn rgb_to_ansi256(r: u8, g: u8, b: u8) -> u8 {
    let r = r as f32;
    let g = g as f32;
    let b = b as f32;
    // Check if grayscale: all channels within ~10% of each other
    let mean = (r + g + b) / 3.0;
    let spread = (r - mean).abs().max((g - mean).abs()).max((b - mean).abs());
    if spread < 15.0 {
        // 24 grayscale steps from 232-255
        let gs = ((mean / 255.0) * 23.0).round() as u8;
        return 232 + gs.min(23);
    }
    // 216-color cube: 6 levels per channel (0, 95, 135, 175, 215, 255)
    let levels: [f32; 6] = [0.0, 95.0, 135.0, 175.0, 215.0, 255.0];
    let nearest = |v: f32| -> u8 {
        let mut best = 0u8;
        let mut best_dist = f32::MAX;
        for (i, &l) in levels.iter().enumerate() {
            let dist = (l - v).abs();
            if dist < best_dist {
                best_dist = dist;
                best = i as u8;
            }
        }
        best
    };
    let ri = nearest(r);
    let gi = nearest(g);
    let bi = nearest(b);
    16 + 36 * ri + 6 * gi + bi
}

/// Converts any Color to its nearest ANSI 256-color equivalent.
pub(crate) fn to_ansi_256(color: Color) -> Color {
    match color {
        Color::Reset => Color::Reset,
        Color::Black => Color::AnsiValue(0),
        Color::Red => Color::AnsiValue(1),
        Color::Green => Color::AnsiValue(2),
        Color::Yellow => Color::AnsiValue(3),
        Color::Blue => Color::AnsiValue(4),
        Color::Magenta => Color::AnsiValue(5),
        Color::Cyan => Color::AnsiValue(6),
        Color::White => Color::AnsiValue(7),
        Color::Grey => Color::AnsiValue(7),
        Color::DarkGrey => Color::AnsiValue(8),
        Color::DarkRed => Color::AnsiValue(9),
        Color::DarkGreen => Color::AnsiValue(10),
        Color::DarkYellow => Color::AnsiValue(11),
        Color::DarkBlue => Color::AnsiValue(12),
        Color::DarkMagenta => Color::AnsiValue(13),
        Color::DarkCyan => Color::AnsiValue(14),
        Color::Rgb { r, g, b } => Color::AnsiValue(rgb_to_ansi256(r, g, b)),
        Color::AnsiValue(v) => Color::AnsiValue(v),
    }
}

/// Parses a color name or hex string into a crossterm Color.
pub(crate) fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim().to_lowercase();
    match s.as_str() {
        "reset" => Some(Color::Reset),
        "black" => Some(Color::Black),
        "dark_grey" | "darkgrey" | "dark_gray" | "darkgray" => Some(Color::DarkGrey),
        "red" => Some(Color::Red),
        "dark_red" | "darkred" => Some(Color::DarkRed),
        "green" => Some(Color::Green),
        "dark_green" | "darkgreen" => Some(Color::DarkGreen),
        "yellow" => Some(Color::Yellow),
        "dark_yellow" | "darkyellow" => Some(Color::DarkYellow),
        "blue" => Some(Color::Blue),
        "light_blue" | "lightblue" => Some(Color::Rgb {
            r: 0x5f,
            g: 0xaf,
            b: 0xff,
        }),
        "dark_blue" | "darkblue" => Some(Color::DarkBlue),
        "magenta" => Some(Color::Magenta),
        "dark_magenta" | "darkmagenta" => Some(Color::DarkMagenta),
        "cyan" => Some(Color::Cyan),
        "dark_cyan" | "darkcyan" => Some(Color::DarkCyan),
        "white" => Some(Color::White),
        "grey" | "gray" => Some(Color::Grey),
        _ => {
            if let Some(hex) = s.strip_prefix('#')
                && hex.len() == 6
                && let (Ok(r), Ok(g), Ok(b)) = (
                    u8::from_str_radix(&hex[0..2], 16),
                    u8::from_str_radix(&hex[2..4], 16),
                    u8::from_str_radix(&hex[4..6], 16),
                )
            {
                return Some(Color::Rgb { r, g, b });
            }
            None
        }
    }
}

/// Formats a tool call showing only the primary file/command parameter.
pub(crate) fn format_tool_call_summary(name: &str, args: &serde_json::Value) -> String {
    let obj = match args {
        serde_json::Value::Object(map) => map,
        _ => return name.to_string(),
    };

    if name == "task" {
        return format_task_summary(obj);
    }

    if name == "read" {
        return format_read_summary(obj);
    }

    let primary_keys: &[&str] = match name {
        "write" | "edit" | "list_dir" => &["path"],
        "grep" => &["pattern", "path"],
        "find_files" => &["pattern"],
        "bash" => &["command"],
        _ => &[],
    };

    let mut shown = Vec::new();
    for key in primary_keys {
        if let Some(serde_json::Value::String(val)) = obj.get(*key) {
            let display_val = if name == "bash" {
                val.clone()
            } else {
                display_value(val)
            };
            shown.push(display_val);
        }
    }

    if shown.is_empty() {
        if let Some((_, serde_json::Value::String(val))) = obj.iter().next() {
            format!("{} {}", name, display_value(val))
        } else {
            name.to_string()
        }
    } else {
        format!("{} {}", name, shown.join(" "))
    }
}

/// `read` shows the file plus the line range the model asked for:
/// `read "f" 100-200`, `read "f" 100-...` when it gave no `limit`, and a bare
/// `read "f"` when it gave neither.
fn format_read_summary(obj: &serde_json::Map<String, serde_json::Value>) -> String {
    let path = match obj.get("path") {
        Some(serde_json::Value::String(path)) => display_value(path),
        _ => return "read".to_string(),
    };
    match read_line_range(obj) {
        Some(range) => format!("read {} {}", path, range),
        None => format!("read {}", path),
    }
}

/// The `read` tool takes a 1-indexed `offset` and a line `limit`; render the
/// inclusive range they select. `None` when the model gave neither, so an
/// unrestricted read stays unadorned.
fn read_line_range(obj: &serde_json::Map<String, serde_json::Value>) -> Option<String> {
    let offset = obj.get("offset").and_then(serde_json::Value::as_u64);
    let limit = obj
        .get("limit")
        .and_then(serde_json::Value::as_u64)
        .filter(|limit| *limit > 0);
    if offset.is_none() && limit.is_none() {
        return None;
    }
    // The tool clamps a missing/zero offset to the first line, so mirror that
    // rather than printing a `0-` range.
    let start = offset.unwrap_or(1).max(1);
    Some(match limit {
        Some(limit) => format!("{}-{}", start, start + limit - 1),
        None => format!("{}-...", start),
    })
}

fn format_task_summary(obj: &serde_json::Map<String, serde_json::Value>) -> String {
    let prompts = match obj.get("prompts") {
        Some(serde_json::Value::Array(arr)) => arr,
        _ => return "task".to_string(),
    };
    let parts: Vec<String> = prompts
        .iter()
        .filter_map(|v| v.as_str())
        .map(display_value)
        .collect();
    if parts.is_empty() {
        "task".to_string()
    } else {
        format!("task {}", parts.join(" "))
    }
}

/// Suggests a permission allow pattern for a tool+input combination.
pub(crate) fn suggest_pattern(tool: &str, input: &str) -> String {
    match tool {
        "bash" => {
            let first = input.split_whitespace().next().unwrap_or("*");
            format!("{} *", first)
        }
        "read" | "write" | "edit" | "list_dir" => {
            let expanded = crate::fs::expand_tilde(input);
            let path = std::path::Path::new(&expanded);
            let parent = path
                .parent()
                .map(|p| p.to_string_lossy())
                .unwrap_or(std::borrow::Cow::Borrowed("*"));
            if parent.is_empty() {
                "**".to_string()
            } else {
                format!("{}/**/*", parent)
            }
        }
        "grep" | "find_files" => {
            let first = input.split_whitespace().next().unwrap_or("*");
            format!("{}*", first)
        }
        _ => "*".to_string(),
    }
}
