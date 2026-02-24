# PERMANENT UNDERCLASS - Current Spec

Last updated: 2026-02-23
Tech: Rust + ratatui (single binary)

This is the single source of truth for current product + runtime behavior.
Command examples and mode-specific CLI details live in `README.md`.

## Goal

Ship and iterate a short, replayable terminal run that produces shareable moments.

Core loop:
Play (simple: ~5-8 min | standard: ~10-15 min) -> Die/End -> Screenshot -> Share -> Friend tries a different character.

## Product Direction

- Text-first game: events and outcomes are the product surface.
- Two run tempos:
  - `simple`: 12 turns (quick session)
  - `standard`: 32 turns (longer arc)
- Strong replayability through character-specific content.
- Freeform action input remains a core differentiator.

## Core Runtime Files

- `src/main.rs`: argument parsing and mode dispatch (`--cli`, `--headless`, `--llm`, `--llm-bench`, etc.)
- `src/app.rs`: TUI state machine, key handling, in-run settings
- `src/engine/game.rs`: turn lifecycle, passive effects, resolution, memory
- `src/engine/showrunner.rs`: event/headline selection, dedup, starvation fallbacks
- `src/engine/content.rs`: JSONL content loading + schema validation
- `src/engine/llm.rs`: Codex/Claude freeform resolution
- `src/engine/endings.rs`: ending catalog + scoring evaluation

## Gameplay Loop (Current)

### Pacing and Phases

- Tempo modes (`src/engine/tempo.rs`):
  - `simple`: 12 turns
  - `standard`: 32 turns
- Phases (`src/engine/ai.rs`) are quartiles of `max_turns`:
  - `simple`: Early `1-3`, Mid `4-6`, Late `7-9`, Endgame `10-12`
  - `standard`: Early `1-8`, Mid `9-16`, Late `17-24`, Endgame `25-32`

### Turn Pipeline (`Game::begin_turn`)

1. Increment turn.
2. Tick AI capability.
3. Apply milestone side effects.
4. Apply passive per-turn deltas (income, rent, subscription, baseline decay).
5. Tick and clear status conditions.
6. Generate one world headline + one personal event.
7. Track seen personal event in run memory.
8. Auto-apply world delta.
9. Update ticker feed.

Baseline passive drift each turn:

- sanity: Early `-1`, Mid `-2`, Late `-3`, Endgame `-3` (Plumber halves this attrition)
- audience: `-1`
- friends: Early `0`, Mid `-1`, Late `-1`, Endgame `-2`

### Personal Resolution (`Game::resolve_choice` / `Game::apply_freeform`)

Resolution order:

1. Compute delta
2. Apply delta to stats
3. Apply memory effects (`set_flag` / `clear_flag`)
4. Update run-pattern counters
5. Sync status conditions
6. Write concise log entries

Authored choice deltas are phase-scaled before condition modifiers:

- Early: `1.00x`
- Mid: `1.10x`
- Late: `1.25x`
- Endgame: `1.40x`

Freeform deltas are clamped and validated, then condition-modified (no phase multiplier in engine).

## Showrunner Behavior

### Event Selection

- Persona-first pools:
  - character phase pool first
  - universal phase pool only when character pool is exhausted
- Recency dedup window: last `8` personal event IDs
- Eligibility filters:
  - phase gate
  - `requires` / `forbids`
  - `once_per_run` via seen IDs

### Starvation Fallbacks

Within each pool (character first, then universal):

1. Disable dedup for this pick.
2. Disable memory gating for this pick.
3. Use ungated repeatable events from the same phase-filtered pool.

If both pools fail, final fallback is a generic emergency personal event.
`once_per_run` is never relaxed.

### World Events

- World panel is non-interactive.
- One phase-gated headline is selected per turn from `data/headlines.jsonl` and auto-applies delta.

## Content and Data

Canonical data files:

- Personal events: `data/events/*.jsonl`
- Headlines: `data/headlines.jsonl`
- Flags: `data/flags.jsonl`
- Endings: `data/endings.jsonl`
- Characters: `data/characters.jsonl`

Current event schema features:

- `requires`, `forbids`, `once_per_run`
- choice-level `effects` (`set_flag` / `clear_flag`)
- `allow_freeform` (default `true`)
- choice lock gate: `min_money`

Validation:

- Strict serde parsing (`deny_unknown_fields`)
- Duplicate event ID checks
- Unknown flag reference checks across `requires` / `forbids` / `effects`

## Endings (Current)

- Endings are loaded from `data/endings.jsonl`.
- Scoring evaluates all non-death endings and picks best score, then specificity tie-break.
- Death endings are selected from death reason.
- Character `origin_line` comes from `data/characters.jsonl`.

## LLM Integration

Providers:

- `codex`
- `claude`
- `auto` (prefers Claude if available)

Freeform contract:

- Structured JSON validated against `llm/freeform.schema.json`
- Delta clamps:
  - money: `[-15000, +15000]`
  - sanity/audience/ai_skill/friends: `[-14, +14]`
- Effects sanitation:
  - max `2`
  - known flags only
  - LLM-settable flags only

## Scope and Priorities

Current non-goals:

- Multiplayer / leaderboards
- Map/location navigation layer
- Interactive stock trading
- Save-export/share-image pipeline
- Inventory persistence beyond current presentation

Current priorities:

1. Keep endings/spec/content in sync with sweep-based balance tuning.
2. Improve memory follow-up coverage for frequently set flags.
3. Keep universal events as fallback pressure, not primary narrative path.
4. Maintain command-center-driven visibility into sim outcomes.
