pub mod load;
pub mod types;

use std::collections::HashMap;

use compact_str::CompactString;
use serde::{Deserialize, Serialize};

pub use load::*;
pub use types::*;

use crate::permission::{PermissionConfig, PermissionConfigs};
use crate::retry::RetryConfig;

#[cfg(feature = "mcp")]
use crate::extras::mcp::config::McpServerConfig;

#[cfg(feature = "acp")]
use crate::extras::acp::config::AcpServerConfig;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<CompactString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<CompactString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// Provider-specific JSON shallow-merged into every completion request body
    /// as a global default. A matching `quick_models` entry's `extra_body`
    /// overrides this. Note: body params are provider-specific, so a global
    /// value does not follow model switches — bundle per-`quick_models` when in
    /// doubt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_body: Option<serde_json::Value>,
    #[serde(default)]
    pub retry: RetryConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_tools: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_context_files: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reserve_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_recent_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_agent_turns: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_text_file_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_read_lines: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_bash_output_lines: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_grep_results: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_find_results: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_list_dir_entries: Option<u64>,
    // --- Subagent tool limits (applied when subagents spawn) ---
    #[cfg(feature = "subagents")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subagent_max_read_lines: Option<u64>,
    #[cfg(feature = "subagents")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subagent_max_grep_results: Option<u64>,
    #[cfg(feature = "subagents")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subagent_max_find_results: Option<u64>,
    #[cfg(feature = "subagents")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subagent_max_list_dir_entries: Option<u64>,
    // --- End subagent limits ---
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compact_enabled: Option<bool>,
    /// Opt-in mid-turn compaction threshold, as a fraction of the context
    /// window (0.0–1.0) of real provider prompt pressure. `None` (default)
    /// disables mid-turn compaction entirely; the agent only compacts between
    /// turns. Honored only when `compact_enabled` is also true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mid_turn_compact_threshold: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub always_show_welcome: Option<bool>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "auto-update-prompts"
    )]
    pub auto_update_prompts: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "auto-update-themes")]
    pub auto_update_themes: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_providers: Option<HashMap<String, types::CustomProviderConfig>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "permission-regex")]
    pub permission_regex: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "permission-allow")]
    pub permission_allow: Option<HashMap<String, Vec<String>>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "permission-ask")]
    pub permission_ask: Option<HashMap<String, Vec<String>>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "permission-deny")]
    pub permission_deny: Option<HashMap<String, Vec<String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restrictive: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accept_all: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yolo: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sandbox: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "sandbox-backend")]
    pub sandbox_backend: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "sandbox-required")]
    pub sandbox_required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "sandbox-expose")]
    pub sandbox_expose: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "sandbox-network")]
    pub sandbox_network: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_all_mcp_calls: Option<bool>,
    #[cfg(feature = "mcp")]
    #[serde(skip_serializing_if = "Option::is_none", rename = "enable-exa-mcp")]
    pub enable_exa_mcp: Option<bool>,
    #[cfg(feature = "mcp")]
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "enable-context7-mcp"
    )]
    pub enable_context7_mcp: Option<bool>,
    #[cfg(feature = "mcp")]
    #[serde(skip_serializing_if = "Option::is_none", rename = "enable-grepapp-mcp")]
    pub enable_grepapp_mcp: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_permission_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "permission-modes")]
    pub permission_modes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_tool_details: Option<ShowToolDetails>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_reasoning: Option<bool>,
    /// Configurable status-bar (up to 3 lines). When absent, a built-in
    /// default layout is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statusline: Option<types::StatusLineConfig>,
    /// Left padding (columns) for the chat area. Default: 0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_left_margin: Option<u16>,
    /// Whether the TUI should enable crossterm mouse capture. Disabling this
    /// returns mouse selection/copy/paste to the terminal emulator, but mouse
    /// scrolling and in-app click-to-place-cursor will no longer work.
    /// Default: true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mouse_capture: Option<bool>,
    /// Which selection copy operations target: `clipboard` (system clipboard,
    /// the default) or `primary` (X11 PRIMARY selection, pasted with the
    /// middle mouse button). Only the copy target changes — selection and the
    /// `y` key work identically either way.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clipboard_selection: Option<types::ClipboardSelection>,
    /// Swap the roles of `Enter` and `Ctrl+J` in the prompt editor: `Enter`
    /// inserts a newline, `Ctrl+J` submits. For terminals that cannot encode
    /// `Shift`/`Alt+Enter` (xterm, the Linux console, screen, mosh), where
    /// `Ctrl+J` is the only portable newline key. Default: false (`Enter`
    /// sends, `Ctrl+J` inserts a newline).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swap_enter_and_newline: Option<bool>,
    /// Whether quitting the TUI with `Ctrl-C` or `Ctrl-D` requires two presses.
    /// When `true`, the first press arms a pending quit and prints a hint; any
    /// other key cancels it, and a second consecutive press of either key
    /// exits. Default: false (a single press quits, matching the historical
    /// behavior).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quit_armed: Option<bool>,
    /// Terminal-compatibility folding of `Esc`/`Alt` key sequences. Some
    /// terminals (xterm, the Linux console, GNU screen, mosh, slow SSH links)
    /// deliver `Alt+<key>` as a lone `Esc` followed by the key instead of one
    /// modified key press, and deliver a double-tapped `Esc` as two events.
    /// When `true`, a lone `Esc` is held for ~250 ms: a following character or
    /// `Enter` becomes `Alt` + that key, a second `Esc` collapses into a single
    /// `Esc`, and any other key releases the held `Esc` first. Default: false
    /// (keys pass through exactly as the terminal sends them).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub esc_alt_compat: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_prompt: Option<CompactString>,
    #[cfg(feature = "git-worktree")]
    #[serde(skip_serializing_if = "Option::is_none", rename = "wt-auto-merge")]
    pub wt_auto_merge: Option<bool>,
    #[cfg(feature = "git-worktree")]
    #[serde(skip_serializing_if = "Option::is_none", rename = "wt-base-dir")]
    pub wt_base_dir: Option<String>,

    #[cfg(feature = "git-worktree")]
    #[serde(skip_serializing_if = "Option::is_none", rename = "wt-force")]
    pub wt_force: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shell: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub editor: Option<String>,
    /// Pager command for the read-only transcript view (`Ctrl-T`, `/transcript`).
    /// Overrides `$PAGER`; when unset, `$PAGER` wins, then `less`. A command
    /// line, so arguments are allowed (`view -`, `less -R`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pager: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_keys: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quick_models: Option<HashMap<String, types::QuickModelConfig>>,
    /// Map prompt names to quick-model names. When switching to a prompt,
    /// zerostack looks up the quick model and switches provider+model.
    /// Empty-string values are treated as "no change".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_to_model: Option<HashMap<String, String>>,
    #[cfg(feature = "mcp")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_servers: Option<HashMap<String, McpServerConfig>>,
    #[cfg(feature = "acp")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acp_servers: Option<HashMap<String, AcpServerConfig>>,
    #[cfg(feature = "acp")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acp_host: Option<String>,
    #[cfg(feature = "acp")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acp_port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edit_system: Option<types::EditSystem>,
    #[cfg(feature = "subagents")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_max_turns: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deny_repeated_reads: Option<bool>,
    /// Show the session cost in the status bar even when it is $0.0000 (e.g. when
    /// the model has no per-token pricing configured). Default: false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_cost_always: Option<bool>,
    #[cfg(feature = "subagents")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_enabled: Option<bool>,
    #[cfg(feature = "subagents")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subagent_model: Option<CompactString>,
    #[cfg(feature = "subagents")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subagent_provider: Option<CompactString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colors: Option<types::ColorsConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain: Option<types::ChainConfig>,
    #[cfg(feature = "lsp")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lsp: Option<types::LspConfig>,
    #[cfg(feature = "rtk")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rtk: Option<types::RtkConfig>,
    #[cfg(feature = "advisor")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub advisor: Option<types::AdvisorConfig>,
}

