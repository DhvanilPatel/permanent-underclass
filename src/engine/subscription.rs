use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubscriptionTier {
    Free,
    Pro,
    Ultra,
    Giga,
}

impl SubscriptionTier {
    pub const ALL: &'static [SubscriptionTier] = &[
        SubscriptionTier::Free,
        SubscriptionTier::Pro,
        SubscriptionTier::Ultra,
        SubscriptionTier::Giga,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SubscriptionTier::Free => "Free",
            SubscriptionTier::Pro => "Pro",
            SubscriptionTier::Ultra => "Ultra",
            SubscriptionTier::Giga => "Giga",
        }
    }

    pub fn cost_per_turn(self) -> i64 {
        match self {
            SubscriptionTier::Free => 0,
            SubscriptionTier::Pro => 20,
            SubscriptionTier::Ultra => 200,
            SubscriptionTier::Giga => 2000,
        }
    }

    pub fn ai_fluency_bonus_per_turn(self) -> i32 {
        match self {
            SubscriptionTier::Free => -1,
            SubscriptionTier::Pro => 1,
            SubscriptionTier::Ultra => 4,
            SubscriptionTier::Giga => 10,
        }
    }

    pub fn ai_event_bonus(self) -> i32 {
        match self {
            SubscriptionTier::Free => -10,
            SubscriptionTier::Pro => 0,
            SubscriptionTier::Ultra => 25,
            SubscriptionTier::Giga => 60,
        }
    }

    pub fn unlock_ai_x10(self) -> u32 {
        match self {
            SubscriptionTier::Free => 0,
            SubscriptionTier::Pro => 0,
            SubscriptionTier::Ultra => 450,
            SubscriptionTier::Giga => 750,
        }
    }

    pub fn is_unlocked(self, ai_x10: u32) -> bool {
        ai_x10 >= self.unlock_ai_x10()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unlock_thresholds() {
        assert!(SubscriptionTier::Ultra.is_unlocked(450));
        assert!(!SubscriptionTier::Ultra.is_unlocked(449));
        assert!(SubscriptionTier::Giga.is_unlocked(750));
        assert!(!SubscriptionTier::Giga.is_unlocked(749));
    }
}
