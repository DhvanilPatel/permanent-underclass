use std::collections::HashMap;
use std::fmt;
use std::sync::OnceLock;

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};

use crate::engine::conditions::Condition;
use crate::engine::subscription::SubscriptionTier;
use crate::engine::types::Stats;

const CHARACTERS_JSONL: &str = include_str!("../../data/characters.jsonl");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Ability {
    VibeCode,
    Rebrand,
    GoldenHandcuffs,
    GoViral,
    Interview,
    BaumolsRevenge,
    ShipIt,
    DeployCapital,
    Prioritize,
    PostBanger,
    Publish,
    ApeIn,
    Restructure,
    GeoArbitrage,
    Unblock,
    #[serde(other)]
    Unknown,
}

impl Ability {
    pub fn label(self) -> &'static str {
        match self {
            Ability::VibeCode => "VIBE CODE",
            Ability::Rebrand => "REBRAND",
            Ability::GoldenHandcuffs => "GOLDEN HANDCUFFS",
            Ability::GoViral => "GO VIRAL",
            Ability::Interview => "INTERVIEW",
            Ability::BaumolsRevenge => "BAUMOL'S REVENGE",
            Ability::ShipIt => "SHIP IT",
            Ability::DeployCapital => "DEPLOY CAPITAL",
            Ability::Prioritize => "PRIORITIZE",
            Ability::PostBanger => "POST BANGER",
            Ability::Publish => "PUBLISH",
            Ability::ApeIn => "APE IN",
            Ability::Restructure => "RESTRUCTURE",
            Ability::GeoArbitrage => "GEO ARBITRAGE",
            Ability::Unblock => "UNBLOCK",
            Ability::Unknown => "SPECIAL",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CharacterId(String);

impl CharacterId {
    pub fn new(raw: impl AsRef<str>) -> Self {
        Self(normalize_character_id(raw.as_ref()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn id(&self) -> &str {
        self.as_str()
    }

    pub fn label(&self) -> String {
        CharacterCatalog::embedded()
            .ok()
            .and_then(|catalog| catalog.get(self))
            .map(|character| character.label.clone())
            .unwrap_or_else(|| self.to_cli_slug())
    }

    pub fn to_cli_slug(&self) -> String {
        self.0.replace('_', "-")
    }

    pub fn matches(&self, raw: &str) -> bool {
        self.0 == normalize_character_id(raw)
    }
}

impl fmt::Display for CharacterId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for CharacterId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for CharacterId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterPresentation {
    pub tag: String,
    pub quote: String,
    pub backstory: String,
    pub accent_rgb: [u8; 3],
    #[serde(default)]
    pub sprite_style: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterDef {
    pub id: CharacterId,
    pub label: String,
    #[serde(default)]
    pub description: Option<String>,
    pub starting_stats: Stats,
    pub starting_income_per_turn: i64,
    #[serde(default = "default_rent")]
    pub starting_rent_per_turn: i64,
    pub starting_subscription: SubscriptionTier,
    #[serde(default)]
    pub starting_conditions: Vec<Condition>,
    pub ability: Ability,
    pub origin_line: String,
    pub event_catalog: String,
    pub presentation: CharacterPresentation,
}

impl CharacterDef {
    pub fn for_character(id: CharacterId) -> Self {
        if let Ok(catalog) = CharacterCatalog::embedded() {
            if let Some(character) = catalog.get(&id) {
                return character.clone();
            }
            if let Some(default) = catalog.all().first() {
                eprintln!(
                    "WARN: unknown character id '{}'; falling back to '{}'",
                    id, default.id
                );
                return default.clone();
            }
        }

        eprintln!(
            "WARN: character catalog unavailable; using hardcoded fallback for '{}'",
            id
        );
        Self::hardcoded_fallback(id)
    }

    fn hardcoded_fallback(id: CharacterId) -> Self {
        Self {
            id,
            label: "Fallback Character".to_string(),
            description: Some("Character catalog failed to load; using safe defaults.".to_string()),
            starting_stats: Stats {
                money: 10_000,
                sanity: 60,
                audience: 20,
                ai_skill: 40,
                friends: 40,
            },
            starting_income_per_turn: 1500,
            starting_rent_per_turn: 500,
            starting_subscription: SubscriptionTier::Free,
            starting_conditions: Vec::new(),
            ability: Ability::Unknown,
            origin_line: "Loaded fallback character defaults.".to_string(),
            event_catalog: "fallback".to_string(),
            presentation: CharacterPresentation {
                tag: "FALLBACK".to_string(),
                quote: "\"data not found\"".to_string(),
                backstory: "Character data failed to parse; running with emergency defaults."
                    .to_string(),
                accent_rgb: [180, 180, 180],
                sprite_style: None,
            },
        }
    }
}

#[derive(Debug)]
pub struct CharacterCatalog {
    ordered: Vec<CharacterDef>,
    by_id: HashMap<CharacterId, usize>,
}

static CHARACTER_CATALOG: OnceLock<CharacterCatalog> = OnceLock::new();

impl CharacterCatalog {
    pub fn embedded() -> Result<&'static CharacterCatalog> {
        if let Some(c) = CHARACTER_CATALOG.get() {
            return Ok(c);
        }
        let parsed = Self::from_jsonl(CHARACTERS_JSONL)?;
        let _ = CHARACTER_CATALOG.set(parsed);
        Ok(CHARACTER_CATALOG
            .get()
            .expect("character catalog initialized"))
    }

    fn from_jsonl(raw: &str) -> Result<CharacterCatalog> {
        let mut ordered: Vec<CharacterDef> = Vec::new();
        for (idx, raw_line) in raw.lines().enumerate() {
            let line_no = idx + 1;
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let character: CharacterDef = serde_json::from_str(line)
                .with_context(|| format!("Failed to parse data/characters.jsonl:{}", line_no))?;
            ordered.push(character);
        }

        if ordered.is_empty() {
            return Err(anyhow!(
                "data/characters.jsonl must contain at least one character"
            ));
        }

        let mut by_id = HashMap::new();
        for (idx, character) in ordered.iter_mut().enumerate() {
            character.id = CharacterId::new(character.id.as_str());
            if character.event_catalog.trim().is_empty() {
                return Err(anyhow!(
                    "Character '{}' has an empty event_catalog",
                    character.id
                ));
            }
            if by_id.insert(character.id.clone(), idx).is_some() {
                return Err(anyhow!(
                    "Duplicate character id '{}' in data/characters.jsonl",
                    character.id
                ));
            }

            // Compatibility alias: allow selecting by event-catalog id when it differs
            // from the canonical character id (for renamed characters).
            let catalog_alias = CharacterId::new(&character.event_catalog);
            by_id.entry(catalog_alias).or_insert(idx);
        }

        Ok(CharacterCatalog { ordered, by_id })
    }

    pub fn all(&self) -> &[CharacterDef] {
        &self.ordered
    }

    pub fn get(&self, id: &CharacterId) -> Option<&CharacterDef> {
        let idx = *self.by_id.get(id)?;
        self.ordered.get(idx)
    }

    pub fn parse_id(&self, raw: &str) -> Option<CharacterId> {
        let id = CharacterId::new(raw);
        if self.by_id.contains_key(&id) {
            Some(id)
        } else {
            None
        }
    }

    pub fn default_id(&self) -> CharacterId {
        self.ordered
            .first()
            .expect("character catalog is non-empty")
            .id
            .clone()
    }

    pub fn cli_ids(&self) -> Vec<String> {
        self.ordered.iter().map(|c| c.id.to_cli_slug()).collect()
    }

    pub fn all_ids(&self) -> Vec<CharacterId> {
        self.ordered.iter().map(|c| c.id.clone()).collect()
    }
}

fn default_rent() -> i64 {
    500
}

fn normalize_character_id(raw: &str) -> String {
    raw.trim()
        .to_ascii_lowercase()
        .chars()
        .map(|ch| match ch {
            '-' | ' ' => '_',
            _ => ch,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_parses_and_has_unique_ids() {
        let catalog = CharacterCatalog::embedded().expect("character catalog");
        assert!(!catalog.all().is_empty());
        // by_id may contain compatibility aliases (event_catalog names that differ
        // from the canonical character id), so it can be >= ordered.len().
        assert!(catalog.by_id.len() >= catalog.all().len());
        // Every character's canonical id should be in by_id.
        for c in catalog.all() {
            assert!(catalog.by_id.contains_key(&c.id), "missing id {:?}", c.id);
        }
    }

    #[test]
    fn parse_id_accepts_kebab_or_snake_case() {
        let catalog = CharacterCatalog::embedded().expect("character catalog");
        let a = catalog.parse_id("junior-dev").expect("junior-dev");
        let b = catalog.parse_id("junior_dev").expect("junior_dev");
        assert_eq!(a, b);
    }
}
