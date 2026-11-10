//! What the two spike programs share: an attempt's timings, and how B's
//! echoes become its result, by rules 2 and 3 of
//! `plans/todo/p2p-library-spike.md`.
//!
//! Each program decides only whether an echo that came back went direct;
//! everything after that is here, so both are judged by the same code.

use std::time::{Duration, Instant};

/// When an attempt's echoes are sent and its readings taken.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Timings {
    /// Rule 2: an echo must come back within this long of B starting.
    pub connect: Duration,
    /// Rule 1: B sends an echo this often.
    pub echo_every: Duration,
    /// An echo not back within this long is lost.
    pub echo_timeout: Duration,
    /// Readings, after the first echo came back: at 10 and 60 seconds
    /// (criterion 4) and rule 3's at 30, in that order.
    pub readings: [Duration; 3],
    /// A reading at `r` is met by a direct echo that comes back between
    /// `r` and `r + window`. It is the echo timeout, so an echo sent at the
    /// reading counts.
    pub window: Duration,
}

impl Timings {
    /// The rule's timings, for real runs.
    pub const RULE: Timings = Timings {
        connect: Duration::from_secs(30),
        echo_every: Duration::from_secs(1),
        echo_timeout: Duration::from_secs(5),
        readings: [
            Duration::from_secs(10),
            Duration::from_secs(30),
            Duration::from_secs(60),
        ],
        window: Duration::from_secs(5),
    };

    /// Shorter, for the tests on one machine. The same shape: readings
    /// well apart, and an echo every quarter of the window.
    pub const TEST: Timings = Timings {
        connect: Duration::from_secs(15),
        echo_every: Duration::from_millis(250),
        echo_timeout: Duration::from_secs(2),
        readings: [
            Duration::from_secs(3),
            Duration::from_secs(6),
            Duration::from_secs(9),
        ],
        window: Duration::from_secs(2),
    };
}

/// One echo that came back.
#[derive(Clone, Copy, Debug)]
struct Echo {
    at: Instant,
    direct: bool,
    relay_open: bool,
}

/// B's view of one attempt, from its start.
#[derive(Debug)]
pub struct Attempt {
    timings: Timings,
    start: Instant,
    echoes: Vec<Echo>,
}

/// Rule 3's outcome for an attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Direct at rule 3's reading.
    Direct,
    /// Connected, but not direct at rule 3's reading.
    Relayed,
    /// No echo came back in time (rule 2).
    Failed,
}

impl Outcome {
    /// The name in the records.
    pub fn name(self) -> &'static str {
        match self {
            Outcome::Direct => "direct",
            Outcome::Relayed => "relayed",
            Outcome::Failed => "failed",
        }
    }
}

/// What B reports when the attempt ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Report {
    /// Rule 3's outcome.
    pub outcome: Outcome,
    /// From B's start to the first echo back, if one came in time.
    pub connect: Option<Duration>,
    /// From B's start to the first direct echo back, if any.
    pub first_direct: Option<Duration>,
    /// Whether each reading was met: 10, 30 and 60 seconds.
    pub direct_at: [bool; 3],
    /// Whether any echo, direct or relayed, came back in each reading's
    /// window: the connection was still up.
    pub alive_at: [bool; 3],
    /// For the echo that met rule 3's reading: whether a relayed path
    /// was still open beside the direct one.
    pub relay_open_at_30: Option<bool>,
}

impl Attempt {
    /// An attempt starting now, as B starts.
    pub fn new(timings: Timings) -> Self {
        Self::starting_at(timings, Instant::now())
    }

    /// An attempt that started at `start`.
    pub fn starting_at(timings: Timings, start: Instant) -> Self {
        Attempt {
            timings,
            start,
            echoes: Vec::new(),
        }
    }

    /// The attempt's timings.
    pub fn timings(&self) -> Timings {
        self.timings
    }

    /// Records an echo that came back at `at`: `direct` if it went without
    /// the relay, `relay_open` if a relayed path was open beside it.
    pub fn echo(&mut self, at: Instant, direct: bool, relay_open: bool) {
        self.echoes.push(Echo {
            at,
            direct,
            relay_open,
        });
    }

