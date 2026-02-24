use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

use rand::SeedableRng;
use rand::prelude::IteratorRandom;
use rand::rngs::StdRng;

use permanent_underclass::engine::characters::{Ability, CharacterCatalog, CharacterId};
use permanent_underclass::engine::endings::{self, EndingId};
use permanent_underclass::engine::events::TurnEvents;
use permanent_underclass::engine::game::Game;
use permanent_underclass::engine::subscription::SubscriptionTier;
use permanent_underclass::engine::tempo::Tempo;

#[derive(Debug, Clone)]
struct Config {
    seeds: u32,
    seed_start: u64,
    policies: Vec<Policy>,
    characters: Vec<CharacterId>,
    tempo: Tempo,
    output_jsonl: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Policy {
    Random,
    Balanced,
    Grind,
    Rest,
    Money,
    Luddite,
    Hedge,
    Clout,
    Social,
    Survivor,
    SelfDestruct,
    Hermit,
    Accelerationist,
}

impl Policy {
    const DEFAULT: [Policy; 13] = [
        Policy::Balanced,
        Policy::Random,
        Policy::Grind,
        Policy::Rest,
        Policy::Money,
        Policy::Luddite,
        Policy::Hedge,
        Policy::Clout,
        Policy::Social,
        Policy::Survivor,
        Policy::SelfDestruct,
        Policy::Hermit,
        Policy::Accelerationist,
    ];

    fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "random" => Some(Policy::Random),
            "balanced" => Some(Policy::Balanced),
            "grind" => Some(Policy::Grind),
            "rest" => Some(Policy::Rest),
            "money" => Some(Policy::Money),
            "luddite" => Some(Policy::Luddite),
            "hedge" => Some(Policy::Hedge),
            "clout" => Some(Policy::Clout),
            "social" => Some(Policy::Social),
            "survivor" => Some(Policy::Survivor),
            "self-destruct" | "self_destruct" => Some(Policy::SelfDestruct),
            "hermit" => Some(Policy::Hermit),
            "accelerationist" => Some(Policy::Accelerationist),
            _ => None,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Policy::Random => "random",
            Policy::Balanced => "balanced",
            Policy::Grind => "grind",
            Policy::Rest => "rest",
            Policy::Money => "money",
            Policy::Luddite => "luddite",
            Policy::Hedge => "hedge",
            Policy::Clout => "clout",
            Policy::Social => "social",
            Policy::Survivor => "survivor",
            Policy::SelfDestruct => "self-destruct",
            Policy::Hermit => "hermit",
            Policy::Accelerationist => "accelerationist",
        }
    }
}

#[derive(Debug, Clone)]
struct RunRecord {
    character: CharacterId,
    policy: Policy,
    seed: u64,
    ending: EndingId,
    turn: u32,
}

fn main() {
    let cfg = parse_args();
    let mut records = Vec::new();

    for character in &cfg.characters {
        for policy in cfg.policies.iter().copied() {
            for i in 0..cfg.seeds {
                let seed = cfg.seed_start + i as u64;
                records.push(run_one(character.clone(), policy, seed, cfg.tempo));
            }
        }
    }

    if let Some(path) = cfg.output_jsonl.as_deref() {
        write_records_jsonl(path, &records, cfg.tempo);
    }

    print_report(&records, cfg.tempo);
}

fn parse_args() -> Config {
    let default_characters = CharacterCatalog::embedded()
        .map(|catalog| catalog.all_ids())
        .unwrap_or_default();
    let mut cfg = Config {
        seeds: 20,
        seed_start: 0,
        policies: Policy::DEFAULT.to_vec(),
        characters: default_characters,
        tempo: Tempo::default(),
        output_jsonl: None,
    };

    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--seeds" => {
                if let Some(v) = it.next()
                    && let Ok(n) = v.parse::<u32>()
                {
                    cfg.seeds = n.max(1);
                }
            }
            "--seed-start" => {
                if let Some(v) = it.next()
                    && let Ok(n) = v.parse::<u64>()
                {
                    cfg.seed_start = n;
                }
            }
            "--policies" => {
                if let Some(v) = it.next() {
                    let parsed: Vec<_> = v.split(',').filter_map(Policy::parse).collect();
                    if !parsed.is_empty() {
                        cfg.policies = parsed;
                    }
                }
            }
            "--character" => {
                if let Some(v) = it.next() {
                    if v.trim().eq_ignore_ascii_case("all") {
                        cfg.characters = CharacterCatalog::embedded()
                            .map(|catalog| catalog.all_ids())
                            .unwrap_or_default();
                    } else if let Some(c) = parse_character(&v) {
                        cfg.characters = vec![c];
                    }
                }
            }
            "--tempo" => {
                if let Some(v) = it.next()
                    && let Some(t) = Tempo::from_cli_value(&v)
                {
                    cfg.tempo = t;
                }
            }
            "--output-jsonl" => {
                if let Some(v) = it.next() {
                    cfg.output_jsonl = Some(v);
                }
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            _ => {}
        }
    }

    cfg
}