impl Config {
    pub fn custom_providers_map(&self) -> HashMap<String, types::CustomProviderConfig> {
        self.custom_providers.clone().unwrap_or_default()
    }

    /// Whether requests for `provider` go through the Anthropic-native API
    /// route. This is the route that enables prompt caching and reports
    /// `input_tokens` *excluding* cached/cache-creation tokens, so it is the
    /// only one whose context accounting must add the cache fields back in
    /// (see [`Session::real_input_tokens`](crate::session::Session::real_input_tokens)).
    ///
    /// Keyed on the resolved provider *kind*, not the user-facing name: a
    /// custom provider registered under any name but with
    /// `provider_type = "anthropic"` still hits the native route, while
    /// OpenRouter — even when serving a Claude model — normalizes usage to the
    /// OpenAI shape (`input_tokens` already includes cached) and must not.
    pub fn is_anthropic_native(&self, provider: &str) -> bool {
        let kind_name = self
            .custom_providers
            .as_ref()
            .and_then(|m| m.get(provider))
            .map(|c| c.provider_type.as_str())
            .unwrap_or(provider);
        matches!(
            crate::auth::ProviderKind::from_name(kind_name),
            Some(crate::auth::ProviderKind::Anthropic)
        )
    }

    pub fn resolve_context_window(
        &self,
        provider: &str,
        model_id: &str,
        qm: &HashMap<String, types::QuickModelConfig>,
    ) -> u64 {
        if let Some(cw) = self.context_window {
            return cw;
        }
        for qmc in qm.values() {
            if qmc.model.as_str() == model_id
                && let Some(cw) = qmc.context_window
            {
                return cw;
            }
        }
        Self::catalog_context_window(provider, model_id).unwrap_or(128_000)
    }

