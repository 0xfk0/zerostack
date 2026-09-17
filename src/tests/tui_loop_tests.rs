//! Integration tests for the TUI main loop (`ui::app::App`), run headless:
//!
//! - **Fake `UserEvent` source**: no crossterm event thread; tests inject
//!   events straight into the loop's channel via `App::inject`.
//! - **Fake renderer backend**: `renderer::FakeBackend` pins a fixed 80x24
//!   geometry and captures every emitted frame, so nothing touches a real
//!   terminal (no raw mode, no alternate screen).
//! - **Scripted agent**: `AnyAgent::Mock` wraps rig's `MockCompletionModel`
//!   (see `fake_model.rs`), so a full user-prompt → agent-response round trip
//!   runs with no network.
//!
//! The loop is pumped one iteration at a time via `App::step`, which makes
//! assertions between event dispatch deterministic: on the current-thread
//! runtime used by `#[tokio::test]`, injected user events are always drained
//! before the freshly spawned agent task is first polled.

// Every test holds `acquire()`'s guard across its awaits on purpose: the lock
// serializes tests that mutate process-global state, so it has to cover the
// whole test body. The lint guards against deadlock from contending tasks,
// which cannot happen here (single-threaded `#[tokio::test]`, and the guard is
// the only thing keeping these tests from racing each other).
#![allow(clippy::await_holding_lock)]

use std::collections::HashMap;
use std::ops::ControlFlow;
use std::sync::{Mutex, MutexGuard, OnceLock};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use rig::completion::Usage;
use rig::tool::Tool;
use serde::Deserialize;

use crate::agent::tools::ToolError;
use crate::cli::Cli;
use crate::config::Config;
use crate::context::ContextFiles;
use crate::event::UserEvent;
use crate::provider::AnyAgent;
use crate::sandbox::Sandbox;
use crate::session::{MessageRole, Session};
use crate::tests::fake_model::{self, FakeModel, MockCompletionModel, MockStreamEvent};
use crate::ui::app::App;
use crate::ui::renderer::FakeBackend;
use crate::ui::state::UiContext;

/// Serializes every test in this file: they share process-global state
/// (`ZS_DATA_DIR`/`ZS_CONFIG_DIR` env, the statusline `OnceLock`, the subagent
/// event-sender singleton set by `runner::spawn_agent`, the model cache).
static LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn acquire() -> MutexGuard<'static, ()> {
    let lock = LOCK.get_or_init(|| Mutex::new(()));
    lock.lock().unwrap_or_else(|e| e.into_inner())
}

fn isolate_data_dirs() {
    let dir = std::env::temp_dir().join(format!("zerostack-tui-loop-tests-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    unsafe { std::env::set_var("ZS_DATA_DIR", &dir) };
    unsafe { std::env::set_var("ZS_CONFIG_DIR", &dir) };
}

/// Build a headless `App` with a scripted mock agent. `turns` scripts one
/// model response per agent run (plain text chunks per turn). Returns the app
/// and the model (for inspecting the requests the loop sent).
async fn headless_app(turns: Vec<Vec<&str>>) -> (App<'static>, FakeModel) {
    let model = fake_model::text_turns(turns);
    let agent = AnyAgent::Mock(rig::agent::AgentBuilder::new(model.clone()).build());
    let app = headless_app_with_agent(agent).await;
    (app, model)
}

/// Build a headless `App` around a caller-supplied mock agent. Split out from
/// [`headless_app`] so a test can install a tool-equipped agent (needed to
/// script a multi-call run, where the usage accounting bug surfaced).
async fn headless_app_with_agent(agent: AnyAgent) -> App<'static> {
    headless_app_with_cfg(agent, Config::default()).await
}

/// Same as [`headless_app_with_agent`], with a caller-supplied config (e.g. to
/// turn on `multiline_prompt`).
async fn headless_app_with_cfg(agent: AnyAgent, cfg: Config) -> App<'static> {
    isolate_data_dirs();
    let cli: &'static Cli = Box::leak(Box::new(Cli {
        api_key: Some("test-key".to_string()),
        no_session: true,
        no_color: true,
        ..Default::default()
    }));
    let cfg: &'static Config = Box::leak(Box::new(cfg));
    let session: &'static mut Session = Box::leak(Box::new(Session::new(
        "anthropic",
        "claude-sonnet-4-5",
        200_000,
        "tui-loop-test",
    )));
    let context: &'static mut ContextFiles =
        Box::leak(Box::new(crate::context::load_with_prompts_dirs(true, &[])));
    let client =
        crate::provider::create_client("anthropic", Some("test-key"), &HashMap::new(), None)
            .expect("create test client");
    let ui = UiContext::new(
        cli,
        cfg,
        session,
        context,
        client,
        None,
        None,
        Sandbox::new(false, "bwrap"),
        None,
    );
    App::new_headless(
        ui,
        Some(agent),
        None,
        None,
        Box::new(FakeBackend::new(80, 24)),
    )
    .await
    .expect("build headless app")
}

/// Like [`headless_app`], with a caller-supplied config.
async fn headless_app_cfg(turns: Vec<Vec<&str>>, cfg: Config) -> App<'static> {
    let model = fake_model::text_turns(turns);
    let agent = AnyAgent::Mock(rig::agent::AgentBuilder::new(model).build());
    headless_app_with_cfg(agent, cfg).await
}

/// Pump one step and report whether the loop asked to exit (`Break`).
async fn step_broke(app: &mut App<'static>) -> bool {
    match tokio::time::timeout(std::time::Duration::from_millis(250), app.step()).await {
        Ok(Ok(cf)) => matches!(cf, ControlFlow::Break(())),
        Ok(Err(e)) => panic!("step failed: {e}"),
        Err(_) => false,
    }
}

