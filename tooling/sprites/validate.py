#!/usr/bin/env python3
"""
Validate the sprite library JSONL (source of truth).

Checks:
  - Required fields: name, category, 1x
  - Optional tag field: tags (if present, must be list[str] tokenizable)
  - 1x: exactly 5 lines, each line exactly 10 codepoints (warn if not, error with --strict)
  - 2x (if present): exactly 10 lines, each line exactly 20 codepoints
  - Color classes are valid
  - No duplicate names within same category

Usage:
  python3 tooling/sprites/validate.py                  # warn on non-standard 1x widths
  python3 tooling/sprites/validate.py --strict         # error on any width deviation
  python3 tooling/sprites/validate.py --fix-widths     # just report width stats
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

JSONL_PATH = Path("data/sprites/sprite_library.jsonl")

VALID_CLASSES = {"g", "a", "r", "b", "p", "c", "y", "k", "o", "d", "m", "w", ""}

SPEC = {
    "1x": {"rows": 5, "cols": 10},
    "2x": {"rows": 10, "cols": 20},
}

TOKEN_RE = re.compile(r"[a-z0-9]+", flags=re.IGNORECASE)


def has_token_chars(value: str) -> bool:
    return bool(TOKEN_RE.search(value))


def validate_token_list_field(rec: dict, field: str, label: str, errors: list[str]) -> None:
    value = rec.get(field)
    if value is None:
        return
    if not isinstance(value, list):
        errors.append(f"{label}: '{field}' must be a list of strings")
        return

    for i, item in enumerate(value, 1):
        if not isinstance(item, str):
            errors.append(f"{label}: '{field}' item {i} must be string, got {type(item).__name__}")
            continue
        if not has_token_chars(item):
            errors.append(f"{label}: '{field}' item {i} has no alphanumeric token content")


def validate(strict: bool = False) -> int:
    if not JSONL_PATH.exists():
        print(f"ERROR: {JSONL_PATH} not found")
        return 1

    errors: list[str] = []
    warnings: list[str] = []
    seen_names: dict[str, set[str]] = {}  # category -> set of names
    total = 0
    width_ok = {"1x": 0, "2x": 0}
    width_bad = {"1x": 0, "2x": 0}

    with JSONL_PATH.open("r", encoding="utf-8") as f:
        for line_num, line in enumerate(f, 1):
            line = line.strip()
            if not line:
                continue

            try:
                rec = json.loads(line)
            except json.JSONDecodeError as e:
                errors.append(f"Line {line_num}: invalid JSON: {e}")
                continue

            total += 1
            name = rec.get("name")
            category = rec.get("category")
            label = f"#{line_num} {name!r}"

            # Required fields
            if not name:
                errors.append(f"Line {line_num}: missing 'name'")
            if not category:
                errors.append(f"Line {line_num}: missing 'category'")
            if "1x" not in rec:
                errors.append(f"{label}: missing '1x' field")
                continue

            validate_token_list_field(rec, "tags", label, errors)

            # Duplicate check
            if category:
                if category not in seen_names:
                    seen_names[category] = set()
                if name in seen_names[category]:
                    errors.append(f"{label}: duplicate name in category '{category}'")
                else:
                    seen_names[category].add(name)

            # Validate each resolution
            for res_key in ("1x", "2x"):
                sprite_data = rec.get(res_key)
                if sprite_data is None:
                    continue

                spec = SPEC[res_key]
                expected_rows = spec["rows"]
                expected_cols = spec["cols"]

                if not isinstance(sprite_data, list):
                    errors.append(f"{label} {res_key}: expected list of lines, got {type(sprite_data).__name__}")
                    continue

                # Row count
                actual_rows = len(sprite_data)
                if actual_rows != expected_rows:
                    errors.append(f"{label} {res_key}: expected {expected_rows} rows, got {actual_rows}")

                # Check each line
                line_ok = True
                for i, line_segs in enumerate(sprite_data):
                    if not isinstance(line_segs, list):
                        errors.append(f"{label} {res_key} line {i+1}: expected list of segments")
                        line_ok = False
                        continue

                    width = 0
                    for seg in line_segs:
                        if not isinstance(seg, list) or len(seg) != 2:
                            errors.append(f"{label} {res_key} line {i+1}: segment must be [text, class], got {seg!r}")
                            line_ok = False
                            continue

                        text, cls = seg
                        width += len(text)

                        if cls not in VALID_CLASSES:
                            errors.append(f"{label} {res_key} line {i+1}: invalid color class {cls!r}")

                    if width != expected_cols:
                        msg = f"{label} {res_key} line {i+1}: width={width}, expected {expected_cols}"
                        if res_key == "2x" or strict:
                            errors.append(msg)
                        else:
                            warnings.append(msg)
                        line_ok = False

                if line_ok:
                    width_ok[res_key] += 1
                else:
                    width_bad[res_key] += 1

    # Report
    print(f"Validated {total} sprites from {JSONL_PATH}")
    print(f"  1x: {width_ok['1x']} ok, {width_bad['1x']} non-standard widths")
    if width_ok["2x"] or width_bad["2x"]:
        print(f"  2x: {width_ok['2x']} ok, {width_bad['2x']} non-standard widths")

    if warnings:
        print(f"\n{len(warnings)} warnings:")
        for w in warnings[:20]:
            print(f"  WARN: {w}")
        if len(warnings) > 20:
            print(f"  ... and {len(warnings) - 20} more warnings")

    if errors:
        print(f"\n{len(errors)} errors:")
        for e in errors[:30]:
            print(f"  ERROR: {e}")
        if len(errors) > 30:
            print(f"  ... and {len(errors) - 30} more errors")
        return 1

    if not warnings:
        print("\nAll sprites valid!")
    else:
        print(f"\n{len(warnings)} width warnings (use --strict to make these errors)")

    return 0


if __name__ == "__main__":
    strict = "--strict" in sys.argv
    raise SystemExit(validate(strict=strict))
