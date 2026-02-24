use std::collections::{HashSet, VecDeque};

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::engine::ai::Phase;
use crate::engine::characters::CharacterId;
use crate::engine::content::{Content, EventDef, PhaseGate};
use crate::engine::events::{
    Choice, PersonalEvent, TurnEvents, WorldEvent, WorldEventId, WorldEventKind,
};
use crate::engine::headlines::Headlines;
use crate::engine::types::{ResourceDelta, Stats};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Showrunner {
    recent_personal_ids: VecDeque<String>,
}

impl Default for Showrunner {
    fn default() -> Self {
        Self::new()
    }
}

impl Showrunner {
    pub fn new() -> Self {
        Self {
            recent_personal_ids: VecDeque::new(),
        }
    }

    pub fn generate_turn<R: Rng>(&mut self, rng: &mut R, ctx: &ShowrunnerContext) -> TurnEvents {
        let world = world_random_headline(rng, ctx.phase);
        let personal = self.pick_catalog_personal(rng, ctx);

        // Track the last N personal events to prevent repeats.
        self.note_personal(&personal.id);

        TurnEvents { world, personal }
    }

    fn note_personal(&mut self, id: &str) {
        const KEEP: usize = 8;
        self.recent_personal_ids.push_back(id.to_string());
        while self.recent_personal_ids.len() > KEEP {
            let _ = self.recent_personal_ids.pop_front();
        }
    }

    fn pick_catalog_personal<R: Rng>(&self, rng: &mut R, ctx: &ShowrunnerContext) -> PersonalEvent {
        let Ok(content) = Content::embedded() else {
            eprintln!("WARN: failed to load event content; using fallback personal event");
            return fallback_generic_turn();
        };

        let recent: HashSet<&str> = self
            .recent_personal_ids
            .iter()
            .map(|s| s.as_str())
            .collect();

        let character_phase_pool =
            character_phase_pool_for_phase(content, &ctx.character, ctx.phase);
        let universal_phase_pool = universal_phase_pool_for_phase(content, ctx.phase);
        let character_any_pool = character_any_pool(content, &ctx.character);
        let universal_any_pool = universal_any_pool(content);
        if let Some(pick) = pick_from_ordered_pools(
            rng,
            &character_phase_pool,
            &character_any_pool,
            &universal_any_pool,
            &universal_phase_pool,
            &recent,
            &ctx.memory_flags,
            &ctx.seen_personal_ids,
            ctx.phase,
        ) {
            return personal_from_def(pick);
        }

        fallback_generic_turn()
    }
}

#[derive(Debug, Clone)]
pub struct ShowrunnerContext {
    pub turn: u32,
    pub max_turns: u32,
    pub ai_x10: u32,
    pub phase: Phase,
    pub character: CharacterId,
    pub stats: Stats,
    pub memory_flags: HashSet<String>,
    pub seen_personal_ids: HashSet<String>,
}

fn character_phase_pool_for_phase<'a>(
    content: &'a Content,
    character: &CharacterId,
    phase: Phase,
) -> Vec<&'a crate::engine::content::EventDef> {
    let mut out = Vec::new();
    if let Some(class_events) = content.per_character.get(character) {
        for ev in class_events {
            if ev.phase != PhaseGate::Any && ev.phase.matches(phase) {
                out.push(ev);
            }
        }
    }
    out
}

fn universal_phase_pool_for_phase(content: &Content, phase: Phase) -> Vec<&EventDef> {
    let mut out = Vec::new();
    for ev in &content.universal {
        if ev.phase != PhaseGate::Any && ev.phase.matches(phase) {
            out.push(ev);
        }
    }
    out
}

fn character_any_pool<'a>(content: &'a Content, character: &CharacterId) -> Vec<&'a EventDef> {
    let mut out = Vec::new();
    if let Some(class_events) = content.per_character.get(character) {
        for ev in class_events {
            if ev.phase == PhaseGate::Any {
                out.push(ev);
            }
        }
    }
    out
}