fn char_key(c: char) -> UserEvent {
    UserEvent::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE))
}

fn enter_key() -> UserEvent {
    UserEvent::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
}

fn ctrl_d() -> UserEvent {
    UserEvent::Key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL))
}

fn left_key() -> UserEvent {
    UserEvent::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE))
}

fn home_key() -> UserEvent {
    UserEvent::Key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE))
}

fn end_key() -> UserEvent {
    UserEvent::Key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE))
}

fn ctrl_j_key() -> UserEvent {
    UserEvent::Key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL))
}

fn ctrl_home_key() -> UserEvent {
    UserEvent::Key(KeyEvent::new(KeyCode::Home, KeyModifiers::CONTROL))
}

fn ctrl_end_key() -> UserEvent {
    UserEvent::Key(KeyEvent::new(KeyCode::End, KeyModifiers::CONTROL))
}

fn ctrl_c() -> UserEvent {
    UserEvent::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL))
}

fn ctrl_z() -> UserEvent {
    UserEvent::Key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL))
}

fn ctrl_l() -> UserEvent {
    UserEvent::Key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::CONTROL))
}

fn ctrl_g() -> UserEvent {
    UserEvent::Key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL))
}

fn ctrl_t() -> UserEvent {
    UserEvent::Key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL))
}

/// Type `text` into the input editor and submit it with Enter.
async fn type_and_submit(app: &App<'static>, text: &str) {
    for c in text.chars() {
        app.inject(char_key(c)).await;
    }
    app.inject(enter_key()).await;
}

/// Pump one iteration, treating an idle (event-less) loop as a no-op instead
/// of blocking forever: `App::step`'s `select!` parks when no branch is ready,
/// which is correct in production (the event thread always feeds it) but would
/// hang a test whose condition can't be reached.
async fn pump(app: &mut App<'static>) {
    if let Ok(result) =
        tokio::time::timeout(std::time::Duration::from_millis(250), app.step()).await
    {
        let _ = result.expect("step failed");
    }
    // Err(_) = idle timeout: no events pending, nothing running.
}

/// Pump the loop until `done` holds, bounded so a stuck loop fails instead of
/// hanging the suite.
async fn step_until(app: &mut App<'static>, mut done: impl FnMut(&App<'static>) -> bool) {
    const MAX_STEPS: usize = 300;
    for _ in 0..MAX_STEPS {
        if done(app) {
            return;
        }
        pump(app).await;
    }
    assert!(done(app), "condition not met within {MAX_STEPS} steps");
}

#[tokio::test]
async fn submit_prompt_streams_response_and_updates_session() {
    let _guard = acquire();
    let (mut app, _model) = headless_app(vec![vec!["hi there"]]).await;

    type_and_submit(&app, "hello").await;
    step_until(&mut app, |a| a.is_running()).await;
    step_until(&mut app, |a| !a.is_running()).await;

    let messages = &app.session().messages;
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].role, MessageRole::User);
    assert_eq!(messages[0].content.as_str(), "hello");
    assert_eq!(messages[1].role, MessageRole::Assistant);
    assert_eq!(messages[1].content.as_str(), "hi there");

    let feed = app.feed_text();
    assert!(
        feed.contains("> hello"),
        "feed should echo the prompt: {feed}"
    );
    assert!(
        feed.contains("hi there"),
        "feed should show the response: {feed}"
    );

    app.teardown().await;
}

