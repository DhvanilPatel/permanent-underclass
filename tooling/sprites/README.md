# Sprite Semantic Mapping Script

`tooling/sprites/build_semantic_map.py` builds embedding-backed sprite mappings for:
- personal events (`event_id -> sprite_id`)
- choices (`event_id + choice_index -> sprite_id`)

It is designed for incremental reruns:
- unchanged texts reuse cached embeddings
- changed sprites/events/choices re-embed only affected rows

Current semantic defaults:
- tags from `data/sprites/sprite_library.jsonl` (name + tag signal)
- generic tags filtered (`--allow-generic-tags` is off by default)
- parallel embedding API batches (`--embed-workers 6`)

Recommended pipeline:
1. Maintain sprite tags directly in `data/sprites/sprite_library.jsonl`.
2. Rebuild mappings with `build_semantic_map.py --action all`.

## Prereqs

1. Python 3.10+
2. OpenAI key in environment:
   - `OPENAI_API_KEY` (default), or
   - `PU_OPENAI_API_KEY`
3. `numpy` for mapping stage:
   - `python3 -m pip install numpy`

## Quick Start

From repo root:

```bash
python3 tooling/sprites/build_semantic_map.py --action all
```

This writes:
- `data/sprites/semantics/embeddings.jsonl`
- `data/sprites/semantics/event_sprite_selection.jsonl`
- `data/sprites/semantics/choice_sprite_selection.jsonl`
- `data/sprites/semantics/metrics.json`

## Common Commands

Generate/update embeddings only:

```bash
python3 tooling/sprites/build_semantic_map.py --action embeddings
```

Recompute mappings from existing embeddings:

```bash
python3 tooling/sprites/build_semantic_map.py --action mapping
```

Dry-run counts + input hashes (no API calls):

```bash
python3 tooling/sprites/build_semantic_map.py --dry-run
```

Use a different model or dimensions:

```bash
python3 tooling/sprites/build_semantic_map.py \
  --model text-embedding-3-large \
  --dimensions 3072
```

Override blend behavior for choice scoring:

```bash
python3 tooling/sprites/build_semantic_map.py \
  --action mapping \
  --choice-text-weight 0.75 \
  --event-context-weight 0.25
```

Control tag filtering:

```bash
python3 tooling/sprites/build_semantic_map.py \
  --action all \
  --max-tags-per-sprite 4
```

```bash
python3 tooling/sprites/build_semantic_map.py \
  --action all \
  --max-tags-per-sprite 4 \
  --allow-generic-tags
```

## Output Shapes

`embeddings.jsonl` row:

```json
{
  "kind": "choice_core",
  "key": "junior_dev_e01_resume_gap#2",
  "model": "text-embedding-3-small",
  "dimensions": 1536,
  "text_sha1": "…",
  "embedding": [0.0123, -0.0044, ...],
  "meta": {
    "event_id": "junior_dev_e01_resume_gap",
    "choice_index": 2,
    "choice_key": "3"
  }
}
```

`event_sprite_selection.jsonl` row:

```json
{
  "event_id": "junior_dev_e01_resume_gap",
  "sprite_id": "work_career_corporate__code_review",
  "score": 0.412345,
  "score_margin": 0.033211,
  "phase": "Early",
  "source_file": "data/events/junior_dev.jsonl"
}
```

`choice_sprite_selection.jsonl` row:

```json
{
  "event_id": "junior_dev_e01_resume_gap",
  "choice_index": 1,
  "choice_key": "2",
  "sprite_id": "social_media_internet_culture__thread",
  "score": 0.388001,
  "score_margin": 0.027665,
  "dominant_axis": "audience",
  "source_file": "data/events/junior_dev.jsonl"
}
```

## Notes

- Choice mappings are currently keyed by `event_id + choice_index`.
- If you reorder choices inside an event later, rerun the script to regenerate mappings.
- Choice scoring blends embeddings from:
  - choice title/body
  - event title/body
- Blend defaults:
  - `--choice-text-weight 0.75`
  - `--event-context-weight 0.25`
- Sprite text construction is deterministic:
  - primary token from sprite name
  - plus up to `--max-tags-per-sprite` filtered tags (default 4)
  - generic tags are removed unless `--allow-generic-tags` is set
- Mapping enforces unique sprite IDs across all visible panels for each event:
  - event panel + 3 choice cards
