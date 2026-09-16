use std::io::Write;

use crossterm::QueueableCommand;
use crossterm::cursor::Show;
use crossterm::event::{
    DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
    EnableFocusChange, EnableMouseCapture, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
    PushKeyboardEnhancementFlags,
};
use crossterm::terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};

/// RAII guard that suspends the TUI (leaves alternate screen, disables raw mode)
/// and restores it on drop. Ensures the alternate screen is re-entered and raw mode
/// re-enabled even if the editor exits abnormally.
pub struct SuspendGuard {
    mouse_capture: bool,
}

impl SuspendGuard {
    pub fn suspend(mouse_capture: bool) -> Self {
        let _ = terminal::disable_raw_mode();
        let mut stdout = std::io::stdout();
        let _ = write_suspend(&mut stdout, mouse_capture);
        let _ = stdout.flush();
        Self { mouse_capture }
    }
}

impl Drop for SuspendGuard {
    fn drop(&mut self) {
        let mut stdout = std::io::stdout();
        let _ = write_resume(&mut stdout, self.mouse_capture);
        let _ = terminal::enable_raw_mode();
        let _ = stdout.flush();
    }
}

/// Emit the sequences that hand the terminal back to the shell (or a child
/// program such as `$EDITOR`). Exact reverse of [`write_resume`].
///
/// Two things matter beyond leaving the alternate screen. First, undo every
/// input mode [`TerminalGuard::new`] turned on — bracketed paste, focus change,
/// and the kitty keyboard protocol. A plain editor does not understand those
/// escape sequences and renders them as garbage when they arrive mid-session,
/// and the shell would inherit them too. Second, emit `Show`: crossterm's
/// alternate screen restores only the cursor *position* (`?1049`), never its
/// visibility, so the caret the renderer hid before suspending would otherwise
/// persist onto the shell prompt (invisible cursor).
pub(crate) fn write_suspend(w: &mut impl Write, mouse_capture: bool) -> std::io::Result<()> {
    w.queue(PopKeyboardEnhancementFlags)?;
    w.queue(DisableBracketedPaste)?;
    w.queue(DisableFocusChange)?;
    if mouse_capture {
        w.queue(DisableMouseCapture)?;
    }
    w.queue(Show)?;
    w.queue(LeaveAlternateScreen)?;
    Ok(())
}

/// Re-enter the TUI after [`write_suspend`]: alternate screen, then the input
/// modes [`TerminalGuard::new`] enables, in the same order. Raw mode is toggled
/// separately by the caller (it is a `termios` call, not a queued sequence).
pub(crate) fn write_resume(w: &mut impl Write, mouse_capture: bool) -> std::io::Result<()> {
    w.queue(EnterAlternateScreen)?;
    w.queue(Clear(ClearType::All))?;
    if mouse_capture {
        w.queue(EnableMouseCapture)?;
    }
    w.queue(EnableBracketedPaste)?;
    w.queue(EnableFocusChange)?;
    w.queue(PushKeyboardEnhancementFlags(
        KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES,
    ))?;
    Ok(())
}

/// Suspend the TUI, run `f`, then restore it. Single shared helper to avoid
/// copy-pasted `disable_raw_mode` / `LeaveAlternateScreen` sequences.
pub fn suspend_tui<F, R>(mouse_capture: bool, f: F) -> R
where
    F: FnOnce() -> R,
{
    let _guard = SuspendGuard::suspend(mouse_capture);
    f()
}

/// Stop the current process with `SIGTSTP` (Ctrl-Z job control). The kernel
/// stops the whole process; resumption arrives later as `SIGCONT`. Must be
/// called with the terminal restored to cooked mode (see [`suspend_tui`]),
/// otherwise the shell prompt would inherit raw mode.
///
/// The single `unsafe` in the TUI: `raise` only delivers a constant,
/// well-formed signal number to the calling process.
#[allow(unsafe_code)]
pub fn raise_sigtstp() {
    unsafe {
        libc::raise(libc::SIGTSTP);
    }
}

/// Split an editor command string (e.g. `"code --wait"`) into program + args,
/// respecting shell quoting. Falls back to treating the whole string as a single
/// program name if splitting fails or yields empty.
pub fn parse_editor_command(editor: &str) -> (String, Vec<String>) {
    match shell_words::split(editor) {
        Ok(mut parts) if !parts.is_empty() => {
            let prog = parts.remove(0);
            (prog, parts)
        }
        _ => (editor.to_string(), Vec::new()),
    }
}

pub struct TerminalGuard {
    mouse_capture: bool,
}

impl TerminalGuard {
    pub fn new(mouse_capture: bool) -> std::io::Result<Self> {
        let mut stdout = std::io::stdout();
        write_resume(&mut stdout, mouse_capture)?;
        stdout.flush()?;
        terminal::enable_raw_mode()?;
        Ok(TerminalGuard { mouse_capture })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let mut stdout = std::io::stdout();
        let _ = write_suspend(&mut stdout, self.mouse_capture);
        let _ = stdout.flush();
    }
}