#[tokio::test]
async fn spinner_cleared_when_run_finishes() {
    let _guard = acquire();
    let (mut app, _model) = headless_app(vec![vec!["done"]]).await;

    type_and_submit(&app, "hi").await;
    step_until(&mut app, |a| a.is_running()).await;

    // Positive control: confirm a spinner was actually painted mid-run, so the
    // idle check below is a real "spinner then cleared" regression rather than
    // a test that never saw a spinner at all.
    let mid = app.backend_output();
    assert!(
        mid.chars()
            .any(|c| ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'].contains(&c)),
        "a spinner frame should be painted while running"
    );

    // Regression: Done/Error set is_running false without repainting the bottom
    // row, and the 100ms refresh is gated on is_running, so the spinner froze on
    // screen until the next keypress. The last bottom draw must now be idle.
    step_until(&mut app, |a| !a.is_running()).await;
    let snap = app
        .last_bottom_snapshot()
        .expect("bottom should have been drawn at least once");
    assert!(
        !snap.is_running,
        "last bottom draw should be idle after the run finishes"
    );
    assert!(
        snap.prompt == crate::ui::renderer::PromptSnapshot::Input,
        "last bottom draw should show the input prompt after the run finishes"
    );

    app.teardown().await;
}

#[tokio::test]
async fn queued_input_replays_after_current_run() {
    let _guard = acquire();
    let (mut app, model) = headless_app(vec![vec!["answer one"], vec!["answer two"]]).await;

    // Inject everything up front: the injected events are drained before the
    // freshly spawned agent task is ever polled, so "second" is submitted
    // while the first run is still active and must be queued.
    type_and_submit(&app, "first").await;
    type_and_submit(&app, "second").await;

    step_until(&mut app, |a| a.feed_text().contains("queued: second")).await;
    step_until(&mut app, |a| {
        a.is_running() && a.session().messages.len() >= 3
    })
    .await;
    step_until(&mut app, |a| !a.is_running()).await;

    assert_eq!(
        model.requests().len(),
        2,
        "both turns should have reached the model"
    );
    let messages = &app.session().messages;
    let roles: Vec<(MessageRole, &str)> = messages
        .iter()
        .map(|m| (m.role, m.content.as_str()))
        .collect();
    assert_eq!(
        roles,
        vec![
            (MessageRole::User, "first"),
            (MessageRole::Assistant, "answer one"),
            (MessageRole::User, "second"),
            (MessageRole::Assistant, "answer two"),
        ]
    );

    app.teardown().await;
}

#[tokio::test]
async fn ctrl_c_exits_main_loop_when_idle() {
    let _guard = acquire();
    let (mut app, _model) = headless_app(vec![]).await;

    app.inject(UserEvent::Key(KeyEvent::new(
        KeyCode::Char('c'),
        KeyModifiers::CONTROL,
    )))
    .await;

    // The full `run()` returns cleanly once the loop breaks on Ctrl-C.
    app.run().await.expect("run should exit on Ctrl-C");
    app.teardown().await;
}

#[tokio::test]
async fn ctrl_d_exits_when_quit_armed_disabled() {
    let _guard = acquire();
    let (mut app, _model) = headless_app(vec![]).await;

    // Default (`quit_armed` off): a single Ctrl-D quits, as before.
    app.inject(ctrl_d()).await;
    app.run().await.expect("run should exit on single Ctrl-D");
    app.teardown().await;
}

/// With text in the prompt, `Ctrl-D` is a delete, not a quit: it must never
/// exit the loop, and it must remove the character under the cursor. Once the
/// prompt is empty again the same key quits.
#[tokio::test]
async fn ctrl_d_deletes_instead_of_quitting_while_the_prompt_has_text() {
    let _guard = acquire();
    let (mut app, _model) = headless_app(vec![]).await;

    for c in "abc".chars() {
        app.inject(char_key(c)).await;
        assert!(!step_broke(&mut app).await);
    }
    // Move the cursor between 'a' and 'b'.
    for _ in 0..2 {
        app.inject(left_key()).await;
        assert!(!step_broke(&mut app).await);
    }
    assert_eq!(app.input_buffer(), "abc");

    app.inject(ctrl_d()).await;
    assert!(
        !step_broke(&mut app).await,
        "Ctrl-D with text must not exit the loop"
    );
    assert_eq!(
        app.input_buffer(),
        "ac",
        "Ctrl-D must delete under the cursor"
    );

    // Back to the start: Ctrl-D eats the remaining chars one by one.
    app.inject(left_key()).await;
    assert!(!step_broke(&mut app).await);
    app.inject(ctrl_d()).await;
    assert!(!step_broke(&mut app).await);
    app.inject(ctrl_d()).await;
    assert!(!step_broke(&mut app).await);
    assert_eq!(app.input_buffer(), "");

    // Empty prompt: Ctrl-D is a quit again.
    app.inject(ctrl_d()).await;
    assert!(
        step_broke(&mut app).await,
        "Ctrl-D on an empty prompt must exit"
    );
    app.teardown().await;
}

#[tokio::test]
async fn ctrl_d_needs_two_presses_when_enabled() {
    let _guard = acquire();
    let mut app = headless_app_cfg(
        vec![],
        Config {
            quit_armed: Some(true),
            ..Default::default()
        },
    )
    .await;

    // First press arms a pending quit and prints a hint; loop stays alive.
    app.inject(ctrl_d()).await;
    assert!(!step_broke(&mut app).await, "first Ctrl-D must not exit");
    assert!(
        app.feed_text()
            .contains("Press Ctrl-C or Ctrl-D again to exit"),
        "hint should be shown: {}",
        app.feed_text()
    );

    // Second consecutive press exits.
    app.inject(ctrl_d()).await;
    assert!(step_broke(&mut app).await, "second Ctrl-D must exit");
    app.teardown().await;
}

#[tokio::test]
async fn ctrl_d_disarmed_by_other_key() {
    let _guard = acquire();
    let mut app = headless_app_cfg(
        vec![],
        Config {
            quit_armed: Some(true),
            ..Default::default()
        },
    )
    .await;

    app.inject(ctrl_d()).await;
    assert!(!step_broke(&mut app).await, "first Ctrl-D must not exit");

    // Any other key (here: a literal char) cancels the pending quit.
    app.inject(char_key('x')).await;
    assert!(!step_broke(&mut app).await);

    // So the next Ctrl-D only re-arms rather than exiting.
    app.inject(ctrl_d()).await;
    assert!(
        !step_broke(&mut app).await,
        "Ctrl-D after a disarming key must only re-arm"
    );
    app.teardown().await;
}

#[tokio::test]
async fn ctrl_c_needs_two_presses_when_enabled() {
    let _guard = acquire();
    let mut app = headless_app_cfg(
        vec![],
        Config {
            quit_armed: Some(true),
            ..Default::default()
        },
    )
    .await;

    // With `quit_armed` on, Ctrl-C is guarded like Ctrl-D.
    app.inject(ctrl_c()).await;
    assert!(!step_broke(&mut app).await, "first Ctrl-C must not exit");
    assert!(
        app.feed_text()
            .contains("Press Ctrl-C or Ctrl-D again to exit"),
        "hint should be shown: {}",
        app.feed_text()
    );

    // Second consecutive press exits.
    app.inject(ctrl_c()).await;
    assert!(step_broke(&mut app).await, "second Ctrl-C must exit");
    app.teardown().await;
}

#[tokio::test]
async fn ctrl_c_and_ctrl_d_share_the_pending_quit_when_enabled() {
    let _guard = acquire();
    let mut app = headless_app_cfg(
        vec![],
        Config {
            quit_armed: Some(true),
            ..Default::default()
        },
    )
    .await;

    // Either key arms; the other confirms, since they share one armed state.
    app.inject(ctrl_c()).await;
    assert!(!step_broke(&mut app).await, "first Ctrl-C must not exit");
    app.inject(ctrl_d()).await;
    assert!(
        step_broke(&mut app).await,
        "Ctrl-D must confirm a pending Ctrl-C quit"
    );
    app.teardown().await;
}

#[tokio::test]
async fn ctrl_c_disarmed_by_other_key() {
    let _guard = acquire();
    let mut app = headless_app_cfg(
        vec![],
        Config {
            quit_armed: Some(true),
            ..Default::default()
        },
    )
    .await;

    app.inject(ctrl_c()).await;
    assert!(!step_broke(&mut app).await, "first Ctrl-C must not exit");

    // Any other key (here: a literal char) cancels the pending quit.
    app.inject(char_key('x')).await;
    assert!(!step_broke(&mut app).await);

    // So the next Ctrl-C only re-arms rather than exiting.
    app.inject(ctrl_c()).await;
    assert!(
        !step_broke(&mut app).await,
        "Ctrl-C after a disarming key must only re-arm"
    );
    app.teardown().await;
}

#[tokio::test]
async fn ctrl_d_aborts_running_agent_when_enabled() {
    let _guard = acquire();
    let mut app = headless_app_cfg(
        vec![vec!["hi there"]],
        Config {
            quit_armed: Some(true),
            ..Default::default()
        },
    )
    .await;

    type_and_submit(&app, "hello").await;
    step_until(&mut app, |a| a.is_running()).await;

    // Ctrl-D while running aborts the run, never quits.
    app.inject(ctrl_d()).await;
    assert!(
        !step_broke(&mut app).await,
        "Ctrl-D while running must not exit"
    );
    step_until(&mut app, |a| !a.is_running()).await;
    app.teardown().await;
}

#[tokio::test]
async fn ctrl_z_is_swallowed_and_does_not_exit() {
    let _guard = acquire();
    let (mut app, _model) = headless_app(vec![]).await;

    app.inject(ctrl_z()).await;
    assert!(!step_broke(&mut app).await, "Ctrl-Z must not exit the loop");
    assert_eq!(app.input_buffer(), "", "Ctrl-Z must not type a literal 'z'");

    // And the buffer is still usable afterwards.
    app.inject(char_key('a')).await;
    assert!(!step_broke(&mut app).await);
    assert_eq!(app.input_buffer(), "a");
    app.teardown().await;
}

#[tokio::test]
async fn ctrl_g_is_inert_without_a_terminal_and_does_not_type_or_exit() {
    let _guard = acquire();
    let (mut app, _model) = headless_app(vec![]).await;

    // Headless has no terminal guard, so Ctrl-G must not try to launch
    // `$EDITOR` (or spawn a stdin reader): it is a no-op.
    app.inject(ctrl_g()).await;
    assert!(!step_broke(&mut app).await, "Ctrl-G must not exit the loop");
    assert_eq!(app.input_buffer(), "", "Ctrl-G must not type a literal 'g'");

    // And the buffer is still usable afterwards.
    app.inject(char_key('a')).await;
    assert!(!step_broke(&mut app).await);
    assert_eq!(app.input_buffer(), "a");
    app.teardown().await;
}

#[tokio::test]
async fn ctrl_t_is_inert_without_a_terminal_and_does_not_type_or_exit() {
    let _guard = acquire();
    let (mut app, _model) = headless_app(vec![]).await;

    // Headless has no terminal guard, so Ctrl-T must not try to launch a
    // pager: `show_transcript` returns early and nothing is typed.
    app.inject(ctrl_t()).await;
    assert!(!step_broke(&mut app).await, "Ctrl-T must not exit the loop");
    assert_eq!(app.input_buffer(), "", "Ctrl-T must not type a literal 't'");

    // And the buffer is still usable afterwards.
    app.inject(char_key('a')).await;
    assert!(!step_broke(&mut app).await);
    assert_eq!(app.input_buffer(), "a");
    app.teardown().await;
}

#[tokio::test]
async fn ctrl_l_repaints_without_typing_or_exiting() {
    let _guard = acquire();
    let (mut app, _model) = headless_app(vec![]).await;

    // Let the first frame land, so the baseline below is a settled screen: an
    // ordinary refresh is a no-op when nothing changed, which is exactly what
    // makes the growth assertion meaningful.
    pump(&mut app).await;
    let before = app.backend_output().len();

    app.inject(ctrl_l()).await;
    assert!(!step_broke(&mut app).await, "Ctrl-L must not exit the loop");
    assert_eq!(app.input_buffer(), "", "Ctrl-L must not type a literal 'l'");
    assert!(
        app.backend_output().len() > before,
        "Ctrl-L must force a repaint of the whole screen"
    );

    // And the buffer is still usable afterwards.
    app.inject(char_key('a')).await;
    assert!(!step_broke(&mut app).await);
    assert_eq!(app.input_buffer(), "a");
    app.teardown().await;
}

#[tokio::test]
async fn ctrl_z_does_not_abort_running_agent() {
    let _guard = acquire();
    let (mut app, _model) = headless_app(vec![vec!["hi there"]]).await;

    type_and_submit(&app, "hello").await;
    step_until(&mut app, |a| a.is_running()).await;

    app.inject(ctrl_z()).await;
    assert!(!step_broke(&mut app).await, "Ctrl-Z must not exit");
    assert!(app.is_running(), "Ctrl-Z must not cancel the running agent");

    step_until(&mut app, |a| !a.is_running()).await;
    app.teardown().await;
}

#[tokio::test]
async fn slash_clear_resets_session_and_feed() {
    let _guard = acquire();
    let (mut app, _model) = headless_app(vec![vec!["hi there"]]).await;

    type_and_submit(&app, "hello").await;
    step_until(&mut app, |a| {
        !a.is_running() && a.session().messages.len() == 2
    })
    .await;
    assert!(app.feed_text().contains("hi there"));

    type_slash_and_submit(&app, "/clear").await;
    step_until(&mut app, |a| {
        a.session().messages.is_empty() && !a.feed_text().contains("hi there")
    })
    .await;

    assert!(app.session().messages.is_empty());
    let feed = app.feed_text();
    assert!(
        feed.contains("zerostack"),
        "cleared feed should show the fresh welcome block: {feed}"
    );

    app.teardown().await;
}

#[tokio::test]
async fn slash_new_starts_a_new_session_while_clear_keeps_id() {
    let _guard = acquire();
    let (mut app, _model) = headless_app(vec![vec!["hi there"]]).await;

    type_and_submit(&app, "hello").await;
    step_until(&mut app, |a| {
        !a.is_running() && a.session().messages.len() == 2
    })
    .await;
    let id_before = app.session().id.clone();

    // /clear resets the conversation in place: same id, empty transcript.
    type_slash_and_submit(&app, "/clear").await;
    step_until(&mut app, |a| a.session().messages.is_empty()).await;
    assert_eq!(
        app.session().id,
        id_before,
        "/clear must keep the current session id"
    );

    // /new mints a brand-new session: different id, empty transcript.
    type_slash_and_submit(&app, "/new").await;
    step_until(&mut app, |a| a.session().id != id_before).await;
    assert_ne!(app.session().id, id_before, "/new must start a new session");
    assert!(app.session().messages.is_empty());
    assert!(
        app.feed_text().contains("new session"),
        "feed should announce the new session: {}",
        app.feed_text()
    );

    app.teardown().await;
}

#[tokio::test]
async fn help_documents_clear_and_new_distinctly() {
    let _guard = acquire();
    let (mut app, _model) = headless_app(vec![]).await;

    type_slash_and_submit(&app, "/help").await;
    step_until(&mut app, |a| a.feed_text().contains("commands:")).await;

    let feed = app.feed_text();
    assert!(
        !feed.contains("clear screen"),
        "/help must not describe /clear as a screen clear: {feed}"
    );
    assert!(
        !feed.contains("/clear [/new]"),
        "/help must not collapse /clear and /new into one entry: {feed}"
    );
    assert_eq!(
        feed.matches("reset the current session in place").count(),
        1,
        "/clear help line must appear exactly once: {feed}"
    );
    assert_eq!(
        feed.matches("start a new session").count(),
        1,
        "/new help line must appear exactly once: {feed}"
    );

    app.teardown().await;
}

#[tokio::test]
async fn fake_backend_captures_output_and_paste_fills_input() {
    let _guard = acquire();
    let (mut app, _model) = headless_app(vec![]).await;

    // The startup render already went through the fake backend.
    let captured = app.backend_output();
    assert!(
        captured.contains('\x1b'),
        "backend should capture ANSI frames, got {} bytes",
        captured.len()
    );

    app.inject(UserEvent::Paste("a\nb".to_string())).await;
    pump(&mut app).await;
    assert_eq!(app.input_buffer(), "a\nb");

    app.teardown().await;
}

/// Submit a slash command through the command picker: typing "/" opens it, so
/// the first Enter only completes the highlighted command into the buffer (or,
/// when the query has args and matches nothing, just closes the picker) and a
/// second Enter submits — same as the interactive flow.
async fn type_slash_and_submit(app: &App<'static>, text: &str) {
    for c in text.chars() {
        app.inject(char_key(c)).await;
    }
    app.inject(enter_key()).await;
    app.inject(enter_key()).await;
}

/// A headless `App` with `multiline_prompt` on, around an idle mock agent
/// (no run is expected; the agent is only there to satisfy the constructor).
async fn swapped_app() -> App<'static> {
    let model = fake_model::text_turns(Vec::<Vec<&str>>::new());
    let agent = AnyAgent::Mock(rig::agent::AgentBuilder::new(model).build());
    headless_app_with_cfg(
        agent,
        Config {
            multiline_prompt: Some(true),
            ..Default::default()
        },
    )
    .await
}