    /// The first echo back, if it came within rule 2's time.
    fn first(&self) -> Option<Instant> {
        let limit = self.start + self.timings.connect;
        self.echoes
            .iter()
            .map(|e| e.at)
            .filter(|&at| at <= limit)
            .min()
    }

    /// When B stops: rule 2's limit if nothing has come back, else just
    /// after the last reading's window.
    pub fn deadline(&self) -> Instant {
        match self.first() {
            None => self.start + self.timings.connect,
            Some(first) => first + self.timings.readings[2] + self.timings.window,
        }
    }

    /// The first echo in reading `i`'s window, direct ones only if `direct`.
    fn in_window(&self, first: Instant, i: usize, direct: bool) -> Option<Echo> {
        let from = first + self.timings.readings[i];
        let to = from + self.timings.window;
        self.echoes
            .iter()
            .filter(|e| (e.direct || !direct) && e.at >= from && e.at <= to)
            .min_by_key(|e| e.at)
            .copied()
    }

    /// The direct echo that meets reading `i`, if any.
    fn meeting(&self, first: Instant, i: usize) -> Option<Echo> {
        self.in_window(first, i, true)
    }

    /// The attempt's result, from the echoes recorded up to its deadline.
    pub fn report(&self) -> Report {
        let Some(first) = self.first() else {
            return Report {
                outcome: Outcome::Failed,
                connect: None,
                first_direct: None,
                direct_at: [false; 3],
                alive_at: [false; 3],
                relay_open_at_30: None,
            };
        };
        let deadline = self.deadline();
        let at_30 = self.meeting(first, 1);
        let direct_at = [0, 1, 2].map(|i| self.meeting(first, i).is_some());
        let alive_at = [0, 1, 2].map(|i| self.in_window(first, i, false).is_some());
        let first_direct = self
            .echoes
            .iter()
            .filter(|e| e.direct && e.at <= deadline)
            .map(|e| e.at)
            .min();
        Report {
            outcome: if at_30.is_some() {
                Outcome::Direct
            } else {
                Outcome::Relayed
            },
            connect: Some(first - self.start),
            first_direct: first_direct.map(|at| at - self.start),
            direct_at,
            alive_at,
            relay_open_at_30: at_30.map(|e| e.relay_open),
        }
    }
}