fn universal_any_pool(content: &Content) -> Vec<&EventDef> {
    let mut out = Vec::new();
    for ev in &content.universal {
        if ev.phase == PhaseGate::Any {
            out.push(ev);
        }
    }
    out
}

fn pick_with_starvation_fallbacks<'a, R: Rng>(
    rng: &mut R,
    pool: &'a [&'a EventDef],
    recent: &HashSet<&str>,
    memory_flags: &HashSet<String>,
    seen_ids: &HashSet<String>,
    phase: Phase,
) -> Option<&'a EventDef> {
    // Strict pick: dedup + memory gates.
    if let Some(pick) = pick_from_pool(
        rng,
        pool,
        recent,
        memory_flags,
        seen_ids,
        true,
        true,
        Some(phase),
    ) {
        return Some(pick);
    }

    // Fallback 1: disable recent dedup for this pick.
    if let Some(pick) = pick_from_pool(
        rng,
        pool,
        recent,
        memory_flags,
        seen_ids,
        false,
        true,
        Some(phase),
    ) {
        return Some(pick);
    }

    // Fallback 2: disable requires/forbids for this pick.
    if let Some(pick) = pick_from_pool(
        rng,
        pool,
        recent,
        memory_flags,
        seen_ids,
        false,
        false,
        Some(phase),
    ) {
        return Some(pick);
    }

    // Fallback 3: strict-phase ungated repeatable events.
    let last_resort: Vec<_> = pool
        .iter()
        .copied()
        .filter(|e| !seen_ids.contains(&e.id))
        .filter(|e| e.requires.is_empty() && e.forbids.is_empty() && !e.once_per_run)
        .collect();
    if last_resort.is_empty() {
        None
    } else {
        Some(last_resort[rng.gen_range(0..last_resort.len())])
    }
}

fn pick_from_character_then_universal<'a, R: Rng>(
    rng: &mut R,
    character_pool: &'a [&'a EventDef],
    universal_pool: &'a [&'a EventDef],
    recent: &HashSet<&str>,
    memory_flags: &HashSet<String>,
    seen_ids: &HashSet<String>,
    phase: Phase,
) -> Option<&'a EventDef> {
    if let Some(pick) =
        pick_with_starvation_fallbacks(rng, character_pool, recent, memory_flags, seen_ids, phase)
    {
        return Some(pick);
    }
    pick_with_starvation_fallbacks(rng, universal_pool, recent, memory_flags, seen_ids, phase)
}

#[allow(clippy::too_many_arguments)]
fn pick_from_ordered_pools<'a, R: Rng>(
    rng: &mut R,
    character_phase_pool: &'a [&'a EventDef],
    character_any_pool: &'a [&'a EventDef],
    universal_any_pool: &'a [&'a EventDef],
    universal_phase_pool: &'a [&'a EventDef],
    recent: &HashSet<&str>,
    memory_flags: &HashSet<String>,
    seen_ids: &HashSet<String>,
    phase: Phase,
) -> Option<&'a EventDef> {
    // Priority order:
    // 1) character (strict phase)
    // 2) Any fallback (character first, then universal)
    // 3) universal (strict phase)
    //
    // Every stage still goes through dedup + memory gates + starvation fallbacks.
    if let Some(pick) = pick_with_starvation_fallbacks(
        rng,
        character_phase_pool,
        recent,
        memory_flags,
        seen_ids,
        phase,
    ) {
        return Some(pick);
    }
    if let Some(pick) = pick_from_character_then_universal(
        rng,
        character_any_pool,
        universal_any_pool,
        recent,
        memory_flags,
        seen_ids,
        phase,
    ) {
        return Some(pick);
    }
    pick_with_starvation_fallbacks(
        rng,
        universal_phase_pool,
        recent,
        memory_flags,
        seen_ids,
        phase,
    )
}

