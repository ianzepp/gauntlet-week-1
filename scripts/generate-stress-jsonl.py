#!/usr/bin/env python3
"""
Generate NDJSON object lines for stress testing and stream to stdout.

Examples:
  python3 scripts/generate-stress-jsonl.py > stress-1k.jsonl
  python3 scripts/generate-stress-jsonl.py | cargo run -p cli -- ws object create --create-board
  python3 scripts/generate-stress-jsonl.py --pattern ring --count 100000 --seed 42 | \
    cargo run -p cli -- ws object create --board-id <board-id> --wait-for-ack false
  python3 scripts/generate-stress-jsonl.py --pattern spiral --count 50000 | \
    cargo run -p cli -- ws object create --create-board
"""

from __future__ import annotations

import argparse
import json
import math
import os
import random
import signal
import sys
from typing import Iterator


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Generate stress-test object JSONL to stdout")
    parser.add_argument("--count", type=int, default=1000, help="Number of objects to emit")
    parser.add_argument("--size", type=float, default=10.0, help="Object width/height in pixels")
    parser.add_argument("--start-x", type=float, default=0.0, help="Pattern center/start X")
    parser.add_argument("--start-y", type=float, default=0.0, help="Pattern center/start Y")
    parser.add_argument(
        "--pattern",
        choices=("snake", "square", "ring", "spiral"),
        default="snake",
        help="Coordinate pattern: snake rows, concentric square rings, circular rings, or outward square spiral",
    )
    parser.add_argument("--columns", type=int, default=40, help="Columns for snake pattern")
    parser.add_argument("--row-gap", type=float, default=10.0, help="Extra vertical spacing between snake rows")
    parser.add_argument("--seed", type=int, default=None, help="Optional random seed")
    return parser.parse_args()


def square_ring_points(step: float) -> Iterator[tuple[float, float]]:
    yield (0.0, 0.0)
    ring = 1
    while True:
        d = ring * step

        # Top edge (left -> right), includes top corners.
        for x in range(-ring, ring + 1):
            yield (x * step, -d)
        # Right edge (top+1 -> bottom), excludes top corner.
        for y in range(-ring + 1, ring + 1):
            yield (d, y * step)
        # Bottom edge (right-1 -> left), excludes bottom-right corner.
        for x in range(ring - 1, -ring - 1, -1):
            yield (x * step, d)
        # Left edge (bottom-1 -> top+1), excludes both corners.
        for y in range(ring - 1, -ring, -1):
            yield (-d, y * step)

        ring += 1


def circle_ring_points(step: float) -> Iterator[tuple[float, float]]:
    yield (0.0, 0.0)
    ring = 1
    while True:
        radius = ring * step
        samples = max(8 * ring, 8)
        for i in range(samples):
            theta = (2.0 * math.pi * i) / samples
            x = radius * math.cos(theta)
            y = radius * math.sin(theta)
            yield (round(x, 3), round(y, 3))
        ring += 1


def snake_points(step: float, columns: int, row_gap: float) -> Iterator[tuple[float, float]]:
    columns = max(columns, 1)
    row_step = step + row_gap
    row = 0
    while True:
        if row % 2 == 0:
            for col in range(columns):
                yield (col * step, row * row_step)
        else:
            for col in range(columns - 1, -1, -1):
                yield (col * step, row * row_step)
        row += 1


def spiral_points(step: float) -> Iterator[tuple[float, float]]:
    # Integer lattice square spiral centered at origin:
    # (0,0), (1,0), (1,1), (0,1), (-1,1), (-1,0), ...
    x = 0
    y = 0
    yield (0.0, 0.0)

    leg_len = 1
    while True:
        # Right
        for _ in range(leg_len):
            x += 1
            yield (x * step, y * step)
        # Up
        for _ in range(leg_len):
            y += 1
            yield (x * step, y * step)
        leg_len += 1
        # Left
        for _ in range(leg_len):
            x -= 1
            yield (x * step, y * step)
        # Down
        for _ in range(leg_len):
            y -= 1
            yield (x * step, y * step)
        leg_len += 1


def random_hex_color(rng: random.Random) -> str:
    return f"#{rng.randint(0, 255):02x}{rng.randint(0, 255):02x}{rng.randint(0, 255):02x}"


def darken_hex(color: str, factor: float) -> str:
    color = color.lstrip("#")
    r = int(color[0:2], 16)
    g = int(color[2:4], 16)
    b = int(color[4:6], 16)
    r = max(0, min(255, int(r * factor)))
    g = max(0, min(255, int(g * factor)))
    b = max(0, min(255, int(b * factor)))
    return f"#{r:02x}{g:02x}{b:02x}"


def main() -> int:
    signal.signal(signal.SIGPIPE, signal.SIG_DFL)
    args = parse_args()
    if args.count <= 0:
        return 0
    if args.size <= 0:
        raise SystemExit("--size must be > 0")

    rng = random.Random(args.seed)
    if args.pattern == "snake":
        points = snake_points(args.size, args.columns, args.row_gap)
    elif args.pattern == "square":
        points = square_ring_points(args.size)
    elif args.pattern == "spiral":
        points = spiral_points(args.size)
    else:
        points = circle_ring_points(args.size)

    for index in range(args.count):
        dx, dy = next(points)
        fill = random_hex_color(rng)
        border = darken_hex(fill, 0.72)
        kind = "ellipse" if index % 2 else "rectangle"

        obj = {
            "type": "object",
            "kind": kind,
            "x": args.start_x + dx,
            "y": args.start_y + dy,
            "width": args.size,
            "height": args.size,
            "rotation": 0.0,
            "z_index": index,
            "props": {
                "fill": fill,
                "stroke": border,
                "strokeWidth": 1,
            },
        }
        try:
            sys.stdout.write(json.dumps(obj, separators=(",", ":")))
            sys.stdout.write("\n")
        except BrokenPipeError:
            try:
                sys.stdout.close()
            finally:
                os._exit(0)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
