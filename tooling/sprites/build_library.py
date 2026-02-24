#!/usr/bin/env python3
"""
Build sprite library metadata from the source-of-truth JSONL.

Reads:  data/sprites/sprite_library.jsonl

Writes:
  - data/sprites/sprite_library_meta.json  (palette + counts + categories)

The command center app (apps/command-center/index.html) reads the JSONL directly
at runtime via Vite. No HTML or JS outputs are needed.

Usage:
  python3 tooling/sprites/build_library.py
"""

from __future__ import annotations

import hashlib
import json
import re
from pathlib import Path

SRC_JSONL = Path("data/sprites/sprite_library.jsonl")
OUT_META = Path("data/sprites/sprite_library_meta.json")

VERSION = 4

PALETTE = {
    "g": "#00ff41",
    "a": "#ffb000",
    "r": "#ff4444",
    "b": "#4488ff",
    "p": "#bb88ff",
    "c": "#00ddff",
    "y": "#ffdd00",
    "k": "#ff66aa",
    "o": "#ff8844",
    "d": "#334455",
    "m": "#556677",
    "w": "#ccddee",
}

DECLARED_STANDARD = {"rows": 5, "cols": 10}


def sha1_hex(s: str) -> str:
    return hashlib.sha1(s.encode("utf-8")).hexdigest()


def slugify(s: str) -> str:
    s = s.lower()
    s = re.sub(r"[^a-z0-9]+", "_", s)
    s = re.sub(r"_+", "_", s).strip("_")
    return s or "x"


def load_sprites() -> list[dict]:
    records = []
    with SRC_JSONL.open("r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line:
                records.append(json.loads(line))
    return records


def main() -> int:
    records = load_sprites()
    print(f"Loaded {len(records)} sprites from {SRC_JSONL}")

    # Build category list preserving order of first appearance
    cat_seen: set[str] = set()
    cat_order: list[dict] = []
    for rec in records:
        c = rec["category"]
        if c not in cat_seen:
            cat_seen.add(c)
            cat_order.append({"name": c, "key": slugify(c)})

    # Diagnostics
    bad_rows = 0
    bad_cols = 0
    for rec in records:
        lines_1x = rec.get("1x", [])
        if len(lines_1x) != DECLARED_STANDARD["rows"]:
            bad_rows += 1
        for line in lines_1x:
            col_count = sum(len(seg[0]) for seg in line)
            if col_count != DECLARED_STANDARD["cols"]:
                bad_cols += 1
                break

    # Write meta
    src_content = SRC_JSONL.read_text(encoding="utf-8")
    meta = {
        "version": VERSION,
        "declared_standard": DECLARED_STANDARD,
        "source": {"file": str(SRC_JSONL), "sha1": sha1_hex(src_content)},
        "palette": PALETTE,
        "counts": {
            "categories": len(cat_order),
            "sprites": len(records),
            "nonstandard_rows_codepoints": bad_rows,
            "nonstandard_cols_codepoints": bad_cols,
        },
        "categories": cat_order,
    }
    OUT_META.parent.mkdir(parents=True, exist_ok=True)
    OUT_META.write_text(json.dumps(meta, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote {OUT_META}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
