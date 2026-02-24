use std::io::{self, Write};
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use crossterm::event::{self, Event};
use permanent_underclass::{app, tui, ui};

fn main() -> Result<()> {
    let opts = parse_args()?;
    if opts.help {
        print_help();
        return Ok(());
    }

    if opts.llm_bench {
        return run_llm_bench(opts);
    }

    if opts.cli {
        return run_cli(opts);
    }

    if opts.headless {
        return run_headless(opts);
    }

    tui::install_panic_hook();

    let mut tui = tui::Tui::enter()?;
    let res = run(&mut tui, &opts);

    // Always try to restore the terminal even if the game loop errors.
    if let Err(exit_err) = tui.exit()
        && res.is_ok()
    {
        return Err(exit_err);
    }

    res
}

fn run(tui: &mut tui::Tui, opts: &Options) -> Result<()> {
    let narrator = build_narrator(opts);
    let llm_provider = if opts.llm {
        Some(opts.llm_provider.as_deref().unwrap_or("auto"))
    } else {
        None
    };
    let mut app = app::App::new(narrator, opts.llm_profile, llm_provider, opts.tempo);

    // Calmer default: keep motion readable without feeling frantic.
    let tick_rate = Duration::from_millis(180);
    let mut last_tick = Instant::now();

    while app.running {
        tui.draw(|frame| ui::render(frame, &app))?;

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) => app.on_key(key),
                Event::Resize(_, _) => {}
                _ => {}
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = Instant::now();
        }
    }

    Ok(())
}

#[derive(Debug, Default)]
struct Options {
    headless: bool,
    cli: bool,
    help: bool,
    llm_bench: bool,
    llm_bench_iters: Option<u32>,
    llm: bool,
    llm_provider: Option<String>,
    llm_profile: bool,
    llm_model: Option<String>,
    llm_reasoning: Option<String>,
    save_file: Option<String>,
    resume_file: Option<String>,
    seed: Option<u64>,
    character: Option<permanent_underclass::engine::characters::CharacterId>,
    tempo: permanent_underclass::engine::tempo::Tempo,
}

fn parse_args() -> Result<Options> {
    let mut opts = Options::default();
    let mut it = std::env::args().skip(1);

    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--help" | "-h" => opts.help = true,
            "--headless" => opts.headless = true,
            "--cli" => opts.cli = true,
            "--llm-bench" => {
                opts.llm = true;
                opts.llm_bench = true;
            }
            "--llm-bench-iters" => {
                let v = next_arg_value(&mut it, "--llm-bench-iters")?;
                let n = v
                    .parse::<u32>()
                    .map_err(|_| anyhow!("Invalid --llm-bench-iters '{}': expected u32", v))?;
                opts.llm = true;
                opts.llm_bench = true;
                opts.llm_bench_iters = Some(n);
            }
            "--llm" => opts.llm = true,
            "--llm-provider" => {
                let v = next_arg_value(&mut it, "--llm-provider")?;
                let provider = v.to_ascii_lowercase();
                if !matches!(provider.as_str(), "auto" | "codex" | "claude") {
                    return Err(anyhow!(
                        "Invalid --llm-provider '{}': expected auto|codex|claude",
                        v
                    ));
                }
                opts.llm = true;
                opts.llm_provider = Some(provider);
            }
            "--llm-profile" => {
                opts.llm = true;
                opts.llm_profile = true;
            }
            "--llm-model" => {
                let v = next_arg_value(&mut it, "--llm-model")?;
                opts.llm = true;
                opts.llm_model = Some(validate_llm_model(&v)?);
            }
            "--llm-reasoning" | "--llm-thinking" => {
                let v = next_arg_value(&mut it, "--llm-reasoning")?;
                opts.llm = true;
                opts.llm_reasoning = Some(parse_llm_reasoning(&v)?);
            }
            "--save-file" => {
                opts.save_file = Some(next_arg_value(&mut it, "--save-file")?);
            }
            "--resume-file" => {
                opts.resume_file = Some(next_arg_value(&mut it, "--resume-file")?);
            }
            "--seed" => {
                let v = next_arg_value(&mut it, "--seed")?;
                let n = v
                    .parse::<u64>()
                    .map_err(|_| anyhow!("Invalid --seed '{}': expected u64", v))?;
                opts.seed = Some(n);
            }
            "--character" => {
                let v = next_arg_value(&mut it, "--character")?;
                let parsed = parse_character(&v).ok_or_else(|| {
                    anyhow!("Invalid --character '{}': run with --help for valid ids", v)
                })?;
                opts.character = Some(parsed);
            }
            "--tempo" => {
                let v = next_arg_value(&mut it, "--tempo")?;
                let tempo = parse_tempo(&v)
                    .ok_or_else(|| anyhow!("Invalid --tempo '{}': expected simple|standard", v))?;
                opts.tempo = tempo;
            }
            _ => return Err(anyhow!("Unknown argument '{}'. Run with --help.", arg)),
        }
    }

    Ok(opts)
}

