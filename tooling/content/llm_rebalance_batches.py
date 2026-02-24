#!/usr/bin/env python3
"""
Batch LLM rebalance runner for one character JSONL.

What it does:
1) Splits events into N batches.
2) Runs Codex and Claude per batch (in parallel).
3) Collects structured outputs.
4) Collates candidate deltas into a single merged update set.
5) Optionally applies merged deltas back to the character JSONL.
"""

from __future__ import annotations

import argparse
import json
import math
import os
import statistics
import subprocess
import sys
import tempfile
import textwrap
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass
from pathlib import Path
from typing import Any


STATS = ["sanity", "audience", "ai_skill", "friends"]
PHASE_ORDER = ["Early", "Mid", "Late", "Endgame"]


SCHEMA_OBJ: dict[str, Any] = {
    "type": "object",
    "properties": {
        "updates": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "id": {"type": "string"},
                    "choice_index": {"type": "integer", "minimum": 0},
                    "delta": {
                        "type": "object",
                        "properties": {
                            "money": {"type": "integer"},
                            "sanity": {"type": "integer", "minimum": -14, "maximum": 14},
                            "audience": {"type": "integer", "minimum": -14, "maximum": 14},
                            "ai_skill": {"type": "integer", "minimum": -14, "maximum": 14},
                            "friends": {"type": "integer", "minimum": -14, "maximum": 14},
                        },
                        "required": ["money", "sanity", "audience", "ai_skill", "friends"],
                        "additionalProperties": False,
                    },
                },
                "required": ["id", "choice_index", "delta"],
                "additionalProperties": False,
            },
        }
    },
    "required": ["updates"],
    "additionalProperties": False,
}


PROMPT_PREFIX = textwrap.dedent(
    """\
    You are the content balancer for Permanent Underclass.

    Goal: Rebalance choice deltas for one character file while preserving narrative intent from exact text.
    Return ONLY JSON matching schema: {"updates":[...]}.

    Input format:
    - You receive an array of event objects.
    - Each event has: id, phase, title, body, choices[].
    - choices are indexed 0..2.

    Output requirements:
    - Emit one update for EVERY choice in EVERY event.
    - Each update key: id, choice_index, delta.
    - Rewrite only delta values.
    - Keep money unchanged from source unless absolutely required; default unchanged.
    - Keep non-money deltas in [-14,14].

    Balancing requirements:
    - Do not default to repeated +-6/+-8 everywhere.
    - Keep tradeoff shape true to text: upside/downside should fit the specific wording.
    - Preserve sign intent when text clearly implies direction.
    - Keep outcomes plausible; avoid random noise.

    Progression requirements:
    - Respect phase progression (Early -> Mid -> Late -> Endgame): expected intensity should generally rise.
    - Late and Endgame can carry more +/-10..14 than Early, but not every choice should be extreme.
    - Keep variance across choices inside each phase.

    Important:
    - No commentary.
    - No markdown.
    - No extra keys.
    - Full coverage is mandatory.

    SOURCE_EVENTS_JSON:
    """
)


@dataclass
class TaskResult:
    provider: str
    batch_idx: int
    ok: bool
    elapsed_s: float
    updates: list[dict[str, Any]]
    error: str | None
    out_path: Path
    log_path: Path


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description="Run batched dual-LLM delta rebalance.")
    p.add_argument("--character", required=True, help="Character id, e.g. junior_dev")
    p.add_argument("--batches", type=int, default=5, help="Number of batches (default: 5)")
    p.add_argument("--repo-root", default=".", help="Repo root (default: .)")
    p.add_argument(
        "--out-dir",
        default=None,
        help="Output dir (default: /tmp/pu-llm-batch-<character>-<pid>)",
    )
    p.add_argument("--codex-model", default="gpt-5.2")
    p.add_argument("--codex-reasoning", default="medium")
    p.add_argument("--claude-model", default="claude-sonnet-4-6")
    p.add_argument("--claude-effort", default="high", choices=["low", "medium", "high"])
    p.add_argument("--max-workers", type=int, default=6, help="Parallel workers (default: 6)")
    p.add_argument("--timeout-sec", type=int, default=1200, help="Per-call timeout (default: 1200)")
    p.add_argument(
        "--providers",
        default="codex,claude",
        help="Comma-separated providers to run (codex,claude)",
    )
    p.add_argument(
        "--apply",
        action="store_true",
        help="Apply merged deltas back to data/events/<character>.jsonl",
    )
    return p.parse_args()


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    events: list[dict[str, Any]] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        s = line.strip()
        if not s:
            continue
        events.append(json.loads(s))
    return events


