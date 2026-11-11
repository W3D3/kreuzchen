use crate::{
    Paths,
    definitions::{self, DefinitionEntry, OpenedDefinition},
    store::{self, SavedRecord, SavedSummary},
};
use kreuzchen_engine::{Grading, Input};
use serde::Serialize;
use std::path::Path;
use tauri::State;
use tauri_plugin_opener::OpenerExt;

#[derive(Serialize)]
pub struct Graded {
    pub grading: Grading,
    pub text: String,
}

#[tauri::command]
pub fn list_definitions(paths: State<Paths>) -> Vec<DefinitionEntry> {
    definitions::scan(&paths.definitions)
}

#[tauri::command]
pub fn get_definition(paths: State<Paths>, file: String) -> Result<OpenedDefinition, String> {
    let (q, hash) = definitions::open(&paths.definitions, &file)?;
    Ok(OpenedDefinition {
        file,
        hash,
        definition: q.def,
    })
}

#[tauri::command]
pub fn grade(
    paths: State<Paths>,
    file: String,
    hash: String,
    input: Input,
) -> Result<Graded, String> {
    let (q, current) = definitions::open(&paths.definitions, &file)?;
    definitions::check_hash(&current, &hash)?;
    let grading = q.grade(&input).map_err(|e| e.to_string())?;
    let text = q.plain_text(&input, &grading);
    Ok(Graded { grading, text })
}

#[tauri::command]
pub fn data_dir(paths: State<Paths>) -> String {
    paths.definitions.to_string_lossy().into_owned()
}

#[tauri::command]
pub fn open_definitions_folder(app: tauri::AppHandle, paths: State<Paths>) -> Result<(), String> {
    app.opener()
        .open_path(paths.definitions.to_string_lossy(), None::<&str>)
        .map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct OpenedResult {
    pub record: SavedRecord,
    pub current: Option<DefinitionEntry>,
    pub outdated: bool,
}

#[tauri::command]
pub fn save_result(paths: State<Paths>, record: SavedRecord) -> Result<String, String> {
    store::save(&paths.results, &record)
}

#[tauri::command]
pub fn list_results(paths: State<Paths>) -> Vec<SavedSummary> {
    store::list(&paths.results)
}

#[tauri::command]
pub fn open_result(paths: State<Paths>, name: String) -> Result<OpenedResult, String> {
    let record = store::load(&paths.results, &name)?;
    let current = definitions::scan(&paths.definitions).into_iter().find(|e| {
        e.summary
            .as_ref()
            .is_some_and(|s| s.id == record.questionnaire.id)
    });
    let outdated = current
        .as_ref()
        .is_some_and(|c| c.hash != record.questionnaire.hash);
    Ok(OpenedResult {
        record,
        current,
        outdated,
    })
}

#[tauri::command]
pub fn delete_result(paths: State<Paths>, name: String) -> Result<(), String> {
    store::delete(&paths.results, &name)
}

#[tauri::command]
pub fn export_result(paths: State<Paths>, name: String, dest: String) -> Result<(), String> {
    store::export(&paths.results, &name, Path::new(&dest))
}

#[tauri::command]
pub fn import_result(paths: State<Paths>, src: String) -> Result<String, String> {
    store::import(&paths.results, Path::new(&src))
}