fn print_help() {
    println!("balance_sim");
    println!();
    println!("Usage:");
    println!("  cargo run --bin balance_sim -- [--seeds N] [--seed-start N]");
    println!("                                 [--character <id>|all] [--tempo simple|standard]");
    println!("                                 [--policies p1,p2,...] [--output-jsonl path]");
    println!();
    println!("Character ids:");
    if let Ok(catalog) = CharacterCatalog::embedded() {
        println!("  {}", catalog.cli_ids().join(" | "));
    } else {
        println!("  (failed to load character catalog)");
    }
    println!(
        "Policies: balanced, random, grind, rest, money, luddite, hedge, clout, social, survivor, self-destruct, hermit, accelerationist"
    );
}

fn parse_character(s: &str) -> Option<CharacterId> {
    let catalog = CharacterCatalog::embedded().ok()?;
    if let Some(id) = catalog.parse_id(s) {
        return Some(id);
    }
    if s.trim().eq_ignore_ascii_case("ex-faang") {
        return Some(CharacterId::new("ex_faang_engineer"));
    }
    None
}

fn run_one(character: CharacterId, policy: Policy, seed: u64, tempo: Tempo) -> RunRecord {
    let mut game = Game::new(tempo, character.clone(), Some(seed));
    let mut pick_rng = StdRng::seed_from_u64(seed ^ 0xD1B5_4A32_9C17_44EF);

    loop {
        let events = game.begin_turn();
        if let Some(reason) = game.death_reason() {
            return RunRecord {
                character: character.clone(),
                policy,
                seed,
                ending: endings::death_ending(reason).id,
                turn: game.state.turn,
            };
        }

        maybe_manage_subscription(&mut game, policy);

        if should_use_ability(&game, policy) {
            let _ = game.use_ability();
        } else {
            let key = choose_key(&game, &events, policy, &mut pick_rng);
            let _ = game.resolve_choice(&events, key);
        }

        if let Some(reason) = game.death_reason() {
            return RunRecord {
                character: character.clone(),
                policy,
                seed,
                ending: endings::death_ending(reason).id,
                turn: game.state.turn,
            };
        }
        if game.is_final_turn() {
            return RunRecord {
                character: character.clone(),
                policy,
                seed,
                ending: endings::evaluate(&game.state).id,
                turn: game.state.turn,
            };
        }
    }
}

