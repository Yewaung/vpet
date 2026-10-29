//! Being a considerate desktop companion: do-not-disturb (quiet hours, and
//! staying quiet while a fullscreen app, game or presentation runs), reduced
//! motion, reminders to rest, and the ways it can get in the way on the
//! desktop (staying on top, catching clicks, chasing the cursor, moving off
//! to a corner, talking or chiming unasked), each of which can be turned off.
//! These are the user's choices, stored with the settings.

use serde::{Deserialize, Serialize};

/// Follow Windows' "reduce motion" setting, or override it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MotionPref {
    #[default]
    System,
    Reduced,
    Full,
}

impl MotionPref {
    pub fn key(self) -> &'static str {
        match self {
            MotionPref::System => "system",
            MotionPref::Reduced => "reduced",
            MotionPref::Full => "full",
        }
    }

    pub fn from_key(k: &str) -> Option<Self> {
        [MotionPref::System, MotionPref::Reduced, MotionPref::Full].into_iter().find(|m| m.key() == k)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Comfort {
    /// Quiet while Windows says the user is busy (fullscreen, presentation).
    pub quiet_when_busy: bool,
    /// Quiet from this hour until that hour (local time), e.g. 22 → 7.
    pub quiet_hours: Option<(u8, u8)>,
    /// Hide the pet completely while quiet, instead of only going silent.
    pub hide_when_quiet: bool,
    pub motion: MotionPref,
    /// Remind to take a break after this many minutes of continuous use (0 = off).
    pub break_mins: u16,
    /// Remind once a day after this many hours of screen time (0 = off).
    pub daily_hours: u8,
    /// Keep the pet above other windows (else it sits behind them, on the desktop).
    pub on_top: bool,
    /// Clicks pass through the pet to what's under it; hold Ctrl to touch it.
    pub click_through: bool,
    /// It comes over to the cursor when it wants you.
    pub follow_cursor: bool,
    /// While you work in another app, it moves off to a corner.
    pub corner_when_working: bool,
    /// It starts conversations and says what it thinks and feels on its own.
    pub talk_on_own: bool,
    /// A chime when it calls for food, play or company.
    pub call_chime: bool,
}

impl Default for Comfort {
    fn default() -> Self {
        Self {
            quiet_when_busy: true,
            quiet_hours: None,
            hide_when_quiet: false,
            motion: MotionPref::System,
            break_mins: 50,
            daily_hours: 6,
            on_top: true,
            click_through: false,
            follow_cursor: true,
            corner_when_working: true,
            talk_on_own: true,
            call_chime: true,
        }
    }
}

impl Comfort {
    /// Should the pet keep quiet right now?
    pub fn quiet(&self, busy: bool, hour: u8) -> bool {
        (self.quiet_when_busy && busy) || self.quiet_hours.is_some_and(|(from, to)| in_hours(hour, from, to))
    }

    pub fn reduced_motion(&self, system_prefers_reduced: bool) -> bool {
        match self.motion {
            MotionPref::System => system_prefers_reduced,
            MotionPref::Reduced => true,
            MotionPref::Full => false,
        }
    }
}

/// `hour` falls in [from, to), wrapping past midnight (22 → 7).
fn in_hours(hour: u8, from: u8, to: u8) -> bool {
    match from.cmp(&to) {
        std::cmp::Ordering::Less => (from..to).contains(&hour),
        std::cmp::Ordering::Greater => hour >= from || hour < to,
        std::cmp::Ordering::Equal => false,
    }
}

/// "22-7" ↔ Some((22, 7)); anything else is "off".
pub fn parse_hours(s: &str) -> Option<(u8, u8)> {
    let (a, b) = s.split_once('-')?;
    let (a, b) = (a.trim().parse::<u8>().ok()?, b.trim().parse::<u8>().ok()?);
    (a < 24 && b < 24 && a != b).then_some((a, b))
}

pub fn format_hours(h: Option<(u8, u8)>) -> String {
    h.map_or_else(|| "off".into(), |(a, b)| format!("{a}-{b}"))
}

/// No input for this long counts as a break.
pub const BREAK_SECS: f64 = 300.0;
/// No input for this long and the time no longer counts as screen time.
const AWAY_SECS: f64 = 120.0;

/// What the pet should remind you of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestNudge {
    /// A long stretch without a break.
    Break,
    /// A lot of screen time today.
    Daily,
}

/// Counts continuous use and today's screen time from how long the user has
/// been idle (the only input: no apps, no keys).
#[derive(Debug, Default)]
pub struct RestTimer {
    /// Seconds of use since the last break.
    session: f64,
    /// Local day `today` belongs to.
    day: i64,
    /// Seconds of screen time today.
    today: f64,
    daily_told: bool,
}

