use std::collections::{HashMap, HashSet};

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha12Rng;

use permanent_underclass::engine::characters::{Ability, CharacterCatalog, CharacterId};
use permanent_underclass::engine::content::Content;
use permanent_underclass::engine::endings;
use permanent_underclass::engine::events::TurnEvents;
use permanent_underclass::engine::game::Game;
use permanent_underclass::engine::subscription::SubscriptionTier;
use permanent_underclass::engine::tempo::Tempo;
use permanent_underclass::engine::types::ResourceDelta;

#[derive(Debug, Clone)]
struct SimResult {
    ending: endings::EndingId,
    turn: u32,
    ai_x10: u32,
    seen_event_ids: HashSet<String>,
    gated_turns: u32,
    total_turns: u32,
}

#[derive(Debug, Clone, Copy)]
enum SimPolicy {
    Random,
    Balanced,
    Grind,
    Rest,
    Money,
    Hedge,
    Luddite,
    Clout,
    Social,
    Survivor,
    SelfDestruct,
    Hermit,
    Accelerationist,
}

const DIVERSITY_POLICIES: [SimPolicy; 12] = [
    SimPolicy::Balanced,
    SimPolicy::Grind,
    SimPolicy::Rest,
    SimPolicy::Money,
    SimPolicy::Hedge,
    SimPolicy::Luddite,
    SimPolicy::Clout,
    SimPolicy::Social,
    SimPolicy::Survivor,
    SimPolicy::SelfDestruct,
    SimPolicy::Hermit,
    SimPolicy::Accelerationist,
];

fn character_pool_stats(character: &CharacterId) -> (HashSet<String>, HashSet<String>) {
    let content = Content::embedded().expect("embedded content must parse for sim tests");
    let mut pool_ids = HashSet::new();
    let mut gated_ids = HashSet::new();

    for ev in content
        .universal
        .iter()
        .chain(content.per_character.get(character).into_iter().flatten())
    {
        pool_ids.insert(ev.id.clone());
        if !ev.requires.is_empty() {
            gated_ids.insert(ev.id.clone());
        }
    }

    (pool_ids, gated_ids)
}

fn choose_key_for_policy(
    game: &Game,
    events: &TurnEvents,
    policy: SimPolicy,
    pick_rng: &mut ChaCha12Rng,
) -> char {
    let available: Vec<_> = events
        .personal
        .choices
        .iter()
        .filter(|c| c.is_available(game.state.stats.money))
        .collect();
    if available.is_empty() {
        return '1';
    }

    if matches!(policy, SimPolicy::Random) {
        return available[pick_rng.gen_range(0..available.len())].key;
    }

    let mut best: Option<(f64, char)> = None;
    for c in available {
        let util = utility(policy, game.preview_choice_delta(c));
        match best {
            None => best = Some((util, c.key)),
            Some((best_util, best_key)) => {
                let better = util > best_util
                    || ((util - best_util).abs() < f64::EPSILON && c.key < best_key);
                if better {
                    best = Some((util, c.key));
                }
            }
        }
    }

    best.map(|(_, key)| key).unwrap_or('1')
}

fn utility(policy: SimPolicy, c: ResourceDelta) -> f64 {
    let money = c.money as f64 / 1000.0;
    let sanity = c.sanity as f64;
    let audience = c.audience as f64;
    let ai_skill = c.ai_skill as f64;
    let friends = c.friends as f64;

    match policy {
        SimPolicy::Balanced => {
            money * 1.2 + sanity * 1.6 + audience * 0.8 + ai_skill * 1.0 + friends * 1.3
        }
        SimPolicy::Grind => {
            money * 2.6 + sanity * 0.3 + audience * 0.8 + ai_skill * 1.8 + friends * 0.1
        }
        SimPolicy::Rest => {
            money * 0.5 + sanity * 2.5 + audience * 0.4 + ai_skill * 0.3 + friends * 1.8
        }
        SimPolicy::Money => {
            money * 3.0 + sanity * 0.2 + audience * 0.6 + ai_skill * 0.9 + friends * 0.2
        }
        SimPolicy::Hedge => {
            money * 1.8 + sanity * 1.2 + audience * 0.8 + ai_skill * 1.1 + friends * 1.0
        }
        SimPolicy::Luddite => {
            money * 0.8 + sanity * 2.0 + audience * 0.5 + ai_skill * -3.0 + friends * 1.4
        }
        SimPolicy::Clout => {
            money * 0.4 + sanity * 0.5 + audience * 3.0 + ai_skill * 0.6 + friends * 0.3
        }
        SimPolicy::Social => {
            money * 0.3 + sanity * 0.8 + audience * 0.5 + ai_skill * 0.2 + friends * 3.0
        }
        SimPolicy::Survivor => {
            money * 2.0 + sanity * 2.0 + audience * 0.3 + ai_skill * 0.3 + friends * 0.3
        }
        SimPolicy::SelfDestruct => -money - sanity - audience - ai_skill - friends,
        SimPolicy::Hermit => {
            money * 2.0 + sanity * 1.5 + audience * -2.0 + ai_skill * 0.5 + friends * -2.0
        }
        SimPolicy::Accelerationist => {
            money * 0.3 + sanity * 0.3 + audience * 0.3 + ai_skill * 3.0 + friends * 0.1
        }
        SimPolicy::Random => {
            money * 1.0 + sanity * 1.0 + audience * 1.0 + ai_skill * 1.0 + friends * 1.0
        }
    }
}

