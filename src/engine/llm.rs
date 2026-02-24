use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::engine::content::FlagRegistry;
use crate::engine::events::{MemoryEffect, TurnEvents};
use crate::engine::game::GameState;
use crate::engine::types::ResourceDelta;

const MAX_CODEX_OUTPUT_BYTES: usize = 8 * 1024;
const FREEFORM_SCHEMA_JSON: &str = include_str!("../../llm/freeform.schema.json");

#[derive(Debug, Clone)]
pub enum LlmNarrator {
    Codex(CodexNarrator),
    Claude(ClaudeNarrator),
}

impl LlmNarrator {
    pub fn codex() -> Self {
        Self::Codex(CodexNarrator::new())
    }

    pub fn claude() -> Self {
        Self::Claude(ClaudeNarrator::new())
    }

    pub fn with_model(self, model: impl Into<String>) -> Self {
        let model = model.into();
        match self {
            Self::Codex(n) => Self::Codex(n.with_model(model)),
            Self::Claude(n) => Self::Claude(n.with_model(model)),
        }
    }

    pub fn with_reasoning_effort(self, effort: impl Into<String>) -> Self {
        let effort = effort.into();
        match self {
            Self::Codex(n) => Self::Codex(n.with_reasoning_effort(effort)),
            Self::Claude(n) => Self::Claude(n.with_reasoning_effort(effort)),
        }
    }

