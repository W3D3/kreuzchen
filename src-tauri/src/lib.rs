use std::{
    fs,
    path::{Path, PathBuf},
};
mod commands;
mod definitions;
mod editor;
mod store;

use tauri::Manager;

pub struct Paths {
    pub definitions: PathBuf,
    pub results: PathBuf,
    pub registry: PathBuf,
}

impl Paths {
    /// The registry folder for locked entries, else the local definitions folder.
    pub fn dir(&self, locked: bool) -> &Path {
        if locked {
            &self.registry
        } else {
            &self.definitions
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            commands::list_definitions,
            commands::get_definition,
            commands::grade,
            commands::data_dir,
            commands::open_definitions_folder,
            commands::save_result,
            commands::list_results,
            commands::open_result,
            commands::delete_result,
            commands::export_result,
            commands::import_result,
            commands::validate_definition,
            commands::definition_to_yaml,
            commands::yaml_to_definition,
            commands::grade_draft,
            commands::read_definition_source,
            commands::save_definition,
            commands::delete_definition,
            commands::duplicate_definition,
        ])
        .setup(|app| {
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;
            let base = app.path().app_data_dir()?;
            let paths = Paths {
                definitions: base.join("definitionen"),
                results: base.join("auswertungen"),
                registry: base.join("registry"),
            };
            fs::create_dir_all(&paths.definitions)?;
            fs::create_dir_all(&paths.results)?;
            fs::create_dir_all(&paths.registry)?;
            app.manage(paths);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
