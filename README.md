# Permanent Underclass

Terminal game about surviving disruption one turn at a time.

## Quick Start

```bash
cargo run
```

CLI help:

```bash
cargo run -- --help
```

## Play From npm

```bash
# stable channel
npx --yes permanent-underclass@latest

# beta channel
npx --yes permanent-underclass@beta
```

## Run Modes (Local)

TUI (default):

```bash
cargo run
```

CLI interactive (no TUI):

```bash
cargo run -- --cli --character junior-dev --seed 1
cargo run -- --cli --character junior-dev --seed 1 --tempo standard
```

Headless autoplay:

```bash
cargo run -- --headless --character junior-dev --seed 1
cargo run -- --headless --character junior-dev --seed 1 --tempo standard
```

Headless with LLM + snapshot save/resume:

```bash
cargo run -- --headless --llm --save-file run.snapshot.json --character junior-dev --seed 1
cargo run -- --headless --llm --resume-file run.snapshot.json
```

LLM benchmark (no TUI):

```bash
cargo run -- --llm-bench --llm-provider auto --llm-bench-iters 1
cargo run -- --llm-bench --llm-provider auto --llm-bench-iters 1 --tempo standard
```

Balance simulation:

```bash
cargo run --bin balance_sim -- --seeds 20
cargo run --bin balance_sim -- --seeds 400 --character all --tempo standard --output-jsonl data/sim/ending_sweep_latest.jsonl
```

## Character IDs

`junior-dev | growth-lead | ex-googler | the-designer | podcast-bro | plumber | stealth-founder | angel-investor | product-guy | twitter-anon | ai-researcher | crypto-bro | hr-lady | digital-nomad | chief-of-staff`

## Controls

- `Ctrl+C`: quit
- `Enter`: confirm / continue
- Arrow keys: navigate cards/menus/settings
- `1` / `2` / `3`: choose a card
- `4`: enter freeform mode (`Enter` to submit, `Esc` to cancel)
- `s`: manage subscription (`Free` / `Pro` / `Ultra` / `Giga`)
- `m`: open in-run settings
- `Esc`: back/cancel

## Tempo Modes

- `simple`: 12 turns (default)
- `standard`: 32 turns

Use `--tempo simple|standard` in `--cli`, `--headless`, and `--llm-bench` modes.

## LLM Mode

Uses your local `codex` or `claude` CLI for freeform resolution.

```bash
# Auto provider (default): prefers Claude if installed, else Codex
cargo run -- --llm

# Codex
codex login
cargo run -- --llm --llm-provider codex

# Claude
claude auth
cargo run -- --llm --llm-provider claude
```

Optional flags: `--llm-model`, `--llm-reasoning low|medium|high|xhigh`, `--llm-profile`.

## Audio

- Default score track: `CHIPTUNE`
- Selection SFX file override: `PU_SELECTION_SFX`
- Title tone file override: `PU_TITLE_TONE`
- Running tone file override: `PU_RUNNING_TONE`
- Optional alternate score tracks: `PU_CHIPTUNE_SCORE`, `PU_TECHNO_SCORE`
- Shepard tone enabled by default; disable with `PU_SHEPARD_TONE=0`

All audio paths accept local file paths or `file:///...` URLs.

## Command Center

```bash
npm run dev
```

Opens `http://localhost:5173/command-center` to browse events, endings, characters, flags, and sprites.
If `data/sim/ending_sweep_latest.jsonl` exists, the `Sim` tab renders ending/policy/character outcome summaries.
Source lives in `apps/command-center/`.

Cloudflare Worker deploy path (same repo, no Pages):

```bash
npm run deploy:command-center:worker
```

## Tests and Security Checks

```bash
# full release gate (single pass/fail summary)
./scripts/release_preflight.sh

# individual checks
cargo test --lib
cargo test --test sim
# one-time: cargo install cargo-audit
cargo audit
# one-time: cargo install cargo-deny
cargo deny check advisories
# one-time: brew install gitleaks  # or install from https://github.com/gitleaks/gitleaks
gitleaks detect --source . --no-git --redact --exit-code 1
```

## Licensing

- Code license: `LICENSE` (MIT)
- Non-code asset licensing and attribution status: `ASSET_LICENSES.md`

## Docs

- `docs/product/current_spec.md` (single source of truth for current product + runtime behavior)
- `docs/product/gameplay_test_criteria.md` (quant + narrative gameplay QA charter)
- `docs/product/event_writing_guide.md` (event writing format + standards)
- `docs/publishing/npm_release_sot.md` (canonical npm release + tag flow)
- `docs/archive/` (archived roadmap/research docs)