fn next_arg_value(it: &mut impl Iterator<Item = String>, flag: &str) -> Result<String> {
    it.next()
        .ok_or_else(|| anyhow!("Missing value for {}", flag))
}

fn parse_llm_reasoning(raw: &str) -> Result<String> {
    let effort = raw.to_ascii_lowercase();
    if matches!(effort.as_str(), "low" | "medium" | "high" | "xhigh") {
        return Ok(effort);
    }
    Err(anyhow!(
        "Invalid --llm-reasoning '{}': expected low|medium|high|xhigh",
        raw
    ))
}

fn validate_llm_model(raw: &str) -> Result<String> {
    let model = raw.trim();
    if model.is_empty() {
        return Err(anyhow!("Invalid --llm-model: value cannot be empty"));
    }
    if model.len() > 128 {
        return Err(anyhow!(
            "Invalid --llm-model '{}': value is too long (max 128 chars)",
            raw
        ));
    }
    let is_valid = model
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':' | '/' | '+'));
    if !is_valid {
        return Err(anyhow!(
            "Invalid --llm-model '{}': allowed chars are A-Z, 0-9, and - _ . : / +",
            raw
        ));
    }
    Ok(model.to_string())
}

fn parse_character(s: &str) -> Option<permanent_underclass::engine::characters::CharacterId> {
    use permanent_underclass::engine::characters::{CharacterCatalog, CharacterId};

    let catalog = CharacterCatalog::embedded().ok()?;
    if let Some(id) = catalog.parse_id(s) {
        return Some(id);
    }

    if s.eq_ignore_ascii_case("ex-faang") {
        return Some(CharacterId::new("ex_faang_engineer"));
    }

    None
}

fn parse_tempo(s: &str) -> Option<permanent_underclass::engine::tempo::Tempo> {
    permanent_underclass::engine::tempo::Tempo::from_cli_value(s)
}

fn print_help() {
    let character_ids = permanent_underclass::engine::characters::CharacterCatalog::embedded()
        .map(|catalog| catalog.cli_ids().join(" | "))
        .unwrap_or_else(|_| "(failed to load character ids)".to_string());

    println!("Permanent Underclass");
    println!();
    println!("Usage:");
    println!("  cargo run");
    println!("  cargo run -- --tempo simple|standard");
    println!("  cargo run -- --cli [--character <id>] [--seed <n>] [--tempo simple|standard]");
    println!("  cargo run -- --headless --character <id> --seed <n> [--tempo simple|standard]");
    println!(
        "  cargo run -- --headless --llm --save-file run.snapshot.json [--tempo simple|standard]"
    );
    println!("  cargo run -- --headless --llm --resume-file run.snapshot.json");
    println!(
        "  cargo run -- --llm-bench [--llm-provider auto|codex|claude] [--llm-model <m>] [--llm-reasoning low|medium|high|xhigh] [--tempo simple|standard]"
    );
    println!();
    println!("Character ids:");
    println!("  {}", character_ids);
    println!();
    println!("Flags:");
    println!("  --cli        Run interactive text mode (no TUI)");
    println!("  --headless   Run without TUI (fast test loop)");
    println!("  --llm-bench  Run a non-TUI benchmark of LLM calls (prints timings)");
    println!("  --llm-bench-iters  Number of benchmark iterations (default: 1)");
    println!("  --llm        Enable LLM freeform resolution (calls `codex exec` or `claude -p`)");
    println!("  --llm-provider  auto|codex|claude (default: auto; prefers claude when available)");
    println!("  --llm-profile  Log LLM timings to the in-game log");
    println!("  --llm-model  Override model (example: gpt-5.2 | sonnet | opus)");
    println!("  --llm-reasoning  low|medium|high|xhigh (default: medium; Claude maps xhigh->high)");
    println!("  --save-file  Headless: write game snapshot JSON after each turn");
    println!("  --resume-file  Headless: resume from snapshot JSON");
    println!("  --character  character id");
    println!("  --seed       deterministic RNG seed");
    println!("  --tempo      simple (12 turns) | standard (32 turns)");
}