def chunk_events(events: list[dict[str, Any]], batches: int) -> list[list[dict[str, Any]]]:
    batches = max(1, min(batches, len(events)))
    size = math.ceil(len(events) / batches)
    out: list[list[dict[str, Any]]] = []
    for i in range(0, len(events), size):
        out.append(events[i : i + size])
    return out


def build_expected_keys(events: list[dict[str, Any]]) -> list[tuple[str, int]]:
    keys: list[tuple[str, int]] = []
    for ev in events:
        eid = ev["id"]
        for i, _ in enumerate(ev.get("choices", [])):
            keys.append((eid, i))
    return keys


def normalize_updates(
    payload: dict[str, Any], expected: set[tuple[str, int]]
) -> tuple[list[dict[str, Any]], list[str]]:
    errs: list[str] = []
    updates = payload.get("updates")
    if not isinstance(updates, list):
        return [], ["missing updates[]"]

    seen: set[tuple[str, int]] = set()
    out: list[dict[str, Any]] = []
    for idx, u in enumerate(updates):
        try:
            eid = str(u["id"])
            ci = int(u["choice_index"])
            d = dict(u["delta"])
            key = (eid, ci)
        except Exception:
            errs.append(f"bad update at index {idx}")
            continue
        if key in seen:
            errs.append(f"duplicate key {key}")
            continue
        if key not in expected:
            errs.append(f"unknown key {key}")
            continue
        try:
            nd = {
                "money": int(d["money"]),
                "sanity": clamp(int(d["sanity"]), -14, 14),
                "audience": clamp(int(d["audience"]), -14, 14),
                "ai_skill": clamp(int(d["ai_skill"]), -14, 14),
                "friends": clamp(int(d["friends"]), -14, 14),
            }
        except Exception:
            errs.append(f"bad delta for {key}")
            continue
        out.append({"id": eid, "choice_index": ci, "delta": nd})
        seen.add(key)

    missing = expected - seen
    if missing:
        errs.append(f"missing {len(missing)} updates")
    return out, errs


def clamp(v: int, lo: int, hi: int) -> int:
    return max(lo, min(hi, v))


def provider_cmd(
    provider: str,
    repo_root: Path,
    prompt: str,
    schema_path: Path,
    out_path: Path,
    args: argparse.Namespace,
) -> list[str]:
    if provider == "codex":
        return [
            "codex",
            "exec",
            "--ephemeral",
            "-C",
            str(repo_root),
            "--skip-git-repo-check",
            "-s",
            "read-only",
            "-c",
            'approval_policy="never"',
            "-c",
            f'model_reasoning_effort="{args.codex_reasoning}"',
            "--model",
            args.codex_model,
            "--output-schema",
            str(schema_path),
            "--output-last-message",
            str(out_path),
            prompt,
        ]
    if provider == "claude":
        return [
            "claude",
            "-p",
            "--output-format",
            "json",
            "--no-session-persistence",
            "--permission-mode",
            "dontAsk",
            "--system-prompt",
            "Return ONLY structured JSON output matching the provided schema. No markdown. No tools.",
            "--tools",
            "",
            "--json-schema",
            json.dumps(SCHEMA_OBJ),
            "--model",
            args.claude_model,
            "--effort",
            args.claude_effort,
            prompt,
        ]
    raise ValueError(f"unknown provider: {provider}")