/// Regression: with `multiline_prompt`, a slash command in the buffer is
/// exempt from the swap, so the picker's `Enter` completes `/mode` and a second
/// `Enter` submits it. Before, the swap applied unconditionally and both Enters
/// only inserted newlines — no command could be sent.
#[tokio::test]
async fn swapped_enter_submits_picked_slash_command() {
    let _guard = acquire();
    let mut app = swapped_app().await;

    type_slash_and_submit(&app, "/mode").await;
    step_until(&mut app, |a| a.feed_text().contains("security mode")).await;
    assert!(!app.is_running());

    app.teardown().await;
}

/// Negative control: outside a slash buffer the swap still applies — a lone
/// `Enter` inserts a newline instead of submitting.
#[tokio::test]
async fn swapped_enter_still_newlines_plain_text() {
    let _guard = acquire();
    let mut app = swapped_app().await;

    for c in "hello".chars() {
        app.inject(char_key(c)).await;
    }
    app.inject(enter_key()).await;
    step_until(&mut app, |a| a.input_buffer() == "hello\n").await;

    assert!(!app.is_running());
    app.teardown().await;
}

/// A headless `App` (in `multiline_prompt` mode when `multiline` is set) whose
/// transcript holds enough lines to scroll — the 80x24 `FakeBackend` shows far
/// fewer rows than the scripted response emits.
async fn scrollable_app(multiline: bool) -> App<'static> {
    let long: &'static str =
        Box::leak(format!("{}\nMARKER_END\n", "x\n".repeat(60)).into_boxed_str());
    let cfg = Config {
        multiline_prompt: multiline.then_some(true),
        ..Default::default()
    };
    let mut app = headless_app_cfg(vec![vec![long]], cfg).await;

    for c in "hi".chars() {
        app.inject(char_key(c)).await;
    }
    // In `multiline_prompt` mode `Enter` inserts a newline; `Ctrl+J` submits.
    app.inject(if multiline { ctrl_j_key() } else { enter_key() })
        .await;
    step_until(&mut app, |a| a.feed_text().contains("MARKER_END")).await;
    app
}

