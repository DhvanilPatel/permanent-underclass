use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::OnceLock;

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};

use crate::engine::ai::Phase;
use crate::engine::characters::{CharacterCatalog, CharacterId};
use crate::engine::events::MemoryEffect;
use crate::engine::types::ResourceDelta;

const FLAGS_JSONL: &str = include_str!("../../data/flags.jsonl");
const EVENTS_DIR_ENV: &str = "PU_EVENTS_DIR";
const BUILD_EVENTS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/data/events");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub enum PhaseGate {
    Any,
    Early,
    Mid,
    Late,
    Endgame,
}

impl PhaseGate {
    pub fn matches(self, phase: Phase) -> bool {
        match self {
            PhaseGate::Any => true,
            PhaseGate::Early => phase == Phase::Early,
            PhaseGate::Mid => phase == Phase::Mid,
            PhaseGate::Late => phase == Phase::Late,
            PhaseGate::Endgame => phase == Phase::Endgame,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChoiceDef {
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub delta: ResourceDelta,
    #[serde(default, alias = "min_cash")]
    pub min_money: Option<i64>,
    #[serde(default)]
    pub effects: Vec<MemoryEffect>,
    // Content safety valve: tolerate accidentally nested gating metadata
    // without crashing runtime parsing.
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub forbids: Vec<String>,
}

fn default_allow_freeform() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EventDef {
    pub id: String,
    pub phase: PhaseGate,
    pub title: String,
    pub body: String,
    pub choices: [ChoiceDef; 3],
    #[serde(default = "default_allow_freeform")]
    pub allow_freeform: bool,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub forbids: Vec<String>,
    #[serde(default)]
    pub once_per_run: bool,
}

#[derive(Debug)]
pub struct Content {
    pub universal: Vec<EventDef>,
    pub per_character: HashMap<CharacterId, Vec<EventDef>>,
}

static EMBEDDED: OnceLock<Content> = OnceLock::new();
static FLAG_REGISTRY: OnceLock<FlagRegistry> = OnceLock::new();

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FlagSpec {
    flag: String,
    #[serde(default)]
    group: Option<String>,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default = "default_true")]
    llm_settable: bool,
    #[serde(default)]
    reserved: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct FlagRegistry {
    known: HashSet<String>,
    llm_settable: HashSet<String>,
    labels: HashMap<String, String>,
}

fn default_true() -> bool {
    true
}

impl FlagRegistry {
    pub fn embedded() -> Result<&'static FlagRegistry> {
        if let Some(r) = FLAG_REGISTRY.get() {
            return Ok(r);
        }
        let parsed = parse_flag_registry(FLAGS_JSONL)?;
        let _ = FLAG_REGISTRY.set(parsed);
        Ok(FLAG_REGISTRY.get().expect("flag registry initialized"))
    }

    pub fn is_known(&self, flag: &str) -> bool {
        self.known.contains(flag)
    }

    pub fn is_llm_settable(&self, flag: &str) -> bool {
        self.llm_settable.contains(flag)
    }

    pub fn known_flags_sorted(&self) -> Vec<&str> {
        let mut out: Vec<&str> = self.known.iter().map(|f| f.as_str()).collect();
        out.sort_unstable();
        out
    }

    pub fn reserved_flags_sorted(&self) -> Vec<&str> {
        let mut out: Vec<&str> = self
            .known
            .iter()
            .filter(|f| !self.llm_settable.contains(*f))
            .map(|f| f.as_str())
            .collect();
        out.sort_unstable();
        out
    }

    pub fn label(&self, flag: &str) -> Option<&str> {
        self.labels.get(flag).map(String::as_str)
    }
}

impl Content {
    pub fn embedded() -> Result<&'static Content> {
        if let Some(c) = EMBEDDED.get() {
            return Ok(c);
        }
        let parsed = Self::from_data_files()?;
        let _ = EMBEDDED.set(parsed);
        Ok(EMBEDDED.get().expect("embedded content initialized"))
    }

    fn from_data_files() -> Result<Content> {
        let catalog = CharacterCatalog::embedded()?;
        let events_dir = resolve_events_dir()?;

        let mut per_character = HashMap::new();
        for character in catalog.all() {
            let path = events_dir.join(format!("{}.jsonl", character.event_catalog));
            let events = match fs::read_to_string(&path) {
                Ok(jsonl) => {
                    let label = path.display().to_string();
                    parse_events_jsonl(&label, &jsonl)?
                }
                Err(err) if err.kind() == ErrorKind::NotFound => {
                    eprintln!(
                        "WARN: missing event catalog '{}' for character '{}'; using universal/fallback events only",
                        path.display(),
                        character.id
                    );
                    Vec::new()
                }
                Err(err) => {
                    return Err(err).with_context(|| format!("Failed to read {}", path.display()));
                }
            };
            per_character.insert(character.id.clone(), events);
        }

        let universal_path = events_dir.join("universal.jsonl");
        let universal_jsonl = fs::read_to_string(&universal_path)
            .with_context(|| format!("Failed to read {}", universal_path.display()))?;
        let universal_label = universal_path.display().to_string();
        let universal = parse_events_jsonl(&universal_label, &universal_jsonl)?;

        validate_unique_ids(&universal, per_character.values().flatten())?;
        let flags = FlagRegistry::embedded()?;
        validate_flag_references(&universal, per_character.values().flatten(), flags)?;

        Ok(Content {
            universal,
            per_character,
        })
    }
}

