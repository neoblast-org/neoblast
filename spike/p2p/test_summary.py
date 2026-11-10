#!/usr/bin/env python3
"""Tests for summary.py on made-up records (criterion 5).

    python3 -I spike/p2p/test_summary.py
"""

import json
import os
import sys
import tempfile
import unittest
from fractions import Fraction

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import summary  # noqa: E402


def pair(name, lib, direct, relayed=None, failed=0, run=1, ip="v4", total=50, start=1):
    """Attempts start..start+total-1 of one library on one pair: `direct`
    direct, `failed` failed, the rest relayed."""
    if relayed is None:
        relayed = total - direct - failed
    assert direct + relayed + failed == total
    outcomes = ["direct"] * direct + ["relayed"] * relayed + ["failed"] * failed
    return [
        {"run": run, "pair": name, "ip": ip, "library": lib, "attempt": start + i, "try": 1,
         "outcome": o, "direct_10": o == "direct", "direct_30": o == "direct", "direct_60": o == "direct",
         "first_direct_ms": 1000 + i if o == "direct" else None}
        for i, o in enumerate(outcomes)
    ]


def both(name, libp2p, iroh, **kw):
    return pair(name, "libp2p", libp2p, **kw) + pair(name, "iroh", iroh, **kw)


class Outcome(unittest.TestCase):
    def test_iroh_wins_by_more_than_five_points(self):
        d = summary.decide(both("home1-cgnat", 20, 24))  # 40% against 48%
        self.assertEqual(d.chosen, "iroh")

    def test_libp2p_wins_by_more_than_five_points(self):
        d = summary.decide(both("home1-cgnat", 24, 20))
        self.assertEqual(d.chosen, "libp2p")

    def test_a_gap_under_five_points_is_a_tie_and_libp2p_wins(self):
        # The same as the iroh win but two attempts closer: 4 points.
        d = summary.decide(both("home1-cgnat", 20, 22))
        self.assertEqual(d.chosen, "libp2p")
        self.assertIn("tie", d.reason)

    def test_a_gap_of_exactly_five_points_is_not_a_tie(self):
        # Pairs 4 and 6 points apart: a mean gap of exactly 5.0.
        recs = both("home1-cgnat", 20, 22) + both("home1-home2", 20, 23)
        d = summary.decide(recs)
        self.assertEqual(d.rates["iroh"] - d.rates["libp2p"], Fraction(5, 100))
        self.assertEqual(d.chosen, "iroh")

    def test_just_under_five_points_over_two_pairs_is_a_tie(self):
        # The case above with one fewer direct attempt for iroh: 4.0 points.
        recs = both("home1-cgnat", 20, 22) + both("home1-home2", 20, 22)
        self.assertEqual(summary.decide(recs).chosen, "libp2p")


class Counting(unittest.TestCase):
    def test_ipv6_records_alone_do_not_make_a_pair_incomplete(self):
        d = summary.decide(both("home1-cgnat", 20, 20) + both("home1-home2", 5, 5, total=10, ip="dual"))
        self.assertEqual(d.incomplete_pairs, [])

    def test_a_failed_attempt_counts_against_the_rate(self):
        d = summary.decide(pair("home1-cgnat", "libp2p", 25, failed=5) + pair("home1-cgnat", "iroh", 25))
        self.assertEqual(d.pair_rates[("home1-cgnat", "libp2p")], Fraction(25, 50))
        self.assertEqual(d.also["libp2p"]["failed"], Fraction(5, 50))

    def test_a_harness_error_does_not_count_and_is_reported(self):
        recs = both("home1-cgnat", 25, 25)
        recs.append({"run": 1, "pair": "home1-cgnat", "ip": "v4", "library": "iroh", "attempt": 3,
                     "try": 2, "outcome": "harness-error", "reason": "relay-after"})
        d = summary.decide(recs)
        self.assertEqual(d.rates["iroh"], Fraction(25, 50))
        self.assertEqual(d.harness_errors, {"libp2p": 0, "iroh": 1})

    def test_an_incomplete_pair_does_not_count(self):
        # Pooling its 30 attempts (all direct for iroh) would change the
        # result; leaving the pair out keeps libp2p ahead.
        recs = both("home1-cgnat", 30, 20) + both("home1-home2", 0, 30, total=30)
        d = summary.decide(recs)
        self.assertEqual(d.complete_pairs, ["home1-cgnat"])
        self.assertEqual(d.incomplete_pairs, ["home1-home2"])
        self.assertEqual(d.chosen, "libp2p")

    def test_a_pair_complete_for_one_library_only_does_not_count(self):
        recs = both("home1-cgnat", 30, 20) + pair("home1-home2", "libp2p", 0) + pair(
            "home1-home2", "iroh", 49, total=49)
        self.assertEqual(summary.decide(recs).complete_pairs, ["home1-cgnat"])

    def test_one_decisive_pair_gives_that_pairs_rates(self):
        d = summary.decide(both("home1-home2", 10, 40))
        self.assertEqual(d.rates, {"libp2p": Fraction(10, 50), "iroh": Fraction(40, 50)})

    def test_two_decisive_pairs_count_equally(self):
        d = summary.decide(both("home1-cgnat", 0, 10) + both("home1-home2", 50, 30))
        self.assertEqual(d.rates["libp2p"], Fraction(1, 2))
        self.assertEqual(d.rates["iroh"], Fraction(2, 5))
        self.assertEqual(d.chosen, "libp2p")

    def test_direct_at_10_but_relayed_at_30_counts_as_relayed(self):
        recs = both("home1-cgnat", 25, 25)
        for r in recs:
            if r["library"] == "iroh" and r["outcome"] == "direct":
                r.update(outcome="relayed", direct_30=False)  # direct_10 stays True
                break
        d = summary.decide(recs)
        self.assertEqual(d.rates["iroh"], Fraction(24, 50))
        self.assertEqual(d.also["iroh"]["direct_10"], Fraction(25, 50))

    def test_controls_and_ipv6_runs_do_not_count(self):
        recs = both("home1-cgnat", 20, 20) + both("home1-vps", 0, 20, total=20) + both(
            "home1-cgnat", 0, 10, total=10, ip="dual")
        d = summary.decide(recs)
        self.assertEqual(d.rates["iroh"], d.rates["libp2p"])
        self.assertEqual(d.controls[("home1-vps", 1, "iroh")], (20, 20))
        self.assertEqual(d.ipv6[("home1-cgnat", "iroh")], (10, 10))

    def test_a_control_with_three_failures_does_not_pass(self):
        recs = both("home1-vps", 20, 20, total=20) + pair("cgnat-vps", "iroh", 17, failed=3, total=20)
        text = summary.report(summary.decide(recs))
        self.assertIn("control cgnat-vps run 1 iroh: 17/20 connected, does not pass", text)
        self.assertIn("control home1-vps run 1 iroh: 20/20 connected, passes", text)