/// With `multiline_prompt` on, bare `Home`/`End` move the prompt cursor to the
/// first line / first char and the last line / past the last char, leaving the
/// transcript alone; the transcript jumps move to `Ctrl+Home`/`Ctrl+End`.
#[tokio::test]
async fn multiline_prompt_routes_home_end_to_the_prompt() {
    let _guard = acquire();
    let mut app = scrollable_app(true).await;

    for c in "abc".chars() {
        app.inject(char_key(c)).await;
    }
    step_until(&mut app, |a| a.input_buffer() == "abc").await;

    // Bare `Enter` inserts a newline in this mode, so the buffer is two lines.
    app.inject(enter_key()).await;
    step_until(&mut app, |a| a.input_buffer() == "abc\n").await;
    for c in "de".chars() {
        app.inject(char_key(c)).await;
    }
    step_until(&mut app, |a| a.input_buffer() == "abc\nde").await;
    assert_eq!(app.input_cursor(), 6);
    assert!(!app.is_scrolling(), "transcript starts at the bottom");

    // Bare `Home`: first line, first char. Transcript untouched.
    app.inject(home_key()).await;
    step_until(&mut app, |a| a.input_cursor() == 0).await;
    assert_eq!(app.input_buffer(), "abc\nde");
    assert!(!app.is_scrolling(), "Home must not scroll the transcript");

    // Bare `End`: last line, past the last char. Transcript untouched.
    app.inject(end_key()).await;
    step_until(&mut app, |a| a.input_cursor() == "abc\nde".len()).await;
    assert!(!app.is_scrolling(), "End must not scroll the transcript");

    // `Ctrl+Home` / `Ctrl+End`: the transcript jumps, prompt cursor stays.
    app.inject(ctrl_home_key()).await;
    step_until(&mut app, |a| a.is_scrolling()).await;
    assert_eq!(
        app.input_cursor(),
        6,
        "Ctrl+Home must not move the prompt cursor"
    );

    app.inject(ctrl_end_key()).await;
    step_until(&mut app, |a| !a.is_scrolling()).await;
    assert_eq!(app.input_cursor(), 6);

    app.teardown().await;
}

