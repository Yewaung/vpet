//! The user's hardware-profile, comfort and update choices, stored with the
//! other settings (so Delete All Pet Data resets them too).

use serde::Serialize;
use vpet_storage::Storage;

use super::views::Frame;
use super::Runtime;
use crate::comfort::{self, Comfort, MotionPref};
use crate::hardware::{self, Hardware, Profile, Tuning};

const SETTING_PROFILE: &str = "hardware_profile";
const SETTING_QUIET_BUSY: &str = "quiet_when_busy";
const SETTING_QUIET_HOURS: &str = "quiet_hours";
const SETTING_HIDE_QUIET: &str = "hide_when_quiet";
const SETTING_MOTION: &str = "reduce_motion";
const SETTING_BREAK_MINS: &str = "rest_break_mins";
const SETTING_DAILY_HOURS: &str = "rest_daily_hours";
const SETTING_ON_TOP: &str = "on_top";
const SETTING_CLICK_THROUGH: &str = "click_through";
const SETTING_FOLLOW_CURSOR: &str = "follow_cursor";
const SETTING_CORNER: &str = "corner_when_working";
const SETTING_TALK_ON_OWN: &str = "talk_on_own";
const SETTING_CALL_CHIME: &str = "call_chime";
const SETTING_UPDATE_CHECKS: &str = "update_checks";
const SETTING_LAST_UPDATE_CHECK: &str = "last_update_check";
const AUTO: &str = "auto";

fn get(storage: &Storage, key: &str) -> Option<String> {
    storage.setting(key).ok().flatten()
}

/// `None` = Auto.
pub(super) fn load_profile(storage: &Storage) -> Option<Profile> {
    get(storage, SETTING_PROFILE).and_then(|k| Profile::from_key(&k))
}

pub(super) fn load_comfort(storage: &Storage) -> Comfort {
    let d = Comfort::default();
    Comfort {
        quiet_when_busy: get(storage, SETTING_QUIET_BUSY).map_or(d.quiet_when_busy, |v| v == "on"),
        quiet_hours: get(storage, SETTING_QUIET_HOURS).and_then(|v| comfort::parse_hours(&v)),
        hide_when_quiet: get(storage, SETTING_HIDE_QUIET).map_or(d.hide_when_quiet, |v| v == "on"),
        motion: get(storage, SETTING_MOTION).and_then(|v| MotionPref::from_key(&v)).unwrap_or(d.motion),
        break_mins: get(storage, SETTING_BREAK_MINS).and_then(|v| v.parse().ok()).unwrap_or(d.break_mins).min(240),
        daily_hours: get(storage, SETTING_DAILY_HOURS).and_then(|v| v.parse().ok()).unwrap_or(d.daily_hours).min(24),
        on_top: get(storage, SETTING_ON_TOP).map_or(d.on_top, |v| v == "on"),
        click_through: get(storage, SETTING_CLICK_THROUGH).map_or(d.click_through, |v| v == "on"),
        follow_cursor: get(storage, SETTING_FOLLOW_CURSOR).map_or(d.follow_cursor, |v| v == "on"),
        corner_when_working: get(storage, SETTING_CORNER).map_or(d.corner_when_working, |v| v == "on"),
        talk_on_own: get(storage, SETTING_TALK_ON_OWN).map_or(d.talk_on_own, |v| v == "on"),
        call_chime: get(storage, SETTING_CALL_CHIME).map_or(d.call_chime, |v| v == "on"),
    }
}

/// One profile as the Settings window describes it.
#[derive(Serialize)]
pub struct ProfileView {
    pub profile: Profile,
    pub tuning: Tuning,
}

/// This PC, the chosen and recommended profiles, and what each would do.
#[derive(Serialize)]
pub struct HardwareView {
    pub hardware: Hardware,
    /// `None` = Auto.
    pub choice: Option<Profile>,
    pub active: Profile,
    pub recommended: Profile,
    pub profiles: Vec<ProfileView>,
}

fn on_off(on: bool) -> &'static str {
    if on {
        "on"
    } else {
        "off"
    }
}

