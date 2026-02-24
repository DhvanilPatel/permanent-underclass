# AGENTS.md

Persistent context for coding agents working in this repository.

## Project Snapshot

- Project: `Permanent Underclass` (terminal game).
- Stack: Rust + `ratatui`.
- Runtime: single binary (`cargo run`).
- Current product direction: viral v1, ~30-turn pacing, character-specific content.

## Read Order (Source of Truth)

1. `README.md` (run commands and controls)
2. `docs/product/current_spec.md` (single source of truth for current product + runtime behavior)
3. `docs/product/gameplay_test_criteria.md` (gameplay QA and balance gates)

## Code Map

- `src/main.rs`: CLI parsing and run modes (`--headless`, `--llm`, `--llm-bench`, etc.)
- `src/app.rs`: app-level TUI state machine and input handling
- `src/engine/game.rs`: turn pipeline and state transitions
- `src/engine/showrunner.rs`: event selection and pacing logic
- `src/engine/content.rs`: embedded personal event catalogs (`data/events/*.jsonl`)
- `src/engine/headlines.rs`: embedded ticker headlines (`data/headlines.jsonl`)
- `src/engine/ai.rs`: AI capability curve and milestones
- `src/engine/endings.rs`: ending/death evaluation
- `src/engine/llm.rs`: freeform narration integration (Codex/Claude CLIs)
- `src/ui_broadcast.rs`: primary TUI theme
- `src/sprites.rs`: sprite library loader/rendering
- `tests/sim.rs`: deterministic end-to-end simulation tests

## Data and Content Flow

- Event catalogs (source of truth): `data/events/*.jsonl` — edit JSONL directly
- Headline content: `data/headlines.jsonl`
- Flag registry: `data/flags.jsonl`
- Sprite library: `data/sprites/sprite_library.jsonl`
- Command center app: `apps/command-center/` (visual browser via `npm run dev` → `http://localhost:5173/command-center`)

## Common Commands

- Run TUI: `cargo run`
- Headless loop: `cargo run -- --headless --character junior-dev --seed 1`
- CLI help: `cargo run -- --help`
- Command center: `npm run dev`
- Simulation tests: `cargo test --test sim`
- Library/unit tests: `cargo test --lib`

LLM mode (optional):

- Codex: `codex login` then `cargo run -- --llm --llm-provider codex`
- Claude: `claude auth` then `cargo run -- --llm --llm-provider claude`

## Editing Expectations

- Keep changes scoped to the user request.
- This game is pre-release. Unless explicitly requested otherwise, prefer hard cutovers over backward-compatibility fallbacks.
- Be reasonable: if rapid switching between behaviors is likely useful in the same context, add an explicit flag or setting instead of implicit fallback logic.
- Prefer updating docs when behavior/commands change.
- For gameplay logic changes, run at least `cargo test --test sim`.
- For UI changes, verify `broadcast` still renders.
- Do not rely on legacy archive text for current behavior unless explicitly requested.
