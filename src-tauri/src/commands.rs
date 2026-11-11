use crate::{
    Paths,
    definitions::{self, DefinitionEntry, OpenedDefinition},
};
use kreuzchen_engine::{Grading, Input};
use serde::Serialize;
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
