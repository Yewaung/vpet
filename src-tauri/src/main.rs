// No console window in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Some(code) = vpet_app_lib::cli() {
        std::process::exit(code);
    }
    vpet_app_lib::run()
}
