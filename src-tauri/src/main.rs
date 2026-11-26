#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(error) = daftar_app::run() {
        eprintln!("Daftar failed to start: {error}");
        std::process::exit(1);
    }
}
