#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Tauri command registration and hardware-core wiring are added in P1-01/P1-02.
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running Shookshmam PC");
}
