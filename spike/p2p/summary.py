#!/usr/bin/env python3
"""Applies ROADMAP item 6's rule, as plans/todo/p2p-library-spike.md fixes
it, to the spike's records alone, and prints each pair's rate, each
library's rate and the outcome.

    python3 spike/p2p/summary.py [--not-completed LIB] [--vetoed LIB] [records ...]

With no record files it reads spike/p2p/records/*.jsonl. Rule 11 (a library
not completed) and rule 12 (a veto) are the maintainer's to declare, so they
are given as options. Rates are exact fractions (rule 9), printed rounded.
"""

import argparse
import glob
import json
import os
import statistics
import sys
from dataclasses import dataclass, field
from fractions import Fraction

LIBRARIES = ("libp2p", "iroh")
DECISIVE = ("home1-cgnat", "home1-home2")  # rule 4
CONTROLS = ("home1-vps", "cgnat-vps")
ATTEMPTS = 50  # rule 8
CONTROL_ATTEMPTS, CONTROL_PASS = 20, 18  # rule 5
TIE = Fraction(5, 100)  # rule 9: under 5.0 points apart
OUTCOMES = ("direct", "relayed", "failed")
HARNESS = "harness-error"


class RecordError(ValueError):
    """The records are inconsistent; the summary refuses to guess."""


@dataclass
class Decision:
    """What the rule gives, and the figures it rests on."""

    chosen: str | None  # None: item 6 stays open
    reason: str
    rates: dict = field(default_factory=dict)  # library -> Fraction
    pair_rates: dict = field(default_factory=dict)  # (pair, library) -> Fraction
    complete_pairs: list = field(default_factory=list)
    incomplete_pairs: list = field(default_factory=list)
    harness_errors: dict = field(default_factory=dict)  # library -> count
    controls: dict = field(default_factory=dict)  # (pair, run, library) -> (connected, attempts)
    also: dict = field(default_factory=dict)  # library -> figures for criterion 6
    ipv6: dict = field(default_factory=dict)  # (pair, library) -> (direct, attempts)


def load(paths):
    """Every record in the given JSON-lines files."""
    records = []
    for path in paths:
        with open(path, encoding="utf-8") as f:
            for n, line in enumerate(f, 1):
                if not line.strip():
                    continue
                try:
                    r = json.loads(line)
                except json.JSONDecodeError as e:
                    raise RecordError(f"{path}:{n}: not JSON: {e}") from e
                for key in ("run", "pair", "ip", "library", "attempt", "outcome"):
                    if key not in r:
                        raise RecordError(f"{path}:{n}: no {key!r}")
                if r["library"] not in LIBRARIES:
                    raise RecordError(f"{path}:{n}: unknown library {r['library']!r}")
                if r["outcome"] not in OUTCOMES + (HARNESS,):
                    raise RecordError(f"{path}:{n}: unknown outcome {r['outcome']!r}")
                records.append(r)
    return records


def counted(records):
    """Attempts that count (rule 10), keyed by run, pair, IP, library and
    attempt; two for one key is an error."""
    out = {}
    for r in records:
        if r["outcome"] == HARNESS:
            continue
        key = (r["run"], r["pair"], r["ip"], r["library"], r["attempt"])
        if key in out:
            raise RecordError(f"two counted records for {key}")
        out[key] = r
    return out


def complete_runs(attempts, pair):
    """The runs of a decisive pair in which both libraries have exactly
    attempts 1 to 50 (rule 7)."""
    runs = {k[0] for k in attempts if k[1] == pair and k[2] == "v4"}
    complete = []
    for run in sorted(runs):
        done = True
        for lib in LIBRARIES:
            numbers = {k[4] for k in attempts if k[:4] == (run, pair, "v4", lib)}
            if numbers - set(range(1, ATTEMPTS + 1)):
                raise RecordError(f"run {run}, {pair}, {lib}: attempts outside 1 to {ATTEMPTS}")
            done = done and numbers == set(range(1, ATTEMPTS + 1))
        if done:
            complete.append(run)
    return complete


def median_ms(values):
    return statistics.median(values) if values else None


