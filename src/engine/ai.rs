use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Early,
    Mid,
    Late,
    Endgame,
}

impl Phase {
    /// v1 pacing is turn-based (not AI%-based) to keep content gating predictable.
    ///
    /// For 12-turn ("simple") runs, phase boundaries are intentionally front-loaded
    /// so endgame content has more room:
    /// - Early:   turns 1..=2
    /// - Mid:     turns 3..=4
    /// - Late:    turns 5..=7
    /// - Endgame: turns 8..=12
    ///
    /// For longer runs, boundaries are quartiles of `max_turns`:
    /// - Early:   0%..25%
    /// - Mid:     25%..50%
    /// - Late:    50%..75%
    /// - Endgame: 75%..100%
    pub fn from_turn(turn: u32, max_turns: u32) -> Self {
        let max = max_turns.max(1);
        let t = turn.clamp(1, max);
        if max <= 12 {
            if t <= 2 {
                return Phase::Early;
            }
            if t <= 4 {
                return Phase::Mid;
            }
            if t <= 7 {
                return Phase::Late;
            }
            return Phase::Endgame;
        }
        let early_end = phase_cutoff(max, 1, 4);
        let mid_end = phase_cutoff(max, 2, 4);
        let late_end = phase_cutoff(max, 3, 4);
        if t <= early_end {
            Phase::Early
        } else if t <= mid_end {
            Phase::Mid
        } else if t <= late_end {
            Phase::Late
        } else {
            Phase::Endgame
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Phase::Early => "EARLY",
            Phase::Mid => "MID",
            Phase::Late => "LATE",
            Phase::Endgame => "ENDGAME",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Milestone {
    Ai15,
    Ai25,
    Ai35,
    Ai45,
    Ai55,
    Ai65,
    Ai75,
    Ai85,
    Ai90,
    Ai95,
    Ai100,
}

impl Milestone {
    pub const ALL: &'static [Milestone] = &[
        Milestone::Ai15,
        Milestone::Ai25,
        Milestone::Ai35,
        Milestone::Ai45,
        Milestone::Ai55,
        Milestone::Ai65,
        Milestone::Ai75,
        Milestone::Ai85,
        Milestone::Ai90,
        Milestone::Ai95,
        Milestone::Ai100,
    ];

    pub fn threshold_x10(self) -> u32 {
        match self {
            Milestone::Ai15 => 150,
            Milestone::Ai25 => 250,
            Milestone::Ai35 => 350,
            Milestone::Ai45 => 450,
            Milestone::Ai55 => 550,
            Milestone::Ai65 => 650,
            Milestone::Ai75 => 750,
            Milestone::Ai85 => 850,
            Milestone::Ai90 => 900,
            Milestone::Ai95 => 950,
            Milestone::Ai100 => 1000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiCapability {
    // AI capability percent in tenths (e.g. 345 == 34.5%).
    x10: u32,
}

impl Default for AiCapability {
    fn default() -> Self {
        Self::new()
    }
}

impl AiCapability {
    pub const START_X10: u32 = 100; // 10.0%
    pub const END_X10: u32 = 1000; // 100.0%

    pub fn new() -> Self {
        Self {
            x10: Self::START_X10,
        }
    }

    #[cfg(test)]
    pub(crate) fn from_x10(x10: u32) -> Self {
        Self {
            x10: x10.clamp(Self::START_X10, Self::END_X10),
        }
    }

    pub fn x10(self) -> u32 {
        self.x10
    }

    pub fn percent_string(self) -> String {
        format!("{}.{}%", self.x10 / 10, self.x10 % 10)
    }

    pub fn tick_delta_x10(turn: u32, max_turns: u32) -> u32 {
        if turn == 0 || turn > max_turns {
            return 0;
        }
        // Target curve: 10.0% -> ~95%+ by end-of-run, with accelerating dread.
        const TARGET_TOTAL_DELTA_X10: u32 = 860;
        let max = max_turns.max(1);
        let phase = Phase::from_turn(turn, max);
        let phase_units = phase_weight(phase);
        let total_units = total_phase_units(max);
        (TARGET_TOTAL_DELTA_X10 * phase_units + (total_units / 2)) / total_units
    }

    pub fn apply_tick(&mut self, turn: u32, max_turns: u32) -> Vec<Milestone> {
        let prev = self.x10;
        let delta = Self::tick_delta_x10(turn, max_turns);
        self.x10 = (self.x10 + delta).min(Self::END_X10);
        milestones_crossed(prev, self.x10)
    }
}

fn phase_cutoff(max_turns: u32, numerator: u32, denominator: u32) -> u32 {
    (max_turns * numerator).div_ceil(denominator)
}

fn phase_weight(phase: Phase) -> u32 {
    match phase {
        Phase::Early => 2,
        Phase::Mid => 3,
        Phase::Late => 4,
        Phase::Endgame => 5,
    }
}

fn total_phase_units(max_turns: u32) -> u32 {
    let max = max_turns.max(1);
    let mut total = 0_u32;
    for t in 1..=max {
        total += phase_weight(Phase::from_turn(t, max));
    }
    total.max(1)
}

fn milestones_crossed(prev_x10: u32, new_x10: u32) -> Vec<Milestone> {
    if new_x10 <= prev_x10 {
        return Vec::new();
    }
    Milestone::ALL
        .iter()
        .copied()
        .filter(|m| prev_x10 < m.threshold_x10() && new_x10 >= m.threshold_x10())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ai_reaches_95_by_turn_12() {
        let mut ai = AiCapability::new();
        for t in 1..=12 {
            ai.apply_tick(t, 12);
        }
        assert!(ai.x10() >= 950);
    }

    #[test]
    fn ai_reaches_95_by_turn_32() {
        let mut ai = AiCapability::new();
        for t in 1..=32 {
            ai.apply_tick(t, 32);
        }
        assert!(ai.x10() >= 950);
    }

    #[test]
    fn milestone_crossing_detected() {
        assert_eq!(milestones_crossed(149, 150), vec![Milestone::Ai15]);
        assert_eq!(milestones_crossed(149, 151), vec![Milestone::Ai15]);
        assert_eq!(milestones_crossed(150, 151), vec![]);
        assert_eq!(
            milestones_crossed(449, 551),
            vec![Milestone::Ai45, Milestone::Ai55]
        );
    }

    #[test]
    fn simple_phase_boundaries_are_front_loaded() {
        assert_eq!(Phase::from_turn(1, 12), Phase::Early);
        assert_eq!(Phase::from_turn(2, 12), Phase::Early);
        assert_eq!(Phase::from_turn(3, 12), Phase::Mid);
        assert_eq!(Phase::from_turn(4, 12), Phase::Mid);
        assert_eq!(Phase::from_turn(5, 12), Phase::Late);
        assert_eq!(Phase::from_turn(7, 12), Phase::Late);
        assert_eq!(Phase::from_turn(8, 12), Phase::Endgame);
        assert_eq!(Phase::from_turn(12, 12), Phase::Endgame);
    }

    #[test]
    fn standard_phase_boundaries_stay_quartile_based() {
        assert_eq!(Phase::from_turn(1, 32), Phase::Early);
        assert_eq!(Phase::from_turn(8, 32), Phase::Early);
        assert_eq!(Phase::from_turn(9, 32), Phase::Mid);
        assert_eq!(Phase::from_turn(16, 32), Phase::Mid);
        assert_eq!(Phase::from_turn(17, 32), Phase::Late);
        assert_eq!(Phase::from_turn(24, 32), Phase::Late);
        assert_eq!(Phase::from_turn(25, 32), Phase::Endgame);
        assert_eq!(Phase::from_turn(32, 32), Phase::Endgame);
    }
}