fn maybe_manage_subscription(game: &mut Game, policy: SimPolicy) {
    if matches!(policy, SimPolicy::Luddite | SimPolicy::SelfDestruct) {
        let _ = game.set_subscription(SubscriptionTier::Free);
        return;
    }

    if matches!(policy, SimPolicy::Rest) {
        if game.state.stats.money < 1_500 {
            let _ = game.set_subscription(SubscriptionTier::Free);
        }
        return;
    }

    if game.can_set_subscription(SubscriptionTier::Giga) && game.state.stats.money > 220_000 {
        let _ = game.set_subscription(SubscriptionTier::Giga);
        return;
    }
    if game.can_set_subscription(SubscriptionTier::Ultra) && game.state.stats.money > 24_000 {
        let _ = game.set_subscription(SubscriptionTier::Ultra);
        return;
    }
    if game.can_set_subscription(SubscriptionTier::Pro) && game.state.stats.money > 4_000 {
        let _ = game.set_subscription(SubscriptionTier::Pro);
        return;
    }
    if game.state.stats.money < 1_500 {
        let _ = game.set_subscription(SubscriptionTier::Free);
    }
}

fn should_use_ability(game: &Game, policy: SimPolicy) -> bool {
    if matches!(policy, SimPolicy::Luddite | SimPolicy::SelfDestruct) {
        return false;
    }
    if !game.can_use_ability_in_turn() {
        return false;
    }
    match game.state.ability {
        Ability::Interview => game.state.stats.money > 1_000 && game.state.stats.sanity < 75,
        Ability::GoViral => {
            game.state.turn >= 4
                && game.state.stats.sanity > 35
                && !matches!(policy, SimPolicy::Rest)
        }
        Ability::VibeCode => game.state.turn >= 3 && !matches!(policy, SimPolicy::Rest),
        _ => false,
    }
}

fn run_sim(
    mut game: Game,
    gated_ids: &HashSet<String>,
    pick_seed: u64,
    policy: SimPolicy,
) -> SimResult {
    let mut pick_rng = ChaCha12Rng::seed_from_u64(pick_seed);
    let mut seen_event_ids = HashSet::new();
    let mut gated_turns = 0_u32;
    let mut total_turns = 0_u32;

    loop {
        let events = game.begin_turn();
        total_turns = total_turns.saturating_add(1);
        if gated_ids.contains(&events.personal.id) {
            gated_turns = gated_turns.saturating_add(1);
        }
        seen_event_ids.insert(events.personal.id.clone());

        if let Some(reason) = game.death_reason() {
            return SimResult {
                ending: endings::death_ending(reason).id,
                turn: game.state.turn,
                ai_x10: game.state.ai.x10(),
                seen_event_ids,
                gated_turns,
                total_turns,
            };
        }

        maybe_manage_subscription(&mut game, policy);
        if should_use_ability(&game, policy) {
            let _ = game.use_ability();
        } else {
            let key = choose_key_for_policy(&game, &events, policy, &mut pick_rng);
            let _ = game.resolve_choice(&events, key);
        }

        if let Some(reason) = game.death_reason() {
            return SimResult {
                ending: endings::death_ending(reason).id,
                turn: game.state.turn,
                ai_x10: game.state.ai.x10(),
                seen_event_ids,
                gated_turns,
                total_turns,
            };
        }

        if game.is_final_turn() {
            return SimResult {
                ending: endings::evaluate(&game.state).id,
                turn: game.state.turn,
                ai_x10: game.state.ai.x10(),
                seen_event_ids,
                gated_turns,
                total_turns,
            };
        }
    }
}