fn build_narrator(opts: &Options) -> Option<permanent_underclass::engine::llm::LlmNarrator> {
    use permanent_underclass::engine::llm::LlmNarrator;
    if !opts.llm {
        return None;
    }

    let provider = opts
        .llm_provider
        .as_deref()
        .unwrap_or("auto")
        .to_ascii_lowercase();

    let mut n = match provider.as_str() {
        "auto" => {
            if is_cli_available("claude") {
                LlmNarrator::claude()
            } else {
                LlmNarrator::codex()
            }
        }
        "claude" => LlmNarrator::claude(),
        _ => LlmNarrator::codex(),
    };
    if let Some(m) = &opts.llm_model {
        n = n.with_model(m.clone());
    }
    if let Some(e) = &opts.llm_reasoning {
        n = n.with_reasoning_effort(e.clone());
    }
    Some(n)
}

fn is_cli_available(cmd: &str) -> bool {
    use std::process::{Command, Stdio};

    // We only need to know if the binary exists in PATH.
    Command::new(cmd)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

fn run_headless(opts: Options) -> Result<()> {
    use permanent_underclass::engine::characters::CharacterCatalog;
    use permanent_underclass::engine::endings;
    use permanent_underclass::engine::game::Game;

    const HEADLESS_FREEFORM_ACTION: &str =
        "I prioritize solvency, ship practical work, and ask one person for help.";

    let mut game = if let Some(path) = opts.resume_file.as_deref() {
        Game::load_snapshot(path)?
    } else {
        let default_character = CharacterCatalog::embedded()?.default_id();
        let character = opts.character.clone().unwrap_or(default_character);
        Game::new(opts.tempo, character, opts.seed)
    };
    let snapshot_path = opts.save_file.clone().or(opts.resume_file.clone());
    let narrator = build_narrator(&opts);
    let llm_enabled = narrator.is_some();
    game.set_narrator(narrator);

    loop {
        if let Some(reason) = game.death_reason() {
            let end = endings::death_ending(reason);
            println!("ENDING: {}", end.title);
            println!("{}", end.body);
            println!();
            println!(
                "FINAL: money=${} sanity={} audience={} ai_skill={} friends={} ai_cap={}",
                game.state.stats.money,
                game.state.stats.sanity,
                game.state.stats.audience,
                game.state.stats.ai_skill,
                game.state.stats.friends,
                game.state.ai.percent_string()
            );
            if let Some(path) = snapshot_path.as_deref() {
                game.save_snapshot(path)?;
            }
            return Ok(());
        }
        if game.is_final_turn() {
            let end = endings::evaluate(&game.state);
            println!("ENDING: {}", end.title);
            println!("{}", end.body);
            println!();
            println!(
                "FINAL: money=${} sanity={} audience={} ai_skill={} friends={} ai_cap={}",
                game.state.stats.money,
                game.state.stats.sanity,
                game.state.stats.audience,
                game.state.stats.ai_skill,
                game.state.stats.friends,
                game.state.ai.percent_string()
            );
            if let Some(path) = snapshot_path.as_deref() {
                game.save_snapshot(path)?;
            }
            return Ok(());
        }

        let events = game.begin_turn();
        if let Some(reason) = game.death_reason() {
            let end = endings::death_ending(reason);
            println!("ENDING: {}", end.title);
            println!("{}", end.body);
            println!();
            println!(
                "FINAL: money=${} sanity={} audience={} ai_skill={} friends={} ai_cap={}",
                game.state.stats.money,
                game.state.stats.sanity,
                game.state.stats.audience,
                game.state.stats.ai_skill,
                game.state.stats.friends,
                game.state.ai.percent_string()
            );
            if let Some(path) = snapshot_path.as_deref() {
                game.save_snapshot(path)?;
            }
            return Ok(());
        }

        if llm_enabled && events.personal.allow_freeform {
            let _ = game.resolve_freeform(&events, HEADLESS_FREEFORM_ACTION);
        } else {
            let key = events
                .personal
                .choices
                .iter()
                .find(|c| c.is_available(game.state.stats.money))
                .map(|c| c.key)
                .unwrap_or('1');
            let _ = game.resolve_choice(&events, key);
        }
        if let Some(path) = snapshot_path.as_deref() {
            game.save_snapshot(path)?;
        }

        if let Some(reason) = game.death_reason() {
            let end = endings::death_ending(reason);
            println!("ENDING: {}", end.title);
            println!("{}", end.body);
            println!();
            println!(
                "FINAL: money=${} sanity={} audience={} ai_skill={} friends={} ai_cap={}",
                game.state.stats.money,
                game.state.stats.sanity,
                game.state.stats.audience,
                game.state.stats.ai_skill,
                game.state.stats.friends,
                game.state.ai.percent_string()
            );
            if let Some(path) = snapshot_path.as_deref() {
                game.save_snapshot(path)?;
            }
            return Ok(());
        }

        if game.is_final_turn() {
            let end = endings::evaluate(&game.state);
            println!("ENDING: {}", end.title);
            println!("{}", end.body);
            println!();
            println!(
                "FINAL: money=${} sanity={} audience={} ai_skill={} friends={} ai_cap={}",
                game.state.stats.money,
                game.state.stats.sanity,
                game.state.stats.audience,
                game.state.stats.ai_skill,
                game.state.stats.friends,
                game.state.ai.percent_string()
            );
            if let Some(path) = snapshot_path.as_deref() {
                game.save_snapshot(path)?;
            }
            return Ok(());
        }
    }
}

fn run_cli(opts: Options) -> Result<()> {
    use permanent_underclass::engine::endings;
    use permanent_underclass::engine::game::Game;

    let character = match opts.character.clone() {
        Some(c) => c,
        None => prompt_for_character()?,
    };

    let mut game = Game::new(opts.tempo, character.clone(), opts.seed);
    game.set_narrator(build_narrator(&opts));

    println!("Permanent Underclass — CLI");
    println!("Character: {}", character.label());
    println!(
        "Tempo: {} ({})",
        opts.tempo.display_name(),
        opts.tempo.summary()
    );
    println!("Type '?' for commands. Press Ctrl+C to quit.");

    loop {
        let events = game.begin_turn();

        if let Some(reason) = game.death_reason() {
            let end = endings::death_ending(reason);
            print_ending_and_final(&game, &end);
            return Ok(());
        }

        print_turn_header(&game, &events);

        let outcome = loop {
            let input = prompt("Choice [1/2/3 | 4 freeform | s sub | ? help]: ")?;
            let cmd = input.trim();
            if cmd.is_empty() {
                continue;
            }

            match cmd {
                "1" | "2" | "3" => {
                    let key = cmd.chars().next().unwrap_or('1');
                    if let Some(choice) = events.personal.choices.iter().find(|c| c.key == key) {
                        if !choice.is_available(game.state.stats.money) {
                            println!(
                                "Choice {} is locked (need at least ${}).",
                                key,
                                choice.min_money.unwrap_or_default()
                            );
                            continue;
                        }
                    } else {
                        println!("Choice {} is not available for this event.", key);
                        continue;
                    }
                    break game.resolve_choice(&events, key);
                }
                "4" => {
                    if !events.personal.allow_freeform {
                        println!("Freeform is disabled for this event.");
                        continue;
                    }
                    let action = prompt("Freeform action: ")?;
                    break game.resolve_freeform(&events, action.trim());
                }
                "s" => {
                    manage_subscription_cli(&mut game)?;
                    continue;
                }
                "?" => {
                    print_cli_help();
                    continue;
                }
                "quit" | "exit" => return Ok(()),
                _ => {
                    println!("Unknown command '{}'. Type '?' for help.", cmd);
                    continue;
                }
            }
        };

        print_outcome(&outcome);

        if let Some(reason) = game.death_reason() {
            let end = endings::death_ending(reason);
            print_ending_and_final(&game, &end);
            return Ok(());
        }
        if game.is_final_turn() {
            let end = endings::evaluate(&game.state);
            print_ending_and_final(&game, &end);
            return Ok(());
        }
    }
}

fn print_cli_help() {
    println!("Commands:");
    println!("  1/2/3      choose a card");
    println!("  4          freeform action");
    println!("  s          change subscription tier");
    println!("  quit/exit  quit run (or press Ctrl+C)");
}

fn prompt_for_character() -> Result<permanent_underclass::engine::characters::CharacterId> {
    use permanent_underclass::engine::characters::CharacterCatalog;

    let catalog = CharacterCatalog::embedded()?;
    println!("Pick character:");
    for (idx, character) in catalog.all().iter().enumerate() {
        println!("  {}) {}", idx + 1, character.id.to_cli_slug());
    }

    loop {
        let input = prompt("Character [number or id]: ")?;
        let s = input.trim().to_ascii_lowercase();
        let character = match s.parse::<usize>() {
            Ok(i) if i >= 1 && i <= catalog.all().len() => Some(catalog.all()[i - 1].id.clone()),
            _ => parse_character(&s),
        };

        if let Some(character) = character {
            return Ok(character);
        }

        println!("Unknown character '{}'. Try again.", input.trim());
    }
}

fn prompt(prompt: &str) -> Result<String> {
    print!("{}", prompt);
    io::stdout().flush()?;
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    Ok(buf)
}

fn manage_subscription_cli(game: &mut permanent_underclass::engine::game::Game) -> Result<()> {
    use permanent_underclass::engine::subscription::SubscriptionTier;

    println!(
        "Current subscription: {} (${}/turn)",
        game.state.subscription.label(),
        game.state.subscription.cost_per_turn()
    );
    println!("Available tiers:");
    for tier in SubscriptionTier::ALL {
        let unlocked = game.can_set_subscription(*tier);
        let state = if unlocked { "unlocked" } else { "locked" };
        println!(
            "  - {} (${}/turn) [{}]",
            tier.label(),
            tier.cost_per_turn(),
            state
        );
    }
    println!("Type: free | pro | ultra | giga | cancel");

    loop {
        let input = prompt("Set subscription: ")?;
        let s = input.trim().to_ascii_lowercase();
        if s.is_empty() || s == "cancel" {
            return Ok(());
        }
        let tier = match s.as_str() {
            "free" => Some(SubscriptionTier::Free),
            "pro" => Some(SubscriptionTier::Pro),
            "ultra" => Some(SubscriptionTier::Ultra),
            "giga" => Some(SubscriptionTier::Giga),
            _ => None,
        };
        let Some(tier) = tier else {
            println!("Unknown tier '{}'.", input.trim());
            continue;
        };
        match game.set_subscription(tier) {
            Ok(()) => {
                println!(
                    "Subscription set: {} (${}/turn)",
                    game.state.subscription.label(),
                    game.state.subscription.cost_per_turn()
                );
                return Ok(());
            }
            Err(e) => {
                println!("Could not set tier: {}", e);
                return Ok(());
            }
        }
    }
}

fn print_turn_header(
    game: &permanent_underclass::engine::game::Game,
    events: &permanent_underclass::engine::events::TurnEvents,
) {
    let s = &game.state;

    println!();
    println!(
        "=== {} / {} | {} {} | AI {} ===",
        s.turn,
        s.max_turns,
        s.phase().label(),
        s.year(),
        s.ai.percent_string()
    );
    println!(
        "Character: {} | Sub: {} (${} /turn)",
        s.character.label(),
        s.subscription.label(),
        s.subscription.cost_per_turn()
    );
    println!(
        "Stats: money=${} sanity={} audience={} ai_skill={} friends={}",
        s.stats.money, s.stats.sanity, s.stats.audience, s.stats.ai_skill, s.stats.friends
    );
    println!();
    println!("WORLD: {}", events.world.title);
    println!("{}", events.world.body);
    if !events.world.delta.is_zero() {
        println!("World delta: {}", format_delta(events.world.delta));
    }

    println!();
    println!("EVENT: {}", events.personal.title);
    println!("{}", events.personal.body);
    println!("Choices:");
    for choice in events.personal.choices.iter().take(3) {
        print_choice_line(game, choice);
    }
    if events.personal.allow_freeform {
        println!("  4) Freeform action");
    }
}

fn print_choice_line(
    game: &permanent_underclass::engine::game::Game,
    choice: &permanent_underclass::engine::events::Choice,
) {
    let availability = if choice.is_available(game.state.stats.money) {
        ""
    } else {
        " [LOCKED]"
    };
    let preview = game.preview_choice_delta(choice);
    println!(
        "  {}) {}{} -> {}",
        choice.key,
        choice.title,
        availability,
        format_delta(preview)
    );
}

fn print_outcome(outcome: &permanent_underclass::engine::events::TurnOutcome) {
    println!();
    println!("{}", outcome.narrative);
    println!("Outcome delta: {}", format_delta(outcome.delta));
}

fn print_ending_and_final(
    game: &permanent_underclass::engine::game::Game,
    ending: &permanent_underclass::engine::endings::Ending,
) {
    println!();
    println!(
        "{}",
        permanent_underclass::engine::endings::origin_line(&game.state)
    );
    println!("You lasted {} turns.", game.state.turn);
    println!();
    println!("ENDING: {}", ending.title);
    println!("{}", ending.body);
    println!();
    println!(
        "FINAL: money=${} sanity={} audience={} ai_skill={} friends={} ai_cap={}",
        game.state.stats.money,
        game.state.stats.sanity,
        game.state.stats.audience,
        game.state.stats.ai_skill,
        game.state.stats.friends,
        game.state.ai.percent_string()
    );
}

fn format_delta(d: permanent_underclass::engine::types::ResourceDelta) -> String {
    let mut parts: Vec<String> = Vec::new();
    if d.money != 0 {
        parts.push(format!("money={:+}", d.money));
    }
    if d.sanity != 0 {
        parts.push(format!("sanity={:+}", d.sanity));
    }
    if d.audience != 0 {
        parts.push(format!("audience={:+}", d.audience));
    }
    if d.ai_skill != 0 {
        parts.push(format!("ai_skill={:+}", d.ai_skill));
    }
    if d.friends != 0 {
        parts.push(format!("friends={:+}", d.friends));
    }
    if parts.is_empty() {
        "no change".to_string()
    } else {
        parts.join(", ")
    }
}

fn run_llm_bench(opts: Options) -> Result<()> {
    use permanent_underclass::engine::characters::CharacterCatalog;
    use permanent_underclass::engine::game::Game;

    let Some(narrator) = build_narrator(&opts) else {
        return Err(anyhow!(
            "--llm-bench requires --llm (and a working LLM CLI)."
        ));
    };

    let iters = opts.llm_bench_iters.unwrap_or(1).max(1);
    let model = opts.llm_model.as_deref().unwrap_or("(default)");
    let reasoning = opts.llm_reasoning.as_deref().unwrap_or("medium");

    println!("LLM bench");
    println!("  model: {}", model);
    println!("  reasoning: {}", reasoning);
    println!("  iters: {}", iters);
    println!();

    let default_character = CharacterCatalog::embedded()?.default_id();
    let character = opts.character.unwrap_or(default_character);
    let mut game = Game::new(opts.tempo, character, opts.seed);

    let mut freeform_times = Vec::<u128>::new();

    for i in 0..iters {
        let events = game.begin_turn();

        {
            let n = narrator.clone();
            let state = game.state.clone();
            let events_snapshot = events.clone();
            let action =
                "I update my resume, apply to 3 jobs, and write a mildly deranged LinkedIn post."
                    .to_string();
            let label = format!("bench {}/{}: freeform", i + 1, iters);
            let (res, ms) = run_with_spinner(&label, move || {
                n.resolve_freeform(&state, &events_snapshot, &action)
            })?;
            freeform_times.push(ms);

            let outcome =
                game.apply_freeform(res.delta, res.effects, res.narrative, res.summary_line);
            println!("turn {}: {}", game.state.turn, outcome.summary_line);
        }
    }

    println!();
    println!("Timings (ms)");
    println!(
        "  freeform: avg={}  min={}  max={}",
        avg_ms(&freeform_times),
        freeform_times.iter().min().copied().unwrap_or(0),
        freeform_times.iter().max().copied().unwrap_or(0),
    );

    Ok(())
}

fn avg_ms(xs: &[u128]) -> u128 {
    if xs.is_empty() {
        return 0;
    }
    let sum: u128 = xs.iter().copied().sum();
    sum / (xs.len() as u128)
}

fn run_with_spinner<T, F>(label: &str, f: F) -> Result<(T, u128)>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T> + Send + 'static,
{
    use std::io::{self, Write};
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    let (tx, rx) = mpsc::channel::<(Result<T>, u128)>();
    std::thread::spawn(move || {
        let start = Instant::now();
        let r = f();
        let ms = start.elapsed().as_millis();
        let _ = tx.send((r, ms));
    });

    let spinner = ['|', '/', '-', '\\'];
    let mut idx = 0usize;

    eprint!("\r{} {}...", spinner[0], label);
    let _ = io::stderr().flush();

    loop {
        match rx.recv_timeout(Duration::from_millis(120)) {
            Ok((r, ms)) => {
                // Overwrite spinner line and terminate with newline.
                eprintln!("\r- {}... done ({}ms)    ", label, ms);
                return Ok((r?, ms));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                idx = idx.wrapping_add(1);
                let ch = spinner[idx % spinner.len()];
                eprint!("\r{} {}...", ch, label);
                let _ = io::stderr().flush();
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                eprintln!();
                return Err(anyhow!("bench worker thread disconnected"));
            }
        }
    }
}