fn maybe_manage_subscription(game: &mut Game, policy: Policy) {
    if matches!(policy, Policy::Luddite | Policy::SelfDestruct) {
        let _ = game.set_subscription(SubscriptionTier::Free);
        return;
    }

    if matches!(policy, Policy::Rest) {
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

fn should_use_ability(game: &Game, policy: Policy) -> bool {
    if matches!(policy, Policy::Luddite | Policy::SelfDestruct) {
        return false;
    }
    if !game.can_use_ability_in_turn() {
        return false;
    }

    match game.state.ability {
        Ability::Interview => game.state.stats.money > 1_000 && game.state.stats.sanity < 75,
        Ability::GoViral => {
            game.state.turn >= 4 && game.state.stats.sanity > 35 && !matches!(policy, Policy::Rest)
        }
        Ability::VibeCode => game.state.turn >= 3 && !matches!(policy, Policy::Rest),
        _ => false,
    }
}

fn choose_key(game: &Game, events: &TurnEvents, policy: Policy, rng: &mut StdRng) -> char {
    if matches!(policy, Policy::Random) {
        return events
            .personal
            .choices
            .iter()
            .filter(|c| c.is_available(game.state.stats.money))
            .choose(rng)
            .map(|c| c.key)
            .or_else(|| events.personal.choices.first().map(|c| c.key))
            .unwrap_or('1');
    }

    let mut best: Option<(f64, char)> = None;

    for c in events
        .personal
        .choices
        .iter()
        .filter(|c| c.is_available(game.state.stats.money))
    {
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

    if let Some((_, key)) = best {
        key
    } else {
        events
            .personal
            .choices
            .first()
            .map(|c| c.key)
            .unwrap_or('1')
    }
}

fn utility(policy: Policy, c: permanent_underclass::engine::types::ResourceDelta) -> f64 {
    let money = c.money as f64 / 1000.0;
    let sanity = c.sanity as f64;
    let audience = c.audience as f64;
    let ai_skill = c.ai_skill as f64;
    let friends = c.friends as f64;

    match policy {
        Policy::Random => {
            money * 1.0 + sanity * 1.0 + audience * 1.0 + ai_skill * 1.0 + friends * 1.0
        }
        Policy::Balanced => {
            money * 1.2 + sanity * 1.6 + audience * 0.8 + ai_skill * 1.0 + friends * 1.3
        }
        Policy::Grind => {
            money * 2.6 + sanity * 0.3 + audience * 0.8 + ai_skill * 1.8 + friends * 0.1
        }
        Policy::Rest => {
            money * 0.5 + sanity * 2.5 + audience * 0.4 + ai_skill * 0.3 + friends * 1.8
        }
        Policy::Money => {
            money * 3.0 + sanity * 0.2 + audience * 0.6 + ai_skill * 0.9 + friends * 0.2
        }
        Policy::Luddite => {
            money * 0.8 + sanity * 2.0 + audience * 0.5 + ai_skill * -3.0 + friends * 1.4
        }
        Policy::Hedge => {
            money * 1.8 + sanity * 1.2 + audience * 0.8 + ai_skill * 1.1 + friends * 1.0
        }
        Policy::Clout => {
            money * 0.4 + sanity * 0.5 + audience * 3.0 + ai_skill * 0.6 + friends * 0.3
        }
        Policy::Social => {
            money * 0.3 + sanity * 0.8 + audience * 0.5 + ai_skill * 0.2 + friends * 3.0
        }
        Policy::Survivor => {
            money * 2.0 + sanity * 2.0 + audience * 0.3 + ai_skill * 0.3 + friends * 0.3
        }
        Policy::SelfDestruct => -money - sanity - audience - ai_skill - friends,
        Policy::Hermit => {
            money * 2.0 + sanity * 1.5 + audience * -2.0 + ai_skill * 0.5 + friends * -2.0
        }
        Policy::Accelerationist => {
            money * 0.3 + sanity * 0.3 + audience * 0.3 + ai_skill * 3.0 + friends * 0.1
        }
    }
}

fn write_records_jsonl(path: &str, records: &[RunRecord], tempo: Tempo) {
    let output_path = Path::new(path);
    if let Some(parent) = output_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).expect("failed to create parent directory for --output-jsonl");
    }

    let file = File::create(output_path).expect("failed to create --output-jsonl file");
    let mut writer = BufWriter::new(file);
    for r in records {
        let row = serde_json::json!({
            "character": character_id(&r.character),
            "policy": r.policy.label(),
            "seed": r.seed,
            "ending": format!("{:?}", r.ending),
            "turn": r.turn,
            "tempo": tempo.cli_name(),
            "is_death": is_death_ending(&r.ending),
        });
        serde_json::to_writer(&mut writer, &row)
            .expect("failed to serialize a run record to --output-jsonl");
        writer
            .write_all(b"\n")
            .expect("failed to write newline to --output-jsonl");
    }
}

fn is_death_ending(ending: &EndingId) -> bool {
    matches!(ending, EndingId::Broke | EndingId::BurnedOut)
}

fn print_report(records: &[RunRecord], tempo: Tempo) {
    println!("TEMPO {}", tempo.cli_name());
    let total = records.len().max(1) as f64;
    let deaths = records
        .iter()
        .filter(|r| is_death_ending(&r.ending))
        .count();
    let survives = records.len().saturating_sub(deaths);
    let avg_turn = records.iter().map(|r| r.turn as f64).sum::<f64>() / total;

    println!("TOTAL_RUNS {}", records.len());
    println!("SURVIVE_RATE {:.1}%", (survives as f64 / total) * 100.0);
    println!("AVG_TURN {:.2}", avg_turn);
    println!();

    println!("BY_CHARACTER");
    let Ok(catalog) = CharacterCatalog::embedded() else {
        println!("  (character catalog unavailable)");
        return;
    };
    for character in catalog.all_ids() {
        let character_rows: Vec<_> = records
            .iter()
            .filter(|r| r.character == character)
            .collect();
        if character_rows.is_empty() {
            continue;
        }
        let character_total = character_rows.len() as f64;
        let character_survive = character_rows
            .iter()
            .filter(|r| !is_death_ending(&r.ending))
            .count() as f64;
        let character_avg_turn =
            character_rows.iter().map(|r| r.turn as f64).sum::<f64>() / character_total;
        println!(
            "  {:<20} runs={:<4} survive={:>5.1}% avg_turn={:>5.2}",
            character_id(&character),
            character_rows.len(),
            (character_survive / character_total) * 100.0,
            character_avg_turn
        );
    }
    println!();

    println!("BY_POLICY");
    for p in Policy::DEFAULT {
        let rows: Vec<_> = records.iter().filter(|r| r.policy == p).collect();
        if rows.is_empty() {
            continue;
        }
        let n = rows.len() as f64;
        let survive = rows.iter().filter(|r| !is_death_ending(&r.ending)).count() as f64;
        let avg_turn = rows.iter().map(|r| r.turn as f64).sum::<f64>() / n;
        println!(
            "  {:<10} runs={:<4} survive={:>5.1}% avg_turn={:>5.2}",
            p.label(),
            rows.len(),
            (survive / n) * 100.0,
            avg_turn
        );
    }
    println!();

    println!("ENDING_COUNTS");
    let mut ending_counts: BTreeMap<String, usize> = BTreeMap::new();
    for r in records {
        *ending_counts.entry(format!("{:?}", r.ending)).or_default() += 1;
    }
    for (ending, count) in ending_counts {
        println!("  {:<20} {}", ending, count);
    }

    println!();
    println!("SAMPLE_ROWS");
    for r in records.iter().take(10) {
        println!(
            "  character={} policy={} seed={} turn={} ending={:?}",
            character_id(&r.character),
            r.policy.label(),
            r.seed,
            r.turn,
            r.ending
        );
    }
}

fn character_id(character: &CharacterId) -> String {
    character.to_cli_slug()
}
