//! Small facts about the desktop, read from Windows itself: whether the
//! user is busy (a fullscreen app or game, or presentation mode — the same
//! signal Windows uses to hold back notifications), whether they're working
//! in another app, and where an auto-hiding taskbar sits. None of them
//! reveals which app is open.

use crate::desktop::{Bounds, Edge};

#[cfg(windows)]
pub fn user_busy() -> bool {
    use windows_sys::Win32::UI::Shell::{SHQueryUserNotificationState, QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN};
    let mut state = 0;
    // SAFETY: writes one enum value into `state`.
    let ok = unsafe { SHQueryUserNotificationState(&mut state) } >= 0;
    ok && matches!(state, QUNS_BUSY | QUNS_RUNNING_D3D_FULL_SCREEN | QUNS_PRESENTATION_MODE)
}

/// The auto-hiding taskbar's edge and rectangle, if the taskbar auto-hides.
#[cfg(windows)]
pub fn autohide_taskbar() -> Option<(Edge, Bounds)> {
    use windows_sys::Win32::UI::Shell::{SHAppBarMessage, ABE_BOTTOM, ABE_LEFT, ABE_RIGHT, ABE_TOP, ABM_GETSTATE, ABM_GETTASKBARPOS, ABS_AUTOHIDE, APPBARDATA};
    // SAFETY: APPBARDATA is zeroed with its size set, as the API requires.
    unsafe {
        let mut data: APPBARDATA = std::mem::zeroed();
        data.cbSize = std::mem::size_of::<APPBARDATA>() as u32;
        if SHAppBarMessage(ABM_GETSTATE, &mut data) as u32 & ABS_AUTOHIDE == 0 {
            return None;
        }
        if SHAppBarMessage(ABM_GETTASKBARPOS, &mut data) == 0 {
            return None;
        }
        let edge = match data.uEdge {
            ABE_LEFT => Edge::Left,
            ABE_TOP => Edge::Top,
            ABE_RIGHT => Edge::Right,
            ABE_BOTTOM => Edge::Bottom,
            _ => return None,
        };
        let r = data.rc;
        Some((edge, Bounds { left: r.left as f64, top: r.top as f64, right: r.right as f64, bottom: r.bottom as f64 }))
    }
}

/// True while another app's window has focus: not the desktop, the taskbar
/// or the pet's own windows. Only window classes are compared; which app it
/// is never leaves this function.
#[cfg(windows)]
pub fn other_app_in_focus() -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetClassNameW, GetForegroundWindow, GetWindowThreadProcessId};
    // SAFETY: plain queries on the foreground window handle; the class name buffer is sized.
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return false;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == std::process::id() {
            return false;
        }
        let mut buf = [0u16; 64];
        let n = GetClassNameW(hwnd, buf.as_mut_ptr(), buf.len() as i32).max(0) as usize;
        let class = String::from_utf16_lossy(&buf[..n]);
        !matches!(class.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd")
    }
}

#[cfg(not(windows))]
pub fn other_app_in_focus() -> bool {
    false
}

#[cfg(not(windows))]
pub fn user_busy() -> bool {
    false
}

#[cfg(not(windows))]
pub fn autohide_taskbar() -> Option<(Edge, Bounds)> {
    None
}
