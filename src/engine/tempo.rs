use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Tempo {
    /// Short run for quick sessions.
    #[default]
    Simple,
    /// Extended run for longer arcs.
    Standard,
}

impl Tempo {
    pub fn from_cli_value(raw: &str) -> Option<Self> {
        let normalized = raw.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "simple" | "short" | "12" => Some(Self::Simple),
            "standard" | "long" | "32" => Some(Self::Standard),
            _ => None,
        }
    }

    pub fn cli_name(self) -> &'static str {
        match self {
            Self::Simple => "simple",
            Self::Standard => "standard",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Simple => "SIMPLE",
            Self::Standard => "STANDARD",
        }
    }

    pub fn all() -> [Self; 2] {
        [Self::Simple, Self::Standard]
    }

    pub fn summary(self) -> &'static str {
        match self {
            Self::Simple => "12 turns",
            Self::Standard => "32 turns",
        }
    }

    pub fn max_turns(self) -> u32 {
        match self {
            Self::Simple => 12,
            Self::Standard => 32,
        }
    }

    pub fn money_scale(self) -> i32 {
        1
    }

    pub fn stat_scale(self) -> i32 {
        1
    }

    pub fn label(self) -> &'static str {
        "TURN"
    }

    pub fn interview_cooldown_turns(self) -> u32 {
        match self {
            Self::Simple => 3,
            Self::Standard => 5,
        }
    }

    pub fn main_character_duration_turns(self) -> u32 {
        match self {
            Self::Simple => 3,
            Self::Standard => 4,
        }
    }
}