    /// The model's context window straight from the static catalog, or `None`
    /// when the provider/model is not listed (custom gateways, ollama, or an id
    /// without a `context` entry). Unlike [`resolve_context_window`], this
    /// ignores the config override and the 128k fallback, so callers can tell a
    /// real catalog value apart from the default.
    pub fn catalog_context_window(provider: &str, model_id: &str) -> Option<u64> {
        let entries = crate::models_catalog::catalog_entries(provider)?;
        entries
            .iter()
            .find(|e| e.id == model_id)
            .and_then(|e| e.context_length)
            .map(|cl| cl as u64)
    }

    /// The model's input/output cost (USD per million tokens) straight from
    /// the static catalog, or `None` when the provider/model isn't listed or
    /// carries no baked-in pricing (e.g. OpenRouter, which prices live via
    /// `fetch_live_model_info` instead).
    pub fn catalog_input_output_cost(provider: &str, model_id: &str) -> Option<(f64, f64)> {
        let entries = crate::models_catalog::catalog_entries(provider)?;
        entries
            .iter()
            .find(|e| e.id == model_id)
            .and_then(|e| e.input_price.zip(e.output_price))
    }

    pub fn resolve_reserve_tokens(
        &self,
        model_id: &str,
        qm: &HashMap<String, types::QuickModelConfig>,
    ) -> u64 {
        if let Some(rt) = self.reserve_tokens {
            return rt;
        }
        for qmc in qm.values() {
            if qmc.model.as_str() == model_id
                && let Some(rt) = qmc.reserve_tokens
            {
                return rt;
            }
        }
        8_192
    }

    pub fn resolve_keep_recent_tokens(&self) -> u64 {
        self.keep_recent_tokens.unwrap_or(10_000)
    }

    pub fn resolve_chat_left_margin(&self) -> u16 {
        self.chat_left_margin.unwrap_or(0)
    }

    pub fn resolve_mouse_capture(&self) -> bool {
        self.mouse_capture.unwrap_or(true)
    }

    /// Pager command for the read-only transcript view (`Ctrl-T`, `/transcript`):
    /// the `pager` config option if set, else `$PAGER`, else `less`. Returned as
    /// a command line so arguments work (`view -`, `less -R`); a blank value
    /// falls through to the next source.
    pub fn resolve_pager(&self) -> String {
        self.pager
            .clone()
            .or_else(|| std::env::var("PAGER").ok())
            .filter(|p| !p.trim().is_empty())
            .unwrap_or_else(|| "less".to_string())
    }