impl RestTimer {
    /// Advances by `dt` seconds. A reminder due during quiet time waits
    /// until it's over.
    pub fn update(&mut self, dt: f64, idle_secs: f64, local_day: i64, c: &Comfort, quiet: bool) -> Option<RestNudge> {
        if local_day != self.day {
            *self = RestTimer { day: local_day, ..RestTimer::default() };
        }
        if idle_secs >= BREAK_SECS {
            self.session = 0.0;
            return None;
        }
        if idle_secs < AWAY_SECS {
            self.session += dt;
            self.today += dt;
        }
        if quiet {
            return None;
        }
        if c.daily_hours > 0 && !self.daily_told && self.today >= f64::from(c.daily_hours) * 3600.0 {
            self.daily_told = true;
            self.session = 0.0;
            return Some(RestNudge::Daily);
        }
        if c.break_mins > 0 && self.session >= f64::from(c.break_mins) * 60.0 {
            // Reminds again after another stretch if no break is taken.
            self.session = 0.0;
            return Some(RestNudge::Break);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs `secs` of use with the given idle time, a minute at a time.
    fn run(r: &mut RestTimer, c: &Comfort, secs: u32, idle: f64, quiet: bool) -> Vec<RestNudge> {
        (0..secs / 60).filter_map(|_| r.update(60.0, idle, 1, c, quiet)).collect()
    }

    #[test]
    fn fifty_minutes_without_a_break_brings_a_reminder() {
        let c = Comfort::default();
        let mut r = RestTimer::default();
        assert!(run(&mut r, &c, 49 * 60, 0.0, false).is_empty());
        assert_eq!(run(&mut r, &c, 60, 0.0, false), vec![RestNudge::Break]);
        // A five-minute break starts the count again.
        assert!(run(&mut r, &c, 40 * 60, 0.0, false).is_empty());
        r.update(1.0, BREAK_SECS, 1, &c, false);
        assert!(run(&mut r, &c, 45 * 60, 0.0, false).is_empty(), "the break reset the stretch");
    }

    #[test]
    fn a_long_day_brings_one_daily_reminder_and_quiet_holds_them() {
        let c = Comfort { break_mins: 0, daily_hours: 2, ..Comfort::default() };
        let mut r = RestTimer::default();
        // Quiet (a presentation): the reminder waits.
        assert!(run(&mut r, &c, 3 * 3600, 0.0, true).is_empty());
        assert_eq!(run(&mut r, &c, 60, 0.0, false), vec![RestNudge::Daily]);
        assert!(run(&mut r, &c, 3 * 3600, 0.0, false).is_empty(), "once a day");
        // Idle time isn't screen time.
        let mut idle = RestTimer::default();
        assert!(run(&mut idle, &c, 4 * 3600, AWAY_SECS + 1.0, false).is_empty());
    }

    #[test]
    fn quiet_hours_wrap_past_midnight() {
        let c = Comfort { quiet_hours: Some((22, 7)), quiet_when_busy: false, ..Comfort::default() };
        assert!(c.quiet(false, 23) && c.quiet(false, 0) && c.quiet(false, 6));
        assert!(!c.quiet(false, 7) && !c.quiet(false, 12) && !c.quiet(false, 21));
        let day = Comfort { quiet_hours: Some((9, 17)), quiet_when_busy: false, ..Comfort::default() };
        assert!(day.quiet(false, 9) && !day.quiet(false, 17));
    }

    #[test]
    fn busy_means_quiet_only_if_the_user_wants_it() {
        assert!(Comfort::default().quiet(true, 12), "on by default: fullscreen games and presentations");
        let off = Comfort { quiet_when_busy: false, ..Comfort::default() };
        assert!(!off.quiet(true, 12));
    }

    #[test]
    fn motion_follows_windows_unless_overridden() {
        let mut c = Comfort::default();
        assert!(c.reduced_motion(true) && !c.reduced_motion(false));
        c.motion = MotionPref::Reduced;
        assert!(c.reduced_motion(false));
        c.motion = MotionPref::Full;
        assert!(!c.reduced_motion(true));
    }

    #[test]
    fn hours_round_trip_and_reject_nonsense() {
        assert_eq!(parse_hours("22-7"), Some((22, 7)));
        assert_eq!(parse_hours(&format_hours(Some((23, 6)))), Some((23, 6)));
        assert_eq!(parse_hours("off"), None);
        assert_eq!(parse_hours("25-3"), None);
        assert_eq!(parse_hours("5-5"), None);
    }
}