/// Negative control: without `multiline_prompt` the old mapping holds — bare
/// `Home`/`End` scroll the transcript and leave the prompt cursor alone.
#[tokio::test]
async fn home_end_still_scroll_the_transcript_by_default() {
    let _guard = acquire();
    let mut app = scrollable_app(false).await;

    for c in "abc".chars() {
        app.inject(char_key(c)).await;
    }
    step_until(&mut app, |a| a.input_buffer() == "abc").await;

    app.inject(home_key()).await;
    step_until(&mut app, |a| a.is_scrolling()).await;
    assert_eq!(
        app.input_cursor(),
        3,
        "Home must not move the prompt cursor"
    );

    app.inject(end_key()).await;
    step_until(&mut app, |a| !a.is_scrolling()).await;
    assert_eq!(app.input_cursor(), 3);

    app.teardown().await;
}

#[tokio::test]
async fn ctrl_c_aborts_running_agent() {
    let _guard = acquire();
    let (mut app, model) = headless_app(vec![vec!["second answer"]]).await;

    type_and_submit(&app, "hello").await;
    // Ctrl-C lands in the same FIFO channel right behind the Enter, so it is
    // processed while the run is active (the agent task is never polled until
    // the injected events are drained).
    app.inject(UserEvent::Key(KeyEvent::new(
        KeyCode::Char('c'),
        KeyModifiers::CONTROL,
    )))
    .await;

    step_until(&mut app, |a| {
        !a.is_running() && a.feed_text().contains("interrupted")
    })
    .await;

    // The aborted runner never reached the model; the aborted turn keeps the
    // user message but adds no assistant reply.
    assert!(model.requests().is_empty());
    let messages = &app.session().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].role, MessageRole::User);

    // The loop is still usable: a fresh prompt starts a new run.
    type_and_submit(&app, "again").await;
    step_until(&mut app, |a| {
        !a.is_running() && a.session().messages.len() == 3
    })
    .await;
    let messages = &app.session().messages;
    assert_eq!(messages[1].content.as_str(), "again");
    assert_eq!(messages[2].content.as_str(), "second answer");

    app.teardown().await;
}

#[tokio::test]
async fn slash_rejected_while_running() {
    let _guard = acquire();
    let (mut app, _model) = headless_app(vec![vec!["answer"]]).await;

    type_and_submit(&app, "hello").await;
    type_slash_and_submit(&app, "/help").await;

    step_until(&mut app, |a| {
        a.feed_text()
            .contains("agent is running — wait for it to finish")
    })
    .await;

    // The running turn is unaffected and completes normally.
    step_until(&mut app, |a| {
        !a.is_running() && a.session().messages.len() == 2
    })
    .await;
    assert_eq!(app.session().messages[1].content.as_str(), "answer");

    app.teardown().await;
}

#[tokio::test]
async fn queue_commands_list_and_pop() {
    let _guard = acquire();
    let (mut app, model) = headless_app(vec![vec!["answer a"]]).await;

    type_and_submit(&app, "a").await;
    type_and_submit(&app, "b").await;
    type_slash_and_submit(&app, "/queue ls").await;
    type_slash_and_submit(&app, "/queue pop").await;

    step_until(&mut app, |a| a.feed_text().contains("  1. b")).await;
    step_until(&mut app, |a| a.feed_text().contains("unqueued: b")).await;
    step_until(&mut app, |a| !a.is_running()).await;

    let feed = app.feed_text();
    assert!(feed.contains("queued: b"), "feed: {feed}");
    assert!(feed.contains("queued (1):"), "feed: {feed}");

    // The popped input never ran: one model call, one exchange.
    assert_eq!(model.requests().len(), 1);
    let messages = &app.session().messages;
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].content.as_str(), "a");

    app.teardown().await;
}

