#!/usr/bin/env python3
"""
Build embedding-backed sprite mappings for events and choices.

Pipeline stages:
1) Generate and persist embeddings (cached, incremental).
2) Compute similarities and persist deterministic mappings.

Default outputs (under data/sprites/semantics):
- embeddings.jsonl
- event_sprite_selection.jsonl
- choice_sprite_selection.jsonl
- metrics.json
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import time
import urllib.error
import urllib.request
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass
from pathlib import Path
from typing import Any


TOKEN_RE = re.compile(r"[a-z0-9_]+", flags=re.IGNORECASE)

GENERIC_TAG_TOKENS = {
    "general",
    "generic",
    "misc",
    "neutral",
    "abstract",
    "concept",
    "concepts",
    "status",
    "effect",
    "effects",
    "icon",
    "symbol",
    "shape",
    "object",
    "item",
    "thing",
    "stuff",
    "work",
    "career",
    "corporate",
    "business",
    "social",
    "media",
    "internet",
    "culture",
    "people",
    "relationship",
    "relationships",
    "tech",
    "software",
    "digital",
    "emotion",
    "emotions",
    "lifestyle",
    "health",
    "nature",
    "environment",
    "place",
    "urban",
    "life",
    "news",
    "politics",
    "government",
    "game",
    "mechanic",
    "mechanics",
    "default",
    "unknown",
    "other",
    "common",
    "normal",
    "standard",
    "system",
    "core",
    "base",
    "meta",
    "theme",
    "topic",
    "domain",
    "category",
    "process",
    "team",
    "organization",
    "office",
    "company",
    "management",
    "product",
    "engineering",
    "communication",
    "decision",
    "strategy",
    "content",
    "online",
    "platform",
    "app",
    "tool",
    "tools",
    "device",
    "devices",
    "technology",
    "visual",
    "iconic",
    "symbolic",
    "placeholder",
    "placeholder_icon",
}

DEFAULT_OUT_DIR = Path("data/sprites/semantics")
DEFAULT_EVENTS_DIR = Path("data/events")
DEFAULT_SPRITES_PATH = Path("data/sprites/sprite_library.jsonl")
DEFAULT_EMBEDDINGS_PATH = DEFAULT_OUT_DIR / "embeddings.jsonl"
DEFAULT_EVENT_MAP_PATH = DEFAULT_OUT_DIR / "event_sprite_selection.jsonl"
DEFAULT_CHOICE_MAP_PATH = DEFAULT_OUT_DIR / "choice_sprite_selection.jsonl"
DEFAULT_METRICS_PATH = DEFAULT_OUT_DIR / "metrics.json"


@dataclass(frozen=True)
class SpriteDef:
    sprite_id: str
    idx: int
    name: str
    category: str
    category_key: str
    tags_raw: list[str]
    tags_compact: list[str]
    visible_tokens: list[str]


@dataclass(frozen=True)
class ChoiceDef:
    event_id: str
    event_title: str
    event_body: str
    phase: str
    choice_index: int
    choice_key: str
    title: str
    body: str
    delta: dict[str, int]
    source_file: str


@dataclass(frozen=True)
class EventDef:
    event_id: str
    phase: str
    title: str
    body: str
    source_file: str


@dataclass(frozen=True)
class EmbeddingItem:
    kind: str
    key: str
    text: str
    text_sha1: str
    meta: dict[str, Any]


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description="Build embedding-based sprite mappings.")
    p.add_argument(
        "--action",
        choices=["all", "embeddings", "mapping"],
        default="all",
        help="Pipeline step to run (default: all).",
    )
    p.add_argument("--repo-root", default=".", help="Repo root path (default: .)")
    p.add_argument(
        "--events-dir",
        default=str(DEFAULT_EVENTS_DIR),
        help=f"Events directory (default: {DEFAULT_EVENTS_DIR})",
    )
    p.add_argument(
        "--sprites-path",
        default=str(DEFAULT_SPRITES_PATH),
        help=f"Sprite source JSONL path (default: {DEFAULT_SPRITES_PATH})",
    )
    p.add_argument(
        "--embeddings-path",
        default=str(DEFAULT_EMBEDDINGS_PATH),
        help=f"Embeddings cache JSONL path (default: {DEFAULT_EMBEDDINGS_PATH})",
    )
    p.add_argument(
        "--event-map-path",
        default=str(DEFAULT_EVENT_MAP_PATH),
        help=f"Event mapping JSONL output (default: {DEFAULT_EVENT_MAP_PATH})",
    )
    p.add_argument(
        "--choice-map-path",
        default=str(DEFAULT_CHOICE_MAP_PATH),
        help=f"Choice mapping JSONL output (default: {DEFAULT_CHOICE_MAP_PATH})",
    )
    p.add_argument(
        "--metrics-path",
        default=str(DEFAULT_METRICS_PATH),
        help=f"Metrics JSON output (default: {DEFAULT_METRICS_PATH})",
    )

    p.add_argument(
        "--model",
        default="text-embedding-3-small",
        help="OpenAI embedding model (default: text-embedding-3-small)",
    )
    p.add_argument(
        "--dimensions",
        type=int,
        default=1536,
        help="Embedding dimensions (set <=0 to use model default).",
    )
    p.add_argument(
        "--api-base",
        default="https://api.openai.com/v1",
        help="OpenAI API base URL (default: https://api.openai.com/v1)",
    )
    p.add_argument(
        "--api-key-env",
        default="OPENAI_API_KEY",
        help="Environment variable containing API key (default: OPENAI_API_KEY)",
    )
    p.add_argument("--batch-size", type=int, default=48, help="Embeddings batch size.")
    p.add_argument(
        "--embed-workers",
        type=int,
        default=6,
        help="Parallel workers for embedding API batches (default: 6).",
    )
    p.add_argument("--timeout-sec", type=int, default=90, help="API request timeout.")
    p.add_argument("--max-retries", type=int, default=6, help="Max retries per batch.")
    p.add_argument(
        "--retry-base-sec",
        type=float,
        default=1.5,
        help="Backoff base seconds (exponential).",
    )
    p.add_argument(
        "--store-text",
        action="store_true",
        help="Store full query text in embeddings.jsonl (default: off).",
    )
    p.add_argument(
        "--dry-run",
        action="store_true",
        help="Plan work and output counts; do not call API or write mappings.",
    )

    p.add_argument(
        "--max-tags-per-sprite",
        type=int,
        default=4,
        help="Max positive semantic tags retained per sprite (default: 4).",
    )
    p.add_argument(
        "--allow-generic-tags",
        action="store_true",
        help="Do not filter generic tags during tag compaction (default: filtered).",
    )
    p.add_argument(
        "--choice-text-weight",
        type=float,
        default=0.75,
        help="Blend weight for choice title/body embedding in choice scoring.",
    )
    p.add_argument(
        "--event-context-weight",
        type=float,
        default=0.25,
        help="Blend weight for event title/body embedding in choice scoring.",
    )
    p.add_argument(
        "--choice-axis-weight",
        type=float,
        default=0.0,
        help="Weight for choice-axis category bonus (default: 0.0).",
    )
    p.add_argument(
        "--choice-candidate-cap",
        type=int,
        default=20,
        help="Top candidates per choice for uniqueness assignment search.",
    )
    p.add_argument(
        "--max-events",
        type=int,
        default=0,
        help="Limit events for quick iteration (0 = all).",
    )
    p.add_argument(
        "--max-sprites",
        type=int,
        default=0,
        help="Limit sprites for quick iteration (0 = all).",
    )
    return p.parse_args()


def sha1_hex(text: str) -> str:
    return hashlib.sha1(text.encode("utf-8")).hexdigest()


def file_sha1(path: Path) -> str:
    h = hashlib.sha1()
    with path.open("rb") as f:
        while True:
            chunk = f.read(1 << 20)
            if not chunk:
                break
            h.update(chunk)
    return h.hexdigest()


def slugify(s: str) -> str:
    out = []
    last_underscore = False
    for ch in s:
        if ch.isascii() and ch.isalnum():
            out.append(ch.lower())
            last_underscore = False
        elif out and not last_underscore:
            out.append("_")
            last_underscore = True
    while out and out[-1] == "_":
        out.pop()
    return "".join(out) or "x"


def tokenize(s: str) -> list[str]:
    return [tok.lower() for tok in TOKEN_RE.findall(s)]


def dedup_keep_order(values: list[str]) -> list[str]:
    seen: set[str] = set()
    out: list[str] = []
    for v in values:
        if v in seen:
            continue
        seen.add(v)
        out.append(v)
    return out


def normalize_semantic_token(value: str) -> str:
    toks = tokenize(str(value))
    if not toks:
        return ""
    return "_".join(toks)


def is_generic_semantic_token(token: str, category_tokens: set[str]) -> bool:
    if not token:
        return True
    parts = [p for p in token.split("_") if p]
    if not parts:
        return True
    if all((p in GENERIC_TAG_TOKENS) or (p in category_tokens) for p in parts):
        return True
    if len(parts) == 1 and (parts[0] in GENERIC_TAG_TOKENS or parts[0] in category_tokens):
        return True
    return False


def compact_semantic_tags(
    *,
    primary_name: str,
    category_key: str,
    raw_tags: list[str],
    max_tags: int,
    allow_generic_tags: bool,
    include_primary_name: bool,
) -> list[str]:
    if max_tags <= 0:
        return []

    category_tokens = {tok for tok in category_key.split("_") if tok}
    out: list[str] = []
    seen: set[str] = set()

    if include_primary_name:
        primary = normalize_semantic_token(primary_name)
        if primary:
            out.append(primary)
            seen.add(primary)

    for raw in raw_tags:
        tok = normalize_semantic_token(raw)
        if not tok or tok in seen:
            continue
        if not allow_generic_tags and is_generic_semantic_token(tok, category_tokens):
            continue
        out.append(tok)
        seen.add(tok)
        if len(out) >= max_tags:
            break

    if not out and include_primary_name:
        primary = normalize_semantic_token(primary_name)
        if primary:
            out.append(primary)
    return out[:max_tags]


def normalize_delta(raw: dict[str, Any] | None) -> dict[str, int]:
    d = dict(raw or {})
    return {
        "money": int(d.get("money", 0)),
        "sanity": int(d.get("sanity", 0)),
        "audience": int(d.get("audience", 0)),
        "ai_skill": int(d.get("ai_skill", 0)),
        "friends": int(d.get("friends", 0)),
    }


def dominant_choice_axis(delta: dict[str, int]) -> str:
    dims = [
        ("money", abs(delta.get("money", 0))),
        ("sanity", abs(delta.get("sanity", 0))),
        ("audience", abs(delta.get("audience", 0))),
        ("ai_skill", abs(delta.get("ai_skill", 0))),
        ("friends", abs(delta.get("friends", 0))),
    ]
    axis, mag = max(dims, key=lambda x: x[1])
    return axis if mag > 0 else "mixed"


def choice_delta_category_bonus(axis: str, category_key: str) -> float:
    if axis == "money":
        return 1.0 if category_key in {"economy_housing_education", "work_career_corporate"} else 0.0
    if axis == "sanity":
        if category_key == "emotions_status_effects":
            return 1.0
        if category_key in {
            "food_drink_lifestyle_health",
            "nature_weather_environment",
            "health_medical_wellness",
        }:
            return 0.5
        return 0.0
    if axis == "friends":
        if category_key == "people_social_relationships":
            return 1.0
        if category_key == "social_media_internet_culture":
            return 0.5
        return 0.0
    if axis == "audience":
        if category_key == "social_media_internet_culture":
            return 1.0
        if category_key == "tech_software_digital":
            return 0.25
        return 0.0
    if axis == "ai_skill":
        if category_key == "tech_software_digital":
            return 1.0
        if category_key == "work_career_corporate":
            return 0.25
        return 0.0
    if axis == "mixed":
        if category_key == "abstract_concepts_game_mechanics":
            return 0.75
        if category_key == "emotions_status_effects":
            return 0.25
        return 0.0
    return 0.0


def collect_sprite_visible_tokens(rec: dict[str, Any]) -> list[str]:
    tokens: list[str] = []
    for key in ("1x", "2x"):
        rows = rec.get(key) or []
        for line in rows:
            for segment in line:
                if not isinstance(segment, list) or len(segment) != 2:
                    continue
                text = str(segment[0])
                if not text:
                    continue
                tokens.extend(tokenize(text))
    tokens = [t for t in tokens if len(t) >= 2 and not t.isdigit()]
    return dedup_keep_order(tokens)


def read_sprites(
    path: Path,
    max_sprites: int,
    max_tags_per_sprite: int,
    allow_generic_tags: bool,
) -> list[SpriteDef]:
    sprites: list[SpriteDef] = []
    for line_no, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        s = raw.strip()
        if not s:
            continue
        rec = json.loads(s)
        name = str(rec["name"])
        category = str(rec["category"])
        category_key = slugify(category)
        sprite_id = f"{category_key}__{slugify(name)}"
        raw_tags = [str(x) for x in (rec.get("tags") or [])]
        sprite = SpriteDef(
            sprite_id=sprite_id,
            idx=len(sprites),
            name=name,
            category=category,
            category_key=category_key,
            tags_raw=raw_tags,
            tags_compact=compact_semantic_tags(
                primary_name=name,
                category_key=category_key,
                raw_tags=raw_tags,
                max_tags=max_tags_per_sprite,
                allow_generic_tags=allow_generic_tags,
                include_primary_name=True,
            ),
            visible_tokens=collect_sprite_visible_tokens(rec),
        )
        sprites.append(sprite)
        if max_sprites > 0 and len(sprites) >= max_sprites:
            break
    if not sprites:
        raise RuntimeError(f"No sprites parsed from {path}")
    return sprites


def read_events(events_dir: Path, max_events: int) -> tuple[list[EventDef], list[ChoiceDef], list[Path]]:
    events: list[EventDef] = []
    choices: list[ChoiceDef] = []
    files = sorted(events_dir.glob("*.jsonl"))
    if not files:
        raise RuntimeError(f"No event catalogs found in {events_dir}")

    for file_path in files:
        rel = file_path.as_posix()
        for raw in file_path.read_text(encoding="utf-8").splitlines():
            s = raw.strip()
            if not s:
                continue
            rec = json.loads(s)
            eid = str(rec["id"])
            phase = str(rec.get("phase", "Any"))
            title = str(rec.get("title", ""))
            body = str(rec.get("body", ""))
            event = EventDef(
                event_id=eid,
                phase=phase,
                title=title,
                body=body,
                source_file=rel,
            )
            events.append(event)

            raw_choices = list(rec.get("choices") or [])
            for i, choice in enumerate(raw_choices[:3]):
                c = ChoiceDef(
                    event_id=eid,
                    event_title=title,
                    event_body=body,
                    phase=phase,
                    choice_index=i,
                    choice_key=str(i + 1),
                    title=str(choice.get("title", "")),
                    body=str(choice.get("body", "")),
                    delta=normalize_delta(choice.get("delta")),
                    source_file=rel,
                )
                choices.append(c)

            if max_events > 0 and len(events) >= max_events:
                return events, choices, files

    return events, choices, files


def sprite_positive_text(s: SpriteDef) -> str:
    tags = ", ".join(s.tags_compact) if s.tags_compact else "none"
    visible = ", ".join(s.visible_tokens) if s.visible_tokens else "none"
    return (
        f"sprite_id: {s.sprite_id}\n"
        f"category: {s.category}\n"
        f"name: {s.name}\n"
        f"tags: {tags}\n"
        f"visible_tokens: {visible}"
    )


def event_query_text(e: EventDef) -> str:
    return (
        f"event_id: {e.event_id}\n"
        f"phase: {e.phase}\n"
        f"title: {e.title}\n"
        f"body: {e.body}"
    )


def event_context_text(e: EventDef) -> str:
    return (
        f"event_id: {e.event_id}\n"
        f"title: {e.title}\n"
        f"body: {e.body}"
    )


def choice_core_text(c: ChoiceDef) -> str:
    return (
        f"event_id: {c.event_id}\n"
        f"choice_index: {c.choice_index}\n"
        f"choice_key: {c.choice_key}\n"
        f"choice_title: {c.title}\n"
        f"choice_body: {c.body}"
    )


def build_embedding_items(
    sprites: list[SpriteDef],
    events: list[EventDef],
    choices: list[ChoiceDef],
) -> list[EmbeddingItem]:
    items: list[EmbeddingItem] = []

    for s in sprites:
        pos_text = sprite_positive_text(s)
        items.append(
            EmbeddingItem(
                kind="sprite_pos",
                key=s.sprite_id,
                text=pos_text,
                text_sha1=sha1_hex(pos_text),
                meta={
                    "sprite_id": s.sprite_id,
                    "category": s.category,
                    "category_key": s.category_key,
                    "name": s.name,
                    "idx": s.idx,
                },
            )
        )

    for e in events:
        text = event_query_text(e)
        items.append(
            EmbeddingItem(
                kind="event",
                key=e.event_id,
                text=text,
                text_sha1=sha1_hex(text),
                meta={
                    "event_id": e.event_id,
                    "phase": e.phase,
                    "title": e.title,
                    "source_file": e.source_file,
                },
            )
        )
        context_text = event_context_text(e)
        items.append(
            EmbeddingItem(
                kind="event_ctx",
                key=e.event_id,
                text=context_text,
                text_sha1=sha1_hex(context_text),
                meta={
                    "event_id": e.event_id,
                    "phase": e.phase,
                    "title": e.title,
                    "source_file": e.source_file,
                },
            )
        )

    for c in choices:
        text = choice_core_text(c)
        items.append(
            EmbeddingItem(
                kind="choice_core",
                key=f"{c.event_id}#{c.choice_index}",
                text=text,
                text_sha1=sha1_hex(text),
                meta={
                    "event_id": c.event_id,
                    "choice_index": c.choice_index,
                    "choice_key": c.choice_key,
                    "phase": c.phase,
                    "source_file": c.source_file,
                },
            )
        )

    return items


class OpenAIEmbeddingClient:
    def __init__(
        self,
        api_key: str,
        model: str,
        dimensions: int,
        api_base: str,
        timeout_sec: int,
        max_retries: int,
        retry_base_sec: float,
    ) -> None:
        self.api_key = api_key
        self.model = model
        self.dimensions = dimensions
        self.api_base = api_base.rstrip("/")
        self.timeout_sec = timeout_sec
        self.max_retries = max_retries
        self.retry_base_sec = retry_base_sec

    def embed_texts(self, texts: list[str]) -> list[list[float]]:
        if not texts:
            return []
        payload: dict[str, Any] = {
            "model": self.model,
            "input": texts,
            "encoding_format": "float",
        }
        if self.dimensions > 0:
            payload["dimensions"] = self.dimensions

        body = json.dumps(payload).encode("utf-8")
        url = f"{self.api_base}/embeddings"

        for attempt in range(self.max_retries + 1):
            req = urllib.request.Request(
                url=url,
                method="POST",
                headers={
                    "Authorization": f"Bearer {self.api_key}",
                    "Content-Type": "application/json",
                },
                data=body,
            )
            try:
                with urllib.request.urlopen(req, timeout=self.timeout_sec) as resp:
                    raw = resp.read().decode("utf-8")
                parsed = json.loads(raw)
                data = parsed.get("data")
                if not isinstance(data, list) or len(data) != len(texts):
                    raise RuntimeError("Unexpected embeddings response shape")
                out: list[list[float]] = []
                for item in data:
                    emb = item.get("embedding")
                    if not isinstance(emb, list):
                        raise RuntimeError("Missing embedding vector in response")
                    out.append([float(x) for x in emb])
                return out
            except urllib.error.HTTPError as e:
                status = e.code
                details = e.read().decode("utf-8", errors="replace")
                retriable = status == 429 or status >= 500
                if attempt >= self.max_retries or not retriable:
                    raise RuntimeError(
                        f"OpenAI embeddings request failed: HTTP {status}: {details[:500]}"
                    ) from e
            except Exception as e:
                if attempt >= self.max_retries:
                    raise
            sleep_sec = self.retry_base_sec * (2**attempt)
            time.sleep(sleep_sec)

        raise RuntimeError("OpenAI embeddings request exhausted retries")


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    out: list[dict[str, Any]] = []
    for raw in path.read_text(encoding="utf-8").splitlines():
        s = raw.strip()
        if not s:
            continue
        out.append(json.loads(s))
    return out


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as f:
        for row in rows:
            f.write(json.dumps(row, ensure_ascii=False))
            f.write("\n")


def embedding_identity(kind: str, key: str) -> str:
    return f"{kind}|{key}"


def generate_and_save_embeddings(
    args: argparse.Namespace,
    sprites: list[SpriteDef],
    events: list[EventDef],
    choices: list[ChoiceDef],
) -> list[dict[str, Any]]:
    """
    Generate embeddings for sprite/event/choice texts and persist cache JSONL.
    Reuses existing rows when kind+key+model+dimensions+text_sha1 are unchanged.
    """
    items = build_embedding_items(
        sprites=sprites,
        events=events,
        choices=choices,
    )
    existing = read_jsonl(args.embeddings_path)

    existing_by_identity: dict[str, dict[str, Any]] = {}
    for row in existing:
        kind = row.get("kind")
        key = row.get("key")
        if not isinstance(kind, str) or not isinstance(key, str):
            continue
        existing_by_identity[embedding_identity(kind, key)] = row

    reusable_rows: list[dict[str, Any]] = []
    missing_items: list[EmbeddingItem] = []
    for item in items:
        ident = embedding_identity(item.kind, item.key)
        row = existing_by_identity.get(ident)
        if (
            row
            and row.get("model") == args.model
            and int(row.get("dimensions", 0)) == int(args.dimensions)
            and row.get("text_sha1") == item.text_sha1
            and isinstance(row.get("embedding"), list)
        ):
            reusable_rows.append(row)
        else:
            missing_items.append(item)

    print(
        f"Embeddings: total={len(items)} reuse={len(reusable_rows)} missing={len(missing_items)}"
    )

    if args.dry_run:
        return sorted(
            reusable_rows,
            key=lambda r: (str(r.get("kind", "")), str(r.get("key", ""))),
        )

    if missing_items:
        api_key = os.environ.get(args.api_key_env) or os.environ.get("PU_OPENAI_API_KEY")
        if not api_key:
            raise RuntimeError(
                f"Missing API key. Set {args.api_key_env} (or PU_OPENAI_API_KEY) before running."
            )
        client = OpenAIEmbeddingClient(
            api_key=api_key,
            model=args.model,
            dimensions=args.dimensions,
            api_base=args.api_base,
            timeout_sec=args.timeout_sec,
            max_retries=args.max_retries,
            retry_base_sec=args.retry_base_sec,
        )

        batch_size = max(1, int(args.batch_size))
        batches: list[tuple[int, list[EmbeddingItem]]] = []
        for i, start in enumerate(range(0, len(missing_items), batch_size), start=1):
            batches.append((i, missing_items[start : start + batch_size]))

        def embed_batch(batch_idx: int, batch_items: list[EmbeddingItem]) -> tuple[int, list[dict[str, Any]]]:
            vectors = client.embed_texts([it.text for it in batch_items])
            rows: list[dict[str, Any]] = []
            for item, vec in zip(batch_items, vectors):
                row = {
                    "kind": item.kind,
                    "key": item.key,
                    "model": args.model,
                    "dimensions": args.dimensions,
                    "text_sha1": item.text_sha1,
                    "embedding": vec,
                    "meta": item.meta,
                }
                if args.store_text:
                    row["text"] = item.text
                rows.append(row)
            return batch_idx, rows

        generated_rows = []
        done_batches = 0
        done_items = 0
        max_workers = max(1, int(args.embed_workers))
        if max_workers == 1 or len(batches) <= 1:
            for batch_idx, batch_items in batches:
                _, rows = embed_batch(batch_idx, batch_items)
                generated_rows.extend(rows)
                done_batches += 1
                done_items += len(batch_items)
                print(
                    f"  embedded batches={done_batches}/{len(batches)} "
                    f"rows={done_items}/{len(missing_items)}"
                )
        else:
            with ThreadPoolExecutor(max_workers=min(max_workers, len(batches))) as ex:
                futures = [ex.submit(embed_batch, batch_idx, batch_items) for batch_idx, batch_items in batches]
                for future in as_completed(futures):
                    _, rows = future.result()
                    generated_rows.extend(rows)
                    done_batches += 1
                    done_items += len(rows)
                    print(
                        f"  embedded batches={done_batches}/{len(batches)} "
                        f"rows={done_items}/{len(missing_items)}"
                    )
    else:
        generated_rows = []

    merged = reusable_rows + generated_rows
    merged.sort(key=lambda r: (str(r.get("kind", "")), str(r.get("key", ""))))
    write_jsonl(args.embeddings_path, merged)
    print(f"Wrote {args.embeddings_path} ({len(merged)} rows)")
    return merged


def require_numpy():
    try:
        import numpy as np  # type: ignore

        return np
    except Exception as e:
        raise RuntimeError(
            "Mapping step requires numpy for vector math. Install with: python3 -m pip install numpy"
        ) from e


def l2_normalize_rows(np, matrix):
    norms = np.linalg.norm(matrix, axis=1, keepdims=True)
    norms[norms == 0] = 1.0
    return matrix / norms


def l2_normalize_vec(np, vec):
    n = float(np.linalg.norm(vec))
    if n == 0.0:
        return vec
    return vec / n


def build_choice_axis_bonus_vectors(np, sprites: list[SpriteDef]) -> dict[str, Any]:
    out: dict[str, Any] = {}
    axes = ["money", "sanity", "audience", "ai_skill", "friends", "mixed"]
    for axis in axes:
        out[axis] = np.array(
            [choice_delta_category_bonus(axis, s.category_key) for s in sprites], dtype=np.float32
        )
    return out


def top_n_indices(np, scores, n: int):
    n = max(1, min(n, len(scores)))
    idx = np.argpartition(scores, -n)[-n:]
    idx = idx[np.argsort(scores[idx])[::-1]]
    return idx


def best_unique_assignment(np, score_matrix, candidate_cap: int) -> list[int]:
    n_choices, n_sprites = score_matrix.shape
    if n_choices <= 1:
        return [int(np.argmax(score_matrix[0]))]

    candidates = [
        list(map(int, top_n_indices(np, score_matrix[i], candidate_cap)))
        for i in range(n_choices)
    ]
    # Ensure each row has at least one candidate.
    for i in range(n_choices):
        if not candidates[i]:
            candidates[i] = [int(np.argmax(score_matrix[i]))]

    best_score = -1e18
    best_pick: list[int] | None = None

    def dfs(i: int, used: set[int], current: list[int], total: float) -> None:
        nonlocal best_score, best_pick
        if i == n_choices:
            if total > best_score:
                best_score = total
                best_pick = current.copy()
            return
        for idx in candidates[i]:
            if idx in used:
                continue
            used.add(idx)
            current.append(idx)
            dfs(i + 1, used, current, total + float(score_matrix[i, idx]))
            current.pop()
            used.remove(idx)

    dfs(0, set(), [], 0.0)
    if best_pick is not None:
        return best_pick

    # Fallback if candidate_cap was too tight and produced no valid unique combo.
    fallback: list[int] = []
    used: set[int] = set()
    for i in range(n_choices):
        ranked = list(map(int, np.argsort(score_matrix[i])[::-1]))
        pick = None
        for idx in ranked:
            if idx not in used:
                pick = idx
                break
        if pick is None:
            pick = int(np.argmax(score_matrix[i]))
        fallback.append(pick)
        used.add(pick)
    return fallback


def generate_and_save_similarity_mappings(
    args: argparse.Namespace,
    sprites: list[SpriteDef],
    events: list[EventDef],
    choices: list[ChoiceDef],
    embedding_rows: list[dict[str, Any]],
) -> dict[str, Any]:
    """
    Compute cosine-similarity mappings and persist:
    - event sprite selections JSONL
    - choice sprite selections JSONL
    - metrics JSON
    """
    np = require_numpy()

    by_kind_key: dict[tuple[str, str], dict[str, Any]] = {}
    for row in embedding_rows:
        kind = row.get("kind")
        key = row.get("key")
        if isinstance(kind, str) and isinstance(key, str):
            by_kind_key[(kind, key)] = row

    sprite_pos_vectors: list[list[float]] = []
    sprite_ids: list[str] = []
    for s in sprites:
        pos = by_kind_key.get(("sprite_pos", s.sprite_id))
        if pos is None:
            raise RuntimeError(f"Missing sprite embeddings for {s.sprite_id}")
        sprite_pos_vectors.append([float(x) for x in pos["embedding"]])
        sprite_ids.append(s.sprite_id)

    sprite_pos = l2_normalize_rows(np, np.array(sprite_pos_vectors, dtype=np.float32))
    axis_bonus_vectors = build_choice_axis_bonus_vectors(np, sprites)

    choice_weight = max(0.0, float(args.choice_text_weight))
    event_weight = max(0.0, float(args.event_context_weight))
    if choice_weight == 0.0 and event_weight == 0.0:
        choice_weight = 1.0
    total_weight = choice_weight + event_weight
    if total_weight <= 0.0:
        total_weight = 1.0
    choice_weight /= total_weight
    event_weight /= total_weight

    choices_by_event: dict[str, list[ChoiceDef]] = {}
    for c in choices:
        choices_by_event.setdefault(c.event_id, []).append(c)
    for eid in choices_by_event:
        choices_by_event[eid].sort(key=lambda c: c.choice_index)

    def row_margin(scores, picked_idx: int) -> float:
        top2 = top_n_indices(np, scores, 2)
        second = None
        for idx in top2:
            if int(idx) != picked_idx:
                second = float(scores[idx])
                break
        best = float(scores[picked_idx])
        return best - second if second is not None else best

    event_map_rows: list[dict[str, Any]] = []
    choice_map_rows: list[dict[str, Any]] = []
    event_scores: list[float] = []
    choice_scores: list[float] = []
    event_margins: list[float] = []
    choice_margins: list[float] = []

    for e in events:
        event_row = by_kind_key.get(("event", e.event_id))
        event_ctx_row = by_kind_key.get(("event_ctx", e.event_id))
        if event_row is None:
            raise RuntimeError(f"Missing event embedding for {e.event_id}")
        if event_ctx_row is None:
            raise RuntimeError(f"Missing event_ctx embedding for {e.event_id}")

        q_event = l2_normalize_vec(np, np.array(event_row["embedding"], dtype=np.float32))
        q_event_ctx = l2_normalize_vec(np, np.array(event_ctx_row["embedding"], dtype=np.float32))

        event_base = sprite_pos @ q_event
        event_scores_vec = event_base

        panel_scores = [event_scores_vec]
        panel_choices: list[ChoiceDef | None] = [None]

        for c in choices_by_event.get(e.event_id, []):
            choice_row = by_kind_key.get(("choice_core", f"{c.event_id}#{c.choice_index}"))
            if choice_row is None:
                raise RuntimeError(f"Missing choice_core embedding for {c.event_id}#{c.choice_index}")
            q_choice = l2_normalize_vec(np, np.array(choice_row["embedding"], dtype=np.float32))
            q_blend = l2_normalize_vec(
                np,
                (choice_weight * q_choice) + (event_weight * q_event_ctx),
            )

            choice_base = sprite_pos @ q_blend
            choice_scores_vec = choice_base

            if float(args.choice_axis_weight) != 0.0:
                axis = dominant_choice_axis(c.delta)
                axis_bonus = axis_bonus_vectors.get(axis, axis_bonus_vectors["mixed"])
                choice_scores_vec = choice_scores_vec + (float(args.choice_axis_weight) * axis_bonus)

            panel_scores.append(choice_scores_vec)
            panel_choices.append(c)

        score_matrix = np.vstack(panel_scores)
        picks = best_unique_assignment(np, score_matrix, int(args.choice_candidate_cap))

        event_pick = int(picks[0])
        event_score = float(score_matrix[0][event_pick])
        event_margin = row_margin(score_matrix[0], event_pick)
        event_map_rows.append(
            {
                "event_id": e.event_id,
                "sprite_id": sprite_ids[event_pick],
                "score": round(event_score, 6),
                "score_margin": round(event_margin, 6),
                "phase": e.phase,
                "source_file": e.source_file,
            }
        )
        event_scores.append(event_score)
        event_margins.append(event_margin)

        for row_idx in range(1, len(panel_scores)):
            c = panel_choices[row_idx]
            if c is None:
                continue
            sprite_idx = int(picks[row_idx])
            scores = score_matrix[row_idx]
            best_score = float(scores[sprite_idx])
            margin = row_margin(scores, sprite_idx)
            choice_map_rows.append(
                {
                    "event_id": c.event_id,
                    "choice_index": c.choice_index,
                    "choice_key": c.choice_key,
                    "sprite_id": sprite_ids[sprite_idx],
                    "score": round(best_score, 6),
                    "score_margin": round(margin, 6),
                    "dominant_axis": dominant_choice_axis(c.delta),
                    "source_file": c.source_file,
                }
            )
            choice_scores.append(best_score)
            choice_margins.append(margin)

    # Deterministic order.
    event_map_rows.sort(key=lambda r: str(r["event_id"]))
    choice_map_rows.sort(key=lambda r: (str(r["event_id"]), int(r["choice_index"])))

    write_jsonl(args.event_map_path, event_map_rows)
    write_jsonl(args.choice_map_path, choice_map_rows)
    print(f"Wrote {args.event_map_path} ({len(event_map_rows)} rows)")
    print(f"Wrote {args.choice_map_path} ({len(choice_map_rows)} rows)")

    metrics = {
        "generated_at_epoch_sec": int(time.time()),
        "config": {
            "model": args.model,
            "dimensions": args.dimensions,
            "max_tags_per_sprite": args.max_tags_per_sprite,
            "allow_generic_tags": bool(args.allow_generic_tags),
            "choice_text_weight": choice_weight,
            "event_context_weight": event_weight,
            "choice_axis_weight": args.choice_axis_weight,
            "choice_candidate_cap": args.choice_candidate_cap,
        },
        "counts": {
            "sprites": len(sprites),
            "events": len(events),
            "choices": len(choices),
            "event_mappings": len(event_map_rows),
            "choice_mappings": len(choice_map_rows),
        },
        "coverage": {
            "event_mapping_pct": round((len(event_map_rows) / max(1, len(events))) * 100.0, 3),
            "choice_mapping_pct": round((len(choice_map_rows) / max(1, len(choices))) * 100.0, 3),
        },
        "score_summary": {
            "event_mean": round(float(sum(event_scores) / max(1, len(event_scores))), 6),
            "event_margin_mean": round(float(sum(event_margins) / max(1, len(event_margins))), 6),
            "choice_mean": round(float(sum(choice_scores) / max(1, len(choice_scores))), 6),
            "choice_margin_mean": round(float(sum(choice_margins) / max(1, len(choice_margins))), 6),
        },
        "reuse": {
            "unique_event_sprites": len({row["sprite_id"] for row in event_map_rows}),
            "unique_choice_sprites": len({row["sprite_id"] for row in choice_map_rows}),
        },
    }
    choice_rows_by_event: dict[str, list[dict[str, Any]]] = {}
    for row in choice_map_rows:
        choice_rows_by_event.setdefault(str(row["event_id"]), []).append(row)
    duplicate_panel_events = 0
    for event_row in event_map_rows:
        eid = str(event_row["event_id"])
        seen = {str(event_row["sprite_id"])}
        dup = False
        for choice_row in choice_rows_by_event.get(eid, []):
            sid = str(choice_row["sprite_id"])
            if sid in seen:
                dup = True
                break
            seen.add(sid)
        if dup:
            duplicate_panel_events += 1
    metrics["quality"] = {
        "events_with_duplicate_event_choice_sprite": duplicate_panel_events
    }
    args.metrics_path.parent.mkdir(parents=True, exist_ok=True)
    args.metrics_path.write_text(json.dumps(metrics, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote {args.metrics_path}")
    return metrics


def resolve_path(repo_root: Path, raw: str) -> Path:
    p = Path(raw)
    if p.is_absolute():
        return p
    return (repo_root / p).resolve()


def build_input_hashes(sprites_path: Path, event_files: list[Path]) -> dict[str, Any]:
    event_sha = hashlib.sha1()
    for path in sorted(event_files):
        event_sha.update(path.as_posix().encode("utf-8"))
        event_sha.update(b"\0")
        event_sha.update(file_sha1(path).encode("utf-8"))
        event_sha.update(b"\n")
    return {
        "sprites_file": sprites_path.as_posix(),
        "sprites_sha1": file_sha1(sprites_path),
        "events_dir_sha1": event_sha.hexdigest(),
        "event_files": [path.as_posix() for path in sorted(event_files)],
    }


def summarize_tag_compaction(sprites: list[SpriteDef]) -> dict[str, Any]:
    raw_tag_total = sum(len(s.tags_raw) for s in sprites)
    compact_tag_total = sum(len(s.tags_compact) for s in sprites)
    return {
        "raw_tag_total": raw_tag_total,
        "compact_tag_total": compact_tag_total,
        "avg_raw_tags_per_sprite": round(raw_tag_total / max(1, len(sprites)), 3),
        "avg_compact_tags_per_sprite": round(compact_tag_total / max(1, len(sprites)), 3),
    }


def main() -> int:
    args = parse_args()
    repo_root = Path(args.repo_root).resolve()

    args.events_dir = resolve_path(repo_root, args.events_dir)
    args.sprites_path = resolve_path(repo_root, args.sprites_path)
    args.embeddings_path = resolve_path(repo_root, args.embeddings_path)
    args.event_map_path = resolve_path(repo_root, args.event_map_path)
    args.choice_map_path = resolve_path(repo_root, args.choice_map_path)
    args.metrics_path = resolve_path(repo_root, args.metrics_path)

    sprites = read_sprites(
        path=args.sprites_path,
        max_sprites=args.max_sprites,
        max_tags_per_sprite=max(1, int(args.max_tags_per_sprite)),
        allow_generic_tags=bool(args.allow_generic_tags),
    )
    events, choices, event_files = read_events(args.events_dir, args.max_events)
    input_hashes = build_input_hashes(args.sprites_path, event_files)
    tag_summary = summarize_tag_compaction(sprites)

    print(
        f"Loaded sprites={len(sprites)} events={len(events)} choices={len(choices)}"
    )
    print(
        "Tag compaction: "
        f"avg_raw={tag_summary['avg_raw_tags_per_sprite']} "
        f"avg_compact={tag_summary['avg_compact_tags_per_sprite']} "
        f"max_tags={max(1, int(args.max_tags_per_sprite))}"
    )

    if args.dry_run:
        items = build_embedding_items(
            sprites=sprites,
            events=events,
            choices=choices,
        )
        print(f"Dry-run embedding items: {len(items)}")
        print(json.dumps(tag_summary, indent=2))
        print(json.dumps(input_hashes, indent=2))
        return 0

    if args.action in {"all", "embeddings"}:
        embeddings = generate_and_save_embeddings(
            args=args,
            sprites=sprites,
            events=events,
            choices=choices,
        )
    else:
        embeddings = read_jsonl(args.embeddings_path)
        if not embeddings:
            raise RuntimeError(
                f"No embeddings found at {args.embeddings_path}. Run with --action embeddings first."
            )

    if args.action in {"all", "mapping"}:
        metrics = generate_and_save_similarity_mappings(
            args=args,
            sprites=sprites,
            events=events,
            choices=choices,
            embedding_rows=embeddings,
        )
        metrics["tag_compaction"] = tag_summary
        metrics["input_hashes"] = input_hashes
        args.metrics_path.write_text(json.dumps(metrics, indent=2) + "\n", encoding="utf-8")
        print(f"Updated {args.metrics_path} with input hashes")
    else:
        # Keep traceability even when only embeddings are generated.
        embed_meta = {
            "generated_at_epoch_sec": int(time.time()),
            "config": {
                "model": args.model,
                "dimensions": args.dimensions,
                "max_tags_per_sprite": args.max_tags_per_sprite,
                "allow_generic_tags": bool(args.allow_generic_tags),
            },
            "counts": {
                "sprites": len(sprites),
                "events": len(events),
                "choices": len(choices),
                "embeddings_rows": len(embeddings),
            },
            "tag_compaction": tag_summary,
            "input_hashes": input_hashes,
        }
        args.metrics_path.parent.mkdir(parents=True, exist_ok=True)
        args.metrics_path.write_text(json.dumps(embed_meta, indent=2) + "\n", encoding="utf-8")
        print(f"Wrote {args.metrics_path}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