def run_one(
    provider: str,
    batch_idx: int,
    batch_events: list[dict[str, Any]],
    repo_root: Path,
    out_dir: Path,
    schema_path: Path,
    args: argparse.Namespace,
) -> TaskResult:
    prompt = PROMPT_PREFIX + json.dumps(batch_events, ensure_ascii=False)
    out_path = out_dir / f"{provider}.batch{batch_idx}.json"
    log_path = out_dir / f"{provider}.batch{batch_idx}.log"
    cmd = provider_cmd(provider, repo_root, prompt, schema_path, out_path, args)

    start = time.time()
    try:
        proc = subprocess.run(
            cmd,
            cwd=repo_root,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=args.timeout_sec,
            check=False,
            text=True,
            encoding="utf-8",
            errors="replace",
        )
    except subprocess.TimeoutExpired:
        return TaskResult(
            provider=provider,
            batch_idx=batch_idx,
            ok=False,
            elapsed_s=float(args.timeout_sec),
            updates=[],
            error="timeout",
            out_path=out_path,
            log_path=log_path,
        )

    elapsed_s = time.time() - start
    log_path.write_text(
        f"exit={proc.returncode}\n\nSTDOUT:\n{proc.stdout}\n\nSTDERR:\n{proc.stderr}\n",
        encoding="utf-8",
    )
    if proc.returncode != 0:
        return TaskResult(
            provider=provider,
            batch_idx=batch_idx,
            ok=False,
            elapsed_s=elapsed_s,
            updates=[],
            error=f"exit={proc.returncode}",
            out_path=out_path,
            log_path=log_path,
        )

    try:
        if provider == "codex":
            payload = json.loads(out_path.read_text(encoding="utf-8"))
        else:
            env = json.loads(proc.stdout)
            payload = env.get("structured_output")
            if payload is None:
                return TaskResult(
                    provider=provider,
                    batch_idx=batch_idx,
                    ok=False,
                    elapsed_s=elapsed_s,
                    updates=[],
                    error="missing structured_output",
                    out_path=out_path,
                    log_path=log_path,
                )
            out_path.write_text(json.dumps(payload, indent=2), encoding="utf-8")
    except Exception as e:  # noqa: BLE001
        return TaskResult(
            provider=provider,
            batch_idx=batch_idx,
            ok=False,
            elapsed_s=elapsed_s,
            updates=[],
            error=f"parse error: {e}",
            out_path=out_path,
            log_path=log_path,
        )

    expected = set(build_expected_keys(batch_events))
    updates, errs = normalize_updates(payload, expected)
    if errs:
        return TaskResult(
            provider=provider,
            batch_idx=batch_idx,
            ok=False,
            elapsed_s=elapsed_s,
            updates=updates,
            error="; ".join(errs),
            out_path=out_path,
            log_path=log_path,
        )

    return TaskResult(
        provider=provider,
        batch_idx=batch_idx,
        ok=True,
        elapsed_s=elapsed_s,
        updates=updates,
        error=None,
        out_path=out_path,
        log_path=log_path,
    )


def collate_updates(
    events: list[dict[str, Any]],
    results: list[TaskResult],
) -> list[dict[str, Any]]:
    base: dict[tuple[str, int], dict[str, int]] = {}
    order: list[tuple[str, int]] = []
    for ev in events:
        eid = ev["id"]
        for i, ch in enumerate(ev.get("choices", [])):
            key = (eid, i)
            order.append(key)
            base[key] = {
                "money": int(ch["delta"]["money"]),
                "sanity": int(ch["delta"]["sanity"]),
                "audience": int(ch["delta"]["audience"]),
                "ai_skill": int(ch["delta"]["ai_skill"]),
                "friends": int(ch["delta"]["friends"]),
            }

    cand: dict[tuple[str, int], list[dict[str, int]]] = {k: [] for k in order}
    for r in results:
        if not r.ok:
            continue
        for u in r.updates:
            k = (u["id"], int(u["choice_index"]))
            if k in cand:
                cand[k].append(u["delta"])

    merged: list[dict[str, Any]] = []
    for key in order:
        eid, ci = key
        cands = cand[key]
        b = base[key]
        if not cands:
            m = dict(b)
        else:
            # Keep money from source by default as requested.
            m = {"money": b["money"]}
            for stat in STATS:
                vals = [int(d[stat]) for d in cands]
                if len(vals) == 1:
                    v = vals[0]
                else:
                    v = int(round(sum(vals) / len(vals)))
                m[stat] = clamp(v, -14, 14)
        merged.append({"id": eid, "choice_index": ci, "delta": m})
    return merged


