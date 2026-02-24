use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ResourceDelta {
    #[serde(alias = "cash")]
    pub money: i64,
    #[serde(alias = "cope")]
    pub sanity: i32,
    #[serde(alias = "clout")]
    pub audience: i32,
    #[serde(alias = "craft")]
    pub ai_skill: i32,
    #[serde(alias = "crew")]
    pub friends: i32,
}

impl ResourceDelta {
    pub const ZERO: ResourceDelta = ResourceDelta {
        money: 0,
        sanity: 0,
        audience: 0,
        ai_skill: 0,
        friends: 0,
    };

    pub fn is_zero(self) -> bool {
        self == ResourceDelta::ZERO
    }

    pub fn scaled(self, numer: i32, denom: i32) -> Self {
        if denom == 0 {
            return self;
        }
        Self {
            money: self
                .money
                .saturating_mul(numer as i64)
                .saturating_div(denom as i64),
            sanity: self.sanity.saturating_mul(numer).saturating_div(denom),
            audience: self.audience.saturating_mul(numer).saturating_div(denom),
            ai_skill: self.ai_skill.saturating_mul(numer).saturating_div(denom),
            friends: self.friends.saturating_mul(numer).saturating_div(denom),
        }
    }

    pub fn scaled_by(self, money_scale: i32, stat_scale: i32) -> Self {
        Self {
            money: self.money.saturating_mul(money_scale as i64),
            sanity: self.sanity.saturating_mul(stat_scale),
            audience: self.audience.saturating_mul(stat_scale),
            ai_skill: self.ai_skill.saturating_mul(stat_scale),
            friends: self.friends.saturating_mul(stat_scale),
        }
    }
}

impl std::ops::Add for ResourceDelta {
    type Output = ResourceDelta;

    fn add(self, rhs: ResourceDelta) -> Self::Output {
        ResourceDelta {
            money: self.money.saturating_add(rhs.money),
            sanity: self.sanity.saturating_add(rhs.sanity),
            audience: self.audience.saturating_add(rhs.audience),
            ai_skill: self.ai_skill.saturating_add(rhs.ai_skill),
            friends: self.friends.saturating_add(rhs.friends),
        }
    }
}

impl std::ops::AddAssign for ResourceDelta {
    fn add_assign(&mut self, rhs: ResourceDelta) {
        *self = *self + rhs;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stats {
    pub money: i64,
    pub sanity: i32,
    pub audience: i32,
    pub ai_skill: i32,
    pub friends: i32,
}

impl Stats {
    pub fn apply_delta(&mut self, delta: ResourceDelta) {
        self.money = self.money.saturating_add(delta.money);
        self.sanity = self.sanity.saturating_add(delta.sanity);
        self.audience = self.audience.saturating_add(delta.audience);
        self.ai_skill = self.ai_skill.saturating_add(delta.ai_skill);
        self.friends = self.friends.saturating_add(delta.friends);
        self.clamp_non_money();
    }

    pub fn clamp_non_money(&mut self) {
        self.sanity = self.sanity.clamp(0, 100);
        self.audience = self.audience.clamp(0, 100);
        self.ai_skill = self.ai_skill.clamp(0, 100);
        self.friends = self.friends.clamp(0, 100);
    }

    pub fn is_dead(&self) -> Option<DeathReason> {
        if self.money <= 0 {
            return Some(DeathReason::Broke);
        }
        if self.sanity <= 0 {
            return Some(DeathReason::BurnedOut);
        }
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeathReason {
    Broke,
    BurnedOut,
}

impl fmt::Display for DeathReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeathReason::Broke => write!(f, "BROKE"),
            DeathReason::BurnedOut => write!(f, "BURNED OUT"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_non_money_stats() {
        let mut s = Stats {
            money: 0,
            sanity: 200,
            audience: -10,
            ai_skill: 150,
            friends: -999,
        };
        s.clamp_non_money();
        assert_eq!(s.sanity, 100);
        assert_eq!(s.audience, 0);
        assert_eq!(s.ai_skill, 100);
        assert_eq!(s.friends, 0);
    }

    #[test]
    fn death_checks() {
        let mut s = Stats {
            money: 1,
            sanity: 1,
            audience: 0,
            ai_skill: 0,
            friends: 0,
        };
        assert_eq!(s.is_dead(), None);
        s.money = 0;
        assert_eq!(s.is_dead(), Some(DeathReason::Broke));
        s.money = 100;
        s.sanity = 0;
        assert_eq!(s.is_dead(), Some(DeathReason::BurnedOut));
    }

    #[test]
    fn resource_delta_scaled_by() {
        let d = ResourceDelta {
            money: 10,
            sanity: -2,
            audience: 3,
            ai_skill: 1,
            friends: -1,
        };
        let s = d.scaled_by(4, 3);
        assert_eq!(s.money, 40);
        assert_eq!(s.sanity, -6);
        assert_eq!(s.audience, 9);
        assert_eq!(s.ai_skill, 3);
        assert_eq!(s.friends, -3);
    }

    #[test]
    fn resource_delta_scaled_saturates() {
        let d = ResourceDelta {
            money: i64::MAX,
            sanity: i32::MAX,
            audience: i32::MAX,
            ai_skill: i32::MAX,
            friends: i32::MAX,
        };
        let s = d.scaled(2, 1);
        assert_eq!(s.money, i64::MAX);
        assert_eq!(s.sanity, i32::MAX);
        assert_eq!(s.audience, i32::MAX);
        assert_eq!(s.ai_skill, i32::MAX);
        assert_eq!(s.friends, i32::MAX);
    }

    #[test]
    fn resource_delta_add_saturates() {
        let a = ResourceDelta {
            money: i64::MAX,
            sanity: i32::MAX,
            audience: i32::MAX,
            ai_skill: i32::MAX,
            friends: i32::MAX,
        };
        let b = ResourceDelta {
            money: 1,
            sanity: 1,
            audience: 1,
            ai_skill: 1,
            friends: 1,
        };
        let sum = a + b;
        assert_eq!(sum.money, i64::MAX);
        assert_eq!(sum.sanity, i32::MAX);
        assert_eq!(sum.audience, i32::MAX);
        assert_eq!(sum.ai_skill, i32::MAX);
        assert_eq!(sum.friends, i32::MAX);
    }
}