#[allow(clippy::too_many_arguments)]
fn pick_from_pool<'a, R: Rng>(
    rng: &mut R,
    pool: &'a [&'a EventDef],
    recent: &HashSet<&str>,
    memory_flags: &HashSet<String>,
    seen_ids: &HashSet<String>,
    enforce_dedup: bool,
    enforce_memory_gates: bool,
    preferred_phase: Option<Phase>,
) -> Option<&'a EventDef> {
    let filtered: Vec<_> = pool
        .iter()
        .copied()
        // Authored events should be seen at most once per run.
        .filter(|e| !seen_ids.contains(&e.id))
        .filter(|e| !enforce_dedup || !recent.contains(e.id.as_str()))
        .filter(|e| !enforce_memory_gates || memory_eligible(e, memory_flags))
        .collect();
    if filtered.is_empty() {
        return None;
    }

    let total_weight: u32 = filtered
        .iter()
        .map(|e| event_pick_weight(e, preferred_phase))
        .sum();
    if total_weight == 0 {
        return Some(filtered[rng.gen_range(0..filtered.len())]);
    }

    let mut roll = rng.gen_range(0..total_weight);
    for ev in filtered {
        let w = event_pick_weight(ev, preferred_phase);
        if roll < w {
            return Some(ev);
        }
        roll -= w;
    }
    None
}

fn event_pick_weight(e: &EventDef, preferred_phase: Option<Phase>) -> u32 {
    let Some(phase) = preferred_phase else {
        return 1;
    };
    match e.phase {
        PhaseGate::Any => 1,
        gate if gate.matches(phase) => 6,
        _ => 1,
    }
}

fn memory_eligible(e: &EventDef, flags: &HashSet<String>) -> bool {
    e.requires.iter().all(|f| flags.contains(f)) && e.forbids.iter().all(|f| !flags.contains(f))
}

fn personal_from_def(def: &crate::engine::content::EventDef) -> PersonalEvent {
    let mut choices: Vec<Choice> = Vec::with_capacity(def.choices.len());
    for (i, c) in def.choices.iter().enumerate() {
        let key = (b'1' + i as u8) as char;
        choices.push(Choice {
            key,
            title: c.title.clone(),
            body: c.body.clone(),
            delta: c.delta,
            min_money: c.min_money,
            effects: c.effects.clone(),
        });
    }

    PersonalEvent {
        id: def.id.clone(),
        title: def.title.clone(),
        body: def.body.clone(),
        choices,
        allow_freeform: def.allow_freeform,
    }
}

fn fallback_generic_turn() -> PersonalEvent {
    PersonalEvent {
        id: "fallback_generic_turn".to_string(),
        title: "YOUR TURN".to_string(),
        body: "Pick a lane. Everything costs something.".to_string(),
        choices: vec![
            Choice {
                key: '1',
                title: "GRIND".to_string(),
                body: "Ship, learn, optimize. Ignore the group chat.".to_string(),
                delta: ResourceDelta {
                    money: 450,
                    ai_skill: 6,
                    sanity: -5,
                    friends: -2,
                    audience: 1,
                },
                min_money: None,
                effects: Vec::new(),
            },
            Choice {
                key: '2',
                title: "NETWORK".to_string(),
                body: "Meet someone who might matter later.".to_string(),
                delta: ResourceDelta {
                    audience: 6,
                    friends: 8,
                    sanity: 1,
                    money: -200,
                    ..ResourceDelta::ZERO
                },
                min_money: Some(200),
                effects: Vec::new(),
            },
            Choice {
                key: '3',
                title: "REST".to_string(),
                body: "Sleep. Eat. Touch grass. The discourse can wait.".to_string(),
                delta: ResourceDelta {
                    sanity: 14,
                    friends: 4,
                    audience: -1,
                    ..ResourceDelta::ZERO
                },
                min_money: None,
                effects: Vec::new(),
            },
        ],
        allow_freeform: true,
    }
}