fn resolve_events_dir() -> Result<PathBuf> {
    for candidate in events_dir_candidates() {
        if candidate.join("universal.jsonl").is_file() {
            return Ok(candidate);
        }
    }
    Err(anyhow!(
        "Could not locate event catalogs. Checked PU_EVENTS_DIR, executable-relative data/events, current-dir data/events, and build-path {}",
        BUILD_EVENTS_DIR
    ))
}

fn events_dir_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();

    if let Ok(raw) = std::env::var(EVENTS_DIR_ENV) {
        let candidate = normalize_path(raw.trim());
        if !candidate.as_os_str().is_empty() {
            out.push(candidate);
        }
    }

    if let Ok(exe) = std::env::current_exe()
        && let Some(root) = exe.parent().and_then(|p| p.parent())
    {
        out.push(root.join("data/events"));
    }

    if let Ok(cwd) = std::env::current_dir() {
        out.push(cwd.join("data/events"));
    }

    out.push(PathBuf::from(BUILD_EVENTS_DIR));
    out
}

fn normalize_path(raw: &str) -> PathBuf {
    if let Some(stripped) = raw.strip_prefix("file://") {
        PathBuf::from(stripped)
    } else {
        PathBuf::from(raw)
    }
}

fn parse_events_jsonl(label: &str, jsonl: &str) -> Result<Vec<EventDef>> {
    let mut out = Vec::new();
    for (idx, raw) in jsonl.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let ev: EventDef = serde_json::from_str(line)
            .with_context(|| format!("Failed to parse JSONL: {}:{}", label, line_no))?;
        out.push(ev);
    }
    Ok(out)
}

fn validate_unique_ids<'a>(
    universal: &[EventDef],
    character_events: impl IntoIterator<Item = &'a EventDef>,
) -> Result<()> {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for e in universal {
        if !seen.insert(&e.id) {
            return Err(anyhow!("Duplicate event id '{}'", e.id));
        }
    }
    for e in character_events {
        if !seen.insert(&e.id) {
            return Err(anyhow!("Duplicate event id '{}'", e.id));
        }
    }
    Ok(())
}

fn parse_flag_registry(raw: &str) -> Result<FlagRegistry> {
    let mut known = HashSet::new();
    let mut llm_settable = HashSet::new();
    let mut labels = HashMap::new();
    for (idx, raw_line) in raw.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let spec: FlagSpec = serde_json::from_str(line)
            .with_context(|| format!("Failed to parse data/flags.jsonl:{}", line_no))?;
        let flag = spec.flag.trim();
        if flag.is_empty() {
            return Err(anyhow!("data/flags.jsonl contains an empty flag name"));
        }
        if !known.insert(flag.to_string()) {
            return Err(anyhow!("Duplicate flag '{}' in data/flags.jsonl", flag));
        }
        let is_llm_settable = if spec.reserved.unwrap_or(false) {
            false
        } else {
            spec.llm_settable
        };
        if is_llm_settable {
            llm_settable.insert(flag.to_string());
        }
        if let Some(label) = spec
            .label
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            labels.insert(flag.to_string(), label.to_string());
        }
    }
    Ok(FlagRegistry {
        known,
        llm_settable,
        labels,
    })
}

fn validate_flag_references<'a>(
    universal: &[EventDef],
    character_events: impl IntoIterator<Item = &'a EventDef>,
    flags: &FlagRegistry,
) -> Result<()> {
    for ev in universal {
        validate_event_flags(ev, flags)?;
    }
    for ev in character_events {
        validate_event_flags(ev, flags)?;
    }
    Ok(())
}

fn validate_event_flags(ev: &EventDef, flags: &FlagRegistry) -> Result<()> {
    for flag in &ev.requires {
        validate_known_flag(flags, flag, &ev.id, "requires")?;
    }
    for flag in &ev.forbids {
        validate_known_flag(flags, flag, &ev.id, "forbids")?;
    }
    for (idx, choice) in ev.choices.iter().enumerate() {
        for effect in &choice.effects {
            validate_known_flag(flags, &effect.flag, &ev.id, "effects")?;
            if effect.flag.trim().is_empty() {
                return Err(anyhow!(
                    "Event '{}' choice {} has empty effects flag",
                    ev.id,
                    idx + 1
                ));
            }
        }
    }
    Ok(())
}

fn validate_known_flag(
    flags: &FlagRegistry,
    flag: &str,
    event_id: &str,
    field: &str,
) -> Result<()> {
    if !flags.is_known(flag) {
        return Err(anyhow!(
            "Event '{}' references unknown flag '{}' in {}",
            event_id,
            flag,
            field
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_content_parses() {
        let c = Content::embedded().unwrap();
        let catalog = CharacterCatalog::embedded().unwrap();
        assert_eq!(c.per_character.len(), catalog.all().len());
    }

    #[test]
    fn per_character_has_endgame_events() {
        let c = Content::embedded().unwrap();
        let catalog = CharacterCatalog::embedded().unwrap();
        for character in catalog.all() {
            let evs = c.per_character.get(&character.id).unwrap();
            assert!(
                evs.iter().any(|e| e.phase == PhaseGate::Endgame),
                "missing Endgame events for character {:?}",
                character.id
            );
        }
    }

    #[test]
    fn ids_are_non_empty() {
        let c = Content::embedded().unwrap();
        for e in c.universal.iter().chain(c.per_character.values().flatten()) {
            assert!(!e.id.trim().is_empty());
        }
    }

    #[test]
    fn phase_gate_matches() {
        assert!(PhaseGate::Any.matches(Phase::Early));
        assert!(PhaseGate::Early.matches(Phase::Early));
        assert!(!PhaseGate::Early.matches(Phase::Mid));
    }
}