    /// Which selection copy operations target. Default: the system clipboard.
    pub fn resolve_clipboard_selection(&self) -> types::ClipboardSelection {
        self.clipboard_selection.unwrap_or_default()
    }

    /// Whether `Enter` and `Ctrl+J` exchange roles in the prompt editor.
    /// Default `false`: `Enter` submits and `Ctrl+J` inserts a newline.
    pub fn resolve_swap_enter_and_newline(&self) -> bool {
        self.swap_enter_and_newline.unwrap_or(false)
    }

    /// Whether quitting the TUI via `Ctrl-C`/`Ctrl-D` needs two presses.
    /// Default: false.
    pub fn resolve_quit_armed(&self) -> bool {
        self.quit_armed.unwrap_or(false)
    }

    /// Whether `Esc`-prefixed `Alt` sequences are folded in the event thread
    /// (see `esc_alt_compat`). Default: false.
    pub fn resolve_esc_alt_compat(&self) -> bool {
        self.esc_alt_compat.unwrap_or(false)
    }

    /// Resolves temperature: CLI `--temperature` > quick-model `temperature` >
    /// global `temperature`. Returns `None` when no temperature is configured.
    pub fn resolve_temperature(
        &self,
        cli: &crate::cli::Cli,
        model_id: &str,
        qm: &HashMap<String, types::QuickModelConfig>,
    ) -> Option<f64> {
        if let Some(temp) = cli.temperature {
            return Some(temp.clamp(0.0, 2.0));
        }
        for qmc in qm.values() {
            if qmc.model.as_str() == model_id
                && let Some(temp) = qmc.temperature
            {
                return Some(temp.clamp(0.0, 2.0));
            }
        }
        self.temperature.map(|t| t.clamp(0.0, 2.0))
    }

    /// Resolves provider-specific request-body params: quick-model `extra_body` >
    /// global `extra_body`. Returns `None` when neither is configured. The
    /// resolved value is shallow-merged into the completion request body at
    /// agent-build time.
    pub fn resolve_extra_body(
        &self,
        model_id: &str,
        qm: &HashMap<String, types::QuickModelConfig>,
    ) -> Option<serde_json::Value> {
        for qmc in qm.values() {
            if qmc.model.as_str() == model_id
                && let Some(eb) = &qmc.extra_body
            {
                return Some(eb.clone());
            }
        }
        self.extra_body.clone()
    }

    pub fn resolve_compact_enabled(&self) -> bool {
        self.compact_enabled.unwrap_or(false)
    }

    /// Mid-turn compaction pressure threshold as a fraction of the context
    /// window. Unlike the other resolvers this one substitutes **no** enabling
    /// default: `None` means the mid-turn trigger never fires (preserving the
    /// historical between-turn-only behavior). Values outside `(0.0, 1.0]` are
    /// treated as unset; [`load`](crate::config::load) warns about such values
    /// once at startup, since this resolver runs in the per-call hot path and
    /// must not log. The caller must additionally check
    /// [`resolve_compact_enabled`](Self::resolve_compact_enabled), which is the
    /// master switch for all compaction.
    pub fn resolve_mid_turn_compact_threshold(&self) -> Option<f64> {
        match self.mid_turn_compact_threshold {
            Some(t) if t > 0.0 && t <= 1.0 => Some(t),
            _ => None,
        }
    }

    pub fn resolve_max_read_lines(&self) -> u64 {
        self.max_read_lines.unwrap_or(2000)
    }

    /// Returns `None` when no cap is configured — preserves the historical
    /// "no bash output truncation" behaviour.
    pub fn resolve_max_bash_output_lines(&self) -> Option<u64> {
        self.max_bash_output_lines
    }

    /// LSP configuration, `Some` only when an `[lsp]` table exists with
    /// `enabled = true`.
    #[cfg(feature = "lsp")]
    pub fn resolve_lsp(&self) -> Option<&types::LspConfig> {
        self.lsp.as_ref().filter(|l| l.enabled)
    }