def decide(records, not_completed=(), vetoed=()):
    """Rules 4 to 13 applied to the records."""
    attempts = counted(records)
    d = Decision(chosen=None, reason="")
    for lib in LIBRARIES:
        d.harness_errors[lib] = sum(1 for r in records if r["outcome"] == HARNESS and r["library"] == lib)

    # Rule 5: the controls, reported; they do not count.
    for (run, pair, ip, lib, _), r in attempts.items():
        if pair in CONTROLS and ip == "v4":
            connected, total = d.controls.get((pair, run, lib), (0, 0))
            d.controls[(pair, run, lib)] = (connected + (r["outcome"] != "failed"), total + 1)

    # Rule 6: the IPv6 runs, reported; they do not count.
    for (run, pair, ip, lib, _), r in attempts.items():
        if ip == "dual":
            direct, total = d.ipv6.get((pair, lib), (0, 0))
            d.ipv6[(pair, lib)] = (direct + (r["outcome"] == "direct"), total + 1)

    # Rule 7: each complete decisive pair's rate, then their mean.
    chosen_runs = {}
    for pair in DECISIVE:
        runs = complete_runs(attempts, pair)
        if len(runs) > 1:
            raise RecordError(f"{pair} is complete in runs {runs}; keep the records of one")
        if runs:
            chosen_runs[pair] = runs[0]
            d.complete_pairs.append(pair)
        elif any(k[1] == pair and k[2] == "v4" for k in attempts):
            d.incomplete_pairs.append(pair)
    for pair, run in chosen_runs.items():
        for lib in LIBRARIES:
            direct = sum(
                1 for k, r in attempts.items() if k[:4] == (run, pair, "v4", lib) and r["outcome"] == "direct"
            )
            d.pair_rates[(pair, lib)] = Fraction(direct, ATTEMPTS)
    if d.complete_pairs:
        for lib in LIBRARIES:
            d.rates[lib] = sum(d.pair_rates[(p, lib)] for p in d.complete_pairs) / len(d.complete_pairs)

    # Criterion 6: the other figures, over the same attempts.
    for lib in LIBRARIES:
        rs = [r for k, r in attempts.items() if k[1] in chosen_runs and k[0] == chosen_runs[k[1]]
              and k[2] == "v4" and k[3] == lib]
        if rs:
            n = len(rs)
            d.also[lib] = {
                "relayed": Fraction(sum(r["outcome"] == "relayed" for r in rs), n),
                "failed": Fraction(sum(r["outcome"] == "failed" for r in rs), n),
                "direct_10": Fraction(sum(bool(r.get("direct_10")) for r in rs), n),
                "direct_60": Fraction(sum(bool(r.get("direct_60")) for r in rs), n),
                "median_first_direct_ms": median_ms(
                    [r["first_direct_ms"] for r in rs if r.get("first_direct_ms") is not None]
                ),
            }

    # Rules 11 and 12: the maintainer's declarations come first. Rule 13
    # cannot precede rule 11: a library not completed leaves every decisive
    # pair incomplete, so rule 11 would never apply (plan, step 3 note).
    # Rule 13 is the maintainer's finding that no decisive pair can be run;
    # without a declaration, no complete pair leaves item 6 open.
    out = set(not_completed) | set(vetoed)
    for lib in out:
        if lib not in LIBRARIES:
            raise RecordError(f"unknown library {lib!r}")
    if out == set(LIBRARIES):
        d.reason = "neither library can be chosen (rules 11 and 12): item 6 stays open"
        return d
    if out:
        (gone,) = out
        (d.chosen,) = set(LIBRARIES) - out
        rule = "11, not completed" if gone in not_completed else "12, vetoed"
        d.reason = f"{gone} is out (rule {rule}), so {d.chosen} is chosen"
        return d

    # Rule 13.
    if not d.complete_pairs:
        d.reason = "no decisive pair is complete (rules 7 and 13): item 6 stays open"
        return d

    # Rule 9.
    gap = d.rates["iroh"] - d.rates["libp2p"]
    if abs(gap) < TIE:
        d.chosen = "libp2p"
        d.reason = f"the rates are {points(abs(gap))} points apart, under 5.0: a tie, so libp2p (rule 9)"
    else:
        d.chosen = "iroh" if gap > 0 else "libp2p"
        d.reason = f"{d.chosen}'s rate is higher by {points(abs(gap))} points, 5.0 or more (rule 9)"
    return d


def points(f):
    """A fraction as percentage points, to two places."""
    return f"{float(f * 100):.2f}"


def report(d):
    lines = []
    for (pair, lib), rate in sorted(d.pair_rates.items()):
        lines.append(f"{pair:12} {lib:7} {rate.numerator * ATTEMPTS // rate.denominator:>2}/{ATTEMPTS} direct  {points(rate)}%")
    for pair in d.incomplete_pairs:
        lines.append(f"{pair:12} not complete: does not count (rule 7)")
    for lib, rate in d.rates.items():
        lines.append(f"{lib:7} direct-connection rate {points(rate)}% (exactly {rate})")
    for lib, n in d.harness_errors.items():
        lines.append(f"{lib:7} harness errors rerun: {n}")
    for (pair, run, lib), (connected, total) in sorted(d.controls.items()):
        ok = "passes" if total >= CONTROL_ATTEMPTS and connected >= CONTROL_PASS else "does not pass"
        lines.append(f"control {pair} run {run} {lib}: {connected}/{total} connected, {ok} (rule 5)")
    for (pair, lib), (direct, total) in sorted(d.ipv6.items()):
        lines.append(f"IPv6 {pair} {lib}: {direct}/{total} direct (does not count; rule 6)")
    for lib, a in d.also.items():
        med = "none" if a["median_first_direct_ms"] is None else f"{a['median_first_direct_ms']:.0f} ms"
        lines.append(
            f"{lib:7} relayed {points(a['relayed'])}%, failed {points(a['failed'])}%, "
            f"direct at 10 s {points(a['direct_10'])}%, at 60 s {points(a['direct_60'])}%, "
            f"median time to direct {med}"
        )
    lines.append(f"outcome: {d.chosen or 'none'}: {d.reason}")
    return "\n".join(lines)


def main(argv):
    here = os.path.dirname(os.path.abspath(__file__))
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--not-completed", action="append", default=[], choices=LIBRARIES)
    p.add_argument("--vetoed", action="append", default=[], choices=LIBRARIES)
    p.add_argument("records", nargs="*")
    a = p.parse_args(argv)
    paths = a.records or sorted(glob.glob(os.path.join(here, "records", "*.jsonl")))
    try:
        print(report(decide(load(paths), a.not_completed, a.vetoed)))
    except RecordError as e:
        print(f"summary.py: {e}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
