#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let result = tauri::Builder::default().run(tauri::generate_context!());
    if let Err(error) = result {
        eprintln!("Daftar Tauri probe failed: {error}");
        std::process::exit(1);
    }
}