    /// rtk output-filtering configuration, `Some` only when an `[rtk]` table
    /// exists with `enabled = true`.
    #[cfg(feature = "rtk")]
    pub fn resolve_rtk(&self) -> Option<&types::RtkConfig> {
        self.rtk.as_ref().filter(|r| r.enabled)
    }

    pub fn resolve_max_grep_results(&self) -> u64 {
        self.max_grep_results.unwrap_or(150)
    }

    pub fn resolve_max_find_results(&self) -> u64 {
        self.max_find_results.unwrap_or(150)
    }

    pub fn resolve_max_list_dir_entries(&self) -> Option<u64> {
        self.max_list_dir_entries.or(Some(150))
    }

    #[cfg(feature = "subagents")]
    pub fn resolve_subagent_max_read_lines(&self) -> u64 {
        self.subagent_max_read_lines.unwrap_or(2000)
    }

    #[cfg(feature = "subagents")]
    pub fn resolve_subagent_max_grep_results(&self) -> u64 {
        self.subagent_max_grep_results.unwrap_or(200)
    }

    #[cfg(feature = "subagents")]
    pub fn resolve_subagent_max_find_results(&self) -> u64 {
        self.subagent_max_find_results.unwrap_or(200)
    }

    #[cfg(feature = "subagents")]
    pub fn resolve_subagent_max_list_dir_entries(&self) -> Option<u64> {
        self.subagent_max_list_dir_entries
    }

    pub fn resolve_always_show_welcome(&self) -> bool {
        self.always_show_welcome.unwrap_or(false)
    }

    pub fn resolve_show_reasoning(&self) -> bool {
        self.show_reasoning.unwrap_or(false)
    }

    pub fn resolve_auto_update_prompts(&self) -> Option<bool> {
        self.auto_update_prompts
    }

    pub fn resolve_auto_update_themes(&self) -> Option<bool> {
        self.auto_update_themes
    }

    pub fn resolve_show_cost_always(&self) -> bool {
        self.show_cost_always.unwrap_or(false)
    }

    /// Look up the quick-model name associated with a prompt in
    /// `[prompt_to_model]`. Returns `None` when the prompt is not mapped or
    /// the value is an empty string (which means "no change").
    pub fn resolve_prompt_model(&self, prompt_name: &str) -> Option<&str> {
        let map = self.prompt_to_model.as_ref()?;
        let val = map.get(prompt_name)?;
        if val.is_empty() {
            None
        } else {
            Some(val.as_str())
        }
    }

    #[cfg(feature = "mcp")]
    pub fn resolve_enable_exa_mcp(&self) -> bool {
        self.enable_exa_mcp.unwrap_or(true)
    }

    #[cfg(feature = "mcp")]
    pub fn resolve_enable_context7_mcp(&self) -> bool {
        self.enable_context7_mcp.unwrap_or(false)
    }

    #[cfg(feature = "mcp")]
    pub fn resolve_enable_grepapp_mcp(&self) -> bool {
        self.enable_grepapp_mcp.unwrap_or(false)
    }

