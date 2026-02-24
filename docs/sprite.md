# Sprite System (Canonical)

Last updated: 2026-02-22

This is the canonical document for all sprite-related behavior in the repo:
- authoring
- data model
- build outputs
- runtime consumption
- semantic tagging plan

## 1) Scope

There are two sprite systems:

1. Library sprites (event + choice icons)
- Source: `data/sprites/sprite_library.jsonl`
- Runtime loader/selector: `src/sprites.rs`
- Displayed in event panel + choice cards (`src/ui_broadcast.rs`)

2. Character portrait sprites (player character art)
- Source code-defined (not JSONL): `src/ui.rs`
- Displayed in sidebar and other character UI surfaces (`src/ui_broadcast.rs`)

Most of this document is about library sprites (the 2329-sprite catalog).

## 2) Current Inventory

- Sprite library entries: `2329` (`data/sprites/sprite_library.jsonl`)
- Events: `2151` total (`data/events/*.jsonl`)
- Choices: `6453` total (up to 3 per event)
- Sprite selection decisions if fully mapped:
  - `2151` event sprite picks
  - `6453` choice sprite picks
  - `8604` total decisions

## 3) Canonical Files

Source of truth:
- `data/sprites/sprite_library.jsonl`

Generated artifacts:
- `data/sprites/sprite_library_meta.json`
- `docs/sprite_library.html` (1x gallery)
- `docs/sprite_library_2x.html` (2x gallery)
- `docs/sprite_library_data.js` (viewer JS payload)
- `docs/sprite_library_data.html` (interactive viewer)

Build/validation tools:
- `tooling/sprites/build_library.py`
- `tooling/sprites/validate.py`
- `tooling/sprites/build_semantic_map.py`

Runtime consumption:
- `src/sprites.rs`
- `src/ui.rs`
- `src/ui_broadcast.rs`

## 4) Library Data Model (`data/sprites/sprite_library.jsonl`)

Each line is one sprite object. Canonical fields for source rows:
- `name: string`
- `category: string`
- `tags: string[]` (authoring policy: `1..4` concise non-generic tags)
- `1x: SegmentLine[]` (5 rows, target width 10 codepoints)
- `2x: SegmentLine[]` (10 rows, target width 20 codepoints)

Segment format:
- A segment is `[text, color_key]`
- A line is an array of segments

Palette color keys:
- `g a r b p c y k o d m w` (see `tooling/sprites/build_library.py` and generated meta)

Important runtime detail:
- In source-row mode, runtime currently renders from `2x` (`src/sprites.rs`, `EntrySourceRaw.sprite_2x`).

## 5) Build + Data Flow

Authoring/build flow:
1. Edit `data/sprites/sprite_library.jsonl`
2. Validate: `python3 tooling/sprites/validate.py`
3. Generate outputs: `python3 tooling/sprites/build_library.py`
4. Commit source + generated artifacts together

What `build_library.py` does:
- Reads `data/sprites/sprite_library.jsonl`
- Writes viewer/gallery docs + meta + JS payload
- Enriches viewer payload with derived IDs/keys/search fields
- If source `tags` missing, viewer generator falls back to category+name tokens

Runtime flow (game binary):
- `src/sprites.rs` embeds `data/sprites/sprite_library.jsonl` and `data/sprites/sprite_library_meta.json` via `include_str!`
- Game loads precomputed semantic mapping files from `data/sprites/semantics/` when available:
  - `event_sprite_selection.jsonl`
  - `choice_sprite_selection.jsonl`
- Semantic mappings are authoritative at runtime.
- Game renders selected sprite line segments to ratatui.

Sprite source/meta changes still require rebuilding/rerunning the binary (embedded with `include_str!`), but semantic mapping files are loaded from disk at runtime and can be regenerated without recompiling.

## 6) Runtime Selection Logic (Current)

Event sprite selection (`src/sprites.rs`):
- First attempt lookup from precomputed `event_sprite_selection.jsonl` by `event_id`.
- If missing/invalid, fallback to deterministic misc sprite:
  - `abstract_concepts_game_mechanics__random_event`

Choice sprite selection (`src/sprites.rs`):
- First attempt lookup from precomputed `choice_sprite_selection.jsonl` by `event_id + choice_index`.
- If missing/invalid, fallback to hardcoded sprite:
  - `emotions_status_effects__curiosity`

Determinism:
- Selection is deterministic (no random sampling).

## 7) Canonical Tagging Policy (Going Forward)

Policy for every sprite row:
- `tags`: max 4

Tag rules:
- lowercase tokenized words (stable matching)
- include object identity first (sprite name token)
- include only specific context/action/mood terms that disambiguate
- include visible text tokens from ASCII art when they are semantically meaningful (e.g. `wifi`, `vr`, `id`, `404`); skip random filler letters/gibberish
- no generic tokens (examples to avoid: `work`, `business`, `people`, `social`, `tech`, `media`, `generic`, `misc`)

For future new sprites (Claude or manual):
- Add row directly in `data/sprites/sprite_library.jsonl`
- Include `tags` only (1 to 4, specific)
- Do not rely on generated viewer fallback tags

Per-event choice rendering rule:
- Semantic mapping is authoritative when present.
- If a choice mapping row is absent or references a missing sprite ID, runtime uses
  `emotions_status_effects__curiosity` for that choice.

## 8) Semantic Mapping Plan (Canonical)

Goal:
- Move from weak name/category token matching to strong hand-authored semantics.

Planned persisted outputs (versioned):
- `data/sprites/semantics/event_sprite_selection.jsonl`
- `data/sprites/semantics/choice_sprite_selection.jsonl`
- `data/sprites/semantics/metrics.json`

Execution phases:
1. Freeze and baseline
- freeze sprite set during tagging pass
- capture baseline match quality

2. Maintain source semantics on all sprites
- source tags per sprite (`<=4`, no generic terms)

3. Deterministic candidate generation
- score against exact event/choice text
- default top-K = 10 (optionally 20 for low-confidence cases)

4. Final selection
- heuristic top-1 where confident
- optional LLM arbitration only for ambiguous top-K sets
- manual override for unresolved edge cases

5. Persist and consume
- load stored event/choice mapping first
- deterministic fallback sprites are used only when mapping rows are missing/invalid

6. QA gates
- 100% sprite rows have source tags (`<=4`, no generic terms)
- 100% events/choices have selected sprites
- ambiguity + reuse metrics reviewed

## 9) Operational Checklist

When editing existing sprites:
1. Edit `data/sprites/sprite_library.jsonl`
2. Run `python3 tooling/sprites/validate.py`
3. Run `python3 tooling/sprites/build_library.py`
4. Regenerate semantic mappings:
- `python3 tooling/sprites/build_semantic_map.py --action all`
5. Verify viewers:
- `docs/sprite_library.html`
- `docs/sprite_library_2x.html`
- `docs/sprite_library_data.html`
6. Run game sanity pass:
- `cargo run -- --headless --character junior-dev --seed 1`

When adding new sprites:
1. Add sprite row with `1x`, `2x`, `tags` (`1..4` specific)
2. Validate/build as above
3. Spot-check for matching collisions

## 10) Notes on 1x vs 2x

- `1x` is used for gallery/reference and source completeness.
- `2x` is used by runtime event/choice rendering path for source rows.
- Keep both coherent; text tokens should match across representations.

## 11) Canonicality

For sprite-related design, process, and implementation details, treat this file as source of truth.