fn world_random_headline<R: Rng>(rng: &mut R, phase: Phase) -> WorldEvent {
    let Ok(headlines) = Headlines::embedded() else {
        eprintln!("WARN: failed to load headlines; using fallback world event");
        return WorldEvent {
            id: WorldEventId::Atmosphere,
            kind: WorldEventKind::Atmosphere,
            title: "BREAKING".to_string(),
            body: "The vibes are off. Nobody knows why.".to_string(),
            delta: ResourceDelta {
                sanity: -1,
                ..ResourceDelta::ZERO
            },
        };
    };
    let mut pool = Vec::new();
    for h in &headlines.list {
        if h.phase.matches(phase) {
            pool.push(h);
        }
    }

    let (body, delta) = if pool.is_empty() {
        (
            "The vibes are off. Nobody knows why.".to_string(),
            ResourceDelta {
                sanity: -1,
                ..ResourceDelta::ZERO
            },
        )
    } else {
        let h = pool[rng.gen_range(0..pool.len())];
        (h.text.clone(), h.delta)
    };

    WorldEvent {
        id: WorldEventId::Atmosphere,
        kind: WorldEventKind::Atmosphere,
        title: "BREAKING".to_string(),
        body,
        delta,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::content::{ChoiceDef, EventDef, PhaseGate};
    use crate::engine::tempo::Tempo;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn showrunner_generates_three_choices() {
        let mut sr = Showrunner::new();
        let mut rng = StdRng::seed_from_u64(0);
        let ctx = ShowrunnerContext {
            turn: 1,
            max_turns: Tempo::default().max_turns(),
            ai_x10: 100,
            phase: Phase::Early,
            character: CharacterId::new("junior_dev"),
            stats: Stats {
                money: 8_000,
                sanity: 70,
                audience: 20,
                ai_skill: 65,
                friends: 45,
            },
            memory_flags: HashSet::new(),
            seen_personal_ids: HashSet::new(),
        };
        let ev = sr.generate_turn(&mut rng, &ctx);
        assert_eq!(ev.personal.choices.len(), 3);
        assert!(!ev.personal.id.is_empty());
    }

    #[test]
    fn world_event_is_random_atmosphere() {
        let mut sr = Showrunner::new();
        let mut rng = StdRng::seed_from_u64(0);
        let ctx = ShowrunnerContext {
            turn: 1,
            max_turns: Tempo::default().max_turns(),
            ai_x10: 1000,
            phase: Phase::Late,
            character: CharacterId::new("junior_dev"),
            stats: Stats {
                money: 8_000,
                sanity: 70,
                audience: 20,
                ai_skill: 65,
                friends: 45,
            },
            memory_flags: HashSet::new(),
            seen_personal_ids: HashSet::new(),
        };
        let ev = sr.generate_turn(&mut rng, &ctx);
        assert_eq!(ev.world.kind, WorldEventKind::Atmosphere);
        assert_eq!(ev.world.id, WorldEventId::Atmosphere);
    }

    fn test_choice() -> ChoiceDef {
        ChoiceDef {
            title: "A".to_string(),
            body: "A".to_string(),
            delta: ResourceDelta::ZERO,
            min_money: None,
            effects: Vec::new(),
            requires: Vec::new(),
            forbids: Vec::new(),
        }
    }

    fn test_event(id: &str) -> EventDef {
        EventDef {
            id: id.to_string(),
            phase: PhaseGate::Early,
            title: id.to_string(),
            body: id.to_string(),
            choices: [test_choice(), test_choice(), test_choice()],
            allow_freeform: true,
            requires: Vec::new(),
            forbids: Vec::new(),
            once_per_run: false,
        }
    }

    fn test_event_with_phase(id: &str, phase: PhaseGate) -> EventDef {
        let mut ev = test_event(id);
        ev.phase = phase;
        ev
    }

    #[test]
    fn memory_eligible_respects_requires_and_forbids() {
        let mut ev = test_event("memory_gate");
        ev.requires = vec!["has_mentor".to_string()];
        ev.forbids = vec!["burned_bridge".to_string()];

        let mut flags = HashSet::new();
        assert!(!memory_eligible(&ev, &flags));

        flags.insert("has_mentor".to_string());
        assert!(memory_eligible(&ev, &flags));

        flags.insert("burned_bridge".to_string());
        assert!(!memory_eligible(&ev, &flags));
    }

    #[test]
    fn pick_from_pool_never_relaxes_once_per_run() {
        let mut rng = StdRng::seed_from_u64(42);
        let mut ev = test_event("once_only");
        ev.once_per_run = true;
        let pool = vec![&ev];
        let recent = HashSet::new();
        let flags = HashSet::new();
        let mut seen = HashSet::new();
        seen.insert("once_only".to_string());

        let picked = pick_from_pool(
            &mut rng,
            &pool,
            &recent,
            &flags,
            &seen,
            false,
            false,
            Some(Phase::Early),
        );
        assert!(picked.is_none());
    }

    #[test]
    fn prefers_character_pool_when_both_have_candidates() {
        let mut rng = StdRng::seed_from_u64(7);
        let character_ev = test_event("character_pick");
        let universal_ev = test_event("universal_pick");
        let character_pool = vec![&character_ev];
        let universal_pool = vec![&universal_ev];
        let recent = HashSet::new();
        let flags = HashSet::new();
        let seen = HashSet::new();

        let picked = pick_from_character_then_universal(
            &mut rng,
            &character_pool,
            &universal_pool,
            &recent,
            &flags,
            &seen,
            Phase::Early,
        )
        .expect("expected a pick");
        assert_eq!(picked.id, "character_pick");
    }

    #[test]
    fn falls_back_to_universal_when_character_pool_is_exhausted() {
        let mut rng = StdRng::seed_from_u64(7);
        let character_ev = test_event("character_pick");
        let universal_ev = test_event("universal_pick");
        let character_pool = vec![&character_ev];
        let universal_pool = vec![&universal_ev];
        let recent = HashSet::new();
        let flags = HashSet::new();
        let mut seen = HashSet::new();
        seen.insert("character_pick".to_string());

        let picked = pick_from_character_then_universal(
            &mut rng,
            &character_pool,
            &universal_pool,
            &recent,
            &flags,
            &seen,
            Phase::Early,
        )
        .expect("expected fallback pick");
        assert_eq!(picked.id, "universal_pick");
    }

    #[test]
    fn phase_specific_pool_is_checked_before_any_fallback_pool() {
        let mut rng = StdRng::seed_from_u64(7);
        let character_phase = test_event_with_phase("character_phase", PhaseGate::Early);
        let character_any = test_event_with_phase("character_any", PhaseGate::Any);
        let universal_any = test_event_with_phase("universal_any", PhaseGate::Any);
        let universal_phase = test_event_with_phase("universal_phase", PhaseGate::Early);
        let recent = HashSet::new();
        let flags = HashSet::new();
        let seen = HashSet::new();
        let character_phase_pool = [&character_phase];
        let character_any_pool = [&character_any];
        let universal_any_pool = [&universal_any];
        let universal_phase_pool = [&universal_phase];

        let phase_pick = pick_from_ordered_pools(
            &mut rng,
            &character_phase_pool,
            &character_any_pool,
            &universal_any_pool,
            &universal_phase_pool,
            &recent,
            &flags,
            &seen,
            Phase::Early,
        )
        .expect("expected a phase-specific pick");
        assert_eq!(phase_pick.id, "character_phase");

        let any_pick = pick_from_ordered_pools(
            &mut rng,
            &[],
            &character_any_pool,
            &universal_any_pool,
            &universal_phase_pool,
            &recent,
            &flags,
            &seen,
            Phase::Early,
        )
        .expect("expected an Any fallback pick");
        assert_eq!(any_pick.id, "character_any");
    }

    #[test]
    fn any_fallback_precedes_universal_phase_pool() {
        let mut rng = StdRng::seed_from_u64(7);
        let character_any = test_event_with_phase("character_any", PhaseGate::Any);
        let universal_phase = test_event_with_phase("universal_phase", PhaseGate::Early);
        let character_any_pool = [&character_any];
        let universal_phase_pool = [&universal_phase];
        let recent = HashSet::new();
        let flags = HashSet::new();
        let seen = HashSet::new();

        let picked = pick_from_ordered_pools(
            &mut rng,
            &[],
            &character_any_pool,
            &[],
            &universal_phase_pool,
            &recent,
            &flags,
            &seen,
            Phase::Early,
        )
        .expect("expected a pick");
        assert_eq!(picked.id, "character_any");
    }
}
