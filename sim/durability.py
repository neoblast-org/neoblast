#!/usr/bin/env python3
"""Durability model for choosing k/n (open item 1).

Probability that a file of `segments` segments is fully readable at a random
moment, if each shard holder is online independently with probability p.

    P(segment) = sum_{i=k}^{n} C(n,i) p^i (1-p)^(n-i)
    P(file)    = P(segment) ** segments

Usage:
    python3 sim/durability.py                 # table for the candidate schemes
    python3 sim/durability.py --target 0.999  # uptime each scheme needs

This assumes independent failures. Correlated outages (one provider going
down) are what placement rules and scenario H1 address; the Monte Carlo and
Shadow runs replace this model once real uptime traces exist.
"""
from __future__ import annotations

import argparse
from math import comb

SCHEMES = [(10, 16), (10, 20), (10, 25), (20, 50), (10, 30)]


def p_segment(k: int, n: int, p: float) -> float:
    return sum(comb(n, i) * p**i * (1 - p) ** (n - i) for i in range(k, n + 1))


def p_file(k: int, n: int, p: float, segments: int) -> float:
    return p_segment(k, n, p) ** segments


def needed_uptime(k: int, n: int, segments: int, target: float) -> float:
    lo, hi = 0.0, 1.0
    for _ in range(60):
        mid = (lo + hi) / 2
        if p_file(k, n, mid, segments) >= target:
            hi = mid
        else:
            lo = mid
    return hi


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--file-mib", type=float, default=1024, help="file size in MiB (default 1 GiB)")
    ap.add_argument("--segment-mib", type=float, default=1, help="segment size in MiB (default 1)")
    ap.add_argument("--target", type=float, default=0.999, help="required probability the whole file is readable")
    args = ap.parse_args()
    segments = max(1, round(args.file_mib / args.segment_mib))
    print(f"{segments} segments; target {args.target:.4%} of the time\n")
    print(f"{'scheme':>8} {'overhead':>9} {'uptime needed':>14}")
    for k, n in SCHEMES:
        need = needed_uptime(k, n, segments, args.target)
        print(f"{k:>3}/{n:<4} {n / k:>8.2f}x {need:>13.1%}")


if __name__ == "__main__":
    main()