#[tokio::test]
async fn paste_multiline_submits_as_one_message() {
    let _guard = acquire();
    let (mut app, _model) = headless_app(vec![vec!["got it"]]).await;

    app.inject(UserEvent::Paste("line1\nline2".to_string()))
        .await;
    app.inject(enter_key()).await;

    step_until(&mut app, |a| {
        !a.is_running() && a.session().messages.len() == 2
    })
    .await;

    let messages = &app.session().messages;
    assert_eq!(messages[0].role, MessageRole::User);
    assert_eq!(messages[0].content.as_str(), "line1\nline2");

    app.teardown().await;
}

/// Regression: XTerm's `Shift+Insert` / middle-click paste delivers a selection
/// with CR instead of LF line endings, so a multi-line selection arrived as one
/// CR-separated line and was submitted that way. It must come back out as real
/// newlines — the model should never see embedded `\r`.
#[tokio::test]
async fn paste_with_cr_line_endings_submits_as_newlines() {
    let _guard = acquire();
    let (mut app, _model) = headless_app(vec![vec!["got it"]]).await;

    app.inject(UserEvent::Paste("line1\rline2\r".to_string()))
        .await;
    app.inject(enter_key()).await;

    step_until(&mut app, |a| {
        !a.is_running() && a.session().messages.len() == 2
    })
    .await;

    let messages = &app.session().messages;
    assert_eq!(messages[0].role, MessageRole::User);
    assert_eq!(messages[0].content.as_str(), "line1\nline2\n");
    assert!(
        !messages[0].content.contains('\r'),
        "a CR must never survive into the submitted prompt"
    );

    app.teardown().await;
}

/// A terminal paste arriving during a middle-button gesture must still reach
/// the editor: only the app's *second* read is suppressed, never the text
/// itself. That the suppression works is pinned by the `MiddleClickPaste`
/// tests in `middle_click_tests`; this covers the wiring.
#[tokio::test]
async fn middle_release_after_a_terminal_paste_does_not_paste_again() {
    let _guard = acquire();
    let (mut app, _model) = headless_app(vec![]).await;

    app.inject(UserEvent::MiddlePress).await;
    app.inject(UserEvent::Paste("from-terminal".to_string()))
        .await;
    app.inject(UserEvent::MiddleRelease).await;
    step_until(&mut app, |a| !a.input_buffer().is_empty()).await;

    assert_eq!(
        app.input_buffer(),
        "from-terminal",
        "the release must not read PRIMARY again"
    );

    app.teardown().await;
}

/// A release with no press pastes nothing: the press may have been lost, and
/// with `mouse_capture` off the terminal pastes by itself and neither event
/// reaches the app at all.
#[tokio::test]
async fn middle_release_without_a_press_is_a_noop() {
    let _guard = acquire();
    let (mut app, _model) = headless_app(vec![]).await;

    app.inject(UserEvent::MiddleRelease).await;
    for _ in 0..3 {
        pump(&mut app).await;
    }

    assert_eq!(app.input_buffer(), "");

    app.teardown().await;
}

#[tokio::test]
async fn ctrl_r_toggles_reasoning_visibility() {
    let _guard = acquire();
    let (mut app, _model) = headless_app(vec![]).await;

    let ctrl_r = || UserEvent::Key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL));
    app.inject(ctrl_r()).await;
    step_until(&mut app, |a| {
        a.feed_text().contains("reasoning visibility: on")
    })
    .await;

    app.inject(ctrl_r()).await;
    step_until(&mut app, |a| {
        a.feed_text().contains("reasoning visibility: off")
    })
    .await;

    app.teardown().await;
}

/// Build a headless app with two quick models on the same provider, so
/// `Alt+M` switching stays local (no client rebuild, no network).
async fn headless_app_with_quick_models() -> App<'static> {
    isolate_data_dirs();
    let mut quick = HashMap::new();
    quick.insert(
        "fast".to_string(),
        crate::config::QuickModelConfig {
            provider: compact_str::CompactString::new("anthropic"),
            model: compact_str::CompactString::new("claude-fast-test"),
            input_token_cost: 1.0,
            output_token_cost: 2.0,
            reserve_tokens: None,
            temperature: None,
            extra_body: None,
            context_window: None,
        },
    );
    quick.insert(
        "pro".to_string(),
        crate::config::QuickModelConfig {
            provider: compact_str::CompactString::new("anthropic"),
            model: compact_str::CompactString::new("claude-pro-test"),
            input_token_cost: 3.0,
            output_token_cost: 4.0,
            reserve_tokens: None,
            temperature: None,
            extra_body: None,
            context_window: None,
        },
    );
    let cfg: &'static Config = Box::leak(Box::new(Config {
        quick_models: Some(quick),
        ..Default::default()
    }));
    let cli: &'static Cli = Box::leak(Box::new(Cli {
        api_key: Some("test-key".to_string()),
        no_session: true,
        no_color: true,
        ..Default::default()
    }));
    let session: &'static mut Session = Box::leak(Box::new(Session::new(
        "anthropic",
        "claude-sonnet-4-5",
        200_000,
        "tui-switcher-test",
    )));
    let context: &'static mut ContextFiles =
        Box::leak(Box::new(crate::context::load_with_prompts_dirs(true, &[])));
    let client =
        crate::provider::create_client("anthropic", Some("test-key"), &HashMap::new(), None)
            .expect("create test client");
    let ui = UiContext::new(
        cli,
        cfg,
        session,
        context,
        client,
        None,
        None,
        Sandbox::new(false, "bwrap"),
        None,
    );
    let model = fake_model::text_turns(Vec::<Vec<&str>>::new());
    let agent = AnyAgent::Mock(rig::agent::AgentBuilder::new(model).build());
    App::new_headless(
        ui,
        Some(agent),
        None,
        None,
        Box::new(FakeBackend::new(80, 24)),
    )
    .await
    .expect("build headless app")
}

