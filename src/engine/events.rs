use crate::engine::types::ResourceDelta;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldEventId {
    Atmosphere,
    Milestone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryEffectOp {
    SetFlag,
    ClearFlag,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryEffect {
    pub op: MemoryEffectOp,
    pub flag: String,
}

#[derive(Debug, Clone)]
pub struct Choice {
    pub key: char,
    pub title: String,
    pub body: String,
    pub delta: ResourceDelta,
    pub min_money: Option<i64>,
    pub effects: Vec<MemoryEffect>,
}

impl Choice {
    pub fn is_available(&self, current_money: i64) -> bool {
        match self.min_money {
            Some(min) => current_money >= min,
            None => true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldEventKind {
    Atmosphere,
    Modifier,
    Milestone,
}

#[derive(Debug, Clone)]
pub struct WorldEvent {
    pub id: WorldEventId,
    pub kind: WorldEventKind,
    pub title: String,
    pub body: String,
    pub delta: ResourceDelta,
}

#[derive(Debug, Clone)]
pub struct PersonalEvent {
    pub id: String,
    pub title: String,
    pub body: String,
    pub choices: Vec<Choice>,
    pub allow_freeform: bool,
}

#[derive(Debug, Clone)]
pub struct TurnEvents {
    pub world: WorldEvent,
    pub personal: PersonalEvent,
}

#[derive(Debug, Clone)]
pub struct TurnOutcome {
    pub narrative: String,
    pub delta: ResourceDelta,
    pub summary_line: String,
    pub tag_changes: Vec<MemoryEffect>,
}
