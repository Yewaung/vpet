//! Keeping the pet's thinking cool: whether the PC runs on battery, and
//! asking Windows to run the app gently (below-normal priority, and its
//! "efficiency mode", EcoQoS), so a thought only uses spare CPU and runs at
//! lower clocks instead of heating the machine and pushing other apps aside.

/// The PC is running on its battery (not plugged in).
#[cfg(windows)]
pub fn on_battery() -> bool {
    use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    // SAFETY: fills one zeroed struct.
    unsafe {
        let mut s: SYSTEM_POWER_STATUS = std::mem::zeroed();
        // ACLineStatus: 0 on battery, 1 plugged in, 255 unknown (e.g. a desktop).
        GetSystemPowerStatus(&mut s) != 0 && s.ACLineStatus == 0
    }
}

#[cfg(not(windows))]
pub fn on_battery() -> bool {
    false
}

/// Below-normal priority and EcoQoS for the whole app. Its windows are drawn
/// by WebView2 in their own processes, so this slows only the thinking (and
/// the pet's light movement loop), never what's on screen.
#[cfg(windows)]
pub fn run_gently() {
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcess, ProcessPowerThrottling, SetPriorityClass, SetProcessInformation, BELOW_NORMAL_PRIORITY_CLASS,
        PROCESS_POWER_THROTTLING_CURRENT_VERSION, PROCESS_POWER_THROTTLING_EXECUTION_SPEED, PROCESS_POWER_THROTTLING_STATE,
    };
    // SAFETY: the current-process pseudo handle, and one struct of the size given.
    unsafe {
        let me = GetCurrentProcess();
        if SetPriorityClass(me, BELOW_NORMAL_PRIORITY_CLASS) == 0 {
            eprintln!("vpet: couldn't lower the app's priority");
        }
        let eco = PROCESS_POWER_THROTTLING_STATE {
            Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
            ControlMask: PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
            StateMask: PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
        };
        let size = std::mem::size_of::<PROCESS_POWER_THROTTLING_STATE>() as u32;
        if SetProcessInformation(me, ProcessPowerThrottling, (&eco as *const PROCESS_POWER_THROTTLING_STATE).cast(), size) == 0 {
            eprintln!("vpet: Windows efficiency mode isn't available here");
        }
    }
}

#[cfg(not(windows))]
pub fn run_gently() {}
