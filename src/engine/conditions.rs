use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Condition {
    BurnedOut,
    ImposterSyndrome,
    LinkedinBrain,
    HustleMode,
    MainCharacter { remaining_turns: u32 },
    Doomer,
    TouchingGrass,
    VibeCoding { remaining_turns: u32 },
}

impl Condition {
    pub fn label(&self) -> &'static str {
        match self {
            Condition::BurnedOut => "BURNED OUT",
            Condition::ImposterSyndrome => "IMPOSTER SYNDROME",
            Condition::LinkedinBrain => "LINKEDIN BRAIN",
            Condition::HustleMode => "HUSTLE MODE",
            Condition::MainCharacter { .. } => "MAIN CHARACTER",
            Condition::Doomer => "DOOMER",
            Condition::TouchingGrass => "TOUCHING GRASS",
            Condition::VibeCoding { .. } => "VIBE CODING",
        }
    }

    pub fn tick_duration(&mut self) {
        match self {
            Condition::MainCharacter { remaining_turns }
            | Condition::VibeCoding { remaining_turns } => {
                *remaining_turns = remaining_turns.saturating_sub(1);
            }
            _ => {}
        }
    }

    pub fn is_expired(&self) -> bool {
        match self {
            Condition::MainCharacter { remaining_turns }
            | Condition::VibeCoding { remaining_turns } => *remaining_turns == 0,
            _ => false,
        }
    }
}
