use std::{fs, path::PathBuf};
use tauri::Manager;

pub struct Paths {
    pub definitions: PathBuf,
    pub results: PathBuf,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let base = app.path().app_data_dir()?;
            let paths = Paths {
                definitions: base.join("definitionen"),
                results: base.join("auswertungen"),
            };
            fs::create_dir_all(&paths.definitions)?;
            fs::create_dir_all(&paths.results)?;
            app.manage(paths);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