impl Report {
    /// The report as one JSON line, for `run.sh` to complete into a record.
    pub fn to_json(&self) -> String {
        let ms = |d: Option<Duration>| d.map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
        serde_json::json!({
            "outcome": self.outcome.name(),
            "connect_ms": ms(self.connect),
            "first_direct_ms": ms(self.first_direct),
            "direct_10": self.direct_at[0],
            "direct_30": self.direct_at[1],
            "direct_60": self.direct_at[2],
            "alive_10": self.alive_at[0],
            "alive_30": self.alive_at[1],
            "alive_60": self.alive_at[2],
            "relay_open_30": self.relay_open_at_30,
        })
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: Timings = Timings::RULE;

    fn s(secs: f64) -> Duration {
        Duration::from_secs_f64(secs)
    }

    /// An attempt with echoes at the given seconds after its start.
    fn attempt(echoes: &[(f64, bool, bool)]) -> Report {
        let start = Instant::now();
        let mut a = Attempt::starting_at(T, start);
        for &(at, direct, relay_open) in echoes {
            a.echo(start + s(at), direct, relay_open);
        }
        a.report()
    }

    #[test]
    fn no_echo_is_failed() {
        let r = attempt(&[]);
        assert_eq!(r.outcome, Outcome::Failed);
        assert_eq!(r.connect, None);
    }

    #[test]
    fn an_echo_after_30_seconds_is_failed() {
        let r = attempt(&[(30.001, true, false), (45.0, true, false)]);
        assert_eq!(r.outcome, Outcome::Failed);
    }

    #[test]
    fn an_echo_at_30_seconds_connects() {
        let r = attempt(&[(30.0, false, true)]);
        assert_eq!(r.outcome, Outcome::Relayed);
        assert_eq!(r.connect, Some(s(30.0)));
    }

    #[test]
    fn relayed_echoes_only_are_relayed() {
        let echoes: Vec<_> = (2..80).map(|i| (f64::from(i), false, true)).collect();
        let r = attempt(&echoes);
        assert_eq!(r.outcome, Outcome::Relayed);
        assert_eq!(r.first_direct, None);
        assert_eq!(r.direct_at, [false; 3]);
        assert_eq!(r.alive_at, [true; 3]);
    }

    #[test]
    fn a_connection_that_dies_is_not_alive_at_later_readings() {
        let r = attempt(&[(2.0, false, true), (12.0, false, true)]);
        assert_eq!(r.outcome, Outcome::Relayed);
        assert_eq!(r.alive_at, [true, false, false]);
    }

    #[test]
    fn direct_at_the_reading_is_direct() {
        // First echo at 2 s, so rule 3's reading is at 32 s.
        let r = attempt(&[(2.0, false, true), (33.0, true, true)]);
        assert_eq!(r.outcome, Outcome::Direct);
        assert_eq!(r.direct_at, [false, true, false]);
        assert_eq!(r.first_direct, Some(s(33.0)));
        assert_eq!(r.relay_open_at_30, Some(true));
    }

    #[test]
    fn the_reading_window_is_inclusive_at_both_ends() {
        assert_eq!(
            attempt(&[(2.0, false, true), (32.0, true, false)]).outcome,
            Outcome::Direct
        );
        assert_eq!(
            attempt(&[(2.0, false, true), (37.0, true, false)]).outcome,
            Outcome::Direct
        );
    }

    #[test]
    fn a_direct_echo_outside_the_window_misses_the_reading() {
        assert_eq!(
            attempt(&[(2.0, false, true), (31.99, true, false)]).outcome,
            Outcome::Relayed
        );
        assert_eq!(
            attempt(&[(2.0, false, true), (37.01, true, false)]).outcome,
            Outcome::Relayed
        );
    }

    #[test]
    fn direct_at_10_but_not_at_30_is_relayed() {
        let r = attempt(&[(2.0, false, true), (12.0, true, true), (32.0, false, true)]);
        assert_eq!(r.outcome, Outcome::Relayed);
        assert_eq!(r.direct_at, [true, false, false]);
        assert_eq!(r.first_direct, Some(s(12.0)));
        assert_eq!(r.relay_open_at_30, None);
    }

    #[test]
    fn a_relayed_echo_at_the_reading_does_not_undo_a_direct_one() {
        // Rule 3: a relayed path still open beside the direct one does not matter.
        let r = attempt(&[(2.0, false, true), (32.5, false, true), (33.0, true, true)]);
        assert_eq!(r.outcome, Outcome::Direct);
    }

    #[test]
    fn each_reading_is_counted_from_the_first_echo() {
        let r = attempt(&[
            (5.0, false, true),
            (15.0, true, false),
            (35.0, true, false),
            (65.0, true, false),
        ]);
        assert_eq!(r.direct_at, [true, true, true]);
        assert_eq!(r.relay_open_at_30, Some(false));
    }

    #[test]
    fn the_deadline_follows_the_first_echo() {
        let start = Instant::now();
        let mut a = Attempt::starting_at(T, start);
        assert_eq!(a.deadline(), start + s(30.0));
        a.echo(start + s(4.0), false, true);
        assert_eq!(a.deadline(), start + s(4.0 + 60.0 + 5.0));
    }

    #[test]
    fn echoes_after_the_deadline_are_ignored() {
        let r = attempt(&[(2.0, false, true), (67.01, true, false)]);
        assert_eq!(r.first_direct, None);
    }

    #[test]
    fn the_json_line_has_the_record_fields() {
        let r = attempt(&[(2.0, false, true), (33.0, true, true)]);
        let v: serde_json::Value = serde_json::from_str(&r.to_json()).unwrap();
        assert_eq!(v["outcome"], "direct");
        assert_eq!(v["connect_ms"], 2000);
        assert_eq!(v["first_direct_ms"], 33000);
        assert_eq!(v["direct_30"], true);
        assert_eq!(v["relay_open_30"], true);
        let failed: serde_json::Value = serde_json::from_str(&attempt(&[]).to_json()).unwrap();
        assert_eq!(failed["connect_ms"], serde_json::Value::Null);
    }
}
