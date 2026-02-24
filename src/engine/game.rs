use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::Path;

use anyhow::{Context, anyhow};
use rand::SeedableRng;
use rand_chacha::ChaCha12Rng;
use serde::{Deserialize, Serialize};

use crate::engine::ai::{AiCapability, Milestone, Phase};
use crate::engine::characters::{Ability, CharacterDef, CharacterId};
use crate::engine::conditions::Condition;
use crate::engine::content::FlagRegistry;
use crate::engine::events::{MemoryEffect, MemoryEffectOp, TurnEvents, TurnOutcome};
use crate::engine::headlines::Headlines;
use crate::engine::llm::LlmNarrator;
use crate::engine::showrunner::{Showrunner, ShowrunnerContext};
use crate::engine::subscription::SubscriptionTier;
use crate::engine::tempo::Tempo;
use crate::engine::ticker::Ticker;
use crate::engine::types::{DeathReason, ResourceDelta, Stats};

const MAX_LOG_LINES: usize = 200;
const MAX_SNAPSHOT_BYTES: u64 = 4 * 1024 * 1024;
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RunMemory {
    pub flags: HashSet<String>,
    #[serde(default)]
    pub late_game_flags: HashSet<String>,
    pub seen_event_ids: HashSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RunPatterns {
    pub turns_recorded: u32,
    pub cash_focus_turns: u32,
    pub audience_push_turns: u32,
    pub ai_push_turns: u32,
    pub recovery_turns: u32,
    pub relationship_turns: u32,
    pub high_pressure_turns: u32,
}

#[derive(Debug, Clone)]
pub struct GameState {
    pub tempo: Tempo,
    pub turn: u32,
    pub max_turns: u32,

    pub character: CharacterId,
    pub ability: Ability,
    pub ability_used: bool,
    pub ability_last_used_turn: Option<u32>,

    pub ai: AiCapability,
    pub subscription: SubscriptionTier,

    pub income_per_turn: i64,
    pub rent_per_turn: i64,

    pub stats: Stats,
    pub conditions: Vec<Condition>,
    pub run_memory: RunMemory,
    pub run_patterns: RunPatterns,

    pub log: VecDeque<String>,
    pub ticker: Ticker,

    pub consecutive_pressure: u32,
}

impl GameState {
    pub fn money_scale(&self) -> i32 {
        self.tempo.money_scale()
    }

    pub fn stat_scale(&self) -> i32 {
        self.tempo.stat_scale()
    }

    pub fn phase(&self) -> Phase {
        Phase::from_turn(self.turn, self.max_turns)
    }

    pub fn year(&self) -> i32 {
        // Rough mapping for UI flavor only.
        let t = self.turn.max(1);
        let max = self.max_turns.max(1);
        let frac = t as f32 / max as f32;
        if frac <= 0.34 {
            2025
        } else if frac <= 0.67 {
            2026
        } else {
            2027
        }
    }

    pub fn turn_label(&self) -> String {
        format!("{} {}", self.tempo.label(), self.turn)
    }

    pub fn push_log(&mut self, line: impl Into<String>) {
        if self.log.len() >= MAX_LOG_LINES {
            self.log.pop_front();
        }
        self.log.push_back(line.into());
    }

    pub fn active_condition_labels(&self) -> Vec<&'static str> {
        Vec::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GameStateSnapshot {
    tempo: Tempo,
    turn: u32,
    max_turns: u32,
    character: CharacterId,
    ability: Ability,
    ability_used: bool,
    ability_last_used_turn: Option<u32>,
    ai: AiCapability,
    subscription: SubscriptionTier,
    income_per_turn: i64,
    rent_per_turn: i64,
    stats: Stats,
    conditions: Vec<Condition>,
    run_memory: RunMemory,
    run_patterns: RunPatterns,
    log: VecDeque<String>,
    ticker: Ticker,
    consecutive_pressure: u32,
}

impl From<&GameState> for GameStateSnapshot {
    fn from(state: &GameState) -> Self {
        Self {
            tempo: state.tempo,
            turn: state.turn,
            max_turns: state.max_turns,
            character: state.character.clone(),
            ability: state.ability,
            ability_used: state.ability_used,
            ability_last_used_turn: state.ability_last_used_turn,
            ai: state.ai,
            subscription: state.subscription,
            income_per_turn: state.income_per_turn,
            rent_per_turn: state.rent_per_turn,
            stats: state.stats,
            conditions: state.conditions.clone(),
            run_memory: state.run_memory.clone(),
            run_patterns: state.run_patterns.clone(),
            log: state.log.clone(),
            ticker: state.ticker.clone(),
            consecutive_pressure: state.consecutive_pressure,
        }
    }
}

impl GameStateSnapshot {
    fn validate(&self) -> anyhow::Result<()> {
        let expected_max_turns = self.tempo.max_turns();
        if self.max_turns != expected_max_turns {
            return Err(anyhow!(
                "invalid snapshot max_turns={} for tempo='{}' (expected {})",
                self.max_turns,
                self.tempo.label(),
                expected_max_turns
            ));
        }
        if self.turn > self.max_turns {
            return Err(anyhow!(
                "invalid snapshot turn={} exceeds max_turns={}",
                self.turn,
                self.max_turns
            ));
        }
        Ok(())
    }

    fn into_state(self) -> GameState {
        GameState {
            tempo: self.tempo,
            turn: self.turn,
            max_turns: self.max_turns,
            character: self.character,
            ability: self.ability,
            ability_used: self.ability_used,
            ability_last_used_turn: self.ability_last_used_turn,
            ai: self.ai,
            subscription: self.subscription,
            income_per_turn: self.income_per_turn,
            rent_per_turn: self.rent_per_turn,
            stats: self.stats,
            conditions: self.conditions,
            run_memory: self.run_memory,
            run_patterns: self.run_patterns,
            log: self.log,
            ticker: self.ticker,
            consecutive_pressure: self.consecutive_pressure,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GameSnapshot {
    state: GameStateSnapshot,
    rng: ChaCha12Rng,
    showrunner: Showrunner,
}

impl From<&Game> for GameSnapshot {
    fn from(game: &Game) -> Self {
        Self {
            state: GameStateSnapshot::from(&game.state),
            rng: game.rng.clone(),
            showrunner: game.showrunner.clone(),
        }
    }
}

impl GameSnapshot {
    fn into_game(self) -> Game {
        Game {
            state: self.state.into_state(),
            rng: self.rng,
            showrunner: self.showrunner,
            narrator: None,
        }
    }
}

pub struct Game {
    pub state: GameState,
    rng: ChaCha12Rng,
    showrunner: Showrunner,
    narrator: Option<LlmNarrator>,
}

impl Game {
    pub fn new(tempo: Tempo, character: CharacterId, seed: Option<u64>) -> Self {
        let def = CharacterDef::for_character(character.clone());
        let rng = match seed {
            Some(s) => ChaCha12Rng::seed_from_u64(s),
            None => ChaCha12Rng::from_entropy(),
        };

        let mut log = VecDeque::new();
        log.push_back("PERMANENT UNDERCLASS: survive 2025-2027.".to_string());
        log.push_back("Press 1/2/3 to pick a card. Press 4 to type.".to_string());

        Self {
            state: GameState {
                tempo,
                turn: 0,
                max_turns: tempo.max_turns(),
                // Always store canonical character id from data/characters.jsonl.
                character: def.id.clone(),
                ability: def.ability,
                ability_used: false,
                ability_last_used_turn: None,
                ai: AiCapability::new(),
                subscription: def.starting_subscription,
                income_per_turn: def.starting_income_per_turn,
                rent_per_turn: def.starting_rent_per_turn,
                stats: def.starting_stats,
                // Conditions are disabled in the simplified gameplay build.
                conditions: Vec::new(),
                run_memory: RunMemory::default(),
                run_patterns: RunPatterns::default(),
                log,
                ticker: Ticker::new(),
                consecutive_pressure: 0,
            },
            rng,
            showrunner: Showrunner::new(),
            narrator: None,
        }
    }

    pub fn set_narrator(&mut self, narrator: Option<LlmNarrator>) {
        self.narrator = narrator;
    }

    pub fn save_snapshot<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let path_ref = path.as_ref();
        let snapshot = GameSnapshot::from(self);
        let json = serde_json::to_vec_pretty(&snapshot)
            .context("failed to serialize game snapshot to JSON")?;
        fs::write(path_ref, json)
            .with_context(|| format!("failed to write snapshot file '{}'", path_ref.display()))
    }

    pub fn load_snapshot<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let path_ref = path.as_ref();
        let metadata = fs::metadata(path_ref)
            .with_context(|| format!("failed to stat snapshot file '{}'", path_ref.display()))?;
        if metadata.len() > MAX_SNAPSHOT_BYTES {
            return Err(anyhow!(
                "snapshot file '{}' is too large ({} bytes > {} bytes)",
                path_ref.display(),
                metadata.len(),
                MAX_SNAPSHOT_BYTES
            ));
        }
        let bytes = fs::read(path_ref)
            .with_context(|| format!("failed to read snapshot file '{}'", path_ref.display()))?;
        if bytes.len() as u64 > MAX_SNAPSHOT_BYTES {
            return Err(anyhow!(
                "snapshot file '{}' exceeded size limit while reading",
                path_ref.display()
            ));
        }
        let snapshot: GameSnapshot = serde_json::from_slice(&bytes).with_context(|| {
            format!(
                "failed to parse snapshot JSON from '{}'",
                path_ref.display()
            )
        })?;
        snapshot.state.validate().with_context(|| {
            format!(
                "invalid snapshot state loaded from '{}'",
                path_ref.display()
            )
        })?;
        Ok(snapshot.into_game())
    }

    pub fn preview_choice_delta(&self, choice: &crate::engine::events::Choice) -> ResourceDelta {
        let mut delta = choice
            .delta
            .scaled_by(self.state.money_scale(), self.state.stat_scale());
        delta = self.apply_authored_phase_scaling(delta);
        delta
    }

    pub fn on_tick(&mut self) {
        self.state.ticker.tick();
    }

    pub fn begin_turn(&mut self) -> TurnEvents {
        self.state.turn = (self.state.turn + 1).min(self.state.max_turns);

        // 1) AI ticks up and milestones apply.
        let milestones = self
            .state
            .ai
            .apply_tick(self.state.turn, self.state.max_turns);
        self.apply_milestone_side_effects(&milestones);

        // 2) Passive effects.
        let money_scale = self.state.money_scale();
        let stat_scale = self.state.stat_scale();

        let base_sanity_decay_per_turn = match self.state.phase() {
            Phase::Early => -1,
            Phase::Mid => -2,
            Phase::Late => -3,
            Phase::Endgame => -3,
        };
        // Relationships now require upkeep too, but gentler than sanity decay.
        let base_friends_decay_per_turn = baseline_friends_decay_per_turn(self.state.phase());

        let mut sanity_decay = base_sanity_decay_per_turn * stat_scale;
        if self.state.character.matches("plumber") {
            // Plumber gets reduced passive sanity attrition (about half, with floor rounding).
            sanity_decay = sanity_decay.div_euclid(2);
        }
        let friends_decay = base_friends_decay_per_turn * stat_scale;

        let passive = ResourceDelta {
            money: (self.state.income_per_turn
                - self.state.rent_per_turn
                - self.state.subscription.cost_per_turn())
                * money_scale as i64,
            sanity: sanity_decay,
            audience: -stat_scale,
            ai_skill: self.state.subscription.ai_fluency_bonus_per_turn() * stat_scale,
            friends: friends_decay,
        };
        self.state.stats.apply_delta(passive);

        // 3) Showrunner generates events.
        let ctx = ShowrunnerContext {
            turn: self.state.turn,
            max_turns: self.state.max_turns,
            ai_x10: self.state.ai.x10(),
            phase: self.state.phase(),
            character: self.state.character.clone(),
            stats: self.state.stats,
            memory_flags: self.state.run_memory.flags.clone(),
            seen_personal_ids: self.state.run_memory.seen_event_ids.clone(),
        };
        let events = self.showrunner.generate_turn(&mut self.rng, &ctx);
        self.state
            .run_memory
            .seen_event_ids
            .insert(events.personal.id.clone());

        // 4) World event auto-applies.
        self.apply_world_event(&events);

        // 5) Ticker updates — pick 3 rotating headlines for the ticker.
        let ticker_headlines = Headlines::pick_random(self.state.phase(), 3, &mut self.rng);
        let headline_refs: Vec<&str> = ticker_headlines.iter().map(|s| s.as_str()).collect();
        self.state.ticker.update_for_turn(
            &mut self.rng,
            self.state.phase(),
            self.state.ai.x10(),
            &headline_refs,
        );

        events
    }

    pub fn resolve_choice(&mut self, events: &TurnEvents, key: char) -> TurnOutcome {
        let Some(choice) = events.personal.choices.iter().find(|c| c.key == key) else {
            return TurnOutcome {
                narrative: "Invalid choice.".to_string(),
                delta: ResourceDelta::ZERO,
                summary_line: "Invalid choice.".to_string(),
                tag_changes: Vec::new(),
            };
        };

        if !choice.is_available(self.state.stats.money) {
            let required = choice
                .min_money
                .map(|min| format!("${min}"))
                .unwrap_or_else(|| "more money".to_string());
            return TurnOutcome {
                narrative: format!("Locked: you need at least {}.", required),
                delta: ResourceDelta::ZERO,
                summary_line: "Choice locked.".to_string(),
                tag_changes: Vec::new(),
            };
        }

        let mut delta = choice
            .delta
            .scaled_by(self.state.money_scale(), self.state.stat_scale());
        delta = self.apply_authored_phase_scaling(delta);

        self.state.stats.apply_delta(delta);
        let tag_changes = self.apply_memory_effects(&choice.effects, "choice");
        self.update_run_patterns(choice.delta);

        let title = choice.title.trim();
        let body = choice.body.trim();
        let narrative = if body.is_empty() || body == title {
            format!("You chose {}.", title)
        } else if let Some(rest) = body.strip_prefix(title) {
            let rest = rest.trim_start_matches(|c: char| {
                c.is_whitespace() || matches!(c, '.' | ',' | ':' | ';' | '-' | '!' | '?')
            });
            if rest.is_empty() {
                format!("You chose {}.", title)
            } else {
                format!("You chose {}. {}", title, rest)
            }
        } else {
            format!("You chose {}. {}", title, body)
        };
        let summary = format!("{}: {}.", self.state.turn_label(), choice.title);

        self.state.push_log(summary.clone());

        TurnOutcome {
            narrative,
            delta,
            summary_line: summary,
            tag_changes,
        }
    }

    pub fn resolve_freeform(&mut self, events: &TurnEvents, action: &str) -> TurnOutcome {
        if let Some(n) = &self.narrator
            && events.personal.allow_freeform
            && let Ok(res) = n.resolve_freeform(&self.state, events, action)
        {
            return self.apply_freeform(res.delta, res.effects, res.narrative, res.summary_line);
        }

        let normalized = action.trim().to_lowercase();
        if normalized.is_empty() {
            return TurnOutcome {
                narrative: "You do nothing. The bar fills anyway.".to_string(),
                delta: ResourceDelta::ZERO,
                summary_line: "Did nothing.".to_string(),
                tag_changes: Vec::new(),
            };
        }

        // Lightweight heuristic parser for offline mode.
        let (title, base_delta) = if contains_any(&normalized, &["rest", "sleep", "nap"]) {
            (
                "REST",
                ResourceDelta {
                    money: 0,
                    sanity: 14,
                    audience: -1,
                    ai_skill: 0,
                    friends: 4,
                },
            )
        } else if contains_any(&normalized, &["meet", "network", "coffee", "meetup"]) {
            (
                "NETWORK",
                ResourceDelta {
                    money: -200,
                    sanity: 1,
                    friends: 8,
                    audience: 6,
                    ai_skill: 0,
                },
            )
        } else if contains_any(&normalized, &["build", "ship", "code", "learn"]) {
            (
                "GRIND",
                ResourceDelta {
                    money: 450,
                    sanity: -5,
                    audience: 1,
                    ai_skill: 6,
                    friends: -2,
                },
            )
        } else if contains_any(&normalized, &["invent agi", "solve alignment"]) {
            (
                "ABSURD",
                ResourceDelta {
                    sanity: -6,
                    ai_skill: 1,
                    ..ResourceDelta::ZERO
                },
            )
        } else {
            (
                "MISC",
                ResourceDelta {
                    sanity: -1,
                    ..ResourceDelta::ZERO
                },
            )
        };

        let narrative = format!(
            "You type: '{}'. The game interprets this as: {}.",
            action, title
        );
        let summary = format!("{}: FREEFORM ({})", self.state.turn_label(), title);
        self.apply_freeform(base_delta, Vec::new(), narrative, summary)
    }

    pub fn apply_freeform(
        &mut self,
        base_delta: ResourceDelta,
        effects: Vec<MemoryEffect>,
        narrative: String,
        summary_line: String,
    ) -> TurnOutcome {
        let delta = base_delta.scaled_by(self.state.money_scale(), self.state.stat_scale());

        self.state.stats.apply_delta(delta);
        let safe_effects = self.sanitize_freeform_effects(&effects);
        let tag_changes = self.apply_memory_effects(&safe_effects, "freeform");
        self.update_run_patterns(base_delta);

        self.state.push_log(summary_line.clone());

        TurnOutcome {
            narrative,
            delta,
            summary_line,
            tag_changes,
        }
    }

    pub fn death_reason(&self) -> Option<DeathReason> {
        self.state.stats.is_dead()
    }

    pub fn is_final_turn(&self) -> bool {
        self.state.turn >= self.state.max_turns
    }

    pub fn can_set_subscription(&self, tier: SubscriptionTier) -> bool {
        tier.is_unlocked(self.state.ai.x10())
    }

    pub fn set_subscription(&mut self, tier: SubscriptionTier) -> Result<(), &'static str> {
        if tier == self.state.subscription {
            return Ok(());
        }
        if !self.can_set_subscription(tier) {
            return Err("Tier locked at current AI capability.");
        }

        // Upgrades cost the delta immediately (no refunds on downgrades).
        let old = self.state.subscription.cost_per_turn();
        let new = tier.cost_per_turn();
        if new > old {
            let diff = (new - old) * self.state.money_scale() as i64;
            self.state.stats.money = self.state.stats.money.saturating_sub(diff);
            self.state.push_log(format!(
                "SUB: {} -> {} (-${} now)",
                self.state.subscription.label(),
                tier.label(),
                diff
            ));
        } else {
            self.state.push_log(format!(
                "SUB: {} -> {}",
                self.state.subscription.label(),
                tier.label()
            ));
        }

        self.state.subscription = tier;
        Ok(())
    }

    pub fn can_use_ability_in_turn(&self) -> bool {
        false
    }

    pub fn can_rebrand(&self) -> bool {
        false
    }

    pub fn use_ability(&mut self) -> TurnOutcome {
        TurnOutcome {
            narrative: "Abilities are disabled in this build.".to_string(),
            delta: ResourceDelta::ZERO,
            summary_line: "Ability disabled.".to_string(),
            tag_changes: Vec::new(),
        }
    }

    pub fn rebrand(&mut self, last_delta: ResourceDelta) -> TurnOutcome {
        let _ = last_delta;
        TurnOutcome {
            narrative: "Abilities are disabled in this build.".to_string(),
            delta: ResourceDelta::ZERO,
            summary_line: "Ability disabled.".to_string(),
            tag_changes: Vec::new(),
        }
    }

    fn apply_world_event(&mut self, events: &TurnEvents) {
        let delta = events
            .world
            .delta
            .scaled_by(self.state.money_scale(), self.state.stat_scale());

        if !delta.is_zero() {
            self.state.stats.apply_delta(delta);
        }
        self.state
            .push_log(format!("WORLD: {}", events.world.body.replace('\n', " ")));
    }

    fn apply_milestone_side_effects(&mut self, milestones: &[Milestone]) {
        for m in milestones {
            match m {
                Milestone::Ai15 => self.state.rent_per_turn += 50,
                Milestone::Ai45 => self.state.rent_per_turn += 100,
                Milestone::Ai55 => self.state.rent_per_turn += 100,
                Milestone::Ai75 => self.state.rent_per_turn += 150,
                _ => {}
            }
        }

        // Character abilities are disabled in the simplified gameplay build.
    }

    fn apply_authored_phase_scaling(&self, delta: ResourceDelta) -> ResourceDelta {
        let (numerator, denominator) = match self.state.phase() {
            Phase::Early => (100, 100),
            Phase::Mid => (110, 100),
            Phase::Late => (125, 100),
            Phase::Endgame => (140, 100),
        };
        if numerator == denominator {
            return delta;
        }
        ResourceDelta {
            money: scale_i64(delta.money, numerator, denominator),
            sanity: scale_i32(delta.sanity, numerator, denominator),
            audience: scale_i32(delta.audience, numerator, denominator),
            ai_skill: scale_i32(delta.ai_skill, numerator, denominator),
            friends: scale_i32(delta.friends, numerator, denominator),
        }
    }

    fn update_run_patterns(&mut self, authored_delta: ResourceDelta) {
        self.state.run_patterns.turns_recorded =
            self.state.run_patterns.turns_recorded.saturating_add(1);
        if is_cash_focus_choice(authored_delta) {
            self.state.run_patterns.cash_focus_turns =
                self.state.run_patterns.cash_focus_turns.saturating_add(1);
        }
        if is_audience_push_choice(authored_delta) {
            self.state.run_patterns.audience_push_turns = self
                .state
                .run_patterns
                .audience_push_turns
                .saturating_add(1);
        }
        if is_ai_progress_choice(authored_delta) {
            self.state.run_patterns.ai_push_turns =
                self.state.run_patterns.ai_push_turns.saturating_add(1);
        }
        if is_recovery_choice(authored_delta) {
            self.state.run_patterns.recovery_turns =
                self.state.run_patterns.recovery_turns.saturating_add(1);
        }
        if authored_delta.friends > 0 {
            self.state.run_patterns.relationship_turns =
                self.state.run_patterns.relationship_turns.saturating_add(1);
        }
        if is_pressure_choice(authored_delta) || is_ai_progress_choice(authored_delta) {
            self.state.consecutive_pressure = self.state.consecutive_pressure.saturating_add(1);
            if is_pressure_choice(authored_delta) {
                self.state.run_patterns.high_pressure_turns = self
                    .state
                    .run_patterns
                    .high_pressure_turns
                    .saturating_add(1);
            }
        } else {
            self.state.consecutive_pressure = 0;
        }
    }

    fn sanitize_freeform_effects(&self, effects: &[MemoryEffect]) -> Vec<MemoryEffect> {
        let Ok(flags) = FlagRegistry::embedded() else {
            return Vec::new();
        };
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        for effect in effects {
            if out.len() >= 2 {
                break;
            }
            let flag = effect.flag.trim();
            if flag.is_empty() || !flags.is_known(flag) || !flags.is_llm_settable(flag) {
                continue;
            }
            let key = format!("{:?}:{}", effect.op, flag);
            if !seen.insert(key) {
                continue;
            }
            out.push(MemoryEffect {
                op: effect.op,
                flag: flag.to_string(),
            });
        }
        out
    }

    fn apply_memory_effects(
        &mut self,
        effects: &[MemoryEffect],
        source: &str,
    ) -> Vec<MemoryEffect> {
        let mut applied = Vec::new();
        let is_late_game = matches!(self.state.phase(), Phase::Late | Phase::Endgame);
        for effect in effects {
            let flag = effect.flag.trim();
            if flag.is_empty() {
                continue;
            }
            match effect.op {
                MemoryEffectOp::SetFlag => {
                    if self.state.run_memory.flags.insert(flag.to_string()) {
                        self.state
                            .push_log(format!("MEMORY [{}]: +{}", source, flag));
                        applied.push(MemoryEffect {
                            op: MemoryEffectOp::SetFlag,
                            flag: flag.to_string(),
                        });
                    }
                    if is_late_game {
                        self.state
                            .run_memory
                            .late_game_flags
                            .insert(flag.to_string());
                    }
                }
                MemoryEffectOp::ClearFlag => {
                    let removed = self.state.run_memory.flags.remove(flag);
                    self.state.run_memory.late_game_flags.remove(flag);
                    if removed {
                        self.state
                            .push_log(format!("MEMORY [{}]: -{}", source, flag));
                        applied.push(MemoryEffect {
                            op: MemoryEffectOp::ClearFlag,
                            flag: flag.to_string(),
                        });
                    }
                }
            }
        }
        applied
    }
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| haystack.contains(n))
}

fn baseline_friends_decay_per_turn(phase: Phase) -> i32 {
    match phase {
        Phase::Early => 0,
        Phase::Mid => -1,
        Phase::Late => -1,
        Phase::Endgame => -2,
    }
}

fn is_ai_progress_choice(delta: ResourceDelta) -> bool {
    delta.ai_skill > 0
}

fn is_pressure_choice(delta: ResourceDelta) -> bool {
    delta.money > 0 && delta.sanity < 0
}

fn is_cash_focus_choice(delta: ResourceDelta) -> bool {
    delta.money > 0
        && delta.sanity >= 0
        && delta.audience <= 0
        && delta.ai_skill <= 0
        && delta.friends <= 0
}

fn is_audience_push_choice(delta: ResourceDelta) -> bool {
    delta.audience > 0 && delta.sanity <= 0
}

fn is_recovery_choice(delta: ResourceDelta) -> bool {
    delta.sanity > 0 && delta.money <= 0 && delta.ai_skill <= 0
}

fn scale_i64(value: i64, numerator: i32, denominator: i32) -> i64 {
    if value == 0 || numerator == denominator {
        return value;
    }
    let n = numerator as i64;
    let d = denominator as i64;
    let scaled = value.saturating_mul(n);
    if scaled >= 0 {
        (scaled + (d / 2)) / d
    } else {
        (scaled - (d / 2)) / d
    }
}

fn scale_i32(value: i32, numerator: i32, denominator: i32) -> i32 {
    let scaled = scale_i64(value as i64, numerator, denominator);
    scaled.clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::characters::CharacterId;
    use crate::engine::events::{MemoryEffect, MemoryEffectOp};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn subscription_upgrade_charges_diff_when_unlocked() {
        let mut g = Game::new(Tempo::default(), CharacterId::new("junior_dev"), Some(1));
        // Force AI to a point where Ultra is unlocked.
        g.state.ai = AiCapability::from_x10(500);
        let before = g.state.stats.money;
        g.set_subscription(SubscriptionTier::Ultra).unwrap();
        assert_eq!(g.state.subscription, SubscriptionTier::Ultra);
        // Pro -> Ultra diff is 180 per turn (scale=1).
        assert_eq!(before - g.state.stats.money, 180);
    }

    #[test]
    fn abilities_are_disabled() {
        let mut g = Game::new(Tempo::default(), CharacterId::new("podcaster"), Some(2));
        assert!(!g.can_use_ability_in_turn());
        let out = g.use_ability();
        assert_eq!(out.summary_line, "Ability disabled.");
    }

    #[test]
    fn rebrand_is_disabled() {
        let mut g = Game::new(
            Tempo::default(),
            CharacterId::new("marketing_manager"),
            Some(3),
        );
        assert!(!g.can_rebrand());
        let rb = g.rebrand(ResourceDelta::ZERO);
        assert_eq!(rb.summary_line, "Ability disabled.");
        assert_eq!(rb.delta, ResourceDelta::ZERO);
    }

    #[test]
    fn snapshot_roundtrip_preserves_next_turn_generation() {
        let mut game = Game::new(
            Tempo::default(),
            CharacterId::new("ex_faang_engineer"),
            Some(7),
        );
        for _ in 0..5 {
            let events = game.begin_turn();
            let _ = game.resolve_choice(&events, '1');
        }

        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "pu_game_snapshot_test_{}_{}.json",
            std::process::id(),
            ts
        ));

        game.save_snapshot(&path).unwrap();
        let mut loaded = Game::load_snapshot(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(game.state.turn, loaded.state.turn);
        assert_eq!(game.state.stats, loaded.state.stats);
        assert_eq!(game.state.conditions, loaded.state.conditions);

        let next_a = game.begin_turn();
        let next_b = loaded.begin_turn();
        assert_eq!(next_a.world.title, next_b.world.title);
        assert_eq!(next_a.world.body, next_b.world.body);
        assert_eq!(next_a.personal.id, next_b.personal.id);
    }

    #[test]
    fn snapshot_rejects_turn_past_max_turns() {
        let mut game = Game::new(Tempo::default(), CharacterId::new("junior_dev"), Some(9));
        game.state.turn = game.state.max_turns + 1;

        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "pu_game_snapshot_invalid_turn_{}_{}.json",
            std::process::id(),
            ts
        ));

        game.save_snapshot(&path).unwrap();
        let err = match Game::load_snapshot(&path) {
            Ok(_) => panic!("expected invalid snapshot to be rejected"),
            Err(err) => err,
        };
        let _ = std::fs::remove_file(&path);
        assert!(err.to_string().contains("invalid snapshot state loaded"));
    }

    #[test]
    fn snapshot_rejects_non_tempo_max_turns() {
        let mut game = Game::new(Tempo::default(), CharacterId::new("junior_dev"), Some(10));
        game.state.max_turns += 1;

        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "pu_game_snapshot_invalid_max_{}_{}.json",
            std::process::id(),
            ts
        ));

        game.save_snapshot(&path).unwrap();
        let err = match Game::load_snapshot(&path) {
            Ok(_) => panic!("expected invalid snapshot to be rejected"),
            Err(err) => err,
        };
        let _ = std::fs::remove_file(&path);
        assert!(err.to_string().contains("invalid snapshot state loaded"));
    }

    #[test]
    fn memory_effects_set_and_clear_flags() {
        let mut g = Game::new(Tempo::default(), CharacterId::new("junior_dev"), Some(11));
        assert!(g.state.run_memory.flags.is_empty());

        let applied = g.apply_memory_effects(
            &[MemoryEffect {
                op: MemoryEffectOp::SetFlag,
                flag: "test_flag".to_string(),
            }],
            "test",
        );
        assert_eq!(applied.len(), 1);
        assert_eq!(applied[0].op, MemoryEffectOp::SetFlag);
        assert_eq!(applied[0].flag, "test_flag");
        assert!(g.state.run_memory.flags.contains("test_flag"));

        let applied = g.apply_memory_effects(
            &[MemoryEffect {
                op: MemoryEffectOp::ClearFlag,
                flag: "test_flag".to_string(),
            }],
            "test",
        );
        assert_eq!(applied.len(), 1);
        assert_eq!(applied[0].op, MemoryEffectOp::ClearFlag);
        assert_eq!(applied[0].flag, "test_flag");
        assert!(!g.state.run_memory.flags.contains("test_flag"));
    }

    #[test]
    fn authored_phase_scaling_is_monotonic() {
        let mut g = Game::new(Tempo::default(), CharacterId::new("junior_dev"), Some(12));
        let pos = ResourceDelta {
            money: 2_500,
            sanity: 6,
            audience: 6,
            ai_skill: 6,
            friends: 6,
        };
        let neg = ResourceDelta {
            money: -2_500,
            sanity: -6,
            audience: -6,
            ai_skill: -6,
            friends: -6,
        };

        g.state.turn = 1;
        let e_pos = g.apply_authored_phase_scaling(pos);
        let e_neg = g.apply_authored_phase_scaling(neg);

        g.state.turn = 4;
        let m_pos = g.apply_authored_phase_scaling(pos);
        let m_neg = g.apply_authored_phase_scaling(neg);

        g.state.turn = 7;
        let l_pos = g.apply_authored_phase_scaling(pos);
        let l_neg = g.apply_authored_phase_scaling(neg);

        g.state.turn = 10;
        let eg_pos = g.apply_authored_phase_scaling(pos);
        let eg_neg = g.apply_authored_phase_scaling(neg);

        assert!(
            e_pos.money <= m_pos.money && m_pos.money <= l_pos.money && l_pos.money <= eg_pos.money
        );
        assert!(
            e_pos.sanity <= m_pos.sanity
                && m_pos.sanity <= l_pos.sanity
                && l_pos.sanity <= eg_pos.sanity
        );
        assert!(
            e_neg.money >= m_neg.money && m_neg.money >= l_neg.money && l_neg.money >= eg_neg.money
        );
        assert!(
            e_neg.sanity >= m_neg.sanity
                && m_neg.sanity >= l_neg.sanity
                && l_neg.sanity >= eg_neg.sanity
        );
    }

    #[test]
    fn authored_phase_scaling_guardrails_stay_reasonable() {
        let mut g = Game::new(Tempo::default(), CharacterId::new("junior_dev"), Some(13));
        g.state.turn = g.state.max_turns;
        let base = ResourceDelta {
            money: 5_000,
            sanity: 14,
            audience: -14,
            ai_skill: 14,
            friends: -14,
        };
        let scaled = g.apply_authored_phase_scaling(base);
        assert_eq!(scaled.money, 7_000);
        assert_eq!(scaled.sanity, 20);
        assert_eq!(scaled.audience, -20);
        assert_eq!(scaled.ai_skill, 20);
        assert_eq!(scaled.friends, -20);
    }

    #[test]
    fn baseline_friends_decay_curve_is_mild() {
        assert_eq!(baseline_friends_decay_per_turn(Phase::Early), 0);
        assert_eq!(baseline_friends_decay_per_turn(Phase::Mid), -1);
        assert_eq!(baseline_friends_decay_per_turn(Phase::Late), -1);
        assert_eq!(baseline_friends_decay_per_turn(Phase::Endgame), -2);
    }
}