impl Runtime {
    pub fn tuning(&self) -> Tuning {
        self.tuning
    }

    fn active_profile(&self) -> Profile {
        self.profile_choice.unwrap_or_else(|| self.hardware.recommend())
    }

    pub fn hardware_view(&self) -> HardwareView {
        HardwareView {
            hardware: self.hardware.clone(),
            choice: self.profile_choice,
            active: self.active_profile(),
            recommended: self.hardware.recommend(),
            profiles: Profile::ALL.into_iter().map(|profile| ProfileView { profile, tuning: hardware::tuning(profile, &self.hardware) }).collect(),
        }
    }

    pub(super) fn set_profile_choice_in_memory(&mut self, choice: Option<Profile>) {
        self.profile_choice = choice;
        self.tuning = hardware::tuning(self.active_profile(), &self.hardware);
    }

    /// Picks a profile (`None` = Auto) and returns what it sets.
    pub fn set_profile_choice(&mut self, choice: Option<Profile>) -> Result<Tuning, String> {
        if let Some(s) = &self.storage {
            s.set_setting(SETTING_PROFILE, choice.map_or(AUTO, Profile::key)).map_err(|e| e.to_string())?;
        }
        self.set_profile_choice_in_memory(choice);
        Ok(self.tuning)
    }

    pub fn comfort(&self) -> Comfort {
        self.comfort
    }

    pub fn set_comfort(&mut self, c: Comfort) -> Result<Frame, String> {
        if let Some(s) = &self.storage {
            let store = |k: &str, v: &str| s.set_setting(k, v).map_err(|e| e.to_string());
            store(SETTING_QUIET_BUSY, on_off(c.quiet_when_busy))?;
            store(SETTING_QUIET_HOURS, &comfort::format_hours(c.quiet_hours))?;
            store(SETTING_HIDE_QUIET, on_off(c.hide_when_quiet))?;
            store(SETTING_MOTION, c.motion.key())?;
            store(SETTING_BREAK_MINS, &c.break_mins.min(240).to_string())?;
            store(SETTING_DAILY_HOURS, &c.daily_hours.min(24).to_string())?;
            store(SETTING_ON_TOP, on_off(c.on_top))?;
            store(SETTING_CLICK_THROUGH, on_off(c.click_through))?;
            store(SETTING_FOLLOW_CURSOR, on_off(c.follow_cursor))?;
            store(SETTING_CORNER, on_off(c.corner_when_working))?;
            store(SETTING_TALK_ON_OWN, on_off(c.talk_on_own))?;
            store(SETTING_CALL_CHIME, on_off(c.call_chime))?;
        }
        let c = Comfort { break_mins: c.break_mins.min(240), daily_hours: c.daily_hours.min(24), ..c };
        self.comfort = c;
        Ok(self.current_frame())
    }

    /// The pet window reports Windows' "show animations" preference.
    pub fn set_system_reduced_motion(&mut self, reduced: bool) {
        self.system_reduced_motion = reduced;
    }

    pub fn update_checks_enabled(&self) -> bool {
        self.storage.as_ref().and_then(|s| get(s, SETTING_UPDATE_CHECKS)).as_deref() == Some("on")
    }

    pub fn set_update_checks(&self, on: bool) -> Result<(), String> {
        match &self.storage {
            Some(s) => s.set_setting(SETTING_UPDATE_CHECKS, on_off(on)).map_err(|e| e.to_string()),
            None => Ok(()),
        }
    }

    /// When updates were last looked for (ms), if ever.
    pub fn last_update_check_ms(&self) -> Option<i64> {
        self.storage.as_ref().and_then(|s| get(s, SETTING_LAST_UPDATE_CHECK)).and_then(|v| v.parse().ok())
    }

    pub fn mark_update_checked(&self, at_ms: i64) {
        if let Some(Err(e)) = self.storage.as_ref().map(|s| s.set_setting(SETTING_LAST_UPDATE_CHECK, &at_ms.to_string())) {
            eprintln!("vpet: could not store the update check time: {e}");
        }
    }
}