    pub fn build_permission_config(&self) -> PermissionConfigs {
        let glob: PermissionConfig = self
            .permission
            .as_ref()
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let regex: PermissionConfig = self
            .permission_regex
            .as_ref()
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let mut perm_configs = PermissionConfigs { glob, regex };

        if let Some(allow) = &self.permission_allow {
            perm_configs.glob.allow_entries = Some(allow.clone());
        }
        if let Some(ask) = &self.permission_ask {
            perm_configs.glob.ask_entries = Some(ask.clone());
        }
        if let Some(deny) = &self.permission_deny {
            perm_configs.glob.deny_entries = Some(deny.clone());
        }

        perm_configs
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ShowToolDetails {
    Bool(bool),
    Lines(usize),
}

impl Default for ShowToolDetails {
    fn default() -> Self {
        ShowToolDetails::Lines(1)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ResolvedShowToolDetails {
    Off,
    Limited(usize),
    Unlimited,
}

/// Convenience: resolves temperature with all sources (CLI, quick model, global config).
pub fn resolve_temperature(cli: &crate::cli::Cli, cfg: &Config, model_id: &str) -> Option<f64> {
    let qm = quick_models_map_ref(cfg);
    cfg.resolve_temperature(cli, model_id, qm)
}

/// Convenience: resolves extra body params (quick model, global config).
pub fn resolve_extra_body(cfg: &Config, model_id: &str) -> Option<serde_json::Value> {
    let qm = quick_models_map_ref(cfg);
    cfg.resolve_extra_body(model_id, qm)
}

impl ShowToolDetails {
    pub fn resolve(&self) -> ResolvedShowToolDetails {
        match self {
            ShowToolDetails::Bool(false) => ResolvedShowToolDetails::Off,
            ShowToolDetails::Bool(true) => ResolvedShowToolDetails::Unlimited,
            ShowToolDetails::Lines(n) => ResolvedShowToolDetails::Limited(*n),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_config(map: HashMap<String, String>) -> Config {
        Config {
            prompt_to_model: Some(map),
            ..Default::default()
        }
    }

    #[test]
    fn resolve_prompt_model_returns_value_for_known_key() {
        let mut map = HashMap::new();
        map.insert("plan".to_string(), "glm-52".to_string());
        let cfg = make_config(map);
        assert_eq!(cfg.resolve_prompt_model("plan"), Some("glm-52"));
    }

    #[test]
    fn resolve_prompt_model_returns_none_for_unknown_key() {
        let mut map = HashMap::new();
        map.insert("plan".to_string(), "glm-52".to_string());
        let cfg = make_config(map);
        assert_eq!(cfg.resolve_prompt_model("code"), None);
    }

    #[test]
    fn resolve_prompt_model_returns_none_for_empty_string() {
        let mut map = HashMap::new();
        map.insert("brainstorm".to_string(), "".to_string());
        let cfg = make_config(map);
        assert_eq!(cfg.resolve_prompt_model("brainstorm"), None);
    }

    #[test]
    fn resolve_prompt_model_returns_none_when_map_is_none() {
        let cfg = Config {
            prompt_to_model: None,
            ..Default::default()
        };
        assert_eq!(cfg.resolve_prompt_model("plan"), None);
    }

    #[test]
    fn resolve_prompt_model_returns_none_for_empty_map() {
        let cfg = make_config(HashMap::new());
        assert_eq!(cfg.resolve_prompt_model("plan"), None);
    }

    #[test]
    fn toml_deserializes_prompt_to_model() {
        let toml_str = r#"
prompt_to_model = { plan = "glm-52", code = "deepseek-v4-pro", empty_val = "" }
"#;
        let cfg: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.resolve_prompt_model("plan"), Some("glm-52"));
        assert_eq!(cfg.resolve_prompt_model("code"), Some("deepseek-v4-pro"));
        assert_eq!(cfg.resolve_prompt_model("empty_val"), None);
        assert_eq!(cfg.resolve_prompt_model("unknown"), None);
    }

    #[test]
    fn toml_prompt_to_model_with_dotted_syntax() {
        let toml_str = r#"
[prompt_to_model]
plan = "glm-52"
code = "deepseek-v4-pro"
"#;
        let cfg: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.resolve_prompt_model("plan"), Some("glm-52"));
        assert_eq!(cfg.resolve_prompt_model("code"), Some("deepseek-v4-pro"));
    }

    #[test]
    fn default_config_has_no_prompt_to_model() {
        let cfg = Config::default();
        assert_eq!(cfg.resolve_prompt_model("plan"), None);
    }

    #[test]
    fn resolve_mouse_capture_defaults_to_true() {
        let cfg = Config::default();
        assert!(cfg.resolve_mouse_capture());
    }

    #[test]
    fn resolve_pager_prefers_the_config_option_over_the_environment() {
        // The config value is a full command line, args included (`view -`).
        let cfg = Config {
            pager: Some("view -".to_string()),
            ..Default::default()
        };
        assert_eq!(cfg.resolve_pager(), "view -");
    }

    #[test]
    fn toml_deserializes_pager() {
        let toml_str = "pager = \"less -R\"\n";
        let cfg: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.pager.as_deref(), Some("less -R"));
        assert_eq!(cfg.resolve_pager(), "less -R");
    }

    #[test]
    fn resolve_mouse_capture_reads_config_value() {
        let cfg = Config {
            mouse_capture: Some(false),
            ..Default::default()
        };
        assert!(!cfg.resolve_mouse_capture());

        let cfg = Config {
            mouse_capture: Some(true),
            ..Default::default()
        };
        assert!(cfg.resolve_mouse_capture());
    }

    #[test]
    fn toml_deserializes_mouse_capture() {
        let toml_str = r#"
mouse_capture = false
"#;
        let cfg: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.mouse_capture, Some(false));
        assert!(!cfg.resolve_mouse_capture());
    }

    #[test]
    fn swap_enter_and_newline_defaults_off() {
        assert!(!Config::default().resolve_swap_enter_and_newline());
    }

    #[test]
    fn toml_deserializes_swap_enter_and_newline() {
        let cfg: Config = toml::from_str("swap_enter_and_newline = true\n").unwrap();
        assert_eq!(cfg.swap_enter_and_newline, Some(true));
        assert!(cfg.resolve_swap_enter_and_newline());

        let cfg = Config {
            swap_enter_and_newline: Some(false),
            ..Default::default()
        };
        assert!(!cfg.resolve_swap_enter_and_newline());
    }

    #[test]
    fn quit_armed_defaults_off() {
        assert!(!Config::default().resolve_quit_armed());
    }

    #[test]
    fn resolve_quit_armed_reads_config_value() {
        let cfg = Config {
            quit_armed: Some(true),
            ..Default::default()
        };
        assert!(cfg.resolve_quit_armed());

        let cfg = Config {
            quit_armed: Some(false),
            ..Default::default()
        };
        assert!(!cfg.resolve_quit_armed());
    }

    #[test]
    fn toml_deserializes_quit_armed() {
        let cfg: Config = toml::from_str("quit_armed = true\n").unwrap();
        assert_eq!(cfg.quit_armed, Some(true));
        assert!(cfg.resolve_quit_armed());
    }

    #[test]
    fn esc_alt_compat_defaults_off() {
        assert!(!Config::default().resolve_esc_alt_compat());
    }

    #[test]
    fn esc_alt_compat_reads_config_value() {
        let cfg = Config {
            esc_alt_compat: Some(true),
            ..Default::default()
        };
        assert!(cfg.resolve_esc_alt_compat());

        let cfg: Config = toml::from_str("esc_alt_compat = true\n").unwrap();
        assert_eq!(cfg.esc_alt_compat, Some(true));
        assert!(cfg.resolve_esc_alt_compat());
    }

    #[test]
    fn clipboard_selection_defaults_to_clipboard() {
        assert_eq!(
            Config::default().resolve_clipboard_selection(),
            types::ClipboardSelection::Clipboard
        );
    }

    #[test]
    fn clipboard_selection_reads_config_value() {
        let cfg = Config {
            clipboard_selection: Some(types::ClipboardSelection::Primary),
            ..Default::default()
        };
        assert_eq!(
            cfg.resolve_clipboard_selection(),
            types::ClipboardSelection::Primary
        );
    }

    #[test]
    fn toml_deserializes_clipboard_selection() {
        let cfg: Config = toml::from_str("clipboard_selection = \"primary\"\n").unwrap();
        assert_eq!(
            cfg.clipboard_selection,
            Some(types::ClipboardSelection::Primary)
        );
        assert_eq!(
            cfg.resolve_clipboard_selection(),
            types::ClipboardSelection::Primary
        );

        let cfg: Config = toml::from_str("clipboard_selection = \"clipboard\"\n").unwrap();
        assert_eq!(
            cfg.resolve_clipboard_selection(),
            types::ClipboardSelection::Clipboard
        );
    }

    #[test]
    fn clipboard_selection_rejects_unknown_value() {
        assert!(toml::from_str::<Config>("clipboard_selection = \"nope\"\n").is_err());
    }
}
