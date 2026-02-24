# Permanent Underclass Gameplay Test Criteria

Last updated: 2026-02-23

This document is the current gameplay QA charter for balance, pacing, and narrative quality.
Use it before merging major changes to `data/events/*`, character economy knobs, showrunner logic, or endings.

## 1) Target Behaviors

These are intended outcomes, not all strict gates.

1. The game should produce diverse endings across characters and strategies.
2. Death pressure should be real, but not dominated by one accidental failure mode for every character.
3. Character balance should be tunable with explicit knobs (`starting_stats`, income, rent), and measured through sweeps.
4. Acceleration should feel stronger by Late/Endgame than Early.
5. Choices should reflect distinct value systems, not cosmetic rewrites.

## 2) Test Tiers

- `HARD GATE`: merge-blocking.
- `BALANCE TARGET`: expected band; may merge with explicit rationale if missed.
- `NARRATIVE CHECK`: reviewer rubric with short notes.

## 3) Quantitative Tests

### A. Engine + Content Sanity (`HARD GATE`)

Run:

```bash
cargo test --lib
cargo test --test content_schema
cargo test --test sim
```

Pass:

- All tests pass.
- No schema regressions.
- No sim panics/regressions.

### B. Canonical Sweep Generation (`HARD GATE` for balance/content PRs)

Generate the authoritative sweep file used by command center:

```bash
cargo run --bin balance_sim -- --seeds 400 --character all --tempo standard --output-jsonl data/sim/ending_sweep_latest.jsonl
```

Pass:

- Command succeeds.
- Output file exists at `data/sim/ending_sweep_latest.jsonl`.
- Total rows = `characters * policies * seeds` (currently `15 * 13 * 400 = 78000`).

### C. Sweep Integrity (`HARD GATE`)

Validate sweep shape:

```bash
jq -r '[.character,.policy]|@tsv' data/sim/ending_sweep_latest.jsonl | sort | uniq -c
```

Pass:

- Every `character x policy` combo has exactly `seeds` rows.
- No missing policy columns in the matrix.

### D. Global Outcome Mix (`BALANCE TARGET`)

Use `ENDING_COUNTS` from `balance_sim` output or aggregate directly from JSONL.

Current target bands:

- Broke share among deaths: `15%` to `40%`
- BurnedOut share among deaths: `60%` to `85%`
- Non-death share across all runs: `40%` to `60%`

Notes:

- We no longer target strict 50/50 Broke vs BurnedOut parity.
- Rebalance by adjusting recurring pressure and character economy knobs before forcing ending rules.

### E. Character Broke Calibration (`BALANCE TARGET`)

Primary measurement:

```bash
cargo run --bin balance_sim -- --seeds 400 --character <id> --tempo standard --output-jsonl /tmp/<id>.jsonl
```

Current explicit calibration targets:

- `podcast-bro`: ~`40%` Broke (`35%` to `45%`)
- `twitter-anon`: ~`40%` Broke (`35%` to `45%`)
- `stealth-founder`: ~`40%` Broke (`35%` to `45%`)

General guardrails:

- Every character should have non-zero Broke rate unless intentionally documented.
- Any character above `85%` single-death dominance must be documented as intentional.

### F. Ending Reachability Coverage (`BALANCE TARGET`)

From full sweep `ENDING_COUNTS`:

- Every non-death ending in `data/endings.jsonl` should appear at least once across broad sweeps.
- If an ending is expected to be ultra-rare, tag it in design notes as `rare_by_design` with trigger path.

### G. LLM Provider Parity (`BALANCE TARGET`, optional when touching LLM behavior)

Run matched seed sets with `--llm-provider codex` and `--llm-provider claude`.

Targets:

- Absolute Broke-rate delta: `<= 5` percentage points.
- Absolute BurnedOut-rate delta: `<= 5` percentage points.
- Non-death share delta: `<= 7` percentage points.

## 4) Command Center Verification

### H. Sim UI Smoke Check (`HARD GATE` for sim/dashboard changes)

With `npm run dev`, open:

- `http://localhost:5173/command-center#sim`

Pass:

- No "No sim file loaded" warning when sweep file exists.
- Matrix renders all strategies with `balanced` then `random` first.
- `all-strategies` aggregate column appears.
- Hover highlights row/column headers clearly.

## 5) Acceleration + Narrative Checks

### I. Structural Acceleration (`BALANCE TARGET`)

Goal: Endgame should feel harsher/faster than Early.

Targets:

- AI reaches `>=95%` capability by end of standard runs.
- Endgame choices/tradeoffs show larger effective stakes than Early.
- Crisis states appear more frequently in Late/Endgame than Early.

### J. Narrative Acceleration (`NARRATIVE CHECK`)

Sample at least `10` events from each phase.

Rubric:

1. Early prose allows scene-setting and context.
2. Late/Endgame prose feels tighter and more urgent.
3. Phase tone shifts are noticeable without rewriting everything as alarmist.

## 6) Choice and Memory Integrity

### K. Mechanical Choice Distinctness (`HARD GATE` for obvious duplicates)

- No event should ship with all choices mechanically identical.
- Avoid 2/3 identical effect vectors unless explicitly intentional and documented.

### L. Value Distinctness (`NARRATIVE CHECK`)

Per event, verify meaningful value tension:

1. survival/pragmatism
2. status/ambition
3. meaning/relationships/craft

### M. Flag Lifecycle Integrity (`HARD GATE`)

For major authored flags:

- At least one reliable setter path.
- At least one meaningful consumer (`requires`, `forbids`, or ending check).

No dead arcs:

- Do not ship follow-up events that depend on flags that are never set.

### N. Repetition + Pool Pressure (`BALANCE TARGET`)

- Character pools remain primary.
- Universal personal events act as starvation fallback, not mainline content.
- Avoid off-phase tonal mismatches in fallback behavior.

## 7) Next Additions

1. Save/resume equivalence checks in headless mode.
2. Per-phase compression metrics (sentence length / urgency markers).
3. Ending funnel guardrail (top 3 endings combined share cap).
4. Arc conversion metrics (`flag set -> downstream consumer` rates).