class MaintainersRules(unittest.TestCase):
    def test_no_complete_decisive_pair_leaves_item_6_open(self):
        d = summary.decide(both("home1-vps", 20, 20, total=20))
        self.assertIsNone(d.chosen)
        self.assertIn("13", d.reason)

    def test_a_library_not_completed_hands_it_to_the_other(self):
        d = summary.decide(both("home1-cgnat", 10, 40), not_completed=["iroh"])
        self.assertEqual(d.chosen, "libp2p")
        self.assertIn("rule 11", d.reason)

    def test_a_veto_hands_it_to_the_other_whatever_the_rates(self):
        d = summary.decide(both("home1-cgnat", 40, 10), vetoed=["libp2p"])
        self.assertEqual(d.chosen, "iroh")
        self.assertIn("rule 12", d.reason)

    def test_one_vetoed_and_the_other_not_completed_leaves_item_6_open(self):
        d = summary.decide(both("home1-cgnat", 40, 10), vetoed=["libp2p"], not_completed=["iroh"])
        self.assertIsNone(d.chosen)

    def test_with_no_records_a_declaration_still_applies(self):
        self.assertEqual(summary.decide([], not_completed=["libp2p"]).chosen, "iroh")


class Refusals(unittest.TestCase):
    def test_two_counted_records_for_one_attempt_are_refused(self):
        recs = both("home1-cgnat", 25, 25)
        recs.append(dict(recs[0]))
        with self.assertRaises(summary.RecordError):
            summary.decide(recs)

    def test_an_attempt_past_50_is_refused(self):
        recs = both("home1-cgnat", 25, 25) + pair("home1-cgnat", "iroh", 1, total=1, start=51)
        with self.assertRaises(summary.RecordError):
            summary.decide(recs)

    def test_a_pair_complete_in_two_runs_is_refused(self):
        with self.assertRaises(summary.RecordError):
            summary.decide(both("home1-cgnat", 25, 25) + both("home1-cgnat", 25, 25, run=2))

    def test_an_unknown_outcome_is_refused_on_loading(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = os.path.join(tmp, "r.jsonl")
            with open(path, "w", encoding="utf-8") as f:
                rec = pair("home1-cgnat", "iroh", 1, total=1)[0]
                rec["outcome"] = "mixed"
                f.write(json.dumps(rec) + "\n")
            with self.assertRaises(summary.RecordError):
                summary.load([path])


class Command(unittest.TestCase):
    def test_main_reads_files_and_prints_the_outcome(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = os.path.join(tmp, "run1-home1-cgnat-v4.jsonl")
            with open(path, "w", encoding="utf-8") as f:
                for r in both("home1-cgnat", 20, 24):
                    f.write(json.dumps(r) + "\n")
            out = os.path.join(tmp, "out.txt")
            saved = sys.stdout
            try:
                with open(out, "w", encoding="utf-8") as sys.stdout:
                    code = summary.main([path])
            finally:
                sys.stdout = saved
            with open(out, encoding="utf-8") as f:
                text = f.read()
        self.assertEqual(code, 0)
        self.assertIn("home1-cgnat  iroh    24/50 direct  48.00%", text)
        self.assertIn("outcome: iroh:", text)


if __name__ == "__main__":
    unittest.main()