fn sweep_seed(i: u64) -> u64 {
    let mut x = i.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

#[test]
fn sim_junior_dev_reaches_an_ending() {
    let character = CharacterId::new("junior_dev");
    let (_pool_ids, gated_ids) = character_pool_stats(&character);
    let game = Game::new(Tempo::default(), character, Some(1));
    let res = run_sim(game, &gated_ids, 10_001, SimPolicy::Random);

    assert!(matches!(
        res.ending,
        endings::EndingId::Broke
            | endings::EndingId::BurnedOut
            | endings::EndingId::Luddite
            | endings::EndingId::Oligarch
            | endings::EndingId::LonelyOligarch
            | endings::EndingId::Irreplaceable
            | endings::EndingId::Builder
            | endings::EndingId::Connector
            | endings::EndingId::Comfortable
            | endings::EndingId::EnlightenedDropout
            | endings::EndingId::Artist
            | endings::EndingId::FamousForNoReason
            | endings::EndingId::PermanentUnderclass
            | endings::EndingId::Survivor
    ));

    let max_turns = Tempo::default().max_turns();
    assert!(res.turn <= max_turns);
    if res.turn == max_turns {
        assert!(res.ai_x10 >= 950);
    }
}

#[test]
fn sim_plumber_reaches_an_ending() {
    let character = CharacterId::new("plumber");
    let (_pool_ids, gated_ids) = character_pool_stats(&character);
    let game = Game::new(Tempo::default(), character, Some(2));
    let res = run_sim(game, &gated_ids, 20_002, SimPolicy::Random);
    assert!(res.turn <= Tempo::default().max_turns());
}

#[test]
fn sim_matrix_no_panics() {
    for character in CharacterCatalog::embedded()
        .expect("character catalog")
        .all_ids()
        .into_iter()
    {
        let (_pool_ids, gated_ids) = character_pool_stats(&character);
        for seed in 0u64..20 {
            let game = Game::new(Tempo::default(), character.clone(), Some(seed));
            let res = run_sim(
                game,
                &gated_ids,
                seed ^ 0xA5A5_A5A5_A5A5_A5A5,
                SimPolicy::Random,
            );
            assert!(
                res.turn <= Tempo::default().max_turns(),
                "character={:?} seed={} exceeded turn limit: {}",
                character,
                seed,
                res.turn
            );
        }
    }
}

#[test]
fn sim_diversity_thresholds_hold() {
    const RUNS_PER_CLASS: u64 = 100;
    const MAX_EVENT_RUN_SHARE: f64 = 0.40;
    const MAX_EVENT_RUN_SHARE_TOLERANCE: f64 = 0.10;
    const MIN_POOL_COVERAGE: f64 = 0.48;
    const MAX_GATED_RATIO_PER_RUN: f64 = 0.35;
    const MIN_DISTINCT_ENDINGS: usize = 4;

    for character in CharacterCatalog::embedded()
        .expect("character catalog")
        .all_ids()
        .into_iter()
    {
        let (pool_ids, gated_ids) = character_pool_stats(&character);
        let pool_size = pool_ids.len();
        let mut per_event_run_count: HashMap<String, usize> = HashMap::new();
        let mut union_events_seen: HashSet<String> = HashSet::new();
        let mut endings_seen: HashSet<endings::EndingId> = HashSet::new();

        for idx in 0u64..RUNS_PER_CLASS {
            let seed = sweep_seed(idx);
            let policy = DIVERSITY_POLICIES[(idx as usize) % DIVERSITY_POLICIES.len()];
            let game = Game::new(Tempo::default(), character.clone(), Some(seed));
            let res = run_sim(game, &gated_ids, seed ^ 0x9E37_79B9_7F4A_7C15, policy);
            endings_seen.insert(res.ending);

            let gated_ratio = if res.total_turns == 0 {
                0.0
            } else {
                res.gated_turns as f64 / res.total_turns as f64
            };
            assert!(
                gated_ratio <= MAX_GATED_RATIO_PER_RUN,
                "character={:?} seed={} gated_ratio={:.2} exceeded {:.2}",
                character,
                seed,
                gated_ratio,
                MAX_GATED_RATIO_PER_RUN
            );

            for id in &res.seen_event_ids {
                if pool_ids.contains(id) {
                    union_events_seen.insert(id.clone());
                }
            }
            for id in res.seen_event_ids {
                if pool_ids.contains(&id) {
                    *per_event_run_count.entry(id).or_default() += 1;
                }
            }
        }

        for (event_id, count) in per_event_run_count {
            let run_share = count as f64 / RUNS_PER_CLASS as f64;
            assert!(
                run_share <= MAX_EVENT_RUN_SHARE + MAX_EVENT_RUN_SHARE_TOLERANCE,
                "character={:?} event={} run_share={:.2} exceeded {:.2} (+{:.2} tolerance)",
                character,
                event_id,
                run_share,
                MAX_EVENT_RUN_SHARE,
                MAX_EVENT_RUN_SHARE_TOLERANCE
            );
        }

        let pool_coverage = if pool_size == 0 {
            0.0
        } else {
            union_events_seen.len() as f64 / pool_size as f64
        };
        assert!(
            pool_coverage >= MIN_POOL_COVERAGE,
            "character={:?} pool_coverage={:.2} below {:.2} (seen={} pool={})",
            character,
            pool_coverage,
            MIN_POOL_COVERAGE,
            union_events_seen.len(),
            pool_size
        );

        assert!(
            endings_seen.len() >= MIN_DISTINCT_ENDINGS,
            "character={:?} distinct_endings={} below {}",
            character,
            endings_seen.len(),
            MIN_DISTINCT_ENDINGS
        );
    }
}
