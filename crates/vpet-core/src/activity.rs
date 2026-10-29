//! Turns foreground-app samples into structured activity sessions.
//!
//! Privacy: only the executable name of *recognised* apps is kept. Window
//! titles and screen contents are never read. Unknown apps become "other".

use serde::{Deserialize, Serialize};

/// Samples further apart than this end the current session.
const MAX_SAMPLE_GAP_MS: i64 = 30_000;
/// No keyboard/mouse input for this long means the user stepped away.
pub const IDLE_CUTOFF_SECS: f64 = 120.0;
/// Shorter sessions are noise (alt-tabbing through apps).
pub const MIN_SESSION_SECS: f64 = 60.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppCategory {
    Programming,
    Browsing,
    Video,
    Music,
    Gaming,
    Chat,
    Office,
    Design,
    Reading,
    Other,
}

impl AppCategory {
    pub const ALL: [AppCategory; 10] = [
        AppCategory::Programming,
        AppCategory::Browsing,
        AppCategory::Video,
        AppCategory::Music,
        AppCategory::Gaming,
        AppCategory::Chat,
        AppCategory::Office,
        AppCategory::Design,
        AppCategory::Reading,
        AppCategory::Other,
    ];

    pub fn key(self) -> &'static str {
        match self {
            AppCategory::Programming => "programming",
            AppCategory::Browsing => "browsing",
            AppCategory::Video => "video",
            AppCategory::Music => "music",
            AppCategory::Gaming => "gaming",
            AppCategory::Chat => "chat",
            AppCategory::Office => "office",
            AppCategory::Design => "design",
            AppCategory::Reading => "reading",
            AppCategory::Other => "other",
        }
    }

    pub fn from_key(key: &str) -> Option<AppCategory> {
        AppCategory::ALL.into_iter().find(|c| c.key() == key)
    }

    /// The word the pet picks up from watching this activity.
    pub fn word(self) -> Option<&'static str> {
        match self {
            AppCategory::Programming => Some("code"),
            AppCategory::Browsing => Some("web"),
            AppCategory::Video => Some("movie"),
            AppCategory::Music => Some("music"),
            AppCategory::Gaming => Some("game"),
            AppCategory::Chat => Some("chat"),
            AppCategory::Office => Some("work"),
            AppCategory::Design => Some("draw"),
            AppCategory::Reading => Some("book"),
            AppCategory::Other => None,
        }
    }

    /// "You seem to like ___"
    pub fn activity_phrase(self) -> &'static str {
        match self {
            AppCategory::Programming => "writing code",
            AppCategory::Browsing => "browsing the web",
            AppCategory::Video => "watching videos",
            AppCategory::Music => "listening to music",
            AppCategory::Gaming => "playing games",
            AppCategory::Chat => "chatting with people",
            AppCategory::Office => "doing office work",
            AppCategory::Design => "making art",
            AppCategory::Reading => "reading",
            AppCategory::Other => "using the computer",
        }
    }

    /// Passive activities where no input doesn't mean the user left.
    fn passive(self) -> bool {
        matches!(self, AppCategory::Video | AppCategory::Music | AppCategory::Gaming)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Classification {
    pub category: AppCategory,
    pub confidence: f64,
}

const TABLE: &[(AppCategory, &[&str])] = &[
    (
        AppCategory::Programming,
        &[
            "code", "code - insiders", "cursor", "devenv", "idea64", "pycharm64", "clion64", "rider64",
            "webstorm64", "goland64", "rustrover64", "studio64", "sublime_text", "notepad++", "zed",
            "windowsterminal", "wt", "powershell", "pwsh", "cmd", "alacritty", "wezterm-gui",
            "nvim-qt", "windsurf", "antigravity", "git-bash", "mintty",
        ],
    ),
    (AppCategory::Browsing, &["chrome", "msedge", "firefox", "brave", "opera", "vivaldi", "arc", "zen"]),
    (
        AppCategory::Video,
        &["vlc", "mpc-hc64", "mpc-hc", "mpc-be64", "potplayer64", "potplayermini64", "mpv", "wmplayer", "video.ui", "stremio", "plex"],
    ),
    (AppCategory::Music, &["spotify", "music.ui", "foobar2000", "aimp", "itunes", "tidal", "deezer"]),
    (
        AppCategory::Gaming,
        &["steam", "steamwebhelper", "epicgameslauncher", "riotclientservices", "leagueclient", "battle.net", "galaxyclient", "eadesktop", "ubisoftconnect", "minecraft", "javaw"],
    ),
    (
        AppCategory::Chat,
        &["discord", "slack", "teams", "ms-teams", "telegram", "whatsapp", "signal", "zoom", "skype", "line", "viber"],
    ),
    (
        AppCategory::Office,
        &["winword", "excel", "powerpnt", "outlook", "olk", "onenote", "notion", "obsidian", "soffice.bin", "wps", "thunderbird"],
    ),
    (
        AppCategory::Design,
        &["photoshop", "illustrator", "afterfx", "premiere pro", "figma", "blender", "krita", "aseprite", "gimp-2.10", "gimp", "inkscape", "clipstudiopaint", "davinci resolve", "resolve"],
    ),
    (AppCategory::Reading, &["sumatrapdf", "acrobat", "acrord32", "kindle", "calibre", "foxitpdfreader"]),
];

/// Shell and system processes the pet ignores entirely.
const IGNORED: &[&str] = &[
    "vpet", "explorer", "searchhost", "searchapp", "shellexperiencehost", "startmenuexperiencehost",
    "lockapp", "applicationframehost", "taskmgr", "systemsettings", "textinputhost", "shellhost",
];

/// Lower-cased file stem, e.g. `C:\...\Code.exe` -> `code`.
pub fn normalize_exe(exe: &str) -> String {
    let name = exe.rsplit(['\\', '/']).next().unwrap_or(exe).to_lowercase();
    name.strip_suffix(".exe").map(str::to_string).unwrap_or(name)
}

/// Classifies a foreground app. `None` = ignore (shell, system, the pet itself).
pub fn classify(exe: &str, fullscreen: bool) -> Option<Classification> {
    let name = normalize_exe(exe);
    if name.is_empty() || IGNORED.contains(&name.as_str()) {
        return None;
    }
    for (category, names) in TABLE {
        if names.contains(&name.as_str()) {
            return Some(Classification {
                category: *category,
                confidence: 0.9,
            });
        }
    }
    // An unknown fullscreen app is probably a game; say so only tentatively.
    Some(if fullscreen {
        Classification {
            category: AppCategory::Gaming,
            confidence: 0.55,
        }
    } else {
        Classification {
            category: AppCategory::Other,
            confidence: 0.3,
        }
    })
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActivitySession {
    /// Executable stem for recognised apps; "unknown" otherwise.
    pub app: String,
    pub category: AppCategory,
    pub confidence: f64,
    pub started_ms: i64,
    pub duration_secs: f64,
    pub local_hour: u8,
    pub local_day: i64,
}

#[derive(Clone, Debug, PartialEq)]
struct Open {
    app: String,
    class: Classification,
    started_ms: i64,
    last_ms: i64,
    local_hour: u8,
    local_day: i64,
}

/// What the host saw in the foreground at one moment.
#[derive(Clone, Debug, PartialEq)]
pub struct ForegroundSample {
    pub exe: String,
    pub fullscreen: bool,
}

#[derive(Debug, Default)]
pub struct ActivityTracker {
    open: Option<Open>,
}

impl ActivityTracker {
    /// Feeds one sample. Returns a finished session when the user switched
    /// away, went idle, or sampling paused.
    pub fn observe(
        &mut self,
        now_ms: i64,
        sample: Option<&ForegroundSample>,
        idle_secs: f64,
        local_hour: u8,
        local_day: i64,
    ) -> Option<ActivitySession> {
        let current = sample.and_then(|s| {
            let class = classify(&s.exe, s.fullscreen)?;
            let active = idle_secs < IDLE_CUTOFF_SECS || class.category.passive();
            let app = if class.category == AppCategory::Other {
                "unknown".to_string()
            } else {
                normalize_exe(&s.exe)
            };
            active.then_some((app, class))
        });

        let continues = match (&self.open, &current) {
            (Some(o), Some((app, _))) => o.app == *app && now_ms - o.last_ms <= MAX_SAMPLE_GAP_MS,
            _ => false,
        };
        if continues {
            if let Some(o) = self.open.as_mut() {
                o.last_ms = now_ms;
            }
            return None;
        }

        let finished = self.open.take().and_then(|o| {
            let duration_secs = (o.last_ms - o.started_ms) as f64 / 1000.0;
            (duration_secs >= MIN_SESSION_SECS).then_some(ActivitySession {
                app: o.app,
                category: o.class.category,
                confidence: o.class.confidence,
                started_ms: o.started_ms,
                duration_secs,
                local_hour: o.local_hour,
                local_day: o.local_day,
            })
        });
        if let Some((app, class)) = current {
            self.open = Some(Open {
                app,
                class,
                started_ms: now_ms,
                last_ms: now_ms,
                local_hour,
                local_day,
            });
        }
        finished
    }

    /// Probabilistic guess at what the user is doing right now (spec §12).
    /// Confidence grows with how long the activity has lasted.
    pub fn likely_activity(&self) -> Option<(AppCategory, f64)> {
        let o = self.open.as_ref()?;
        let minutes = (o.last_ms - o.started_ms) as f64 / 60_000.0;
        let settle = (minutes / 10.0).min(1.0);
        Some((o.class.category, o.class.confidence * (0.5 + 0.5 * settle)))
    }

    pub fn current_minutes(&self) -> f64 {
        self.open
            .as_ref()
            .map(|o| (o.last_ms - o.started_ms) as f64 / 60_000.0)
            .unwrap_or(0.0)
    }

    pub fn clear(&mut self) {
        self.open = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(exe: &str) -> ForegroundSample {
        ForegroundSample {
            exe: exe.into(),
            fullscreen: false,
        }
    }

    #[test]
    fn classifies_known_apps_and_ignores_the_shell() {
        let c = classify(r"C:\Users\x\AppData\Local\Programs\Microsoft VS Code\Code.exe", false).unwrap();
        assert_eq!(c.category, AppCategory::Programming);
        assert!(classify("explorer.exe", false).is_none());
        assert!(classify("vpet.exe", false).is_none());
    }

    #[test]
    fn unknown_fullscreen_app_is_only_probably_a_game() {
        let c = classify("SomeGame.exe", true).unwrap();
        assert_eq!(c.category, AppCategory::Gaming);
        assert!(c.confidence < 0.6);
        assert_eq!(classify("SomeTool.exe", false).unwrap().category, AppCategory::Other);
    }

    #[test]
    fn unknown_apps_are_not_named() {
        let mut t = ActivityTracker::default();
        t.observe(0, Some(&sample("SecretThing.exe")), 0.0, 10, 1);
        for i in 1..=20 {
            t.observe(i * 5_000, Some(&sample("SecretThing.exe")), 0.0, 10, 1);
        }
        let s = t.observe(105_000, None, 0.0, 10, 1).unwrap();
        assert_eq!(s.app, "unknown");
    }

    #[test]
    fn switching_apps_closes_the_session() {
        let mut t = ActivityTracker::default();
        for i in 0..=24 {
            assert!(t.observe(i * 5_000, Some(&sample("code.exe")), 1.0, 9, 1).is_none());
        }
        let s = t.observe(125_000, Some(&sample("chrome.exe")), 1.0, 9, 1).unwrap();
        assert_eq!(s.category, AppCategory::Programming);
        assert_eq!(s.duration_secs, 120.0);
        assert_eq!(s.app, "code");
    }

    #[test]
    fn brief_sessions_are_discarded() {
        let mut t = ActivityTracker::default();
        t.observe(0, Some(&sample("code.exe")), 0.0, 9, 1);
        t.observe(5_000, Some(&sample("code.exe")), 0.0, 9, 1);
        assert!(t.observe(10_000, Some(&sample("chrome.exe")), 0.0, 9, 1).is_none());
    }

    #[test]
    fn going_idle_ends_active_work_but_not_watching_video() {
        let mut t = ActivityTracker::default();
        for i in 0..=24 {
            t.observe(i * 5_000, Some(&sample("code.exe")), 0.0, 9, 1);
        }
        assert!(t.observe(125_000, Some(&sample("code.exe")), 500.0, 9, 1).is_some());

        let mut v = ActivityTracker::default();
        for i in 0..=24 {
            assert!(v.observe(i * 5_000, Some(&sample("vlc.exe")), 500.0, 21, 1).is_none());
        }
        assert!(v.likely_activity().is_some());
    }

    #[test]
    fn intent_confidence_grows_with_time() {
        let mut t = ActivityTracker::default();
        t.observe(0, Some(&sample("code.exe")), 0.0, 9, 1);
        t.observe(5_000, Some(&sample("code.exe")), 0.0, 9, 1);
        let early = t.likely_activity().unwrap().1;
        for i in 2..200 {
            t.observe(i * 5_000, Some(&sample("code.exe")), 0.0, 9, 1);
        }
        let late = t.likely_activity().unwrap().1;
        assert!(late > early && late <= 0.9);
    }
}