    pub fn resolve_freeform(
        &self,
        state: &GameState,
        events: &TurnEvents,
        action: &str,
    ) -> Result<FreeformResolution> {
        match self {
            Self::Codex(n) => n.resolve_freeform(state, events, action),
            Self::Claude(n) => n.resolve_freeform(state, events, action),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CodexNarrator {
    workdir: PathBuf,
    schema_freeform: PathBuf,

    model: Option<String>,
    reasoning_effort: String,
}

impl Default for CodexNarrator {
    fn default() -> Self {
        Self::new()
    }
}

impl CodexNarrator {
    pub fn new() -> Self {
        let workdir = std::env::temp_dir().join("permanent_underclass_codex");
        let _ = fs::create_dir_all(&workdir);

        // `codex exec --output-schema` needs a filesystem path.
        let schema_freeform = workdir.join("freeform.schema.json");
        let _ = fs::write(&schema_freeform, FREEFORM_SCHEMA_JSON);

        Self {
            workdir,
            schema_freeform,
            model: None,
            reasoning_effort: "medium".to_string(),
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn with_reasoning_effort(mut self, effort: impl Into<String>) -> Self {
        self.reasoning_effort = effort.into();
        self
    }

    pub fn resolve_freeform(
        &self,
        state: &GameState,
        events: &TurnEvents,
        action: &str,
    ) -> Result<FreeformResolution> {
        let prompt = prompt_freeform(state, events, action);
        let json = self.run_codex_json(&self.schema_freeform, &prompt)?;
        let parsed: FreeformResolutionWire = serde_json::from_str(&json)
            .with_context(|| format!("Invalid JSON from Codex (freeform): {}", truncate(&json)))?;

        Ok(FreeformResolution {
            title: normalize_freeform_title(&parsed.title),
            delta: clamp_delta_base(parsed.delta.into()),
            effects: parsed.effects,
            narrative: normalize_freeform_narrative(&parsed.narrative),
            summary_line: parsed.summary_line,
        })
    }

    fn run_codex_json(&self, schema: &Path, prompt: &str) -> Result<String> {
        let out_file = temp_out_path("permanent_underclass_codex_last_message");

        let mut cmd = Command::new("codex");
        cmd.arg("exec")
            .arg("--ephemeral")
            .arg("-C")
            .arg(&self.workdir)
            .arg("--skip-git-repo-check")
            .arg("-s")
            .arg("read-only")
            .arg("-c")
            .arg("approval_policy=\"never\"")
            .arg("-c")
            .arg("suppress_unstable_features_warning=true")
            .arg("-c")
            .arg("mcp_servers.blender.enabled=false")
            .arg("-c")
            .arg(format!(
                "model_reasoning_effort=\"{}\"",
                self.reasoning_effort
            ))
            .arg("--output-schema")
            .arg(schema)
            .arg("--output-last-message")
            .arg(&out_file)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(model) = &self.model {
            cmd.arg("--model").arg(model);
        }

        cmd.arg(prompt);

        let output = cmd.output().context("Failed to start `codex exec`")?;
        if !output.status.success() {
            return Err(anyhow!(
                "codex exec failed (exit={:?}) stdout={} stderr={}",
                output.status.code(),
                truncate_bytes(&output.stdout),
                truncate_bytes(&output.stderr),
            ));
        }

        fs::read_to_string(&out_file)
            .with_context(|| format!("Failed to read Codex output file: {}", out_file.display()))
    }
}

#[derive(Debug, Clone)]
pub struct ClaudeNarrator {
    model: Option<String>,
    reasoning_effort: String,
}

impl Default for ClaudeNarrator {
    fn default() -> Self {
        Self::new()
    }
}

impl ClaudeNarrator {
    pub fn new() -> Self {
        Self {
            model: None,
            reasoning_effort: "medium".to_string(),
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn with_reasoning_effort(mut self, effort: impl Into<String>) -> Self {
        self.reasoning_effort = effort.into();
        self
    }

    pub fn resolve_freeform(
        &self,
        state: &GameState,
        events: &TurnEvents,
        action: &str,
    ) -> Result<FreeformResolution> {
        let prompt = prompt_freeform(state, events, action);
        let parsed: FreeformResolutionWire =
            self.run_claude_structured(FREEFORM_SCHEMA_JSON, &prompt)?;

        Ok(FreeformResolution {
            title: normalize_freeform_title(&parsed.title),
            delta: clamp_delta_base(parsed.delta.into()),
            effects: parsed.effects,
            narrative: normalize_freeform_narrative(&parsed.narrative),
            summary_line: parsed.summary_line,
        })
    }

    fn run_claude_structured<T: DeserializeOwned>(&self, schema: &str, prompt: &str) -> Result<T> {
        let mut cmd = Command::new("claude");
        cmd.arg("-p")
            .arg("--output-format")
            .arg("json")
            .arg("--no-session-persistence")
            .arg("--permission-mode")
            .arg("dontAsk")
            // Claude Code's default system prompt is agentic and can add overhead. We just want
            // strict structured output.
            .arg("--system-prompt")
            .arg(
                "Return ONLY a valid JSON object that matches the provided JSON schema. \
No markdown, no code fences, no explanation. Do not use tools.",
            )
            // Disable all tools. Prevents Claude Code from trying to read/write files or run commands.
            .arg("--tools")
            .arg("")
            .arg("--json-schema")
            .arg(schema)
            .arg("--effort")
            .arg(map_effort_claude(&self.reasoning_effort))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(model) = &self.model {
            cmd.arg("--model").arg(model);
        }

        cmd.arg(prompt);

        let output = cmd.output().context("Failed to start `claude`")?;
        if !output.status.success() {
            return Err(anyhow!(
                "claude failed (exit={:?}) stdout={} stderr={}",
                output.status.code(),
                truncate_bytes(&output.stdout),
                truncate_bytes(&output.stderr),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let parsed: ClaudePrintEnvelope<T> = serde_json::from_str(&stdout)
            .with_context(|| format!("Invalid JSON from Claude CLI: {}", truncate(&stdout)))?;
        parsed
            .structured_output
            .ok_or_else(|| anyhow!("Claude did not produce structured_output"))
    }
}

#[derive(Debug, Clone)]
pub struct FreeformResolution {
    pub title: String,
    pub delta: ResourceDelta,
    pub effects: Vec<MemoryEffect>,
    pub narrative: String,
    pub summary_line: String,
}

#[derive(Debug, Deserialize)]
struct FreeformResolutionWire {
    title: String,
    delta: ResourceDeltaWire,
    #[serde(default)]
    effects: Vec<MemoryEffect>,
    narrative: String,
    summary_line: String,
}

#[derive(Debug, Deserialize)]
struct ResourceDeltaWire {
    money: i64,
    sanity: i32,
    audience: i32,
    ai_skill: i32,
    friends: i32,
}

impl From<ResourceDeltaWire> for ResourceDelta {
    fn from(v: ResourceDeltaWire) -> Self {
        Self {
            money: v.money,
            sanity: v.sanity,
            audience: v.audience,
            ai_skill: v.ai_skill,
            friends: v.friends,
        }
    }
}

fn clamp_delta_base(mut d: ResourceDelta) -> ResourceDelta {
    // These are base deltas. The engine may scale and apply condition modifiers.
    d.money = d.money.clamp(-15_000, 15_000);
    d.sanity = d.sanity.clamp(-14, 14);
    d.audience = d.audience.clamp(-14, 14);
    d.ai_skill = d.ai_skill.clamp(-14, 14);
    d.friends = d.friends.clamp(-14, 14);
    d
}

fn normalize_freeform_title(raw: &str) -> String {
    let title = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if title.is_empty() {
        return "WEEKLY RESULT".to_string();
    }
    title
}

fn normalize_freeform_narrative(raw: &str) -> String {
    let narrative = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if narrative.is_empty() {
        return "You hesitate, and the moment passes.".to_string();
    }
    narrative
}

fn prompt_freeform(state: &GameState, events: &TurnEvents, action: &str) -> String {
    let mut s = String::new();
    s.push_str(
        "You are the narrator and referee for Permanent Underclass.\n\
The player typed a freeform action. Interpret it plausibly given the player's state.\n\
Return a short narrative plus a BASE delta (the engine may scale and apply condition modifiers).\n\
The `title` field must be a punchy headline (3-7 words).\n\
The `narrative` field must be one short paragraph (2-3 sentences) with no line breaks.\n\
Output MUST be valid JSON that matches the provided schema. Do not include any extra keys.\n\n",
    );

    s.push_str("HARD STATE:\n");
    s.push_str(&format!(
        "- Turn: {} / {} ({})\n",
        state.turn,
        state.max_turns,
        state.turn_label()
    ));
    s.push_str(&format!("- Year: {}\n", state.year()));
    s.push_str(&format!("- Character: {}\n", state.character.label()));
    s.push_str(&format!("- AI Capability: {}\n", state.ai.percent_string()));
    s.push_str(&format!("- Subscription: {}\n", state.subscription.label()));
    s.push_str(&format!(
        "- Stats: money=${} sanity={} audience={} ai_skill={} friends={}\n",
        state.stats.money,
        state.stats.sanity,
        state.stats.audience,
        state.stats.ai_skill,
        state.stats.friends
    ));
    let conds = state.active_condition_labels();
    if conds.is_empty() {
        s.push_str("- Conditions: []\n");
    } else {
        s.push_str(&format!("- Conditions: [{}]\n", conds.join(", ")));
    }
    let mut memory_flags: Vec<&str> = state.run_memory.flags.iter().map(|f| f.as_str()).collect();
    memory_flags.sort_unstable();
    if memory_flags.is_empty() {
        s.push_str("- Memory flags: []\n");
    } else {
        s.push_str(&format!("- Memory flags: [{}]\n", memory_flags.join(", ")));
    }

    if let Ok(registry) = FlagRegistry::embedded() {
        let known = registry.known_flags_sorted();
        let reserved = registry.reserved_flags_sorted();
        if known.is_empty() {
            s.push_str("- Valid flags: []\n");
        } else {
            s.push_str(&format!("- Valid flags: [{}]\n", known.join(", ")));
        }
        if reserved.is_empty() {
            s.push_str("- Reserved flags: []\n");
        } else {
            s.push_str(&format!("- Reserved flags: [{}]\n", reserved.join(", ")));
        }
    }
    s.push('\n');

    s.push_str("CONTEXT:\n");
    s.push_str(&format!(
        "WORLD: {} — {}\n",
        events.world.title,
        events.world.body.replace('\n', " ")
    ));
    s.push_str(&format!(
        "PERSONAL: {} — {}\n",
        events.personal.title,
        events.personal.body.replace('\n', " ")
    ));
    s.push('\n');

    s.push_str("PLAYER ACTION (verbatim):\n");
    s.push_str(action);
    s.push_str("\n\n");

    s.push_str(
        "DELTA GUIDELINES (base):\n\
- money: roughly -2000..+5000 for normal actions, up to +/-15000 for high-risk swings.\n\
- sanity/audience/ai_skill/friends: roughly -12..+12.\n\
- If the action is impossible or delusional, narrate the failure and use a small negative delta.\n\
- Every action should include at least one upside and one downside when plausible.\n\
- effects are optional; set at most 2 and only when directly implied by the action.\n",
    );

    s
}

fn temp_out_path(prefix: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let pid = std::process::id();
    std::env::temp_dir().join(format!("{}_{}_{}.json", prefix, pid, ts))
}

fn truncate(s: &str) -> String {
    const MAX_CHARS: usize = 300;
    if s.chars().count() <= MAX_CHARS {
        return s.to_string();
    }
    let prefix: String = s.chars().take(MAX_CHARS).collect();
    format!("{prefix}…")
}

fn truncate_bytes(bytes: &[u8]) -> String {
    let slice = if bytes.len() <= MAX_CODEX_OUTPUT_BYTES {
        bytes
    } else {
        &bytes[..MAX_CODEX_OUTPUT_BYTES]
    };
    String::from_utf8_lossy(slice).to_string()
}

fn map_effort_claude(effort: &str) -> &'static str {
    match effort {
        "low" => "low",
        "high" => "high",
        // Claude Code supports low/medium/high; treat xhigh as high.
        "xhigh" => "high",
        _ => "medium",
    }
}

#[derive(Debug, Deserialize)]
struct ClaudePrintEnvelope<T> {
    structured_output: Option<T>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn freeform_schema_matches_current_stats() {
        let v: serde_json::Value = serde_json::from_str(FREEFORM_SCHEMA_JSON).unwrap();
        let required = v["properties"]["delta"]["required"]
            .as_array()
            .expect("schema delta.required must be an array");
        let req: std::collections::BTreeSet<&str> =
            required.iter().filter_map(|x| x.as_str()).collect();

        for k in ["money", "sanity", "audience", "ai_skill", "friends"] {
            assert!(req.contains(k), "schema missing required key '{}'", k);
        }
    }

    #[test]
    fn truncate_handles_multibyte_utf8() {
        let s = "🙂".repeat(350);
        let out = truncate(&s);
        assert_eq!(out.chars().count(), 301);
        assert!(out.ends_with('…'));
    }
}