def phase_stats(events: list[dict[str, Any]], updates_map: dict[tuple[str, int], dict[str, int]]) -> dict[str, float]:
    phase_vals: dict[str, list[int]] = {p: [] for p in PHASE_ORDER}
    for ev in events:
        phase = ev.get("phase")
        if phase not in phase_vals:
            continue
        eid = ev["id"]
        for i, _ in enumerate(ev.get("choices", [])):
            d = updates_map[(eid, i)]
            mag = sum(abs(int(d[s])) for s in STATS)
            phase_vals[phase].append(mag)
    out: dict[str, float] = {}
    for p in PHASE_ORDER:
        vals = phase_vals[p]
        out[p] = round(statistics.mean(vals), 3) if vals else 0.0
    return out


def main() -> int:
    args = parse_args()
    repo_root = Path(args.repo_root).resolve()
    char_path = repo_root / "data" / "events" / f"{args.character}.jsonl"
    if not char_path.exists():
        print(f"Character file not found: {char_path}", file=sys.stderr)
        return 2

    events = read_jsonl(char_path)
    if not events:
        print("No events loaded", file=sys.stderr)
        return 2

    providers = [p.strip() for p in args.providers.split(",") if p.strip()]
    bad = [p for p in providers if p not in {"codex", "claude"}]
    if bad:
        print(f"Unsupported providers: {bad}", file=sys.stderr)
        return 2

    out_dir = Path(args.out_dir) if args.out_dir else Path(
        tempfile.mkdtemp(prefix=f"pu-llm-batch-{args.character}-")
    )
    out_dir.mkdir(parents=True, exist_ok=True)
    schema_path = out_dir / "rebalance.schema.json"
    schema_path.write_text(json.dumps(SCHEMA_OBJ, indent=2), encoding="utf-8")

    batches = chunk_events(events, args.batches)
    print(
        f"Running {len(batches)} batches for {args.character}: "
        f"{len(events)} events, providers={providers}, out_dir={out_dir}"
    )

    futures = []
    results: list[TaskResult] = []
    with ThreadPoolExecutor(max_workers=max(1, args.max_workers)) as ex:
        for i, batch_events in enumerate(batches):
            for p in providers:
                futures.append(
                    ex.submit(run_one, p, i, batch_events, repo_root, out_dir, schema_path, args)
                )
        for f in as_completed(futures):
            r = f.result()
            results.append(r)
            if r.ok:
                print(
                    f"[ok] {r.provider} batch={r.batch_idx} "
                    f"updates={len(r.updates)} elapsed={r.elapsed_s:.1f}s"
                )
            else:
                print(
                    f"[err] {r.provider} batch={r.batch_idx} "
                    f"elapsed={r.elapsed_s:.1f}s error={r.error}"
                )

    merged = collate_updates(events, results)
    merged_path = out_dir / f"{args.character}.merged.updates.json"
    merged_path.write_text(json.dumps({"updates": merged}, indent=2), encoding="utf-8")

    # Produce candidate coverage summary.
    expected_total = sum(len(ev.get("choices", [])) for ev in events)
    ok_results = [r for r in results if r.ok]
    produced = sum(len(r.updates) for r in ok_results)
    print(
        f"\nCoverage: expected choices={expected_total}, "
        f"successful provider-updates={produced}"
    )

    updates_map = {(u["id"], int(u["choice_index"])): u["delta"] for u in merged}
    pstats = phase_stats(events, updates_map)
    print("Phase intensity (avg abs non-money sum):")
    for p in PHASE_ORDER:
        print(f"  {p:8s}: {pstats[p]:.3f}")

    monotonic = all(
        pstats[PHASE_ORDER[i]] <= pstats[PHASE_ORDER[i + 1]]
        for i in range(len(PHASE_ORDER) - 1)
    )
    print(f"Monotonic Early<=Mid<=Late<=Endgame: {monotonic}")

    if args.apply:
        merged_lookup = {(u["id"], int(u["choice_index"])): u["delta"] for u in merged}
        lines = []
        for ev in events:
            eid = ev["id"]
            for i, _ in enumerate(ev.get("choices", [])):
                ev["choices"][i]["delta"] = merged_lookup[(eid, i)]
            lines.append(json.dumps(ev, ensure_ascii=False))
        char_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
        print(f"Applied merged deltas to: {char_path}")

    print(f"Merged updates file: {merged_path}")
    print("Batch logs and raw outputs are in:", out_dir)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