fn alt_key(c: char) -> UserEvent {
    UserEvent::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::ALT))
}

fn esc_key() -> UserEvent {
    UserEvent::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
}

#[tokio::test]
async fn alt_m_switcher_esc_cancels_without_switching() {
    let _guard = acquire();
    let mut app = headless_app_with_quick_models().await;
    let before = app.session().model.to_string();

    app.inject(alt_key('m')).await;
    step_until(&mut app, |a| a.switcher_kind() == Some("model")).await;
    app.inject(esc_key()).await;
    step_until(&mut app, |a| a.switcher_kind().is_none()).await;

    assert_eq!(app.session().model.as_str(), before);

    app.teardown().await;
}

#[tokio::test]
async fn scroll_and_resize_events() {
    let _guard = acquire();
    // A long response so the feed overflows the 80x24 fake terminal.
    let long_response: String = (0..45)
        .map(|i| format!("response line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let (mut app, _model) = headless_app(vec![vec![long_response.as_str()]]).await;

    type_and_submit(&app, "hello").await;
    step_until(&mut app, |a| {
        !a.is_running() && a.session().messages.len() == 2
    })
    .await;
    assert!(!app.is_scrolling());

    for _ in 0..3 {
        app.inject(UserEvent::ScrollUp).await;
    }
    step_until(&mut app, |a| a.is_scrolling()).await;
    assert!(
        app.backend_output().contains("SCROLL"),
        "scrolled viewport should paint a scroll indicator"
    );

    for _ in 0..3 {
        app.inject(UserEvent::ScrollDown).await;
    }
    app.inject(UserEvent::Resize).await;
    step_until(&mut app, |a| !a.is_scrolling()).await;

    app.teardown().await;
}

#[derive(Debug, Deserialize)]
struct EchoArgs {
    text: String,
}

/// Minimal tool so a scripted run can make a second model call (a tool-call
/// turn, then a closing text turn) without going near the network.
struct EchoTool;

impl Tool for EchoTool {
    const NAME: &'static str = "echo";

    type Error = ToolError;
    type Args = EchoArgs;
    type Output = String;

    fn description(&self) -> String {
        "Echoes the given text back.".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": { "text": { "type": "string" } },
            "required": ["text"]
        })
    }

    async fn call(&self, args: EchoArgs) -> Result<String, ToolError> {
        Ok(format!("echoed: {}", args.text))
    }
}

/// A single model call with real usage: rig emits one `CompletionCall` (that
/// call's usage) and then a `Done` whose usage is the run *aggregate*. For one
/// call the two are equal, so the totals must count it once — not twice.
#[tokio::test]
async fn a_single_model_call_is_not_double_counted() {
    let _guard = acquire();
    let usage = Usage {
        input_tokens: 1000,
        output_tokens: 100,
        ..Usage::new()
    };
    let model = MockCompletionModel::from_stream_turns(vec![vec![
        MockStreamEvent::text("hi".to_string()),
        MockStreamEvent::final_response(usage),
    ]]);
    let agent = AnyAgent::Mock(rig::agent::AgentBuilder::new(model).build());
    let mut app = headless_app_with_agent(agent).await;

    type_and_submit(&app, "hello").await;
    step_until(&mut app, |a| a.is_running()).await;
    step_until(&mut app, |a| !a.is_running()).await;

    assert_eq!(
        app.session().total_input_tokens,
        1000,
        "Done's aggregate must not be re-added on top of the CompletionCall"
    );
    assert_eq!(app.session().total_output_tokens, 100);

    app.teardown().await;
}

/// A two-call run (tool call, then closing text). Totals must sum each call
/// once, and the context anchor must be the *last* call's prompt — not `Done`'s
/// aggregate, which sums both prompts and would inflate the context meter.
#[tokio::test]
async fn multi_call_run_uses_last_call_usage_for_context_anchor() {
    let _guard = acquire();
    let first = Usage {
        input_tokens: 500,
        output_tokens: 20,
        ..Usage::new()
    };
    let last = Usage {
        input_tokens: 900,
        output_tokens: 30,
        ..Usage::new()
    };
    let model = MockCompletionModel::from_stream_turns(vec![
        vec![
            MockStreamEvent::tool_call("call-1", "echo", serde_json::json!({ "text": "x" })),
            MockStreamEvent::final_response(first),
        ],
        vec![
            MockStreamEvent::text("done".to_string()),
            MockStreamEvent::final_response(last),
        ],
    ]);
    let agent = AnyAgent::Mock(
        rig::agent::AgentBuilder::new(model)
            .tool(EchoTool)
            .default_max_turns(4)
            .build(),
    );
    let mut app = headless_app_with_agent(agent).await;

    type_and_submit(&app, "go").await;
    step_until(&mut app, |a| a.is_running()).await;
    step_until(&mut app, |a| !a.is_running()).await;

    // One sum per call, not `Done`'s aggregate (500+900) added again.
    assert_eq!(app.session().total_input_tokens, 1400);
    assert_eq!(app.session().total_output_tokens, 50);
    // Anchor is the last call's prompt (900) + its output (30), not the
    // aggregate (1400 + 50).
    assert_eq!(app.session().calibrated_tokens, 930);
    assert_eq!(
        app.session().calibrated_msg_count,
        app.session().messages.len()
    );

    app.teardown().await;
}
